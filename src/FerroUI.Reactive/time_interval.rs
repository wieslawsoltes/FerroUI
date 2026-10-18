//! `TimeInterval<T>` and `Timestamped<T>` (`TimeInterval.cs`, `Timestamped.cs`).

use std::fmt;
use std::time::{Duration, SystemTime};

/// A value with the time since the previous value (`TimeInterval<T>`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeInterval<T> {
    /// The value (`Value`).
    pub value: T,
    /// The time since the previous value (`Interval`).
    pub interval: Duration,
}

impl<T> TimeInterval<T> {
    pub fn new(value: T, interval: Duration) -> Self {
        Self { value, interval }
    }
}

impl<T: fmt::Display> fmt::Display for TimeInterval<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{:?}", self.value, self.interval)
    }
}

/// A value with the time it was produced (`Timestamped<T>`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Timestamped<T> {
    /// The value (`Value`).
    pub value: T,
    /// The time of the value (`Timestamp`).
    pub timestamp: SystemTime,
}

impl<T> Timestamped<T> {
    pub fn new(value: T, timestamp: SystemTime) -> Self {
        Self { value, timestamp }
    }
}

impl<T: fmt::Display> fmt::Display for Timestamped<T> {
    /// `value@timestamp` (`ToString`), the time as the duration since the epoch of the system time.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let since_epoch = self.timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
        write!(f, "{}@{:?}", self.value, since_epoch)
    }
}
