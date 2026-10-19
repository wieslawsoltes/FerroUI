//! The tests of the scheduler's queue, its drains and its cancellation, of the installation of the
//! synchronization context, and of a post of the view thread invoker. The port's own: upstream
//! tests the queue in the tests of its `Primitives` (`CoalescingDispatchStateTests`), against
//! another platform.

use super::binding_support::{run_jobs, start_app, TestObject};
use crate::{FerroObjectHandles, FerroScheduler, FerroViewThreadInvoker};
use ferroui_base::threading::Dispatcher;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::IViewThreadInvoker;
use ferroui_system::threading::{current_synchronization_context, set_synchronization_context};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

/// Work that appends `value` to `log`.
fn logging_work(log: &Rc<RefCell<Vec<i32>>>, value: i32) -> ferroui_reactive::concurrency::ScheduledAction {
    let log = log.clone();
    Box::new(move |_: &Rc<dyn IScheduler>| {
        log.borrow_mut().push(value);
        Disposable::empty()
    })
}

#[test]
fn schedule_runs_the_work_in_a_job_of_the_dispatcher_not_inline() {
    let _app = start_app();
    let log = Rc::new(RefCell::new(Vec::new()));

    let _ = FerroScheduler::instance().schedule(logging_work(&log, 1));

    assert!(log.borrow().is_empty());
    run_jobs();
    assert_eq!(*log.borrow(), vec![1]);
}

#[test]
fn schedule_runs_the_work_in_order() {
    let _app = start_app();
    let scheduler = FerroScheduler::instance();
    let log = Rc::new(RefCell::new(Vec::new()));

    let _ = scheduler.schedule(logging_work(&log, 1));
    let _ = scheduler.schedule(logging_work(&log, 2));
    let _ = scheduler.schedule(logging_work(&log, 3));
    run_jobs();

    assert_eq!(*log.borrow(), vec![1, 2, 3]);
}

#[test]
fn work_scheduled_by_work_runs_after_it() {
    let _app = start_app();
    let scheduler = FerroScheduler::instance();
    let log = Rc::new(RefCell::new(Vec::new()));
    let inner_log = log.clone();
    let inner_ran_inline = Rc::new(Cell::new(false));
    let inline = inner_ran_inline.clone();

    let _ = scheduler.schedule(Box::new(move |scheduler: &Rc<dyn IScheduler>| {
        let _ = scheduler.schedule(logging_work(&inner_log, 2));
        inline.set(!inner_log.borrow().is_empty());
        inner_log.borrow_mut().push(1);
        Disposable::empty()
    }));
    run_jobs();

    assert!(!inner_ran_inline.get());
    assert_eq!(*log.borrow(), vec![1, 2]);
}

#[test]
fn disposing_work_that_has_not_run_drops_it() {
    let _app = start_app();
    let log = Rc::new(RefCell::new(Vec::new()));

    let work = FerroScheduler::instance().schedule(logging_work(&log, 1));
    work.dispose();
    run_jobs();

    assert!(log.borrow().is_empty());
}

#[test]
fn disposing_work_that_has_run_disposes_what_it_returned() {
    let _app = start_app();
    let released = Rc::new(Cell::new(false));
    let release = released.clone();

    let work = FerroScheduler::instance().schedule(Box::new(move |_: &Rc<dyn IScheduler>| {
        let release = release.clone();
        Disposable::create(move || release.set(true))
    }));
    run_jobs();
    assert!(!released.get());
    work.dispose();

    assert!(released.get());
}

#[test]
fn disposing_delayed_work_stops_its_timer() {
    let _app = start_app();
    let log = Rc::new(RefCell::new(Vec::new()));
    let before = Dispatcher::timers_for_unit_tests().len();

    let work = FerroScheduler::instance().schedule_after(Duration::from_secs(1), logging_work(&log, 1));
    assert_eq!(Dispatcher::timers_for_unit_tests().len(), before + 1);
    work.dispose();

    assert_eq!(Dispatcher::timers_for_unit_tests().len(), before);
    assert!(log.borrow().is_empty());
}

#[test]
fn for_dispatcher_of_the_ui_thread_is_the_instance() {
    let _app = start_app();

    let scheduler = FerroScheduler::for_dispatcher(&Dispatcher::ui_thread());

    assert!(Rc::ptr_eq(&scheduler, &FerroScheduler::instance()));
}

#[test]
fn a_new_scheduler_posts_at_background_priority() {
    let _app = start_app();

    let scheduler = FerroScheduler::new();

    assert!(Arc::ptr_eq(scheduler.dispatcher(), &Dispatcher::ui_thread()));
    assert_eq!(scheduler.priority(), ferroui_base::threading::DispatcherPriority::BACKGROUND);
}

#[test]
fn install_on_ui_thread_sets_the_synchronization_context_of_the_thread() {
    let _app = start_app();
    set_synchronization_context(None);

    FerroScheduler::install_on_ui_thread();
    let installed = current_synchronization_context().expect("a synchronization context");
    FerroScheduler::install_on_ui_thread();
    let again = current_synchronization_context().expect("a synchronization context");

    assert!(std::ptr::addr_eq(Arc::as_ptr(&installed), Arc::as_ptr(&again)));
    set_synchronization_context(None);
}

#[test]
fn the_view_thread_invoker_posts_to_the_scheduler_of_the_target() {
    let _app = start_app();
    let target = FerroObjectHandles::handle_of(&TestObject::new());
    let ran = Rc::new(Cell::new(false));
    let record = ran.clone();

    FerroViewThreadInvoker::instance().post(&target, Box::new(move || record.set(true)));

    assert!(!ran.get());
    run_jobs();
    assert!(ran.get());
}
