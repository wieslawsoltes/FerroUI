//! Port of `AsyncSetupTests.cs` of the upstream unit test project of the
//! headless platform, a test of the NUnit integration: a set-up and a
//! tear-down that are asynchronous.
//!
//! The set-up and the tear-down are awaited around the test, on the
//! dispatcher thread, as the NUnit integration awaits them. The static
//! count of the original is a count of the session thread (see
//! `setup_tests`).

use super::delay;
use super::test_application::ferro_theory;
use std::cell::Cell;
use std::time::Duration;

thread_local! {
    static INSTANCE_COUNT: Cell<i32> = const { Cell::new(0) };
}

async fn set_up() {
    delay(Duration::from_millis(100)).await;
    INSTANCE_COUNT.set(INSTANCE_COUNT.get() + 1);
}

async fn tear_down() {
    delay(Duration::from_millis(100)).await;
    INSTANCE_COUNT.set(INSTANCE_COUNT.get() - 1);
}

// The parameter is used to run the test several times.
async fn async_setup_tear_down_should_work(_index: i32) {
    set_up().await;
    let result = std::panic::catch_unwind(|| assert_eq!(1, INSTANCE_COUNT.get()));
    tear_down().await;
    if let Err(ex) = result {
        std::panic::resume_unwind(ex);
    }
}
ferro_theory!(async async_setup_tear_down_should_work {
    index_1: (1),
    index_2: (2),
});
