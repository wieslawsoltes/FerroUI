//! `EventObservableTests` (`ReactiveUI.Binding.Tests/Observables/EventObservableTests.cs`).
//!
//! The event is a slot holding the handler, which the tests raise by calling it; a getter that
//! throws is a getter that panics, and the test catches the panic.

use ferroui_reactive::Observer;
use ferroui_reactiveui::*;
use ferroui_system::{IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

const INITIAL_VALUE: &str = "initial";

const EXPECTED_TWO_EMISSIONS: usize = 2;

/// The handler of the event, as the add and remove delegates of the tests keep it.
type HandlerSlot = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

fn handler_slot() -> HandlerSlot {
    Rc::new(RefCell::new(None))
}

/// Raises the event: calls the handler in the slot, if any (`handler?.Invoke(this, EventArgs.Empty)`).
fn raise(slot: &HandlerSlot) {
    let handler = slot.borrow().clone();
    if let Some(handler) = handler {
        handler();
    }
}

/// An observer that adds each value to the returned list (upstream's `AnonymousObserver` with
/// `results.Add`).
fn recording() -> (Rc<RefCell<Vec<String>>>, Rc<dyn IObserver<String>>) {
    let results: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = results.clone();
    (results, Observer::create(move |value: String| sink.borrow_mut().push(value)))
}

/// The add and remove delegates `h => handler = h` and `h => handler = null`.
fn assigning(slot: &HandlerSlot) -> (impl Fn(Rc<dyn Fn()>) + 'static, impl Fn(Rc<dyn Fn()>) + 'static) {
    let (added, removed) = (slot.clone(), slot.clone());
    (move |handler: Rc<dyn Fn()>| *added.borrow_mut() = Some(handler), move |_: Rc<dyn Fn()>| *removed.borrow_mut() = None)
}

// Not ported: Constructor_NullAddHandler_ThrowsArgumentNullException - argument validation

// Not ported: Constructor_NullRemoveHandler_ThrowsArgumentNullException - argument validation

// Not ported: Constructor_NullGetter_ThrowsArgumentNullException - argument validation

// Not ported: Subscribe_NullObserver_ThrowsArgumentNullException - argument validation

#[test]
fn subscribe_emits_initial_value() {
    let (results, observer) = recording();
    let observable = EventObservable::new(|_| {}, |_| {}, || INITIAL_VALUE.to_string(), false);

    let _ = observable.subscribe(observer);

    assert_eq!(results.borrow().len(), 1);
    assert_eq!(results.borrow()[0], INITIAL_VALUE);
}

#[test]
fn subscribe_getter_throws_removes_handler() {
    let slot = handler_slot();
    let fail = Rc::new(Cell::new(true));
    let (results, observer) = recording();
    let (added, removed) = (slot.clone(), slot.clone());
    let failing = fail.clone();
    let observable = EventObservable::new(
        move |handler: Rc<dyn Fn()>| *added.borrow_mut() = Some(handler),
        move |handler: Rc<dyn Fn()>| {
            let mut current = removed.borrow_mut();
            if current.as_ref().is_some_and(|current| Rc::ptr_eq(current, &handler)) {
                *current = None;
            }
        },
        move || {
            if failing.get() {
                panic!("getter");
            }
            INITIAL_VALUE.to_string()
        },
        false,
    );

    let outcome = catch_unwind(AssertUnwindSafe(|| observable.subscribe(observer.clone())));
    assert!(outcome.is_err());

    fail.set(false);
    raise(&slot);

    assert!(slot.borrow().is_none());
    assert!(results.borrow().is_empty());
}

#[test]
fn subscribe_event_fired_emits_new_value() {
    let slot = handler_slot();
    let value = Rc::new(RefCell::new(INITIAL_VALUE.to_string()));
    let (results, observer) = recording();
    let (add, remove) = assigning(&slot);
    let current = value.clone();
    let observable = EventObservable::new(add, remove, move || current.borrow().clone(), false);

    let _ = observable.subscribe(observer);

    *value.borrow_mut() = "updated".to_string();
    raise(&slot);

    assert_eq!(results.borrow().len(), EXPECTED_TWO_EMISSIONS);
    assert_eq!(results.borrow()[1], "updated");
}

#[test]
fn subscribe_distinct_until_changed_suppresses_duplicates() {
    let slot = handler_slot();
    let value = Rc::new(RefCell::new("same".to_string()));
    let (results, observer) = recording();
    let (add, remove) = assigning(&slot);
    let current = value.clone();
    let observable = EventObservable::new(add, remove, move || current.borrow().clone(), true);

    let _ = observable.subscribe(observer);

    // The event with the same value is suppressed.
    raise(&slot);

    assert_eq!(results.borrow().len(), 1);

    // The event with another value is delivered.
    *value.borrow_mut() = "different".to_string();
    raise(&slot);

    assert_eq!(results.borrow().len(), EXPECTED_TWO_EMISSIONS);
    assert_eq!(results.borrow()[1], "different");
}

#[test]
fn subscribe_no_distinct_emits_duplicates() {
    let slot = handler_slot();
    let (results, observer) = recording();
    let (add, remove) = assigning(&slot);
    let observable = EventObservable::new(add, remove, || "same".to_string(), false);

    let _ = observable.subscribe(observer);

    // The event with the same value is delivered.
    raise(&slot);

    assert_eq!(results.borrow().len(), EXPECTED_TWO_EMISSIONS);
}

#[test]
fn dispose_unsubscribes_handler() {
    let slot = handler_slot();
    let (_results, observer) = recording();
    let (added, removed) = (slot.clone(), slot.clone());
    let observable = EventObservable::new(
        move |handler: Rc<dyn Fn()>| *added.borrow_mut() = Some(handler),
        move |handler: Rc<dyn Fn()>| {
            let mut current = removed.borrow_mut();
            if !current.as_ref().is_some_and(|current| Rc::ptr_eq(current, &handler)) {
                return;
            }
            *current = None;
        },
        || INITIAL_VALUE.to_string(),
        false,
    );

    let subscription = observable.subscribe(observer);

    subscription.dispose();

    assert!(slot.borrow().is_none());
}

#[test]
fn dispose_event_after_dispose_ignored() {
    let slot = handler_slot();
    let value = Rc::new(RefCell::new(INITIAL_VALUE.to_string()));
    let (results, observer) = recording();
    let added = slot.clone();
    let current = value.clone();
    let observable = EventObservable::new(move |handler: Rc<dyn Fn()>| *added.borrow_mut() = Some(handler), |_| {}, move || current.borrow().clone(), false);

    let subscription = observable.subscribe(observer);

    // The handler is kept before the disposal.
    let saved_handler = slot.borrow().clone();
    subscription.dispose();

    *value.borrow_mut() = "after-dispose".to_string();
    if let Some(handler) = saved_handler {
        handler();
    }

    assert_eq!(results.borrow().len(), 1);
}

#[test]
fn dispose_called_twice_is_safe() {
    let remove_count = Rc::new(Cell::new(0));
    let removals = remove_count.clone();
    let observable = EventObservable::new(|_| {}, move |_| removals.set(removals.get() + 1), || "test".to_string(), false);

    let subscription = observable.subscribe(Observer::create(|_: String| {}));

    subscription.dispose();
    subscription.dispose();

    assert_eq!(remove_count.get(), 1);
}
