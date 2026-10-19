//! `RxTestTests` (`ReactiveUI.Testing.Tests/RxTestTests.cs`).

use crate::mocks::{delay, is_invalid_operation, yield_now, InvalidOperationException};
use ferroui_reactive::linq::TimeoutException;
use ferroui_reactiveui::PanicException;
use ferroui_reactiveui_testing::RxTest;
use ferroui_system::threading::tasks::{block_on, Task};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The number of tests run one after the other.
const EXPECTED_SEQUENTIAL_CALL_COUNT: i32 = 3;

/// A time limit the test keeps within, in milliseconds.
const CUSTOM_TIMEOUT_MILLISECONDS: u64 = 5_000;

/// A time limit the test exceeds, in milliseconds.
const SHORT_TIMEOUT_MILLISECONDS: u64 = 100;

/// How long the test that exceeds the limit runs, in milliseconds.
const DELAY_LONGER_THAN_TIMEOUT_MILLISECONDS: u64 = 2_000;

/// A test body that counts its runs in `count`.
fn counting_body(count: &Rc<Cell<i32>>) -> impl FnOnce() -> Task<()> + 'static {
    let count = count.clone();
    move || {
        count.set(count.get() + 1);
        Task::from_result(())
    }
}

#[test]
fn app_builder_test_async_executes_test_body() {
    // Arrange
    let executed = Rc::new(Cell::new(false));

    // Act
    let flag = executed.clone();
    block_on(RxTest::app_builder_test_async(move || {
        flag.set(true);
        Task::from_result(())
    }))
    .expect("the test passes");

    // Assert
    assert!(executed.get());
}

// Not ported: AppBuilderTestAsync_ThrowsArgumentNullException_WhenTestBodyIsNull - argument validation

#[test]
fn app_builder_test_async_propagates_exceptions() {
    // The body throws before it returns a task: here it panics, and the task ends with the error of
    // the panic.
    let error = block_on(RxTest::app_builder_test_async(|| panic!("Test exception"))).expect_err("the panic of the test is passed on");

    assert_eq!(error.downcast_ref::<PanicException>().map(|exception| exception.message.as_str()), Some("Test exception"));
}

#[test]
fn app_builder_test_async_propagates_async_exceptions() {
    let error = block_on(RxTest::app_builder_test_async(|| {
        Task::spawn_local(async {
            yield_now().await?;
            Err::<(), _>(InvalidOperationException::error("Async test exception"))
        })
    }))
    .expect_err("the exception of the test is passed on");

    assert!(is_invalid_operation(&error));
}

#[test]
fn app_builder_test_async_allows_sequential_calls() {
    // Arrange
    let count = Rc::new(Cell::new(0));

    // Act
    for _ in 0..EXPECTED_SEQUENTIAL_CALL_COUNT {
        block_on(RxTest::app_builder_test_async(counting_body(&count))).expect("the test passes");
    }

    // Assert
    assert_eq!(count.get(), EXPECTED_SEQUENTIAL_CALL_COUNT);
}

#[test]
fn app_builder_test_async_resets_builder_state_between_tests() {
    // This test verifies that multiple calls don't interfere with each other.
    // Arrange & Act
    block_on(RxTest::app_builder_test_async(|| Task::from_result(()))).expect("the first test passes");

    let second_test_executed = Rc::new(Cell::new(false));
    let flag = second_test_executed.clone();
    block_on(RxTest::app_builder_test_async(move || {
        // Second test - should have clean state.
        flag.set(true);
        Task::from_result(())
    }))
    .expect("the second test passes");

    // Assert
    assert!(second_test_executed.get());
}

#[test]
fn app_builder_test_async_with_custom_timeout_executes_within_timeout() {
    // Arrange
    let executed = Rc::new(Cell::new(false));

    // Act
    let flag = executed.clone();
    block_on(RxTest::app_builder_test_async_with(
        move || {
            flag.set(true);
            Task::from_result(())
        },
        CUSTOM_TIMEOUT_MILLISECONDS,
    ))
    .expect("the test passes");

    // Assert
    assert!(executed.get());
}

#[test]
fn app_builder_test_async_throws_timeout_exception_when_test_exceeds_timeout() {
    let error = block_on(RxTest::app_builder_test_async_with(
        || delay(Duration::from_millis(DELAY_LONGER_THAN_TIMEOUT_MILLISECONDS)),
        SHORT_TIMEOUT_MILLISECONDS,
    ))
    .expect_err("the test runs longer than its time limit");

    assert!(error.downcast_ref::<TimeoutException>().is_some());
}

#[test]
fn app_builder_test_async_handles_task_completed_task() {
    block_on(RxTest::app_builder_test_async(|| Task::from_result(()))).expect("the test passes");
}
