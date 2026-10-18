//! `Interval` (`QueryLanguage.Time.cs`): the periodic `Timer` whose due time is its period
//! (`Linq/Observable/Timer.cs`, `Timer.Periodic.Relative`).

use super::timer::TimerPeriodic;
use crate::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_system::IObservable;
use std::rc::Rc;
use std::time::Duration;

impl crate::linq::Observable {
    /// The integers from zero, one every `period`, on the scheduler of time-based operations
    /// (`Interval(TimeSpan)`).
    pub fn interval(period: Duration) -> Rc<dyn IObservable<i64>> {
        Self::interval_on(period, SchedulerDefaults::time_based_operations())
    }

    /// The integers from zero, one every `period`, through the periodic scheduling of `scheduler`
    /// (`Interval(TimeSpan, IScheduler)`).
    pub fn interval_on(period: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<i64>> {
        TimerPeriodic::interval(period, scheduler)
    }
}
