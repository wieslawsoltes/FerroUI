use super::*;
use crate::test_environment::Environment;
use ferroui_reactive::concurrency::{HistoricalScheduler, IScheduler, ScheduledAction};
use ferroui_system::threading::{set_synchronization_context, SynchronizationContext};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
struct UnusedContext;
impl SynchronizationContext for UnusedContext {
    fn post(&self, _: Box<dyn FnOnce() + Send>) {
        panic!("must use Rx scheduler")
    }
    fn send(&self, _: Box<dyn FnOnce() + Send>) {
        panic!("must use Rx scheduler")
    }
    fn post_local(&self, _: Box<dyn FnOnce()>) {
        panic!("must use Rx scheduler")
    }
    fn send_local(&self, _: Box<dyn FnOnce()>) {
        panic!("must use Rx scheduler")
    }
}
#[test]
fn invokes_the_current_scheduler_per_call_and_ignores_same_synchronization_context() {
    let _env = Environment::new();
    let first = HistoricalScheduler::historical();
    let second = HistoricalScheduler::historical();
    RxSchedulers::set_main_thread_scheduler(first.clone());
    set_synchronization_context(Some(Arc::new(UnusedContext)));
    let dispatcher: Rc<dyn IDockDispatcher> = MainThreadDispatcher::new();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let action: Rc<dyn Fn()> = Rc::new(move || seen.set(seen.get() + 1));
    let a = dispatcher.invoke_async(action.clone());
    RxSchedulers::set_main_thread_scheduler(second.clone());
    let b = dispatcher.invoke_async(action);
    assert!(!a.is_completed());
    assert!(!b.is_completed());
    assert_eq!(calls.get(), 0);
    first.start();
    assert!(a.is_completed_successfully());
    assert!(!b.is_completed());
    assert_eq!(calls.get(), 1);
    second.start();
    assert!(b.is_completed_successfully());
    assert_eq!(calls.get(), 2);
}
#[test]
fn immediate_and_queued_actions_capture_failure_in_task_and_success_complete_once() {
    let _env = Environment::new();
    let dispatcher = MainThreadDispatcher::new();
    let succeeded = dispatcher.invoke_async(Rc::new(|| {}));
    assert!(succeeded.is_completed_successfully());
    let failed = dispatcher.invoke_async(Rc::new(|| panic!("immediate failure")));
    assert_eq!(
        failed.result().unwrap_err().to_string(),
        "immediate failure"
    );
    let scheduler = HistoricalScheduler::historical();
    RxSchedulers::set_main_thread_scheduler(scheduler.clone());
    let failed = dispatcher.invoke_async(Rc::new(|| panic!("queued failure")));
    assert!(!failed.is_completed());
    scheduler.start();
    assert_eq!(failed.result().unwrap_err().to_string(), "queued failure");
}
#[test]
fn reentrant_invocation_uses_scheduler_changed_by_the_outer_action() {
    let _env = Environment::new();
    let first = HistoricalScheduler::historical();
    let second = HistoricalScheduler::historical();
    RxSchedulers::set_main_thread_scheduler(first.clone());
    let dispatcher = MainThreadDispatcher::new();
    let seen_dispatcher = dispatcher.clone();
    let log = Rc::new(RefCell::new(Vec::new()));
    let seen = log.clone();
    let next = second.clone();
    let inner = Rc::new(RefCell::new(None));
    let stored = inner.clone();
    let outer = dispatcher.invoke_async(Rc::new(move || {
        seen.borrow_mut().push("outer");
        RxSchedulers::set_main_thread_scheduler(next.clone());
        let seen = seen.clone();
        stored.replace(Some(
            seen_dispatcher.invoke_async(Rc::new(move || seen.borrow_mut().push("inner"))),
        ));
    }));
    first.start();
    assert!(outer.is_completed_successfully());
    assert_eq!(&*log.borrow(), &["outer"]);
    assert!(!inner.borrow().as_ref().unwrap().is_completed());
    second.start();
    assert_eq!(&*log.borrow(), &["outer", "inner"]);
    assert!(inner.borrow().as_ref().unwrap().is_completed_successfully());
}
struct FailingScheduler;
impl IScheduler for FailingScheduler {
    fn now(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH
    }
    fn schedule(&self, _: ScheduledAction) -> Rc<dyn IDisposable> {
        panic!("scheduling failed")
    }
    fn schedule_after(&self, _: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule(action)
    }
    fn schedule_at(&self, _: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule(action)
    }
}
#[test]
fn scheduling_failure_propagates_synchronously_without_executing_action() {
    let _env = Environment::new();
    RxSchedulers::set_main_thread_scheduler(Rc::new(FailingScheduler));
    let dispatcher = MainThreadDispatcher::new();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispatcher.invoke_async(Rc::new(move || seen.set(seen.get() + 1)))
    }));
    assert!(result.is_err());
    assert_eq!(calls.get(), 0);
}
