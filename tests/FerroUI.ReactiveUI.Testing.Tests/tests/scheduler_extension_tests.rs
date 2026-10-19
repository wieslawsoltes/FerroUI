//! `SchedulerExtensionTests` (`ReactiveUI.Testing.Tests/SchedulerExtensionTests.cs`).
//!
//! Upstream's `VirtualTimeScheduler` of `ReactiveUI.Test.Utilities` wraps the `TestScheduler` of
//! Microsoft.Reactive.Testing; the tests use the [`TestScheduler`] directly.

use ferroui_reactive_testing::{assert_throws, TestScheduler};
use ferroui_reactiveui::RxSchedulers;
use ferroui_reactiveui_testing::SchedulerExtensions;
use ferroui_system::threading::tasks::{block_on, Task};
use std::cell::Cell;
use std::rc::Rc;

/// The message of the panic of a block that does not run with its scheduler.
const SCHEDULER_NOT_SET_MESSAGE: &str = "Scheduler not set correctly";

/// What the functions of the tests return.
const EXPECTED_VALUE: i32 = 42;

/// Whether `scheduler` is the main thread scheduler of [`RxSchedulers`] (`IsSameReferenceAs`).
fn is_main_thread_scheduler(scheduler: &TestScheduler) -> bool {
    Rc::ptr_eq(&RxSchedulers::main_thread_scheduler(), &scheduler.scheduler())
}

#[test]
fn with_scheduler_should_set_both_rx_app_and_rx_schedulers_schedulers() {
    let test_scheduler = TestScheduler::new();
    let original_main_thread = RxSchedulers::main_thread_scheduler();
    let original_taskpool = RxSchedulers::task_pool_scheduler();

    let restore = SchedulerExtensions::with_scheduler(&test_scheduler);
    // Verify the schedulers of RxSchedulers are set.
    assert!(Rc::ptr_eq(&RxSchedulers::main_thread_scheduler(), &test_scheduler.scheduler()));
    assert!(Rc::ptr_eq(&RxSchedulers::task_pool_scheduler(), &test_scheduler.scheduler()));
    restore.dispose();

    // Verify schedulers are restored after disposal.
    assert!(Rc::ptr_eq(&RxSchedulers::main_thread_scheduler(), &original_main_thread));
    assert!(Rc::ptr_eq(&RxSchedulers::task_pool_scheduler(), &original_taskpool));
}

#[test]
fn with_scheduler_nested_calls_should_work_sequentially() {
    let scheduler1 = TestScheduler::new();
    let scheduler2 = TestScheduler::new();
    let original_main_thread = RxSchedulers::main_thread_scheduler();

    let outer = SchedulerExtensions::with_scheduler(&scheduler1);
    assert!(is_main_thread_scheduler(&scheduler1));

    let inner = SchedulerExtensions::with_scheduler(&scheduler2);
    assert!(is_main_thread_scheduler(&scheduler2));
    inner.dispose();

    // After inner scope, should restore to scheduler1.
    assert!(is_main_thread_scheduler(&scheduler1));
    outer.dispose();

    // After outer scope, should restore to original.
    assert!(Rc::ptr_eq(&RxSchedulers::main_thread_scheduler(), &original_main_thread));
}

#[test]
fn with_scheduler_exception_in_critical_section_should_still_release_gate() {
    let scheduler1 = TestScheduler::new();
    let scheduler2 = TestScheduler::new();
    let original_main_thread = RxSchedulers::main_thread_scheduler();

    // First call throws an exception (here, panics); the schedulers are restored on the way out.
    assert_throws(|| scheduler1.with::<()>(|_| panic!("Test exception")));
    assert!(Rc::ptr_eq(&RxSchedulers::main_thread_scheduler(), &original_main_thread));

    // Second call should succeed.
    let restore = SchedulerExtensions::with_scheduler(&scheduler2);
    assert!(is_main_thread_scheduler(&scheduler2));
    restore.dispose();
}

#[test]
fn with_function_should_execute_and_return_value() {
    let scheduler = TestScheduler::new();
    let original_scheduler = RxSchedulers::main_thread_scheduler();

    let result = scheduler.with(|s| {
        // Inside the block, scheduler should be active.
        assert!(is_main_thread_scheduler(s), "{SCHEDULER_NOT_SET_MESSAGE}");
        EXPECTED_VALUE
    });

    assert_eq!(result, EXPECTED_VALUE);

    // After the block, original scheduler should be restored.
    assert!(Rc::ptr_eq(&RxSchedulers::main_thread_scheduler(), &original_scheduler));
}

#[test]
fn with_action_should_execute() {
    let scheduler = TestScheduler::new();
    let original_scheduler = RxSchedulers::main_thread_scheduler();
    let executed = Cell::new(false);

    scheduler.with(|s| {
        executed.set(true);

        // Inside the block, scheduler should be active.
        assert!(is_main_thread_scheduler(s), "{SCHEDULER_NOT_SET_MESSAGE}");
    });

    assert!(executed.get());
    assert!(Rc::ptr_eq(&RxSchedulers::main_thread_scheduler(), &original_scheduler));
}

#[test]
fn with_async_function_should_execute_and_return_value() {
    let scheduler = TestScheduler::new();
    let original_scheduler = RxSchedulers::main_thread_scheduler();

    let result = block_on(scheduler.with_async(|s| {
        // Inside the block, scheduler should be active.
        assert!(is_main_thread_scheduler(s), "{SCHEDULER_NOT_SET_MESSAGE}");
        Task::from_result(EXPECTED_VALUE)
    }))
    .expect("the block ends with its value");

    assert_eq!(result, EXPECTED_VALUE);
    assert!(Rc::ptr_eq(&RxSchedulers::main_thread_scheduler(), &original_scheduler));
}

#[test]
fn with_async_action_should_execute() {
    let scheduler = TestScheduler::new();
    let original_scheduler = RxSchedulers::main_thread_scheduler();
    let executed = Rc::new(Cell::new(false));

    let flag = executed.clone();
    block_on(scheduler.with_async(move |s| {
        flag.set(true);

        // Inside the block, scheduler should be active.
        assert!(is_main_thread_scheduler(s), "{SCHEDULER_NOT_SET_MESSAGE}");
        Task::from_result(())
    }))
    .expect("the block ends");

    assert!(executed.get());
    assert!(Rc::ptr_eq(&RxSchedulers::main_thread_scheduler(), &original_scheduler));
}

#[test]
fn with_scheduler_rapid_sequential_calls_should_work() {
    const ITERATIONS: usize = 100;

    for _ in 0..ITERATIONS {
        let scheduler = TestScheduler::new();
        let restore = SchedulerExtensions::with_scheduler(&scheduler);
        assert!(is_main_thread_scheduler(&scheduler));
        restore.dispose();
    }

    // No assertions needed - if we get here, the test passed.
}
