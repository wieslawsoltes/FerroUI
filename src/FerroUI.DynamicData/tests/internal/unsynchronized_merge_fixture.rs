//! `UnsynchronizedMergeFixture` (`DynamicData.Tests/Internal/UnsynchronizedMergeFixture.cs`): the
//! terminal semantics of `Merge`, without a gate.

use super::test_error;
use crate::internal::unsynchronized_merge;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn subject() -> Rc<Subject<i32>> {
    Subject::new()
}

fn is_error(captured: &RefCell<Option<ObservableError>>, expected: &ObservableError) -> bool {
    captured.borrow().as_ref().is_some_and(|captured| Rc::ptr_eq(captured, expected))
}

#[test]
fn on_next_from_both_sources_is_forwarded_in_arrival_order() {
    let (a, b) = (subject(), subject());
    let received = Rc::new(RefCell::new(Vec::new()));
    let target = received.clone();
    let sub = unsynchronized_merge(a.as_observable(), vec![b.as_observable()]).subscribe_next(move |value| target.borrow_mut().push(value));

    a.on_next(1);
    b.on_next(2);
    a.on_next(3);
    b.on_next(4);

    assert_eq!(*received.borrow(), vec![1, 2, 3, 4]);
    sub.dispose();
}

#[test]
fn on_completed_fires_only_after_all_sources_complete() {
    let (a, b, c) = (subject(), subject(), subject());
    let completed = Rc::new(Cell::new(false));
    let flag = completed.clone();
    let sub = unsynchronized_merge(a.as_observable(), vec![b.as_observable(), c.as_observable()]).subscribe_next_completed(|_| {}, move || flag.set(true));

    a.on_completed();
    assert!(!completed.get(), "a single source completion must not terminate the merged stream");
    b.on_completed();
    assert!(!completed.get(), "two of three completions still leave one source live");
    c.on_completed();
    assert!(completed.get(), "after every source has completed the merged stream must emit OnCompleted");
    sub.dispose();
}

#[test]
fn on_error_from_any_source_terminates_immediately() {
    let (a, b, c) = (subject(), subject(), subject());
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let completed = Rc::new(Cell::new(false));
    let (target, flag) = (captured.clone(), completed.clone());
    let sub = unsynchronized_merge(a.as_observable(), vec![b.as_observable(), c.as_observable()])
        .subscribe_all(|_| {}, move |error| *target.borrow_mut() = Some(error), move || flag.set(true));

    let error = test_error("first");
    b.on_error(error.clone());

    assert!(is_error(&captured, &error));
    assert!(!completed.get(), "OnCompleted must not fire after OnError");
    sub.dispose();
}

#[test]
fn on_error_after_first_error_is_ignored() {
    let (a, b) = (subject(), subject());
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let target = captured.clone();
    let sub = unsynchronized_merge(a.as_observable(), vec![b.as_observable()]).subscribe_all(|_| {}, move |error| *target.borrow_mut() = Some(error), || {});

    let (first, second) = (test_error("first"), test_error("second"));
    a.on_error(first.clone());
    b.on_error(second);

    assert!(is_error(&captured, &first), "first error wins; subsequent errors from other sources must be dropped");
    sub.dispose();
}

#[test]
fn on_completed_after_error_is_ignored() {
    let (a, b) = (subject(), subject());
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let completed = Rc::new(Cell::new(false));
    let (target, flag) = (captured.clone(), completed.clone());
    let sub = unsynchronized_merge(a.as_observable(), vec![b.as_observable()]).subscribe_all(|_| {}, move |error| *target.borrow_mut() = Some(error), move || flag.set(true));

    let error = test_error("error");
    a.on_error(error.clone());
    b.on_completed();

    assert!(is_error(&captured, &error));
    assert!(!completed.get(), "a late OnCompleted from a surviving source must not arrive after OnError has fired");
    sub.dispose();
}

#[test]
fn subscription_occurs_in_argument_order() {
    let subscribed = Rc::new(RefCell::new(Vec::new()));
    let create = |index: i32| -> Rc<dyn IObservable<i32>> {
        let subscribed = subscribed.clone();
        Observable::create_with_action(move |_: Rc<dyn IObserver<i32>>| {
            subscribed.borrow_mut().push(index);
            || {}
        })
    };
    let (first, second, third) = (create(0), create(1), create(2));

    let sub = unsynchronized_merge(first, vec![second, third]).subscribe_next(|_| {});

    assert_eq!(*subscribed.borrow(), vec![0, 1, 2]);
    sub.dispose();
}

#[test]
fn synchronous_terminal_before_other_sources_subscribe_is_handled() {
    // A source that completes at its subscription counts down at once; a wrong count would complete
    // the merge too early or never.
    let immediate = Observable::empty::<i32>();
    let live = subject();
    let completed = Rc::new(Cell::new(false));
    let flag = completed.clone();
    let sub = unsynchronized_merge(immediate, vec![live.as_observable()]).subscribe_next_completed(|_| {}, move || flag.set(true));

    assert!(!completed.get(), "the live source has not completed yet");
    live.on_completed();
    assert!(completed.get());
    sub.dispose();
}

#[test]
fn synchronous_error_before_other_sources_subscribe_terminates_immediately() {
    let error = test_error("error");
    let immediate = Observable::throw::<i32>(error.clone());
    let live = subject();
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let target = captured.clone();
    let sub = unsynchronized_merge(immediate, vec![live.as_observable()]).subscribe_next_error(|_| {}, move |error| *target.borrow_mut() = Some(error));

    assert!(is_error(&captured, &error));
    sub.dispose();
}

#[test]
fn no_others_falls_back_to_first_alone() {
    let a = subject();
    let received = Rc::new(RefCell::new(Vec::new()));
    let completed = Rc::new(Cell::new(false));
    let (target, flag) = (received.clone(), completed.clone());
    let sub = unsynchronized_merge(a.as_observable(), Vec::new()).subscribe_next_completed(move |value| target.borrow_mut().push(value), move || flag.set(true));

    a.on_next(7);
    a.on_next(11);
    a.on_completed();

    assert_eq!(*received.borrow(), vec![7, 11]);
    assert!(completed.get());
    sub.dispose();
}
