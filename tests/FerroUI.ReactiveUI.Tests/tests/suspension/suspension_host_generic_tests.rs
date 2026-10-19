//! `SuspensionHostGenericTests` (`ReactiveUI.Tests/Suspension/SuspensionHostGenericTests.cs`).

use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::{IDisposable, IObservable};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

const SAMPLE_STATE_VALUE: i32 = 42;
const FACTORY_STATE_VALUE: i32 = 99;
const SECOND_STATE_VALUE: i32 = 2;
const EXPECTED_EMISSION_COUNT: usize = 2;

/// The state of the tests (`DummyAppState`).
#[derive(Debug, Default)]
struct DummyAppState {
    #[allow(dead_code)]
    value: i32,
}

/// A state of another type (`OtherAppState`).
#[derive(Debug, Default)]
struct OtherAppState {
    #[allow(dead_code)]
    name: Option<String>,
}

fn state(value: i32) -> Rc<DummyAppState> {
    Rc::new(DummyAppState { value })
}

fn errors_on_subscribe<T: 'static>(source: Rc<dyn IObservable<T>>) -> bool {
    let got_error = Rc::new(Cell::new(false));
    let seen = got_error.clone();
    let _ = source.subscribe_next_error(|_| {}, move |_| seen.set(true));
    got_error.get()
}

fn triggers<T: 'static>(source: Rc<dyn IObservable<T>>) -> bool {
    let was_triggered = Rc::new(Cell::new(false));
    let seen = was_triggered.clone();
    let subscription = source.subscribe_next(move |_| seen.set(true));
    subscription.dispose();
    was_triggered.get()
}

#[test]
fn constructor_default_observables_throw_exception_on_subscribe() {
    let host = SuspensionHostOf::<DummyAppState>::new();

    assert!(errors_on_subscribe(host.is_launching_new()));
    assert!(errors_on_subscribe(host.is_resuming()));
    assert!(errors_on_subscribe(host.is_unpausing()));
    assert!(errors_on_subscribe(host.is_continuing()));
    assert!(errors_on_subscribe(host.should_invalidate_state()));
    assert!(errors_on_subscribe(host.should_persist_state()));
    host.dispose();
}

#[test]
fn is_launching_new_set_and_get_returns_correct_observable() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.set_is_launching_new(Observable::return_(()));
    assert!(triggers(host.is_launching_new()));
    host.dispose();
}

// Not ported: IsLaunchingNew_SetNull_ThrowsArgumentException - argument validation

#[test]
fn is_resuming_set_and_get_returns_correct_observable() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.set_is_resuming(Observable::return_(()));
    assert!(triggers(host.is_resuming()));
    host.dispose();
}

// Not ported: IsResuming_SetNull_ThrowsArgumentException - argument validation

#[test]
fn is_unpausing_set_and_get_returns_correct_observable() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.set_is_unpausing(Observable::return_(()));
    assert!(triggers(host.is_unpausing()));
    host.dispose();
}

// Not ported: IsUnpausing_SetNull_ThrowsArgumentException - argument validation

#[test]
fn is_continuing_set_and_get_returns_correct_observable() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.set_is_continuing(Observable::return_(()));
    assert!(triggers(host.is_continuing()));
    host.dispose();
}

// Not ported: IsContinuing_SetNull_ThrowsArgumentException - argument validation

#[test]
fn should_persist_state_set_and_get_returns_correct_observable() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.set_should_persist_state(Observable::return_(Disposable::empty()));
    assert!(triggers(host.should_persist_state()));
    host.dispose();
}

// Not ported: ShouldPersistState_SetNull_ThrowsArgumentException - argument validation

#[test]
fn should_invalidate_state_set_and_get_returns_correct_observable() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.set_should_invalidate_state(Observable::return_(()));
    assert!(triggers(host.should_invalidate_state()));
    host.dispose();
}

// Not ported: ShouldInvalidateState_SetNull_ThrowsArgumentException - argument validation

#[test]
fn app_state_value_set_and_get_returns_correct_value() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let state = state(SAMPLE_STATE_VALUE);

    host.set_app_state_value(Some(state.clone()));

    assert!(host.app_state_value().is_some_and(|current| Rc::ptr_eq(&current, &state)));
    host.dispose();
}

#[test]
fn app_state_value_property_changed_raises_notification() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let property_changed = Rc::new(Cell::new(false));
    let raised = property_changed.clone();
    host.property_changed().add(Rc::new(move |name: &str| {
        if name == "AppStateValue" {
            raised.set(true);
        }
    }));

    host.set_app_state_value(Some(Rc::new(DummyAppState::default())));

    assert!(property_changed.get());
    host.dispose();
}

#[test]
fn app_state_value_changed_emits_when_app_state_value_is_set() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let received_states = Rc::new(RefCell::new(Vec::new()));
    let log = received_states.clone();
    let subscription = host.app_state_value_changed().observe_on(ImmediateScheduler::instance()).subscribe_next(move |state| log.borrow_mut().push(state));

    let state1 = state(1);
    host.set_app_state_value(Some(state1.clone()));
    let state2 = state(SECOND_STATE_VALUE);
    host.set_app_state_value(Some(state2.clone()));

    let received_states = received_states.borrow();
    assert_eq!(received_states.len(), EXPECTED_EMISSION_COUNT);
    assert!(received_states[0].as_ref().is_some_and(|state| Rc::ptr_eq(state, &state1)));
    assert!(received_states[1].as_ref().is_some_and(|state| Rc::ptr_eq(state, &state2)));
    subscription.dispose();
    host.dispose();
}

#[test]
fn app_state_value_changed_emits_null() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let received_states = Rc::new(RefCell::new(Vec::new()));
    let log = received_states.clone();
    let subscription = host.app_state_value_changed().observe_on(ImmediateScheduler::instance()).subscribe_next(move |state| log.borrow_mut().push(state));

    host.set_app_state_value(Some(Rc::new(DummyAppState::default())));
    host.set_app_state_value(None);

    let received_states = received_states.borrow();
    assert_eq!(received_states.len(), EXPECTED_EMISSION_COUNT);
    assert!(received_states[1].is_none());
    subscription.dispose();
    host.dispose();
}

#[test]
fn create_new_app_state_typed_set_and_get_returns_correct_func() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let expected_state = state(FACTORY_STATE_VALUE);
    let created = expected_state.clone();

    host.set_create_new_app_state_typed(Some(Rc::new(move || created.clone())));

    let factory = host.create_new_app_state_typed();
    assert!(factory.is_some());
    assert!(Rc::ptr_eq(&factory.unwrap()(), &expected_state));
    host.dispose();
}

#[test]
fn create_new_app_state_typed_property_changed_raises_notification() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let property_changed = Rc::new(Cell::new(false));
    let raised = property_changed.clone();
    host.property_changed().add(Rc::new(move |name: &str| {
        if name == "CreateNewAppStateTyped" {
            raised.set(true);
        }
    }));

    host.set_create_new_app_state_typed(Some(Rc::new(|| Rc::new(DummyAppState::default()))));

    assert!(property_changed.get());
    host.dispose();
}

#[test]
fn i_suspension_host_app_state_get_projects_from_typed_value() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let state = state(SAMPLE_STATE_VALUE);
    host.set_app_state_value(Some(state.clone()));

    let untyped = ISuspensionHost::app_state(&*host);

    assert!(untyped.is_some_and(|untyped| std::ptr::addr_eq(Rc::as_ptr(&untyped), Rc::as_ptr(&state))));
    host.dispose();
}

#[test]
fn i_suspension_host_app_state_set_with_valid_value_updates_typed_property() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let state = state(SAMPLE_STATE_VALUE);

    ISuspensionHost::set_app_state(&*host, Some(state.clone() as Rc<dyn Any>));

    assert!(host.app_state_value().is_some_and(|current| Rc::ptr_eq(&current, &state)));
    host.dispose();
}

#[test]
fn i_suspension_host_app_state_set_null_sets_typed_property_to_default() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.set_app_state_value(Some(Rc::new(DummyAppState::default())));

    ISuspensionHost::set_app_state(&*host, None);

    assert!(host.app_state_value().is_none());
    host.dispose();
}

#[test]
fn i_suspension_host_app_state_set_invalid_type_throws_invalid_cast_exception() {
    let host = SuspensionHostOf::<DummyAppState>::new();

    let outcome = catch_unwind(AssertUnwindSafe(|| ISuspensionHost::set_app_state(&*host, Some(Rc::new(OtherAppState::default()) as Rc<dyn Any>))));

    assert!(outcome.is_err());
}

#[test]
fn i_suspension_host_create_new_app_state_get_projects_from_typed_factory() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let expected_state = state(FACTORY_STATE_VALUE);
    let created = expected_state.clone();
    host.set_create_new_app_state_typed(Some(Rc::new(move || created.clone())));

    let factory = ISuspensionHost::create_new_app_state(&*host);

    assert!(factory.is_some());
    assert!(std::ptr::addr_eq(Rc::as_ptr(&factory.unwrap()()), Rc::as_ptr(&expected_state)));
    host.dispose();
}

#[test]
fn i_suspension_host_create_new_app_state_get_when_typed_is_null_returns_null() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.set_create_new_app_state_typed(None);

    assert!(ISuspensionHost::create_new_app_state(&*host).is_none());
    host.dispose();
}

#[test]
fn i_suspension_host_create_new_app_state_set_with_valid_factory_updates_typed_property() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let expected_state = state(FACTORY_STATE_VALUE);
    let created = expected_state.clone();

    ISuspensionHost::set_create_new_app_state(&*host, Some(Rc::new(move || created.clone() as Rc<dyn Any>)));

    let typed_factory = host.create_new_app_state_typed();
    assert!(typed_factory.is_some());
    assert!(Rc::ptr_eq(&typed_factory.unwrap()(), &expected_state));
    host.dispose();
}

#[test]
fn i_suspension_host_create_new_app_state_set_null_sets_typed_property_to_null() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.set_create_new_app_state_typed(Some(Rc::new(|| Rc::new(DummyAppState::default()))));

    ISuspensionHost::set_create_new_app_state(&*host, None);

    assert!(host.create_new_app_state_typed().is_none());
    host.dispose();
}

#[test]
fn i_suspension_host_create_new_app_state_set_invalid_factory_throws_invalid_cast_exception() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    ISuspensionHost::set_create_new_app_state(&*host, Some(Rc::new(|| Rc::new(OtherAppState::default()) as Rc<dyn Any>)));
    let factory = ISuspensionHost::create_new_app_state(&*host).expect("the factory was set");

    let outcome = catch_unwind(AssertUnwindSafe(|| factory()));

    assert!(outcome.is_err());
}

#[test]
fn dispose_called_multiple_times_does_not_throw() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.dispose();
    host.dispose();
}

#[test]
fn dispose_disposes_all_subjects() {
    // Set observables before disposal
    let host = SuspensionHostOf::<DummyAppState>::new();
    host.set_is_launching_new(Observable::return_(()));
    host.set_is_resuming(Observable::return_(()));
    host.set_is_unpausing(Observable::return_(()));
    host.set_is_continuing(Observable::return_(()));
    host.set_should_persist_state(Observable::return_(Disposable::empty()));
    host.set_should_invalidate_state(Observable::return_(()));

    host.dispose();

    // The host has been disposed successfully.
}

#[test]
fn app_state_value_changed_subscription_works_before_dispose() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let received_count = Rc::new(Cell::new(0));
    let count = received_count.clone();
    let subscription = host.app_state_value_changed().observe_on(ImmediateScheduler::instance()).subscribe_next(move |_| count.set(count.get() + 1));

    // Set values and verify subscription works
    host.set_app_state_value(Some(Rc::new(DummyAppState::default())));
    host.set_app_state_value(Some(state(1)));
    assert_eq!(received_count.get(), EXPECTED_EMISSION_COUNT as i32);

    // Dispose the host
    host.dispose();
    subscription.dispose();
}

#[test]
fn observable_properties_can_be_replaced_dynamically() {
    let host = SuspensionHostOf::<DummyAppState>::new();
    let launch_count = Rc::new(Cell::new(0));

    // Set initial observable
    let count = launch_count.clone();
    host.set_is_launching_new(Observable::return_(()).do_(move |_| count.set(count.get() + 1)));

    // Subscribe and verify initial observable works
    let subscription = host.is_launching_new().subscribe_ignore();

    // Replace the observable with a new one
    let count = launch_count.clone();
    host.set_is_launching_new(Observable::return_(()).do_(move |_| count.set(count.get() + 1)));

    // The existing subscription sees the new observable through the replay and the switch.
    const MINIMUM_LAUNCH_COUNT: i32 = 2;
    assert!(launch_count.get() >= MINIMUM_LAUNCH_COUNT);
    subscription.dispose();
    host.dispose();
}
