//! `ReactiveUserControlActivationTests` (`ReactiveUserControlActivationTests.cs`): the activation
//! of a view model by a reactive user control without an activation block of its own.

use super::view_support::{run_jobs, start, window_with};
use crate::ReactiveUserControl;
use ferroui_base::BoxedValue;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::{reactive_object, IActivatableViewModel, ReactiveObject, ViewModelActivationMixins, ViewModelActivator};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// A view model that owns its activation lifetime (`ActivatableModel`).
struct ActivatableModel {
    reactive: ReactiveObject,
    activator: ViewModelActivator,
}
reactive_object!(ActivatableModel);

impl IActivatableViewModel for ActivatableModel {
    fn activator(&self) -> &ViewModelActivator {
        &self.activator
    }
}

impl PartialEq for ActivatableModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ActivatableModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new(), activator: ViewModelActivator::new() })
    }

    /// `Dispose`: disposes the activator.
    fn dispose(&self) {
        self.activator.dispose();
    }
}

/// Counts how many times it is disposed (`DisposeCounter`).
fn dispose_counter(count: &Rc<Cell<i32>>) -> Rc<dyn IDisposable> {
    let count = count.clone();
    Disposable::create(move || count.set(count.get() + 1))
}

#[test]
fn base_control_activates_view_model_without_custom_view_block() {
    let _app = start();
    let model = ActivatableModel::new();
    let activation_count = Rc::new(Cell::new(0));
    let counted = activation_count.clone();
    model.when_activated_disposables(move |_disposables| counted.set(counted.get() + 1));
    let control = ReactiveUserControl::<ActivatableModel>::new();
    control.set_view_model(Some(model.clone()));
    let window = window_with(Some(control.control().clone().upcast()));

    window.show();
    run_jobs();
    assert_eq!(activation_count.get(), 1);

    window.close();
    model.dispose();
}

#[test]
fn swapping_view_model_while_shown_moves_activation_to_new_view_model() {
    let _app = start();
    let first = ActivatableModel::new();
    let second = ActivatableModel::new();
    let first_deactivation = Rc::new(Cell::new(0));
    let second_activations = Rc::new(Cell::new(0));
    let deactivation = first_deactivation.clone();
    first.when_activated_disposables(move |disposables| disposables.add(dispose_counter(&deactivation)));
    let activations = second_activations.clone();
    second.when_activated_disposables(move |_disposables| activations.set(activations.get() + 1));
    let control = ReactiveUserControl::<ActivatableModel>::new();
    control.set_view_model(Some(first.clone()));
    let window = window_with(Some(control.control().clone().upcast()));

    window.show();
    run_jobs();

    control.set_data_context(Some(second.clone() as BoxedValue));
    run_jobs();

    assert!(control.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &second)));
    assert_eq!(first_deactivation.get(), 1);
    assert_eq!(second_activations.get(), 1);

    window.close();
    first.dispose();
    second.dispose();
}
