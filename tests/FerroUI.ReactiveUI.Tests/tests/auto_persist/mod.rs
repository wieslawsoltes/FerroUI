//! `ReactiveUI.Tests/AutoPersist`: the persistence of reactive objects and of the items of
//! collections.

mod auto_persist_collection_test;
mod auto_persist_helper_test;
mod auto_persist_helper_test_collection_persistence;
mod auto_persist_helper_test_handle_lifetime;

use crate::mocks::TestFixture;
use ferroui_reactive::Observable;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_reactiveui::{DataContract, RxSchedulers};
use ferroui_system::IObservable;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The persisted properties of [`TestFixture`]: its properties with `[DataMember]` upstream
/// (`[DataContract] TestFixture`).
impl DataContract for TestFixture {
    fn data_members() -> &'static [&'static str] {
        &["IsNotNullString", "IsOnlyOneWord", "NullableInt", "StackOverflowTrigger", "TestCollection", "UsesExprRaiseSet"]
    }
}

/// Two fixtures are equal when they are the same object, as the objects of .NET compare by
/// default; a collection of shared fixtures needs it.
impl PartialEq for TestFixture {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// Runs `test` with a virtual-time scheduler as the main-thread and the task-pool scheduler, and
/// restores the schedulers (`WithVirtualTimeSchedulerExecutor`).
pub(crate) fn with_virtual_time(test: impl FnOnce(&TestScheduler)) {
    let scheduler = TestScheduler::new();
    let main_thread = RxSchedulers::main_thread_scheduler();
    let task_pool = RxSchedulers::task_pool_scheduler();
    RxSchedulers::set_main_thread_scheduler(scheduler.scheduler());
    RxSchedulers::set_task_pool_scheduler(scheduler.scheduler());
    test(&scheduler);
    RxSchedulers::set_main_thread_scheduler(main_thread);
    RxSchedulers::set_task_pool_scheduler(task_pool);
}

/// `milliseconds` of virtual time.
pub(crate) fn ms(milliseconds: u64) -> i64 {
    duration_to_ticks(Duration::from_millis(milliseconds))
}

/// `seconds` of virtual time.
pub(crate) fn seconds(seconds: u64) -> i64 {
    duration_to_ticks(Duration::from_secs(seconds))
}

/// A save that counts itself and completes at once (`saveCount++; return ImmutableReturnRxVoidSignal.Instance`).
pub(crate) fn counting_save<T>(count: &Rc<Cell<i32>>) -> impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static {
    let count = count.clone();
    move |_| {
        count.set(count.get() + 1);
        Observable::return_(())
    }
}
