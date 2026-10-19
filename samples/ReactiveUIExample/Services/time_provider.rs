//! The clock of the services and view models (`System.TimeProvider`).
//!
//! Not an upstream file: the reference takes .NET's `TimeProvider`, which has no counterpart in the
//! port; this is the part of it the example uses.

use ferroui_base::utilities::DateTimeOffset;

/// A clock (`TimeProvider`): the current time, in the coordinated universal time and in the local
/// time zone.
pub trait TimeProvider {
    /// The current coordinated universal time (`GetUtcNow()`).
    fn get_utc_now(&self) -> DateTimeOffset;

    /// The current local time (`GetLocalNow()`): the universal time in the local time zone.
    fn get_local_now(&self) -> DateTimeOffset {
        self.get_utc_now().to_local_time()
    }
}

/// The clock of the system (`TimeProvider.System`).
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemTimeProvider;

impl TimeProvider for SystemTimeProvider {
    fn get_utc_now(&self) -> DateTimeOffset {
        DateTimeOffset::utc_now()
    }

    fn get_local_now(&self) -> DateTimeOffset {
        DateTimeOffset::now()
    }
}
