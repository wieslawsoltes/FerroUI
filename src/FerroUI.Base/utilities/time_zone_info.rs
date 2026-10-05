//! The local time zone hook (a minimal counterpart of .NET `System.TimeZoneInfo`).
//!
//! The base library carries no time zone data. The offset of local time from
//! UTC is supplied per thread by a provider that a platform layer (or a test)
//! installs; without one, local time is UTC.

use super::DateTime;
use crate::animation::TimeSpan;
use std::cell::RefCell;
use std::rc::Rc;

/// Gives the offset of local time from UTC at a UTC instant.
pub type LocalUtcOffsetProvider = Rc<dyn Fn(DateTime) -> TimeSpan>;

thread_local! {
    // TIMEZONE-SEAM: platform crates install the offset of the system time zone here
    // (`TimeZoneInfo::set_local_utc_offset_provider`); the default is UTC.
    static LOCAL_UTC_OFFSET_PROVIDER: RefCell<Option<LocalUtcOffsetProvider>> = const { RefCell::new(None) };
}

/// Access to the local time zone of the current thread.
pub struct TimeZoneInfo;

impl TimeZoneInfo {
    /// The largest offset from UTC, 14 hours (in either direction).
    pub const MAX_OFFSET: TimeSpan = TimeSpan::from_ticks(14 * TimeSpan::TICKS_PER_HOUR);

    /// Installs (or with `None` removes) the provider of the local UTC offset
    /// of the current thread and returns the previous one.
    pub fn set_local_utc_offset_provider(provider: Option<LocalUtcOffsetProvider>) -> Option<LocalUtcOffsetProvider> {
        LOCAL_UTC_OFFSET_PROVIDER.with(|current| current.replace(provider))
    }

    /// The offset of local time from UTC at the UTC instant `utc_time`
    /// (zero when no provider is installed). The offset is truncated to whole
    /// minutes and limited to 14 hours in either direction.
    pub fn get_local_utc_offset(utc_time: DateTime) -> TimeSpan {
        let provider = LOCAL_UTC_OFFSET_PROVIDER.with(|current| current.borrow().clone());
        match provider {
            Some(provider) => {
                let ticks = provider(utc_time).ticks();
                let ticks = ticks - ticks % TimeSpan::TICKS_PER_MINUTE;
                TimeSpan::from_ticks(ticks.clamp(-Self::MAX_OFFSET.ticks(), Self::MAX_OFFSET.ticks()))
            }
            None => TimeSpan::ZERO,
        }
    }

    /// The offset of local time from UTC for the local clock time
    /// `local_time`. Around a transition of the offset the clock time can be
    /// ambiguous or invalid; the offset in effect after the transition is
    /// preferred.
    pub fn get_utc_offset_of_local_time(local_time: DateTime) -> TimeSpan {
        let first = Self::get_local_utc_offset(local_time);
        if first == TimeSpan::ZERO && LOCAL_UTC_OFFSET_PROVIDER.with(|current| current.borrow().is_none()) {
            return first;
        }
        let ticks = (local_time.ticks() - first.ticks()).clamp(DateTime::MIN_VALUE.ticks(), DateTime::MAX_VALUE.ticks());
        Self::get_local_utc_offset(DateTime::from_ticks(ticks))
    }
}
