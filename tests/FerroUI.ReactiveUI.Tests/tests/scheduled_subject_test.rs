//! `ScheduledSubjectTest` (`ReactiveUI.Tests/ScheduledSubjectTest.cs`).

use crate::mocks::TestObserver;
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn constructor_with_default_observer_sends_values_to_it() {
    let scheduler = ImmediateScheduler::instance();
    let results = Rc::new(RefCell::new(Vec::<i32>::new()));
    let sink = results.clone();
    let default_observer = TestObserver::create(move |value: i32| sink.borrow_mut().push(value));

    const SECOND_VALUE: i32 = 2;
    const EXPECTED_COUNT: usize = 2;
    let subject = ScheduledSubject::with_default_observer(scheduler, Some(default_observer));
    subject.on_next(1);
    subject.on_next(SECOND_VALUE);

    assert_eq!(results.borrow().len(), EXPECTED_COUNT);
}

#[test]
fn dispose_cleans_up_resources() {
    let scheduler = TestScheduler::new();
    let subject = ScheduledSubject::<i32>::new(scheduler.scheduler());

    subject.dispose();

    assert!(subject.is_disposed());
}

#[test]
fn on_completed_completes_observable() {
    let scheduler = TestScheduler::new();
    let subject = ScheduledSubject::<i32>::new(scheduler.scheduler());
    let completed = Rc::new(Cell::new(false));

    let flag = completed.clone();
    let _ = subject.as_observable().subscribe_next_completed(|_| {}, move || flag.set(true));
    subject.on_completed();
    scheduler.run();

    assert!(completed.get());
}

#[test]
fn on_error_sends_error_to_observers() {
    let scheduler = TestScheduler::new();
    let subject = ScheduledSubject::<i32>::new(scheduler.scheduler());
    let received_error = Rc::new(RefCell::new(None::<ObservableError>));

    let sink = received_error.clone();
    let _ = subject.as_observable().subscribe_next_error(|_| {}, move |error| *sink.borrow_mut() = Some(error));
    let error: ObservableError = Rc::new(PanicException { message: "Test error".to_owned() });
    subject.on_error(error.clone());
    scheduler.run();

    let received = received_error.borrow().clone().expect("the error is delivered");
    assert!(Rc::ptr_eq(&received, &error));
}

#[test]
fn on_next_emits_values() {
    let scheduler = TestScheduler::new();
    let subject = ScheduledSubject::<i32>::new(scheduler.scheduler());
    let results = Rc::new(RefCell::new(Vec::<i32>::new()));

    const SECOND_VALUE: i32 = 2;
    const EXPECTED_COUNT: usize = 2;
    let sink = results.clone();
    let _ = subject.as_observable().subscribe_next(move |value| sink.borrow_mut().push(value));
    subject.on_next(1);
    subject.on_next(SECOND_VALUE);
    scheduler.run();

    assert_eq!(results.borrow().len(), EXPECTED_COUNT);
    assert_eq!(results.borrow()[0], 1);
    assert_eq!(results.borrow()[1], SECOND_VALUE);
}

#[test]
fn subscribe_returns_disposable() {
    let scheduler = TestScheduler::new();
    let subject = ScheduledSubject::<i32>::new(scheduler.scheduler());

    let subscription = subject.as_observable().subscribe_next(|_| {});

    assert!(subject.has_observers());
    subscription.dispose();
    assert!(!subject.has_observers());
}

#[test]
fn subscribe_schedules_on_specified_scheduler() {
    let scheduler = TestScheduler::new();
    let subject = ScheduledSubject::<i32>::new(scheduler.scheduler());
    let results = Rc::new(RefCell::new(Vec::<i32>::new()));

    let sink = results.clone();
    let _ = subject.as_observable().subscribe_next(move |value| sink.borrow_mut().push(value));
    subject.on_next(1);
    assert!(results.borrow().is_empty());

    scheduler.run();

    assert_eq!(results.borrow().len(), 1);
}

#[test]
fn subscription_when_disposed_stops_receiving_values() {
    let subject = ScheduledSubject::<i32>::new(ImmediateScheduler::instance());
    let results = Rc::new(RefCell::new(Vec::<i32>::new()));

    const VALUE_AFTER_DISPOSE: i32 = 2;
    let sink = results.clone();
    let subscription = subject.as_observable().subscribe_next(move |value| sink.borrow_mut().push(value));
    subject.on_next(1);
    subscription.dispose();
    subject.on_next(VALUE_AFTER_DISPOSE);

    assert_eq!(results.borrow().len(), 1);
}
