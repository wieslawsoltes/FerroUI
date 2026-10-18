//! `Timer` (`Linq/Observable/Timer.cs`).

use super::creation::IdentitySink;
use crate::concurrency::{IScheduler, SchedulerDefaults, SchedulerExt};
use crate::disposables::{SingleAssignmentDisposable, StableCompositeDisposable};
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// When a timer first fires: after a time relative to the subscription (`Relative`) or at an
/// absolute time (`Absolute`).
#[derive(Clone, Copy)]
enum Due {
    Relative(Duration),
    Absolute(SystemTime),
}

/// `Timer.Single.Relative` and `Timer.Single.Absolute`: zero and the end, at the due time.
struct TimerSingle {
    due_time: Due,
    scheduler: Rc<dyn IScheduler>,
}

impl IObservable<i64> for TimerSingle {
    fn subscribe(&self, observer: Rc<dyn IObserver<i64>>) -> Rc<dyn IDisposable> {
        let due_time = self.due_time;
        let scheduler = self.scheduler.clone();
        subscribe_sink(observer, IdentitySink::new, move |sink: &Rc<IdentitySink<i64>>| {
            let target = sink.clone();
            let invoke = move || {
                target.sink.forward_on_next(0);
                target.sink.forward_on_completed();
            };
            let upstream = match due_time {
                Due::Relative(due_time) => scheduler.schedule_action_after(due_time, invoke),
                Due::Absolute(due_time) => scheduler.schedule_action_at(due_time, invoke),
            };
            sink.sink.set_upstream(upstream);
        })
    }
}

/// `Timer.Periodic.Relative` and `Timer.Periodic.Absolute`: zero at the due time, then the
/// following integers every period.
pub(crate) struct TimerPeriodic {
    due_time: Due,
    period: Duration,
    scheduler: Rc<dyn IScheduler>,
}

impl TimerPeriodic {
    /// The periodic timer of `Interval(period, scheduler)` (`Timer_(period, period, scheduler)`).
    pub(crate) fn interval(period: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<i64>> {
        Rc::new(TimerPeriodic { due_time: Due::Relative(period), period, scheduler })
    }
}

/// Disposes a disposable if a panic unwinds while it is alive (the `catch` that disposes the
/// periodic work and rethrows upstream).
struct DisposeOnPanic(Rc<dyn IDisposable>);

impl Drop for DisposeOnPanic {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.dispose();
        }
    }
}

/// The sink of `Timer.Periodic` (`Timer.Periodic._`).
struct PeriodicSink {
    sink: Sink<i64>,
    period: Duration,
    index: Cell<i64>,
    pending_tick_count: Cell<i32>,
    periodic: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl PeriodicSink {
    fn run(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>, due_time: Due) {
        let this = self.clone();
        let upstream = match due_time {
            // The form of `Interval`: the periodic scheduling of the scheduler from the start.
            Due::Relative(due_time) if due_time == self.period => scheduler.schedule_periodic_action(self.period, move || this.tick()),
            Due::Relative(due_time) => scheduler.schedule_after(due_time, Box::new(move |inner| this.invoke_start(inner))),
            Due::Absolute(due_time) => scheduler.schedule_at(due_time, Box::new(move |inner| this.invoke_start(inner))),
        };
        self.sink.set_upstream(upstream);
    }

    fn tick(&self) {
        let count = self.index.get();
        self.index.set(count.wrapping_add(1));
        self.sink.forward_on_next(count);
    }

    /// Notifies zero and starts the periodic work for the following values. The periodic work may
    /// come due while zero is notified (when the observer takes longer than the period); the ticks
    /// it then counts are caught up by recursive scheduling (`InvokeStart`).
    fn invoke_start(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) -> Rc<dyn IDisposable> {
        // One tick is pending: the notification of zero about to be made.
        self.pending_tick_count.set(1);

        let d = SingleAssignmentDisposable::new();
        let periodic: Rc<dyn IDisposable> = d.clone();
        *self.periodic.borrow_mut() = Some(periodic.clone());
        self.index.set(1);
        let this = self.clone();
        d.set_disposable(Some(scheduler.schedule_periodic_action(self.period, move || this.tock())));

        {
            let _guard = DisposeOnPanic(periodic.clone());
            self.sink.forward_on_next(0);
        }

        let pending = self.pending_tick_count.get() - 1;
        self.pending_tick_count.set(pending);
        if pending > 0 {
            let this = self.clone();
            let c = scheduler.schedule_recursive_with(1i64, move |count, recurse| this.catch_up(count, recurse));
            return StableCompositeDisposable::create(periodic, c);
        }

        periodic
    }

    /// The periodic work: notifies the next value unless the notification of zero, or a catch-up
    /// of it, is running, in which case it counts a pending tick (`Tock`).
    fn tock(&self) {
        let pending = self.pending_tick_count.get() + 1;
        self.pending_tick_count.set(pending);
        if pending == 1 {
            let count = self.index.get();
            self.index.set(count.wrapping_add(1));
            self.sink.forward_on_next(count);
            self.pending_tick_count.set(self.pending_tick_count.get() - 1);
        }
    }

    /// Notifies a value counted while zero was notified, and schedules the next while ticks are
    /// pending (`CatchUp`).
    fn catch_up(&self, count: i64, recurse: &dyn Fn(i64)) {
        {
            let periodic = self.periodic.borrow().clone().expect("the periodic work is assigned before it is caught up");
            let _guard = DisposeOnPanic(periodic);
            self.sink.forward_on_next(count);
        }

        // At no pending tick the periodic work takes over again.
        let pending = self.pending_tick_count.get() - 1;
        self.pending_tick_count.set(pending);
        if pending > 0 {
            recurse(count.wrapping_add(1));
        }
    }
}

impl IDisposable for PeriodicSink {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

impl IObservable<i64> for TimerPeriodic {
    fn subscribe(&self, observer: Rc<dyn IObserver<i64>>) -> Rc<dyn IDisposable> {
        let due_time = self.due_time;
        let period = self.period;
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new(PeriodicSink {
                    sink: Sink::new(observer),
                    period,
                    index: Cell::new(0),
                    pending_tick_count: Cell::new(0),
                    periodic: RefCell::new(None),
                })
            },
            move |sink: &Rc<PeriodicSink>| sink.run(&scheduler, due_time),
        )
    }
}

impl crate::linq::Observable {
    /// Zero after `due_time`, then the end, on the scheduler of time-based operations
    /// (`Timer(TimeSpan)`).
    pub fn timer(due_time: Duration) -> Rc<dyn IObservable<i64>> {
        Self::timer_on(due_time, SchedulerDefaults::time_based_operations())
    }

    /// Zero at `due_time`, then the end, on the scheduler of time-based operations
    /// (`Timer(DateTimeOffset)`).
    pub fn timer_at(due_time: SystemTime) -> Rc<dyn IObservable<i64>> {
        Self::timer_at_on(due_time, SchedulerDefaults::time_based_operations())
    }

    /// Zero after `due_time`, then the following integers every `period`, on the scheduler of
    /// time-based operations (`Timer(TimeSpan, TimeSpan)`).
    pub fn timer_periodic(due_time: Duration, period: Duration) -> Rc<dyn IObservable<i64>> {
        Self::timer_periodic_on(due_time, period, SchedulerDefaults::time_based_operations())
    }

    /// Zero at `due_time`, then the following integers every `period`, on the scheduler of
    /// time-based operations (`Timer(DateTimeOffset, TimeSpan)`).
    pub fn timer_at_periodic(due_time: SystemTime, period: Duration) -> Rc<dyn IObservable<i64>> {
        Self::timer_at_periodic_on(due_time, period, SchedulerDefaults::time_based_operations())
    }

    /// Zero after `due_time`, then the end, on `scheduler` (`Timer(TimeSpan, IScheduler)`).
    pub fn timer_on(due_time: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<i64>> {
        Rc::new(TimerSingle { due_time: Due::Relative(due_time), scheduler })
    }

    /// Zero at `due_time`, then the end, on `scheduler` (`Timer(DateTimeOffset, IScheduler)`).
    pub fn timer_at_on(due_time: SystemTime, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<i64>> {
        Rc::new(TimerSingle { due_time: Due::Absolute(due_time), scheduler })
    }

    /// Zero after `due_time`, then the following integers every `period`, on `scheduler`
    /// (`Timer(TimeSpan, TimeSpan, IScheduler)`).
    pub fn timer_periodic_on(due_time: Duration, period: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<i64>> {
        Rc::new(TimerPeriodic { due_time: Due::Relative(due_time), period, scheduler })
    }

    /// Zero at `due_time`, then the following integers every `period`, on `scheduler`
    /// (`Timer(DateTimeOffset, TimeSpan, IScheduler)`).
    pub fn timer_at_periodic_on(due_time: SystemTime, period: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<i64>> {
        Rc::new(TimerPeriodic { due_time: Due::Absolute(due_time), period, scheduler })
    }
}
