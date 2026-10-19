//! `BuilderSchedulerMixinsTests` (`ReactiveUI.Builder.Tests/Mixins/BuilderSchedulerMixinsTests.cs`).

use super::super::executors::reset_only;
use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler};
use ferroui_reactiveui::*;
use std::rc::Rc;

fn same(left: &Rc<dyn IScheduler>, right: &Rc<dyn IScheduler>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(left), Rc::as_ptr(right))
}

#[test]
fn with_task_pool_scheduler_sets_scheduler_and_rx_schedulers() {
    reset_only(|| {
        let original = RxSchedulers::task_pool_scheduler();
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let scheduler = ImmediateScheduler::instance();
        let _ = builder.with_task_pool_scheduler(scheduler.clone());
        let _ = builder.with_core_services().build();

        assert!(builder.task_pool_scheduler().is_some_and(|value| same(&value, &scheduler)));
        assert!(same(&RxSchedulers::task_pool_scheduler(), &scheduler));
        RxSchedulers::set_task_pool_scheduler(original);
    });
}

#[test]
fn with_main_thread_scheduler_sets_scheduler_and_rx_schedulers() {
    reset_only(|| {
        let original = RxSchedulers::main_thread_scheduler();
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let scheduler = ImmediateScheduler::instance();
        let _ = builder.with_main_thread_scheduler(scheduler.clone());
        let _ = builder.with_core_services().build();

        assert!(builder.main_thread_scheduler().is_some_and(|value| same(&value, &scheduler)));
        assert!(same(&RxSchedulers::main_thread_scheduler(), &scheduler));
        RxSchedulers::set_main_thread_scheduler(original);
    });
}

#[test]
fn with_task_pool_scheduler_returns_builder() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();

        let result = builder.with_task_pool_scheduler(ImmediateScheduler::instance());

        assert!(std::ptr::eq(result, &builder));
    });
}

#[test]
fn with_main_thread_scheduler_returns_builder() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();

        let result = builder.with_main_thread_scheduler(ImmediateScheduler::instance());

        assert!(std::ptr::eq(result, &builder));
    });
}
