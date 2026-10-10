//! What the tests of the crate share.

use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use std::time::{Duration, Instant};

/// Runs the jobs of the dispatcher of the test thread until `done` says
/// so. Completions that come from the threads of the D-Bus library wake
/// the dispatcher by posting to it, so the loop sleeps between rounds.
///
/// # Panics
/// Panics when `done` is not true within ten seconds.
pub(crate) fn pump_until(mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        Dispatcher::ui_thread().run_jobs(None);
        if done() {
            return;
        }
        assert!(Instant::now() < deadline, "the condition was not met within ten seconds");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Makes the test thread its own UI thread for the duration of a test.
pub(crate) fn scope() -> UnitTestDispatcherScope {
    Dispatcher::unit_test_scope()
}
