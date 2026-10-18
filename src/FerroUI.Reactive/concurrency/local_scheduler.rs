//! What `LocalScheduler` gives the schedulers of the current machine (`LocalScheduler.cs`): the
//! system clock and a stopwatch on the monotonic clock.

use super::{IStopwatch, IStopwatchProvider};
use std::time::{Duration, Instant};

/// A stopwatch on the monotonic clock of the system (`StopwatchImpl` of the platform services).
pub(crate) struct SystemStopwatch {
    start: Instant,
}

impl SystemStopwatch {
    pub(crate) fn start() -> Self {
        Self { start: Instant::now() }
    }
}

impl IStopwatch for SystemStopwatch {
    fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

/// The stopwatch provider of the local schedulers.
pub(crate) struct SystemStopwatchProvider;

impl IStopwatchProvider for SystemStopwatchProvider {
    fn start_stopwatch(&self) -> Box<dyn IStopwatch> {
        Box::new(SystemStopwatch::start())
    }
}

/// Blocks the thread for `duration` (`ConcurrencyAbstractionLayer.Current.Sleep`): what the
/// schedulers that run work on the current thread do for delayed work, as upstream does.
pub(crate) fn sleep(duration: Duration) {
    if !duration.is_zero() {
        std::thread::sleep(duration);
    }
}
