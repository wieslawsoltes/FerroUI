//! `UIThreadTestsMain` of the tests of the integration (the tests of the scheduler of the
//! user-interface thread).
//!
//! Upstream tests the scheduler of the `Primitives` flavour, whose work items and timestamps are
//! the sequencer's; the port is the `Reactive` flavour, so a work item is a scheduled action and a
//! timestamp is a time of the scheduler's clock. The delayed work waits on a timer of the
//! dispatcher, which the test fires.

use super::binding_support::{run_jobs, start_app};
use crate::FerroScheduler;
use ferroui_base::threading::Dispatcher;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::IThreadAffineSequencer;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// The delay before the delayed work is due.
const FUTURE_SCHEDULE_DELAY: Duration = Duration::from_millis(20);

/// Work that records that it ran.
fn recording_work(ran: &Rc<Cell<bool>>) -> ferroui_reactive::concurrency::ScheduledAction {
    let ran = ran.clone();
    Box::new(move |_: &Rc<dyn IScheduler>| {
        ran.set(true);
        Disposable::empty()
    })
}

#[test]
fn ferro_scheduler_instance_is_not_null() {
    let _app = start_app();

    let scheduler = FerroScheduler::instance();

    assert!(Arc::ptr_eq(scheduler.dispatcher(), &Dispatcher::ui_thread()));
}

#[test]
fn ferro_scheduler_instance_is_singleton() {
    let _app = start_app();

    let instance1 = FerroScheduler::instance();
    let instance2 = FerroScheduler::instance();

    assert!(Rc::ptr_eq(&instance1, &instance2));
}

#[test]
fn ferro_scheduler_implements_i_sequencer() {
    let _app = start_app();

    let sequencer: Rc<dyn IThreadAffineSequencer> = FerroScheduler::instance();

    assert!(sequencer.check_access());
}

#[test]
fn ferro_scheduler_timestamp_returns_positive_value() {
    let _app = start_app();

    assert!(FerroScheduler::instance().now() > SystemTime::UNIX_EPOCH);
}

// Not ported: FerroScheduler_ScheduleWorkItem_ThrowsOnNull - argument validation

#[test]
fn ferro_scheduler_schedule_work_item_executes() {
    let _app = start_app();
    let scheduler = FerroScheduler::instance();
    let ran = Rc::new(Cell::new(false));

    let _ = scheduler.schedule(recording_work(&ran));
    run_jobs();

    assert!(ran.get());
}

// Not ported: FerroScheduler_ScheduleWorkItemWithTimestamp_ThrowsOnNull - argument validation

#[test]
fn ferro_scheduler_schedule_work_item_with_past_timestamp_executes() {
    let _app = start_app();
    let scheduler = FerroScheduler::instance();
    let ran = Rc::new(Cell::new(false));

    let _ = scheduler.schedule_at(scheduler.now() - Duration::from_millis(1), recording_work(&ran));
    run_jobs();

    assert!(ran.get());
}

#[test]
fn ferro_scheduler_schedule_work_item_with_future_timestamp_executes() {
    let _app = start_app();
    let scheduler = FerroScheduler::instance();
    let ran = Rc::new(Cell::new(false));

    let _ = scheduler.schedule_at(scheduler.now() + FUTURE_SCHEDULE_DELAY, recording_work(&ran));
    run_jobs();
    for timer in Dispatcher::timers_for_unit_tests() {
        Dispatcher::force_fire_timer_for_unit_tests(&timer);
    }

    assert!(ran.get());
}
