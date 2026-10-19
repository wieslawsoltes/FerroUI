//! `RxSchedulersTest` (`ReactiveUI.Tests/RxSchedulersTest.cs`).

use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui::*;
use std::rc::Rc;
use std::time::SystemTime;

fn same(first: &Rc<dyn IScheduler>, second: &Rc<dyn IScheduler>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(first), Rc::as_ptr(second))
}

#[test]
fn schedulers_can_be_accessed_without_attributes() {
    // This test method itself should not require RequiresUnreferencedCode because it uses
    // RxSchedulers instead of RxApp.
    let main_scheduler = RxSchedulers::main_thread_scheduler();
    let taskpool_scheduler = RxSchedulers::task_pool_scheduler();

    // Until a scheduler is set, both are the default scheduler of the thread.
    assert!(same(&main_scheduler, &SchedulerDefaults::default_scheduler()));
    assert!(same(&taskpool_scheduler, &SchedulerDefaults::default_scheduler()));
}

#[test]
fn schedulers_can_be_set_and_retrieved() {
    let test_scheduler = TestScheduler::new().scheduler();

    // Store original schedulers to ensure test isolation
    let original_main_scheduler = RxSchedulers::main_thread_scheduler();
    let original_taskpool_scheduler = RxSchedulers::task_pool_scheduler();

    // Set schedulers
    RxSchedulers::set_main_thread_scheduler(test_scheduler.clone());
    RxSchedulers::set_task_pool_scheduler(test_scheduler.clone());

    // Verify they were set
    assert!(same(&RxSchedulers::main_thread_scheduler(), &test_scheduler));
    assert!(same(&RxSchedulers::task_pool_scheduler(), &test_scheduler));

    // Always restore original schedulers to ensure test isolation
    RxSchedulers::set_main_thread_scheduler(original_main_scheduler);
    RxSchedulers::set_task_pool_scheduler(original_taskpool_scheduler);
}

#[test]
fn schedulers_provide_basic_functionality() {
    let main_scheduler = RxSchedulers::main_thread_scheduler();
    let taskpool_scheduler = RxSchedulers::task_pool_scheduler();

    // They are schedulers (`IsAssignableTo<ISequencer>` holds by their type), with a clock.
    assert!(main_scheduler.now() > SystemTime::UNIX_EPOCH);
    assert!(taskpool_scheduler.now() > SystemTime::UNIX_EPOCH);
}

/// Upstream calls `EnsureStaticConstructorRun`, which forces the static constructor that assigns
/// the defaults; the defaults need no initialization here.
#[test]
fn ensure_static_constructor_run_assigns_defaults() {
    RxSchedulers::reset_for_testing();

    assert!(same(&RxSchedulers::main_thread_scheduler(), &SchedulerDefaults::default_scheduler()));
    assert!(same(&RxSchedulers::task_pool_scheduler(), &SchedulerDefaults::default_scheduler()));
}
