//! `PropertyObservableTests` (`ReactiveUI.Binding.Tests/Observables/PropertyObservableTests.cs`).
//!
//! A getter that throws is a getter that panics; the tests catch the panic. Upstream's `null`
//! property name has no counterpart (the name is a `&str`): the empty name stands for both.

use crate::tests::binding::test_models::TestViewModel;
use ferroui_reactive::Observer;
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanged;
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

fn name_of(vm: &Rc<TestViewModel>) -> PropertyObservable<TestViewModel, String> {
    PropertyObservable::new(vm, "Name", |x: &Rc<TestViewModel>| x.name(), false)
}

// Not ported: OnNext_WaitsForCompetingWriter_DeliversLatestValueWithoutOverlap - real threads

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
    let observable = PropertyObservable::new(&vm, "Name", |x: &Rc<ThrowingGetterViewModel>| x.name(), false);

    let outcome = catch_unwind(AssertUnwindSafe(|| observable.subscribe(observer)));
    assert!(outcome.is_err());

    vm.fail.set(false);
    vm.raise_property_changed("Name");

    assert_eq!(vm.handler_count(), 0);
    assert!(results.borrow().is_empty());
}

#[test]
fn property_changed_triggers_emission() {
    let vm = view_model(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = name_of(&vm);

    let _ = observable.subscribe(observer);
    vm.set_name("Bob".to_string());

    assert_eq!(results.borrow().len(), EXPECTED_TWO_EMISSIONS);
    assert_eq!(results.borrow()[1], "Bob");
}

#[test]
fn property_changed_wrong_property_does_not_trigger_emission() {
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
fn property_changed_null_or_empty_triggers_emission() {
    let vm = ManualTestViewModel::new(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = PropertyObservable::new(&vm, "Name", |x: &Rc<ManualTestViewModel>| x.name(), false);

    let _ = observable.subscribe(observer);

    vm.raise_property_changed("");
    assert_eq!(results.borrow().len(), EXPECTED_TWO_EMISSIONS);

    // Upstream raises `null` here; the port has no null name, the empty one means every property.
    vm.raise_property_changed("");
    assert_eq!(results.borrow().len(), EXPECTED_THREE_EMISSIONS);
}

#[test]
fn distinct_until_changed_true_suppresses_duplicates() {
    let vm = ManualTestViewModel::new(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = PropertyObservable::new(&vm, "Name", |x: &Rc<ManualTestViewModel>| x.name(), true);

    let _ = observable.subscribe(observer);
    vm.set_name(INITIAL_NAME); // Same value
    vm.raise_property_changed("Name");

    assert_eq!(results.borrow().len(), 1);
}

#[test]
fn distinct_until_changed_false_allows_duplicates() {
    let vm = ManualTestViewModel::new(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = PropertyObservable::new(&vm, "Name", |x: &Rc<ManualTestViewModel>| x.name(), false);

    let _ = observable.subscribe(observer);
    vm.set_name(INITIAL_NAME); // Same value
    vm.raise_property_changed("Name");

    assert_eq!(results.borrow().len(), EXPECTED_TWO_EMISSIONS);
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
fn property_changed_after_dispose_does_not_throw() {
    let vm = ManualTestViewModel::new(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = PropertyObservable::new(&vm, "Name", |x: &Rc<ManualTestViewModel>| x.name(), false);

    let subscription = observable.subscribe(observer);
    subscription.dispose();

    // Fire event after dispose - should be safely ignored
    vm.raise_property_changed("Name");

    // Only the initial value should have been emitted
    assert_eq!(results.borrow().len(), 1);
}

#[test]
fn on_property_changed_handler_captured_before_disposal_does_not_deliver() {
    let vm = ManualTestViewModel::new(INITIAL_NAME);
    let (results, observer) = recording::<String>();
    let observable = PropertyObservable::new(&vm, "Name", |x: &Rc<ManualTestViewModel>| x.name(), false);

    let subscription = observable.subscribe(observer);

    let disposing = subscription.clone();
    vm.raise_property_changed_with("Name", move || disposing.dispose());
    assert_eq!(results.borrow().len(), 1);
    subscription.dispose();
}

// Not ported: PropertyChanged_ConcurrentDispose_ObserverNullGuardDoesNotThrow - real threads

/// A view model whose notification is raised by hand (`ManualTestViewModel`).
struct ManualTestViewModel {
    name: RefCell<String>,
    property_changed: Event<str>,
}

impl ManualTestViewModel {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(name.to_string()), property_changed: Event::new() })
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    /// Sets the name without raising anything.
    fn set_name(&self, name: &str) {
        *self.name.borrow_mut() = name.to_string();
    }

    fn raise_property_changed(&self, name: &str) {
        self.raise_property_changed_with(name, || {});
    }

    /// Raises `PropertyChanged` to the handlers attached before `before_raise` runs
    /// (`RaisePropertyChanged(string?, Action?)`).
    fn raise_property_changed_with(&self, name: &str, before_raise: impl FnOnce()) {
        let handlers = self.property_changed.snapshot();
        before_raise();
        for (_, handler) in handlers.iter() {
            handler(name);
        }
    }
}

impl INotifyPropertyChanged for ManualTestViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

/// A view model whose getter panics on demand and which counts its attached handlers
/// (`ThrowingGetterViewModel`).
struct ThrowingGetterViewModel {
    fail: Cell<bool>,
    property_changed: Event<str>,
}

impl ThrowingGetterViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { fail: Cell::new(true), property_changed: Event::new() })
    }

    fn name(&self) -> String {
        if self.fail.get() {
            panic!("getter");
        }
        INITIAL_NAME.to_string()
    }

    fn handler_count(&self) -> usize {
        self.property_changed.handler_count()
    }

    fn raise_property_changed(&self, name: &str) {
        self.property_changed.raise(name);
    }
}

impl INotifyPropertyChanged for ThrowingGetterViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}
