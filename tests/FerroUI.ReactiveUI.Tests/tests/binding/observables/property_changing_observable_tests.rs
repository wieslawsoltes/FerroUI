//! `PropertyChangingObservableTests`
//! (`ReactiveUI.Binding.Tests/Observables/PropertyChangingObservableTests.cs`).
//!
//! A getter that throws is a getter that panics; the tests catch the panic. Upstream's `null`
//! property name has no counterpart (the name is a `&str`): the empty name stands for both.

use crate::tests::binding::test_models::TestViewModel;
use ferroui_reactive::Observer;
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanging;
use ferroui_system::events::Event;
use ferroui_system::{IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// The initial name value assigned to the view model under test.
const INITIAL_NAME: &str = "Alice";

/// The expected number of emissions when the initial value plus one change are observed.
const EXPECTED_TWO_EMISSIONS: usize = 2;

/// The expected number of emissions after a third notification.
const EXPECTED_THREE_EMISSIONS: usize = 3;

/// A sample age value assigned to an unrelated property.
const SAMPLE_AGE: i32 = 26;

/// An observer that adds each value to the returned list (upstream's `AnonymousObserver` with
/// `results.Add`).
fn recording<T: 'static>() -> (Rc<RefCell<Vec<T>>>, Rc<dyn IObserver<T>>) {
    let results: Rc<RefCell<Vec<T>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = results.clone();
    (results, Observer::create(move |value: T| sink.borrow_mut().push(value)))
}

fn view_model(name: &str) -> Rc<TestViewModel> {
    let vm = TestViewModel::new();
    vm.set_name(name.to_string());
    vm
}

fn name_of(vm: &Rc<TestViewModel>) -> PropertyChangingObservable<TestViewModel, String> {
    PropertyChangingObservable::new(vm, "Name", |x: &Rc<TestViewModel>| x.name())
}

// Not ported: OnNext_WaitsForCompetingWriter_DeliversCapturedValueWithoutOverlap - real threads

#[test]
fn on_next_reentrant_write_delivers_before_nested_write() {
    let source = view_model(INITIAL_NAME);
    let values: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let value_seen_during_nested_notification = Rc::new(RefCell::new(String::new()));
    let observable = name_of(&source);

    let (sink, seen, weak_source) = (values.clone(), value_seen_during_nested_notification.clone(), Rc::downgrade(&source));
    let subscription = observable.subscribe(Observer::create(move |value: String| {
        let count = {
            let mut values = sink.borrow_mut();
            values.push(value);
            values.len()
        };
        let Some(source) = weak_source.upgrade() else {
            return;
        };
        if count == 1 {
            source.set_name("Bob".to_string());
        } else {
            *seen.borrow_mut() = source.name();
        }
    }));

    assert_eq!(values.borrow().len(), EXPECTED_TWO_EMISSIONS);
    assert_eq!(values.borrow()[1], INITIAL_NAME);
    assert_eq!(*value_seen_during_nested_notification.borrow(), INITIAL_NAME);
    assert_eq!(source.name(), "Bob");
    subscription.dispose();
}

// Not ported: Subscribe_NullObserver_ThrowsArgumentNullException - argument validation

// Not ported: Constructor_NullSource_ThrowsArgumentNullException - argument validation

// Not ported: Constructor_NullPropertyName_ThrowsArgumentNullException - argument validation

// Not ported: Constructor_NullGetter_ThrowsArgumentNullException - argument validation

#[test]
fn subscribe_emits_current_value_immediately() {
    let vm = view_model(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = name_of(&vm);

    let _ = observable.subscribe(observer);

    assert_eq!(results.borrow().len(), 1);
    assert_eq!(results.borrow()[0], INITIAL_NAME);
}

#[test]
fn subscribe_getter_throws_detaches_handler() {
    let vm = ThrowingGetterViewModel::new();
    let (results, observer) = recording::<String>();
    let observable = PropertyChangingObservable::new(&vm, "Name", |x: &Rc<ThrowingGetterViewModel>| x.name());

    let outcome = catch_unwind(AssertUnwindSafe(|| observable.subscribe(observer)));
    assert!(outcome.is_err());

    vm.fail.set(false);
    vm.raise_property_changing("Name");

    assert_eq!(vm.handler_count(), 0);
    assert!(results.borrow().is_empty());
}

// Not ported: Subscribe_GetterThrowsDuringAttachment_DetachesHandler - needs an event whose add
// accessor runs code (the port's `Event` attaches a handler without calling back)

#[test]
fn property_changing_triggers_emission() {
    let vm = view_model(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = name_of(&vm);

    let _ = observable.subscribe(observer);
    vm.set_name("Bob".to_string());

    // Should have 2 values: initial "Alice", and "Alice" again when PropertyChanging fired before it became "Bob"
    assert_eq!(results.borrow().len(), EXPECTED_TWO_EMISSIONS);
    assert_eq!(results.borrow()[0], INITIAL_NAME);
    assert_eq!(results.borrow()[1], INITIAL_NAME);
}

#[test]
fn property_changing_wrong_property_does_not_trigger_emission() {
    let vm = view_model(INITIAL_NAME);
    vm.set_age(SAMPLE_AGE);
    let (results, observer) = recording::<String>();
    let observable = name_of(&vm);

    let _ = observable.subscribe(observer);
    vm.set_age(SAMPLE_AGE);

    assert_eq!(results.borrow().len(), 1);
    assert_eq!(results.borrow()[0], INITIAL_NAME);
}

#[test]
fn property_changing_null_or_empty_triggers_emission() {
    let vm = ManualTestViewModel::new(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = PropertyChangingObservable::new(&vm, "Name", |x: &Rc<ManualTestViewModel>| x.name());

    let _ = observable.subscribe(observer);

    vm.raise_property_changing("");
    assert_eq!(results.borrow().len(), EXPECTED_TWO_EMISSIONS);

    // Upstream raises `null` here; the port has no null name, the empty one means every property.
    vm.raise_property_changing("");
    assert_eq!(results.borrow().len(), EXPECTED_THREE_EMISSIONS);
}

#[test]
fn dispose_removes_event_handler() {
    let vm = view_model(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = name_of(&vm);

    let subscription = observable.subscribe(observer);
    subscription.dispose();

    vm.set_name("Bob".to_string());
    assert_eq!(results.borrow().len(), 1);
    assert_eq!(results.borrow()[0], INITIAL_NAME);
}

#[test]
fn dispose_called_twice_no_exception() {
    let vm = view_model(INITIAL_NAME);
    let (_results, observer) = recording::<String>();
    let observable = name_of(&vm);

    let subscription = observable.subscribe(observer);
    subscription.dispose();
    subscription.dispose();
}

#[test]
fn property_changing_after_dispose_does_not_throw() {
    let vm = ManualTestViewModel::new(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = PropertyChangingObservable::new(&vm, "Name", |x: &Rc<ManualTestViewModel>| x.name());

    let subscription = observable.subscribe(observer);
    subscription.dispose();

    // Fire event after dispose - should be safely ignored
    vm.raise_property_changing("Name");

    // Only the initial value should have been emitted
    assert_eq!(results.borrow().len(), 1);
}

#[test]
fn on_property_changing_handler_captured_before_disposal_does_not_deliver() {
    let vm = ManualTestViewModel::new(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = PropertyChangingObservable::new(&vm, "Name", |x: &Rc<ManualTestViewModel>| x.name());

    let subscription = observable.subscribe(observer);

    let disposing = subscription.clone();
    vm.raise_property_changing_with("Name", move || disposing.dispose());
    assert_eq!(results.borrow().len(), 1);
    subscription.dispose();
}

// Not ported: PropertyChanging_ConcurrentDispose_ObserverNullGuardDoesNotThrow - real threads

/// A view model whose notification is raised by hand (`ManualTestViewModel`).
struct ManualTestViewModel {
    name: RefCell<String>,
    property_changing: Event<str>,
}

impl ManualTestViewModel {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(name.to_string()), property_changing: Event::new() })
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    fn raise_property_changing(&self, name: &str) {
        self.raise_property_changing_with(name, || {});
    }

    /// Raises `PropertyChanging` to the handlers attached before `before_raise` runs
    /// (`RaisePropertyChanging(string?, Action?)`).
    fn raise_property_changing_with(&self, name: &str, before_raise: impl FnOnce()) {
        let handlers = self.property_changing.snapshot();
        before_raise();
        for (_, handler) in handlers.iter() {
            handler(name);
        }
    }
}

impl INotifyPropertyChanging for ManualTestViewModel {
    fn property_changing(&self) -> &Event<str> {
        &self.property_changing
    }
}

/// A view model whose getter panics on demand and which counts its attached handlers
/// (`ThrowingGetterViewModel`).
struct ThrowingGetterViewModel {
    fail: Cell<bool>,
    property_changing: Event<str>,
}

impl ThrowingGetterViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { fail: Cell::new(true), property_changing: Event::new() })
    }

    fn name(&self) -> String {
        if self.fail.get() {
            panic!("getter");
        }
        INITIAL_NAME.to_string()
    }

    fn handler_count(&self) -> usize {
        self.property_changing.handler_count()
    }

    fn raise_property_changing(&self, name: &str) {
        self.property_changing.raise(name);
    }
}

impl INotifyPropertyChanging for ThrowingGetterViewModel {
    fn property_changing(&self) -> &Event<str> {
        &self.property_changing
    }
}
