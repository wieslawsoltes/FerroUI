//! `IScheduler`, the services a scheduler may offer (`ISchedulerPeriodic`, `IStopwatchProvider`,
//! `IStopwatch`) and the extension methods of `Scheduler` (`Scheduler.cs`, `Scheduler.Simple.cs`,
//! `Scheduler.Recursive.cs`, `Scheduler.Services.cs`, `Scheduler.Services.Emulation.cs`).

use crate::disposables::{CompositeDisposable, Disposable, SingleAssignmentDisposable};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// The work a scheduler runs: it receives the scheduler it runs on (which the work may schedule
/// more work on, now or later) and returns the disposable that cancels what it started
/// (`Func<IScheduler, TState, IDisposable>`; the state of upstream is what the closure captures).
pub type ScheduledAction = Box<dyn FnOnce(&Rc<dyn IScheduler>) -> Rc<dyn IDisposable>>;

/// Schedules units of work (`IScheduler`).
///
/// Deviation (DEVIATIONS.md, Reactive): absolute times are `SystemTime` where upstream has
/// `DateTimeOffset`, and relative times `Duration` where upstream has `TimeSpan`. A `TimeSpan` may
/// be negative upstream and is normalized to zero by every scheduler (`Scheduler.Normalize`); a
/// `Duration` cannot be negative.
pub trait IScheduler {
    /// The scheduler's notion of the current time.
    fn now(&self) -> SystemTime;

    /// Schedules work to run as soon as possible.
    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable>;

    /// Schedules work to run after `due_time`.
    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable>;

    /// Schedules work to run at `due_time`.
    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable>;

    /// The periodic scheduling of the scheduler, if it has its own (`GetService(typeof(ISchedulerPeriodic))`).
    fn as_periodic(&self) -> Option<&dyn ISchedulerPeriodic> {
        None
    }

    /// The stopwatch of the scheduler, if it has its own (`GetService(typeof(IStopwatchProvider))`).
    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        None
    }
}

/// Schedules periodic work (`ISchedulerPeriodic`).
pub trait ISchedulerPeriodic {
    /// Runs `action` every `period`, passing the state it returned the previous time
    /// (`SchedulePeriodic<TState>(TState, TimeSpan, Func<TState, TState>)`; the closure keeps the
    /// state).
    fn schedule_periodic(&self, period: Duration, action: Box<dyn FnMut()>) -> Rc<dyn IDisposable>;
}

/// Measures elapsed time (`IStopwatch`).
pub trait IStopwatch {
    /// The time since the stopwatch was started.
    fn elapsed(&self) -> Duration;
}

/// Starts stopwatches (`IStopwatchProvider`).
pub trait IStopwatchProvider {
    fn start_stopwatch(&self) -> Box<dyn IStopwatch>;
}

/// The static members of `Scheduler`.
pub struct Scheduler;

impl Scheduler {
    /// The current time of the system (`Scheduler.Now`).
    pub fn now() -> SystemTime {
        SystemTime::now()
    }

    /// The scheduler that runs work at once on the current thread (`Scheduler.Immediate`).
    pub fn immediate() -> Rc<dyn IScheduler> {
        super::ImmediateScheduler::instance()
    }

    /// The scheduler that queues work on the current thread (`Scheduler.CurrentThread`).
    pub fn current_thread() -> Rc<dyn IScheduler> {
        super::CurrentThreadScheduler::instance()
    }

    /// The time of `due_time` relative to now on `scheduler`, zero when it is past.
    pub(crate) fn relative_to(scheduler: &dyn IScheduler, due_time: SystemTime) -> Duration {
        due_time.duration_since(scheduler.now()).unwrap_or(Duration::ZERO)
    }
}

/// The extension methods of `Scheduler` on a scheduler (`Scheduler.Simple.cs`,
/// `Scheduler.Recursive.cs`, `Scheduler.Services.cs`).
pub trait SchedulerExt {
    /// Schedules an action to run as soon as possible (`Schedule(Action)`).
    fn schedule_action(&self, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action to run after `due_time` (`Schedule(TimeSpan, Action)`).
    fn schedule_action_after(&self, due_time: Duration, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action to run at `due_time` (`Schedule(DateTimeOffset, Action)`).
    fn schedule_action_at(&self, due_time: SystemTime, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action that can schedule itself again (`Schedule(Action<Action>)`).
    fn schedule_recursive(&self, action: impl Fn(&dyn Fn()) + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action with a state that can schedule itself again with a new state
    /// (`Schedule<TState>(TState, Action<TState, Action<TState>>)`).
    fn schedule_recursive_with<S: 'static>(&self, state: S, action: impl Fn(S, &dyn Fn(S)) + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action after `due_time` that can schedule itself again after a time it chooses
    /// (`Schedule(TimeSpan, Action<Action<TimeSpan>>)`).
    fn schedule_recursive_after(&self, due_time: Duration, action: impl Fn(&dyn Fn(Duration)) + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action with a state after `due_time` that can schedule itself again with a new
    /// state after a time it chooses (`Schedule<TState>(TState, TimeSpan, Action<TState, Action<TState, TimeSpan>>)`).
    fn schedule_recursive_after_with<S: 'static>(
        &self,
        state: S,
        due_time: Duration,
        action: impl Fn(S, &dyn Fn(S, Duration)) + 'static,
    ) -> Rc<dyn IDisposable>;

    /// Schedules an action at `due_time` that can schedule itself again at a time it chooses
    /// (`Schedule(DateTimeOffset, Action<Action<DateTimeOffset>>)`).
    fn schedule_recursive_at(&self, due_time: SystemTime, action: impl Fn(&dyn Fn(SystemTime)) + 'static) -> Rc<dyn IDisposable>;

    /// Runs `action` every `period` (`SchedulePeriodic(TimeSpan, Action)`): through the periodic
    /// scheduling of the scheduler if it has its own, else emulated with recursive scheduling.
    fn schedule_periodic_action(&self, period: Duration, action: impl FnMut() + 'static) -> Rc<dyn IDisposable>;

    /// Starts a stopwatch (`StartStopwatch`): the scheduler's own if it has one, else one that
    /// measures with the scheduler's clock.
    fn start_stopwatch(&self) -> Box<dyn IStopwatch>;
}

impl SchedulerExt for Rc<dyn IScheduler> {
    fn schedule_action(&self, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        self.schedule(Box::new(move |_| {
            action();
            Disposable::empty()
        }))
    }

    fn schedule_action_after(&self, due_time: Duration, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        self.schedule_after(
            due_time,
            Box::new(move |_| {
                action();
                Disposable::empty()
            }),
        )
    }

    fn schedule_action_at(&self, due_time: SystemTime, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        self.schedule_at(
            due_time,
            Box::new(move |_| {
                action();
                Disposable::empty()
            }),
        )
    }

    fn schedule_recursive(&self, action: impl Fn(&dyn Fn()) + 'static) -> Rc<dyn IDisposable> {
        self.schedule_recursive_with((), move |(), recurse| action(&|| recurse(())))
    }

    fn schedule_recursive_with<S: 'static>(&self, state: S, action: impl Fn(S, &dyn Fn(S)) + 'static) -> Rc<dyn IDisposable> {
        let action: Rc<dyn Fn(S, &dyn Fn(S))> = Rc::new(action);
        self.schedule(Box::new(move |scheduler| {
            let invoker = Rc::new(RecursiveInvoker { scheduler: scheduler.clone(), group: CompositeDisposable::new() });
            invoker.clone().invoke_now(state, action);
            invoker
        }))
    }

    fn schedule_recursive_after(&self, due_time: Duration, action: impl Fn(&dyn Fn(Duration)) + 'static) -> Rc<dyn IDisposable> {
        self.schedule_recursive_after_with((), due_time, move |(), recurse| action(&|time| recurse((), time)))
    }

    fn schedule_recursive_after_with<S: 'static>(
        &self,
        state: S,
        due_time: Duration,
        action: impl Fn(S, &dyn Fn(S, Duration)) + 'static,
    ) -> Rc<dyn IDisposable> {
        let action: Rc<dyn Fn(S, &dyn Fn(S, Duration))> = Rc::new(action);
        self.schedule_after(
            due_time,
            Box::new(move |scheduler| {
                let invoker = Rc::new(RecursiveInvoker { scheduler: scheduler.clone(), group: CompositeDisposable::new() });
                invoker.clone().invoke_now_after(state, action);
                invoker
            }),
        )
    }

    fn schedule_recursive_at(&self, due_time: SystemTime, action: impl Fn(&dyn Fn(SystemTime)) + 'static) -> Rc<dyn IDisposable> {
        let action: Rc<dyn Fn(&dyn Fn(SystemTime))> = Rc::new(action);
        self.schedule_at(
            due_time,
            Box::new(move |scheduler| {
                let invoker = Rc::new(RecursiveInvoker { scheduler: scheduler.clone(), group: CompositeDisposable::new() });
                invoker.clone().invoke_now_at(action);
                invoker
            }),
        )
    }

    fn schedule_periodic_action(&self, period: Duration, action: impl FnMut() + 'static) -> Rc<dyn IDisposable> {
        if let Some(periodic) = self.as_periodic() {
            return periodic.schedule_periodic(period, Box::new(action));
        }
        schedule_periodic_emulated(self, period, Box::new(action))
    }

    fn start_stopwatch(&self) -> Box<dyn IStopwatch> {
        if let Some(provider) = self.as_stopwatch_provider() {
            return provider.start_stopwatch();
        }
        Box::new(EmulatedStopwatch { scheduler: self.clone(), start: self.now() })
    }
}

/// The state of a recursive schedule (`InvokeRecBaseState` and its three forms): the group of the
/// pending recursions, each of which leaves the group when it runs.
struct RecursiveInvoker {
    scheduler: Rc<dyn IScheduler>,
    group: Rc<CompositeDisposable>,
}

impl RecursiveInvoker {
    fn invoke_now<S: 'static>(self: Rc<Self>, state: S, action: Rc<dyn Fn(S, &dyn Fn(S))>) {
        let this = self.clone();
        let recursion = action.clone();
        action(state, &move |next: S| this.clone().schedule_next(next, recursion.clone()));
    }

    fn schedule_next<S: 'static>(self: Rc<Self>, state: S, action: Rc<dyn Fn(S, &dyn Fn(S))>) {
        let slot = SingleAssignmentDisposable::new();
        self.group.add(slot.clone());
        let this = self.clone();
        let entry = slot.clone();
        let scheduled = self.scheduler.schedule(Box::new(move |_| {
            this.group.remove(&(entry as Rc<dyn IDisposable>));
            this.invoke_now(state, action);
            Disposable::empty()
        }));
        slot.set_disposable(Some(scheduled));
    }

    fn invoke_now_after<S: 'static>(self: Rc<Self>, state: S, action: Rc<dyn Fn(S, &dyn Fn(S, Duration))>) {
        let this = self.clone();
        let recursion = action.clone();
        action(state, &move |next: S, time: Duration| this.clone().schedule_next_after(next, time, recursion.clone()));
    }

    fn schedule_next_after<S: 'static>(self: Rc<Self>, state: S, time: Duration, action: Rc<dyn Fn(S, &dyn Fn(S, Duration))>) {
        let slot = SingleAssignmentDisposable::new();
        self.group.add(slot.clone());
        let this = self.clone();
        let entry = slot.clone();
        let scheduled = self.scheduler.schedule_after(
            time,
            Box::new(move |_| {
                this.group.remove(&(entry as Rc<dyn IDisposable>));
                this.invoke_now_after(state, action);
                Disposable::empty()
            }),
        );
        slot.set_disposable(Some(scheduled));
    }

    fn invoke_now_at(self: Rc<Self>, action: Rc<dyn Fn(&dyn Fn(SystemTime))>) {
        let this = self.clone();
        let recursion = action.clone();
        action(&move |time: SystemTime| this.clone().schedule_next_at(time, recursion.clone()));
    }

    fn schedule_next_at(self: Rc<Self>, time: SystemTime, action: Rc<dyn Fn(&dyn Fn(SystemTime))>) {
        let slot = SingleAssignmentDisposable::new();
        self.group.add(slot.clone());
        let this = self.clone();
        let entry = slot.clone();
        let scheduled = self.scheduler.schedule_at(
            time,
            Box::new(move |_| {
                this.group.remove(&(entry as Rc<dyn IDisposable>));
                this.invoke_now_at(action);
                Disposable::empty()
            }),
        );
        slot.set_disposable(Some(scheduled));
    }
}

impl IDisposable for RecursiveInvoker {
    fn dispose(&self) {
        self.group.dispose();
    }
}

/// A stopwatch that measures with the clock of a scheduler (`EmulatedStopwatch`).
struct EmulatedStopwatch {
    scheduler: Rc<dyn IScheduler>,
    start: SystemTime,
}

impl IStopwatch for EmulatedStopwatch {
    fn elapsed(&self) -> Duration {
        self.scheduler.now().duration_since(self.start).unwrap_or(Duration::ZERO)
    }
}

/// Periodic scheduling on a scheduler without its own (`SchedulePeriodic_` of
/// `Scheduler.Services.Emulation.cs`): each run is scheduled at the start time plus a multiple of
/// the period, so that the time the action takes does not drift the schedule; a run that is late
/// catches up without waiting.
fn schedule_periodic_emulated(scheduler: &Rc<dyn IScheduler>, period: Duration, action: Box<dyn FnMut()>) -> Rc<dyn IDisposable> {
    let action = Rc::new(RefCell::new(action));
    let stopwatch = scheduler.start_stopwatch();
    let next_due = Rc::new(Cell::new(period));
    let state = Rc::new(PeriodicState { stopwatch, next_due, period, action });
    scheduler.schedule_recursive_after(period, move |recurse| {
        (state.action.borrow_mut())();
        let due = state.next_due.get() + state.period;
        state.next_due.set(due);
        let elapsed = state.stopwatch.elapsed();
        recurse(due.saturating_sub(elapsed));
    })
}

struct PeriodicState {
    stopwatch: Box<dyn IStopwatch>,
    next_due: Rc<Cell<Duration>>,
    period: Duration,
    action: Rc<RefCell<Box<dyn FnMut()>>>,
}
