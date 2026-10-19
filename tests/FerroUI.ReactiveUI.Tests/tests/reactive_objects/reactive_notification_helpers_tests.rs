//! `ReactiveNotificationHelpersTests` (`ReactiveUI.Tests/ReactiveObjects/ReactiveNotificationHelpersTests.cs`).

use ferroui_reactiveui::*;
use ferroui_system::events::Event;
use ferroui_system::{IObservable, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const EXPECTED_HANDLER_CALLS: i32 = 2;

/// A reactive object with one plain property (`TestReactiveObject`).
#[derive(Default)]
struct TestReactiveObject {
    reactive: ReactiveObject,
    value: Cell<i32>,
}
reactive_object!(TestReactiveObject);

#[test]
fn add_property_changing_new_handlers_initializes_and_combines_handlers() {
    let source = TestReactiveObject::default();
    let subscribed = Cell::new(false);
    let handlers: Event<str> = Event::new();
    let calls = Rc::new(Cell::new(0));

    let count = calls.clone();
    ReactiveNotificationHelpers::add_property_changing(&source, &subscribed, &handlers, Rc::new(move |_: &str| count.set(count.get() + 1)));
    let count = calls.clone();
    ReactiveNotificationHelpers::add_property_changing(&source, &subscribed, &handlers, Rc::new(move |_: &str| count.set(count.get() + 1)));
    source.value.set(1);
    handlers.raise("Value");

    assert!(subscribed.get());
    assert_eq!(calls.get(), EXPECTED_HANDLER_CALLS);
}

#[test]
fn add_property_changed_new_handlers_initializes_and_combines_handlers() {
    let source = TestReactiveObject::default();
    let subscribed = Cell::new(false);
    let handlers: Event<str> = Event::new();
    let calls = Rc::new(Cell::new(0));

    let count = calls.clone();
    ReactiveNotificationHelpers::add_property_changed(&source, &subscribed, &handlers, Rc::new(move |_: &str| count.set(count.get() + 1)));
    let count = calls.clone();
    ReactiveNotificationHelpers::add_property_changed(&source, &subscribed, &handlers, Rc::new(move |_: &str| count.set(count.get() + 1)));
    source.value.set(1);
    handlers.raise("Value");

    assert!(subscribed.get());
    assert_eq!(calls.get(), EXPECTED_HANDLER_CALLS);
}

#[test]
fn get_changing_repeated_calls_returns_cached_observable() {
    let source = Rc::new(TestReactiveObject::default());
    let observable: RefCell<Option<ChangeObservable<TestReactiveObject>>> = RefCell::new(None);

    let first = ReactiveNotificationHelpers::get_changing(&source, &observable);
    let second = ReactiveNotificationHelpers::get_changing(&source, &observable);

    assert!(std::ptr::addr_eq(Rc::as_ptr(&first), Rc::as_ptr(&second)));
}

#[test]
fn get_changed_repeated_calls_returns_cached_observable() {
    let source = Rc::new(TestReactiveObject::default());
    let observable: RefCell<Option<ChangeObservable<TestReactiveObject>>> = RefCell::new(None);

    let first = ReactiveNotificationHelpers::get_changed(&source, &observable);
    let second = ReactiveNotificationHelpers::get_changed(&source, &observable);

    assert!(std::ptr::addr_eq(Rc::as_ptr(&first), Rc::as_ptr(&second)));
}

#[test]
fn get_thrown_exceptions_repeated_calls_returns_cached_observable() {
    let source = TestReactiveObject::default();
    let observable: RefCell<Option<Rc<dyn IObservable<ObservableError>>>> = RefCell::new(None);

    let first = ReactiveNotificationHelpers::get_thrown_exceptions(&source, &observable);
    let second = ReactiveNotificationHelpers::get_thrown_exceptions(&source, &observable);

    assert!(std::ptr::addr_eq(Rc::as_ptr(&first), Rc::as_ptr(&second)));
}
