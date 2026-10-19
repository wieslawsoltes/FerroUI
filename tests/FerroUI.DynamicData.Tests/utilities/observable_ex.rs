//! `ObservableEx` (`Utilities/ObservableEx.cs`): more observable factories for the tests.

use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::Observable;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;
use std::time::Duration;

/// The factories of `ObservableEx`.
pub struct ObservableEx;

struct IntervalState {
    observer: Rc<dyn IObserver<i64>>,
    next_interval: Rc<dyn Fn() -> Duration>,
    scheduler: Rc<dyn IScheduler>,
}

/// Schedules the value `counter` after the next interval; delivering it schedules the next one.
fn schedule_next(state: Rc<IntervalState>, counter: i64) -> Rc<dyn IDisposable> {
    let due_time = (state.next_interval)();
    let handle = state.clone();
    state.scheduler.schedule_after(
        due_time,
        Box::new(move |_| {
            handle.observer.on_next(counter);
            schedule_next(handle.clone(), counter + 1)
        }),
    )
}

impl ObservableEx {
    /// The numbers 0, 1, 2, ... with an interval that `next_interval` computes again before each
    /// one, on `scheduler`, or on the default scheduler of the thread when `None`
    /// (`Interval(Func<TimeSpan> nextInterval, IScheduler? scheduler = null)`).
    pub fn interval(next_interval: impl Fn() -> Duration + 'static, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<dyn IObservable<i64>> {
        let next_interval: Rc<dyn Fn() -> Duration> = Rc::new(next_interval);
        Observable::create(move |observer: Rc<dyn IObserver<i64>>| {
            let scheduler = scheduler.clone().unwrap_or_else(SchedulerDefaults::default_scheduler);
            schedule_next(Rc::new(IntervalState { observer, next_interval: next_interval.clone(), scheduler }), 0)
        })
    }
}
