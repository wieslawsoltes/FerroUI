//! `FakerExtensions` (`Utilities/FakerExtensions.cs`): items generated at intervals, as methods of
//! [`Faker`].

use crate::domain::{Faker, Randomizer};
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::IObservable;
use std::rc::Rc;
use std::time::Duration;

impl<T: 'static> Faker<T> {
    /// A new item at each random interval of `randomizer` up to `max_time`, on `scheduler` or the
    /// default scheduler of the thread (`IntervalGenerate(Randomizer, TimeSpan maxTime,
    /// IScheduler? scheduler = null)`).
    pub fn interval_generate(&self, randomizer: &Randomizer, max_time: Duration, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<dyn IObservable<T>> {
        let faker = self.clone();
        randomizer.interval(max_time, scheduler).select(move |_| faker.generate_one())
    }

    /// A new item every `period`, on `scheduler` or the default scheduler of the thread
    /// (`IntervalGenerate(TimeSpan period, IScheduler? scheduler = null)`).
    pub fn interval_generate_every(&self, period: Duration, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<dyn IObservable<T>> {
        let faker = self.clone();
        let scheduler = scheduler.unwrap_or_else(SchedulerDefaults::default_scheduler);
        Observable::interval_on(period, scheduler).select(move |_| faker.generate_one())
    }
}
