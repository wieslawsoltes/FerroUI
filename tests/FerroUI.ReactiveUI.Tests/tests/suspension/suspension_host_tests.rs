//! `SuspensionHostTests` (`ReactiveUI.Tests/Suspension/SuspensionHostTests.cs`).

use super::DummyAppState;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::IDisposable;
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

fn dummy_state() -> Rc<dyn Any> {
    Rc::new(DummyAppState)
}

#[test]
fn app_state_property_changed_raises_notification() {
    let host = SuspensionHost::new();
    let property_changed = Rc::new(Cell::new(false));
    let raised = property_changed.clone();
    host.property_changed().add(Rc::new(move |name: &str| {
        if name != "AppState" {
            return;
        }
        raised.set(true);
    }));

    host.set_app_state(Some(dummy_state()));

    assert!(property_changed.get());
    host.dispose();
}

#[test]
fn app_state_set_and_get_returns_correct_value() {
    let host = SuspensionHost::new();
    let state = dummy_state();

    host.set_app_state(Some(state.clone()));

    assert!(host.app_state().is_some_and(|current| Rc::ptr_eq(&current, &state)));
    host.dispose();
}

#[test]
fn constructor_default_observables_throw_exception_on_subscribe() {
    let host = SuspensionHost::new();
    let got_error = Rc::new(Cell::new(false));
    let seen = got_error.clone();

    let _ = host.is_launching_new().subscribe_next_error(|_| {}, move |_| seen.set(true));

    assert!(got_error.get());
    host.dispose();
}

#[test]
fn create_new_app_state_set_and_get_returns_correct_func() {
    let host = SuspensionHost::new();
    host.set_create_new_app_state(Some(Rc::new(dummy_state)));

    let factory = host.create_new_app_state();
    assert!(factory.is_some());
    assert!(factory.unwrap()().downcast::<DummyAppState>().is_ok());
    host.dispose();
}

#[test]
fn dispose_called_multiple_times_does_not_throw() {
    let host = SuspensionHost::new();
    host.dispose();
    host.dispose();
}

#[test]
fn dispose_disposes_all_subjects() {
    // Set observables before disposal
    let host = SuspensionHost::new();
    host.set_is_launching_new(Observable::return_(()));
    host.set_is_resuming(Observable::return_(()));
    host.set_is_unpausing(Observable::return_(()));
    host.set_should_persist_state(Observable::return_(Disposable::empty()));
    host.set_should_invalidate_state(Observable::return_(()));

    host.dispose();

    // The host has been disposed successfully.
}

#[test]
fn is_launching_new_set_and_get_returns_correct_observable() {
    let host = SuspensionHost::new();
    let was_triggered = Rc::new(Cell::new(false));
    let seen = was_triggered.clone();

    host.set_is_launching_new(Observable::return_(()));
    let subscription = host.is_launching_new().subscribe_next(move |_| seen.set(true));

    assert!(was_triggered.get());
    subscription.dispose();
    host.dispose();
}

#[test]
fn is_resuming_set_and_get_returns_correct_observable() {
    let host = SuspensionHost::new();
    let was_triggered = Rc::new(Cell::new(false));
    let seen = was_triggered.clone();

    host.set_is_resuming(Observable::return_(()));
    let subscription = host.is_resuming().subscribe_next(move |_| seen.set(true));

    assert!(was_triggered.get());
    subscription.dispose();
    host.dispose();
}

#[test]
fn is_unpausing_set_and_get_returns_correct_observable() {
    let host = SuspensionHost::new();
    let was_triggered = Rc::new(Cell::new(false));
    let seen = was_triggered.clone();

    host.set_is_unpausing(Observable::return_(()));
    let subscription = host.is_unpausing().subscribe_next(move |_| seen.set(true));

    assert!(was_triggered.get());
    subscription.dispose();
    host.dispose();
}

#[test]
fn should_invalidate_state_set_and_get_returns_correct_observable() {
    let host = SuspensionHost::new();
    let was_triggered = Rc::new(Cell::new(false));
    let seen = was_triggered.clone();

    host.set_should_invalidate_state(Observable::return_(()));
    let subscription = host.should_invalidate_state().subscribe_next(move |_| seen.set(true));

    assert!(was_triggered.get());
    subscription.dispose();
    host.dispose();
}

#[test]
fn should_persist_state_set_and_get_returns_correct_observable() {
    let host = SuspensionHost::new();
    let was_triggered = Rc::new(Cell::new(false));
    let seen = was_triggered.clone();

    host.set_should_persist_state(Observable::return_(Disposable::empty()));
    let subscription = host.should_persist_state().subscribe_next(move |_| seen.set(true));

    assert!(was_triggered.get());
    subscription.dispose();
    host.dispose();
}
