//! `ViewModelActivatorTests` (`ReactiveUI.Tests/Activation/ViewModelActivatorTests.cs`).

use crate::mocks::collect_items;
use ferroui_reactiveui::*;

#[test]
fn test_activating_ticks_activated_observable() {
    let view_model_activator = ViewModelActivator::new();
    let activated = collect_items(&view_model_activator.activated());

    let _ = view_model_activator.activate();

    assert_eq!(activated.borrow().len(), 1);
}

#[test]
fn test_deactivating_count_doesnt_tick_deactivated_observable() {
    let view_model_activator = ViewModelActivator::new();
    let deactivated = collect_items(&view_model_activator.deactivated());

    view_model_activator.deactivate();

    assert!(deactivated.borrow().is_empty());
}

#[test]
fn test_deactivating_following_activating_ticks_deactivated_observable() {
    let view_model_activator = ViewModelActivator::new();
    let deactivated = collect_items(&view_model_activator.deactivated());

    let _ = view_model_activator.activate();
    view_model_activator.deactivate();

    assert_eq!(deactivated.borrow().len(), 1);
}

#[test]
fn test_deactivating_ignoring_ref_count_ticks_deactivated_observable() {
    let view_model_activator = ViewModelActivator::new();
    let deactivated = collect_items(&view_model_activator.deactivated());

    view_model_activator.deactivate_with(true);

    assert_eq!(deactivated.borrow().len(), 1);
}

#[test]
fn test_disposing_after_activation_deactivates_view_model() {
    let view_model_activator = ViewModelActivator::new();
    let activated = collect_items(&view_model_activator.activated());
    let deactivated = collect_items(&view_model_activator.deactivated());

    {
        let activation = view_model_activator.activate();
        assert_eq!(activated.borrow().len(), 1);
        assert!(deactivated.borrow().is_empty());
        activation.dispose();
    }

    assert_eq!(activated.borrow().len(), 1);
    assert_eq!(deactivated.borrow().len(), 1);
}
