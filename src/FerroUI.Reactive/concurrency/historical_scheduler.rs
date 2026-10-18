//! `HistoricalScheduler` and `HistoricalSchedulerBase` (`HistoricalScheduler.cs`).

use super::{VirtualTimeClock, VirtualTimeScheduler};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// The time of a historical scheduler: absolute times of the system and durations
/// (`HistoricalSchedulerBase : VirtualTimeSchedulerBase<DateTimeOffset, TimeSpan>`).
pub struct HistoricalTime;

impl VirtualTimeClock for HistoricalTime {
    type Absolute = SystemTime;
    type Relative = Duration;

    fn add(absolute: SystemTime, relative: Duration) -> SystemTime {
        absolute + relative
    }

    fn to_system_time(absolute: SystemTime) -> SystemTime {
        absolute
    }

    fn to_relative(duration: Duration) -> Duration {
        duration
    }

    fn subtract(absolute: SystemTime, duration: Duration) -> SystemTime {
        absolute.checked_sub(duration).unwrap_or(SystemTime::UNIX_EPOCH)
    }
}

/// A virtual-time scheduler over historical time (`HistoricalScheduler`), for replaying recorded
/// sequences at the times they happened.
pub type HistoricalScheduler = VirtualTimeScheduler<HistoricalTime>;

impl VirtualTimeScheduler<HistoricalTime> {
    /// A historical scheduler with its clock at the epoch of the system time.
    ///
    /// Deviation (DEVIATIONS.md, Reactive): upstream starts at `DateTimeOffset.MinValue`; the
    /// system time has no such minimum, and the epoch is the earliest time every platform has.
    pub fn historical() -> Rc<Self> {
        Self::new(SystemTime::UNIX_EPOCH)
    }

    /// A historical scheduler with its clock at `initial_clock`.
    pub fn historical_at(initial_clock: SystemTime) -> Rc<Self> {
        Self::new(initial_clock)
    }
}
