//! `ReactiveWindowActivationTests` (`ReactiveWindowActivationTests.cs`): regression tests of the
//! activation of a view model by a reactive window (issue 157 upstream).

use super::view_support::{run_jobs, start};
use crate::ReactiveWindow;
use ferroui_reactiveui::{reactive_object, IActivatableViewModel, ReactiveObject, ViewModelActivationMixins, ViewModelActivator};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// An activatable view model used to observe activation from the view (`ActivatableViewModel`).
struct ActivatableViewModel {
    reactive: ReactiveObject,
    activator: ViewModelActivator,
}
reactive_object!(ActivatableViewModel);

impl IActivatableViewModel for ActivatableViewModel {
    fn activator(&self) -> &ViewModelActivator {
        &self.activator
    }
}

impl PartialEq for ActivatableViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A strongly typed reactive window under test (`ActivatableWindow`).
type ActivatableWindow = ReactiveWindow<ActivatableViewModel>;

// Not ported: ReactiveWindow_Resolves_Single_Typed_ViewModel_Property - reflection (the lookup of
// the `ViewModel` property by name that threw `AmbiguousMatchException`; the typed handle has one
// `view_model` accessor, and ReactiveUI finds the view model through `IViewFor`).

#[test]
fn view_activation_activates_view_model() {
    let _app = start();
    let window = ActivatableWindow::new();
    let view_model = Rc::new(ActivatableViewModel { reactive: ReactiveObject::new(), activator: ViewModelActivator::new() });
    let activation_count = Rc::new(Cell::new(0));
    let counted = activation_count.clone();
    view_model.when_activated_disposables(move |_| counted.set(counted.get() + 1));
    window.set_view_model(Some(view_model.clone()));

    window.show();
    run_jobs();

    assert!(activation_count.get() > 0);

    window.close();
    view_model.activator.dispose();
}
