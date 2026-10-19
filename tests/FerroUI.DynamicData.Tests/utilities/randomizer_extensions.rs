//! `RandomizerExtensions` (`Utilities/RandomizerExtensions.cs`): random times, chances and random
//! intervals, as methods of [`Randomizer`].

use super::ObservableEx;
use crate::domain::Randomizer;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive_testing::{duration_to_ticks, ticks_to_duration};
use ferroui_system::IObservable;
use std::rc::Rc;
use std::time::Duration;

impl Randomizer {
    /// A time in `min_time..=max_time`, to the tick (`TimeSpan(TimeSpan minTime, TimeSpan
    /// maxTime)`).
    pub fn time_span_range(&self, min_time: Duration, max_time: Duration) -> Duration {
        ticks_to_duration(self.long_range(duration_to_ticks(min_time), duration_to_ticks(max_time)))
    }

    /// A time in `0..=max_time`, to the tick (`TimeSpan(TimeSpan maxTime)`).
    pub fn time_span(&self, max_time: Duration) -> Duration {
        self.time_span_range(Duration::ZERO, max_time)
    }

    /// Runs `action` on a coin flip; whether it ran (`CoinFlip`).
    pub fn coin_flip(&self, action: impl FnOnce()) -> bool {
        if self.bool() {
            action();
            return true;
        }
        false
    }

    /// Runs `action` with the probability `chance_percent`, from 0 to 1; whether it ran (`Chance`).
    pub fn chance(&self, chance_percent: f64, action: impl FnOnce()) -> bool {
        debug_assert!((0.0..=1.0).contains(&chance_percent));
        if self.double() <= chance_percent {
            action();
            return true;
        }
        false
    }

    /// The numbers 0, 1, 2, ... at random intervals in `min_time..=max_time`, on `scheduler` or
    /// the default scheduler of the thread (`Interval(TimeSpan minTime, TimeSpan maxTime,
    /// IScheduler? scheduler = null)`).
    pub fn interval_range(&self, min_time: Duration, max_time: Duration, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<dyn IObservable<i64>> {
        let randomizer = self.clone();
        ObservableEx::interval(move || randomizer.time_span_range(min_time, max_time), scheduler)
    }

    /// The numbers 0, 1, 2, ... at random intervals in `0..=max_time` (`Interval(TimeSpan
    /// maxTime, IScheduler? scheduler = null)`).
    pub fn interval(&self, max_time: Duration, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<dyn IObservable<i64>> {
        self.interval_range(Duration::ZERO, max_time, scheduler)
    }
}
