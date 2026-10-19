//! `ActivatingViewTests` (`ReactiveUI.Tests/Activation/ActivatingViewTests.cs`).
//!
//! Upstream builds an application with the core services and a custom registration of the
//! fetcher; here the fetcher is registered in the locator of the thread
//! ([`register_activating_view_fetcher`]). Upstream resets the cache of the fetchers
//! (`ResetActivationFetcherCacheForTesting`); the port has no cache: the fetcher is chosen each
//! time a view sets up its activation.

use super::{register_activating_view_fetcher, ActivatingView, ActivatingViewModel};
use ferroui_reactiveui::binding::interfaces::IActivatableView;
use ferroui_reactiveui::*;
use ferroui_system::IObserver;
use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// A view that no fetcher handles (`DefaultDesignModeActivatableView`).
struct DefaultDesignModeActivatableView {
    reactive: ReactiveObject,
}
reactive_object!(DefaultDesignModeActivatableView);

impl IActivatableView for DefaultDesignModeActivatableView {}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => payload.downcast::<&str>().map(|message| (*message).to_owned()).unwrap_or_default(),
    }
}

#[test]
fn get_is_design_mode_returns_false_by_default() {
    let fixture = Rc::new(DefaultDesignModeActivatableView { reactive: ReactiveObject::new() });
    assert!(!fixture.get_is_design_mode());
}

#[test]
fn when_activated_without_fetcher_throws_when_default_design_mode_is_false() {
    let fixture = Rc::new(DefaultDesignModeActivatableView { reactive: ReactiveObject::new() });

    let outcome = catch_unwind(AssertUnwindSafe(|| fixture.when_activated_disposables(|_| {})));

    let message = panic_message(outcome.err().expect("activating a view no fetcher handles panics"));
    assert!(message.contains("Don't know how to detect when"));
}

#[test]
fn activating_view_smoke_test() {
    register_activating_view_fetcher();
    let vm = ActivatingViewModel::new();
    let fixture = ActivatingView::new();
    fixture.set_view_model(Some(vm.clone()));
    assert_eq!(vm.is_active_count(), 0);
    assert_eq!(fixture.is_active_count(), 0);

    fixture.loaded().on_next(());
    assert_eq!(vm.is_active_count(), 1);
    assert_eq!(fixture.is_active_count(), 1);

    fixture.unloaded().on_next(());
    assert_eq!(vm.is_active_count(), 0);
    assert_eq!(fixture.is_active_count(), 0);
}

#[test]
fn can_unload_and_load_view_again() {
    register_activating_view_fetcher();
    let vm = ActivatingViewModel::new();
    let fixture = ActivatingView::new();
    fixture.set_view_model(Some(vm.clone()));
    assert_eq!(vm.is_active_count(), 0);
    assert_eq!(fixture.is_active_count(), 0);

    fixture.loaded().on_next(());
    assert_eq!(vm.is_active_count(), 1);
    assert_eq!(fixture.is_active_count(), 1);

    fixture.unloaded().on_next(());
    assert_eq!(vm.is_active_count(), 0);
    assert_eq!(fixture.is_active_count(), 0);

    fixture.loaded().on_next(());
    assert_eq!(vm.is_active_count(), 1);
    assert_eq!(fixture.is_active_count(), 1);
}

#[test]
fn nulling_view_model_deactivate_it() {
    register_activating_view_fetcher();
    let vm = ActivatingViewModel::new();
    let fixture = ActivatingView::new();
    fixture.set_view_model(Some(vm.clone()));
    assert_eq!(vm.is_active_count(), 0);
    assert_eq!(fixture.is_active_count(), 0);

    fixture.loaded().on_next(());
    assert_eq!(vm.is_active_count(), 1);
    assert_eq!(fixture.is_active_count(), 1);

    fixture.set_view_model(None);
    assert_eq!(vm.is_active_count(), 0);
}

#[test]
fn setting_view_model_after_loaded_loads_it() {
    register_activating_view_fetcher();
    let vm = ActivatingViewModel::new();
    let fixture = ActivatingView::new();
    assert_eq!(vm.is_active_count(), 0);
    assert_eq!(fixture.is_active_count(), 0);

    fixture.loaded().on_next(());
    assert_eq!(fixture.is_active_count(), 1);

    fixture.set_view_model(Some(vm.clone()));
    assert_eq!(fixture.is_active_count(), 1);
    assert_eq!(vm.is_active_count(), 1);

    fixture.unloaded().on_next(());
    assert_eq!(fixture.is_active_count(), 0);
    assert_eq!(vm.is_active_count(), 0);
}

#[test]
fn switching_view_model_deactivates_it() {
    register_activating_view_fetcher();
    let vm = ActivatingViewModel::new();
    let fixture = ActivatingView::new();
    fixture.set_view_model(Some(vm.clone()));
    assert_eq!(vm.is_active_count(), 0);
    assert_eq!(fixture.is_active_count(), 0);

    fixture.loaded().on_next(());
    assert_eq!(vm.is_active_count(), 1);
    assert_eq!(fixture.is_active_count(), 1);

    let new_vm = ActivatingViewModel::new();
    assert_eq!(new_vm.is_active_count(), 0);

    fixture.set_view_model(Some(new_vm.clone()));
    assert_eq!(vm.is_active_count(), 0);
    assert_eq!(new_vm.is_active_count(), 1);
}

/// Upstream registers only the fetcher, so that no property observer is registered and the view
/// model is read once. In the port, the default property observers are used while none is
/// registered (DEVIATIONS.md, ReactiveUI); the view model activates either way.
#[test]
fn view_model_activates_when_no_plugin_can_watch_view_model() {
    register_activating_view_fetcher();
    let vm = ActivatingViewModel::new();
    let fixture = ActivatingView::new();
    fixture.set_view_model(Some(vm.clone()));

    fixture.loaded().on_next(());
    assert_eq!(vm.is_active_count(), 1);

    fixture.unloaded().on_next(());
    assert_eq!(vm.is_active_count(), 0);
}
