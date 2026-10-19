//! `AppBuilderTestBaseTests` (`ReactiveUI.Testing.Tests/AppBuilderTestBaseTests.cs`).

use crate::mocks::{is_invalid_operation, yield_now, InvalidOperationException};
use ferroui_reactiveui::PanicException;
use ferroui_reactiveui_testing::AppBuilderTestBase;
use ferroui_system::threading::tasks::{block_on, Task};
use std::cell::Cell;
use std::rc::Rc;

/// The number of tests run one after the other.
const EXPECTED_EXECUTION_COUNT: i32 = 3;

/// A fixture that derives from the base (`TestHelper`).
struct TestHelper;

impl AppBuilderTestBase for TestHelper {}

#[test]
fn run_app_builder_test_async_with_async_test_body_executes_test() {
    // Arrange
    let executed = Rc::new(Cell::new(false));

    // Act
    let flag = executed.clone();
    block_on(TestHelper::run_app_builder_test_async(move || {
        Task::spawn_local(async move {
            flag.set(true);
            Ok(())
        })
    }))
    .expect("the test passes");

    // Assert
    assert!(executed.get());
}

#[test]
fn run_app_builder_test_async_with_sync_test_body_executes_test() {
    // Arrange
    let executed = Rc::new(Cell::new(false));

    // Act
    let flag = executed.clone();
    block_on(TestHelper::run_app_builder_test_action(move || flag.set(true))).expect("the test passes");

    // Assert
    assert!(executed.get());
}

#[test]
fn run_app_builder_test_async_with_async_test_body_propagates_exceptions() {
    let error = block_on(TestHelper::run_app_builder_test_async(|| {
        Task::spawn_local(async {
            yield_now().await?;
            Err::<(), _>(InvalidOperationException::error("Test exception"))
        })
    }))
    .expect_err("the exception of the test is passed on");

    assert!(is_invalid_operation(&error));
}

#[test]
fn run_app_builder_test_async_with_sync_test_body_propagates_exceptions() {
    // The synchronous body throws: here it panics, and the task ends with the error of the panic.
    let error = block_on(TestHelper::run_app_builder_test_action(|| panic!("Test exception"))).expect_err("the panic of the test is passed on");

    assert_eq!(error.downcast_ref::<PanicException>().map(|exception| exception.message.as_str()), Some("Test exception"));
}

#[test]
fn run_app_builder_test_async_allows_multiple_sequential_calls() {
    // Arrange
    let execution_count = Rc::new(Cell::new(0));

    // Act
    for _ in 0..EXPECTED_EXECUTION_COUNT {
        let count = execution_count.clone();
        block_on(TestHelper::run_app_builder_test_action(move || count.set(count.get() + 1))).expect("the test passes");
    }

    // Assert
    assert_eq!(execution_count.get(), EXPECTED_EXECUTION_COUNT);
}
