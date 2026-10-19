//! `ObservableAsPropertyHelperTests`, its partial file `ObservableAsPropertyHelperTests.Callbacks.cs`
//! (`ReactiveUI.Binding.Tests/Bindings`).
//!
//! An error of the source without a `thrown_exceptions` observer goes to
//! `RxState::default_exception_handler()` where upstream rethrows it on the producing thread
//! (DEVIATIONS.md, ReactiveUI); the tests install a recording handler and check that the error
//! reached it.

use super::observable_as_property_helper_tests::SUPPLIED_VALUE;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observer, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The owner of a helper made by `create`.
struct Owner;

/// Installs a handler of the errors nobody handles that records them.
fn record_unhandled_errors() -> Rc<RefCell<Vec<ObservableError>>> {
    let errors: Rc<RefCell<Vec<ObservableError>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = errors.clone();
    RxState::initialize_exception_handler(Observer::create(move |error: ObservableError| sink.borrow_mut().push(error)));
    errors
}

#[test]
fn on_changing_observes_old_value_on_changed_observes_new_value_already_stored() {
    const UPDATED_VALUE: i32 = 2;
    let source = Subject::<i32>::new();
    let slot: Rc<RefCell<Option<ObservableAsPropertyHelper<i32>>>> = Rc::new(RefCell::new(None));
    let changing_saw_value = Rc::new(Cell::new(-1));
    let changed_saw_value = Rc::new(Cell::new(-1));

    let (changed_slot, changed_saw) = (slot.clone(), changed_saw_value.clone());
    let (changing_slot, changing_saw) = (slot.clone(), changing_saw_value.clone());
    let fixture = ObservableAsPropertyHelper::with_options(
        source.as_observable(),
        move |_| {
            let helper = changed_slot.borrow().clone();
            if let Some(helper) = helper {
                changed_saw.set(helper.value());
            }
        },
        Some(Rc::new(move |_: i32| {
            let helper = changing_slot.borrow().clone();
            if let Some(helper) = helper {
                changing_saw.set(helper.value());
            }
        })),
        ObservableAsPropertyInitialValue::Value(SUPPLIED_VALUE),
        true,
        None,
    );
    *slot.borrow_mut() = Some(fixture.clone());

    let _ = fixture.value(); // activates without raising callbacks
    source.on_next(UPDATED_VALUE);

    assert_eq!(changing_saw_value.get(), SUPPLIED_VALUE);
    assert_eq!(changed_saw_value.get(), UPDATED_VALUE);
    assert_eq!(fixture.value(), UPDATED_VALUE);
    fixture.dispose();
    *slot.borrow_mut() = None;
}

#[test]
fn create_with_initial_value_passes_owner_to_on_changed_and_on_changing() {
    const PUSHED_VALUE: i32 = 2;
    let owner = Rc::new(Owner);
    let source = Subject::<i32>::new();
    let changed_owner: Rc<RefCell<Option<Rc<Owner>>>> = Rc::new(RefCell::new(None));
    let changing_owner: Rc<RefCell<Option<Rc<Owner>>>> = Rc::new(RefCell::new(None));
    let changed_value = Rc::new(Cell::new(-1));

    let (changed_sink, value_sink, changing_sink) = (changed_owner.clone(), changed_value.clone(), changing_owner.clone());
    let fixture = ObservableAsPropertyHelper::create(
        source.as_observable(),
        &owner,
        move |o: &Rc<Owner>, v: i32| {
            *changed_sink.borrow_mut() = Some(o.clone());
            value_sink.set(v);
        },
        Some(Rc::new(move |o: &Rc<Owner>, _: i32| *changing_sink.borrow_mut() = Some(o.clone()))),
        ObservableAsPropertyInitialValue::Value(0),
        false,
        None,
    );
    source.on_next(PUSHED_VALUE);

    assert!(Rc::ptr_eq(changed_owner.borrow().as_ref().expect("on_changed ran"), &owner));
    assert!(Rc::ptr_eq(changing_owner.borrow().as_ref().expect("on_changing ran"), &owner));
    assert_eq!(changed_value.get(), PUSHED_VALUE);
    fixture.dispose();
}

#[test]
fn create_with_initial_value_factory_passes_owner_and_evaluates_factory() {
    let owner = Rc::new(Owner);
    let source = Subject::<i32>::new();
    let changed_owner: Rc<RefCell<Option<Rc<Owner>>>> = Rc::new(RefCell::new(None));
    let accessed = Rc::new(Cell::new(false));

    let (changed_sink, factory_accessed) = (changed_owner.clone(), accessed.clone());
    let fixture = ObservableAsPropertyHelper::create(
        source.as_observable(),
        &owner,
        move |o: &Rc<Owner>, _: i32| *changed_sink.borrow_mut() = Some(o.clone()),
        None::<Rc<dyn Fn(&Rc<Owner>, i32)>>,
        ObservableAsPropertyInitialValue::factory(move || {
            factory_accessed.set(true);
            SUPPLIED_VALUE
        }),
        false,
        None,
    );

    assert!(accessed.get());
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    assert!(Rc::ptr_eq(changed_owner.borrow().as_ref().expect("on_changed ran"), &owner));
    fixture.dispose();
}

// Not ported: Create_WithInitialValue_NullOwner_ThrowsArgumentNullException - argument validation

// Not ported: Create_WithInitialValue_NullOnChanged_ThrowsArgumentNullException - argument validation

// Not ported: Create_WithInitialValueFactory_NullOwner_ThrowsArgumentNullException - argument validation

// Not ported: Create_WithInitialValueFactory_NullOnChanged_ThrowsArgumentNullException - argument validation

#[test]
fn create_with_no_on_changing_only_invokes_on_changed() {
    const EXPECTED_DELIVERY_COUNT: i32 = 2;
    let owner = Rc::new(Owner);
    let source = Subject::<i32>::new();
    let changed_count = Rc::new(Cell::new(0));

    let counter = changed_count.clone();
    let fixture = ObservableAsPropertyHelper::create(
        source.as_observable(),
        &owner,
        move |_: &Rc<Owner>, _: i32| counter.set(counter.get() + 1),
        None::<Rc<dyn Fn(&Rc<Owner>, i32)>>,
        ObservableAsPropertyInitialValue::Value(0),
        false,
        None,
    );
    source.on_next(1);

    // The initial value delivery plus the one source value.
    assert_eq!(changed_count.get(), EXPECTED_DELIVERY_COUNT);
    fixture.dispose();
}

#[test]
fn thrown_exceptions_no_observer_rethrows_on_producing_thread() {
    let unhandled = record_unhandled_errors();
    let source = Subject::<i32>::new();
    let fixture = ObservableAsPropertyHelper::with_initial_value(source.as_observable(), |_| {}, 0);

    let error = new_error("die");
    source.on_error(error.clone());

    assert_eq!(unhandled.borrow().len(), 1);
    assert!(Rc::ptr_eq(&unhandled.borrow()[0], &error));
    fixture.dispose();
}

#[test]
fn thrown_exceptions_with_observer_receives_error_value_retains_last() {
    let source = Subject::<i32>::new();
    let fixture = ObservableAsPropertyHelper::with_initial_value(source.as_observable(), |_| {}, 0);
    let errors: Rc<RefCell<Vec<ObservableError>>> = Rc::new(RefCell::new(Vec::new()));
    source.on_next(SUPPLIED_VALUE);

    let sink = errors.clone();
    let subscription = fixture.thrown_exceptions().subscribe_next(move |error| sink.borrow_mut().push(error));
    let error = new_error("die");
    source.on_error(error.clone());

    assert_eq!(errors.borrow().len(), 1);
    assert!(Rc::ptr_eq(&errors.borrow()[0], &error));
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    subscription.dispose();
    fixture.dispose();
}

#[test]
fn thrown_exceptions_after_subscription_disposed_rethrows_again() {
    let unhandled = record_unhandled_errors();
    let source = Subject::<i32>::new();
    let fixture = ObservableAsPropertyHelper::with_initial_value(source.as_observable(), |_| {}, 0);
    let errors: Rc<RefCell<Vec<ObservableError>>> = Rc::new(RefCell::new(Vec::new()));

    let sink = errors.clone();
    let subscription = fixture.thrown_exceptions().subscribe_next(move |error| sink.borrow_mut().push(error));
    subscription.dispose();
    source.on_error(new_error("die again"));

    assert_eq!(unhandled.borrow().len(), 1);
    assert!(errors.borrow().is_empty());
    fixture.dispose();
}

#[test]
fn completion_is_ignored_value_retains_last_and_no_exception_thrown() {
    let source = Subject::<i32>::new();
    let fixture = ObservableAsPropertyHelper::with_initial_value(source.as_observable(), |_| {}, 0);
    source.on_next(SUPPLIED_VALUE);

    source.on_completed();

    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    fixture.dispose();
}
