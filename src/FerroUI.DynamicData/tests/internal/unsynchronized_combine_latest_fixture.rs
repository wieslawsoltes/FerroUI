//! `UnsynchronizedCombineLatestFixture`
//! (`DynamicData.Tests/Internal/UnsynchronizedCombineLatestFixture.cs`): `CombineLatest` of two
//! sequences without a gate.

use super::test_error;
use crate::internal::unsynchronized_combine_latest;
use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn is_error(captured: &RefCell<Option<ObservableError>>, expected: &ObservableError) -> bool {
    captured.borrow().as_ref().is_some_and(|captured| Rc::ptr_eq(captured, expected))
}

fn pair(x: i32, y: String) -> String {
    format!("{x}:{y}")
}

#[test]
fn on_next_does_not_emit_until_both_sources_have_produced() {
    let (a, b) = (Subject::<i32>::new(), Subject::<String>::new());
    let received = Rc::new(RefCell::new(Vec::new()));
    let target = received.clone();
    let sub = unsynchronized_combine_latest(a.as_observable(), b.as_observable(), pair).subscribe_next(move |value| target.borrow_mut().push(value));

    a.on_next(1);
    assert!(received.borrow().is_empty(), "only the first source has produced");
    a.on_next(2);
    assert!(received.borrow().is_empty(), "the second source still has not produced");
    b.on_next("first".to_owned());
    assert_eq!(*received.borrow(), vec!["2:first"]);
    sub.dispose();
}

#[test]
fn on_next_after_both_have_produced_emits_on_every_subsequent_value() {
    let (a, b) = (Subject::<i32>::new(), Subject::<String>::new());
    let received = Rc::new(RefCell::new(Vec::new()));
    let target = received.clone();
    let sub = unsynchronized_combine_latest(a.as_observable(), b.as_observable(), pair).subscribe_next(move |value| target.borrow_mut().push(value));

    a.on_next(1);
    b.on_next("x".to_owned());
    a.on_next(2);
    b.on_next("y".to_owned());
    b.on_next("z".to_owned());
    a.on_next(3);

    assert_eq!(*received.borrow(), vec!["1:x", "2:x", "2:y", "2:z", "3:z"]);
    sub.dispose();
}

#[test]
fn on_completed_fires_only_after_both_sources_complete() {
    let (a, b) = (Subject::<i32>::new(), Subject::<String>::new());
    let completed = Rc::new(Cell::new(false));
    let flag = completed.clone();
    let sub = unsynchronized_combine_latest(a.as_observable(), b.as_observable(), pair).subscribe_next_completed(|_| {}, move || flag.set(true));

    a.on_completed();
    assert!(!completed.get(), "the second source is still live");
    b.on_completed();
    assert!(completed.get());
    sub.dispose();
}

#[test]
fn on_error_from_any_source_terminates_immediately() {
    let (a, b) = (Subject::<i32>::new(), Subject::<String>::new());
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let completed = Rc::new(Cell::new(false));
    let (target, flag) = (captured.clone(), completed.clone());
    let sub = unsynchronized_combine_latest(a.as_observable(), b.as_observable(), pair).subscribe_all(|_| {}, move |error| *target.borrow_mut() = Some(error), move || flag.set(true));

    let error = test_error("first");
    b.on_error(error.clone());

    assert!(is_error(&captured, &error));
    assert!(!completed.get(), "OnCompleted must not fire after OnError");
    sub.dispose();
}

#[test]
fn on_error_after_first_error_is_ignored() {
    let (a, b) = (Subject::<i32>::new(), Subject::<String>::new());
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let target = captured.clone();
    let sub = unsynchronized_combine_latest(a.as_observable(), b.as_observable(), pair).subscribe_all(|_| {}, move |error| *target.borrow_mut() = Some(error), || {});

    let (first, second) = (test_error("first"), test_error("second"));
    a.on_error(first.clone());
    b.on_error(second);

    assert!(is_error(&captured, &first), "first error wins; subsequent errors from other sources must be dropped");
    sub.dispose();
}

#[test]
fn on_next_after_error_is_ignored() {
    let (a, b) = (Subject::<i32>::new(), Subject::<String>::new());
    let received = Rc::new(RefCell::new(Vec::new()));
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let (target, error_target) = (received.clone(), captured.clone());
    let sub = unsynchronized_combine_latest(a.as_observable(), b.as_observable(), pair)
        .subscribe_next_error(move |value| target.borrow_mut().push(value), move |error| *error_target.borrow_mut() = Some(error));

    a.on_next(1);
    b.on_next("x".to_owned());
    assert_eq!(*received.borrow(), vec!["1:x"]);

    let error = test_error("error");
    a.on_error(error.clone());
    a.on_next(2);
    b.on_next("y".to_owned());

    assert_eq!(*received.borrow(), vec!["1:x"], "no further OnNext must arrive after OnError has fired");
    assert!(is_error(&captured, &error));
    sub.dispose();
}

#[test]
fn on_completed_after_error_is_ignored() {
    let (a, b) = (Subject::<i32>::new(), Subject::<String>::new());
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let completed = Rc::new(Cell::new(false));
    let (target, flag) = (captured.clone(), completed.clone());
    let sub = unsynchronized_combine_latest(a.as_observable(), b.as_observable(), pair).subscribe_all(|_| {}, move |error| *target.borrow_mut() = Some(error), move || flag.set(true));

    let error = test_error("error");
    a.on_error(error.clone());
    b.on_completed();

    assert!(is_error(&captured, &error));
    assert!(!completed.get(), "a late OnCompleted from a surviving source must not arrive after OnError");
    sub.dispose();
}

#[test]
fn result_selector_receives_most_recent_value_from_each_source() {
    let (a, b) = (Subject::<i32>::new(), Subject::<i32>::new());
    let received = Rc::new(RefCell::new(Vec::new()));
    let target = received.clone();
    let sub = unsynchronized_combine_latest(a.as_observable(), b.as_observable(), |x, y| x * 10 + y).subscribe_next(move |value| target.borrow_mut().push(value));

    a.on_next(1);
    b.on_next(2);
    assert_eq!(*received.borrow(), vec![12]);
    a.on_next(3);
    assert_eq!(*received.borrow(), vec![12, 32], "the second source's most recent value (2) must still be in effect");
    b.on_next(4);
    assert_eq!(*received.borrow(), vec![12, 32, 34]);
    sub.dispose();
}

#[test]
fn synchronous_values_at_subscribe_time_are_combined_correctly() {
    // Behavior subjects deliver their value at the subscription: the first value is kept before the
    // second source is subscribed, and the second's value combines at once.
    let (a, b) = (BehaviorSubject::new(7), BehaviorSubject::new(11));
    let received = Rc::new(RefCell::new(Vec::new()));
    let target = received.clone();
    let sub = unsynchronized_combine_latest(a.as_observable(), b.as_observable(), |x: i32, y: i32| x + y).subscribe_next(move |value| target.borrow_mut().push(value));

    assert_eq!(*received.borrow(), vec![18], "both subjects delivered synchronously at subscribe time");
    sub.dispose();
}

#[test]
fn synchronous_completion_before_other_still_completes_only_after_both() {
    let immediate = Observable::empty::<i32>();
    let live = Subject::<i32>::new();
    let completed = Rc::new(Cell::new(false));
    let flag = completed.clone();
    let sub = unsynchronized_combine_latest(immediate, live.as_observable(), |x, y| x + y).subscribe_next_completed(|_| {}, move || flag.set(true));

    assert!(!completed.get(), "the live source has not completed yet");
    live.on_completed();
    assert!(completed.get());
    sub.dispose();
}

#[test]
fn synchronous_error_before_other_terminates_immediately() {
    let error = test_error("error");
    let immediate = Observable::throw::<i32>(error.clone());
    let live = Subject::<i32>::new();
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let target = captured.clone();
    let sub = unsynchronized_combine_latest(immediate, live.as_observable(), |x, y| x + y).subscribe_next_error(|_| {}, move |error| *target.borrow_mut() = Some(error));

    assert!(is_error(&captured, &error));
    sub.dispose();
}
