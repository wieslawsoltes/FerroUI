//! Port of `ThreadingTests.cs` of the upstream unit test project of the
//! headless platform.
//!
//! `ValidateTestContext` of the original compares the name of the running
//! test with the test context of NUnit, and does nothing under xUnit: there
//! is no test context here either.

use super::test_application::{ferro_fact, ferro_theory};
use super::{delay, oneshot};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer, FerroSynchronizationContext};
use std::time::Duration;

fn validate_test_context() {}

fn should_be_on_dispatcher_thread() {
    validate_test_context();
    Dispatcher::ui_thread().verify_access();
}
ferro_fact!(should_be_on_dispatcher_thread);

fn should_fail_test_on_delayed_post_when_flush_dispatcher() {
    Dispatcher::ui_thread().post(|| panic!("Operation is not valid due to the current state of the object."), DispatcherPriority::DEFAULT);
}
ferro_fact!(
    #[ignore = "This test should always fail, enable to test if it fails"]
    should_fail_test_on_delayed_post_when_flush_dispatcher
);

async fn dispatcher_timer_works_on_the_same_thread(interval: u64) {
    assert!(FerroSynchronizationContext::current().is_some());
    validate_test_context();
    let current_thread = std::thread::current().id();

    delay(Duration::from_millis(100)).await;

    validate_test_context();
    assert_eq!(current_thread, std::thread::current().id());

    let (tcs, task) = oneshot::<std::thread::Result<()>>();

    // A tick is a hundred nanoseconds.
    DispatcherTimer::run_once(
        move || {
            tcs.send(std::panic::catch_unwind(|| {
                validate_test_context();
                assert_eq!(current_thread, std::thread::current().id());
            }));
        },
        Duration::from_nanos(interval * 100),
        DispatcherPriority::DEFAULT,
    );

    if let Err(ex) = task.await {
        std::panic::resume_unwind(ex);
    }
}
ferro_theory!(async dispatcher_timer_works_on_the_same_thread {
    interval_1: (1),
    interval_10: (10),
    interval_100: (100),
});
