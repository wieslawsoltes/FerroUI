//! `ActivatingViewModel` (`ReactiveUI.Tests/Activation/ActivatingViewModel.cs`).

use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::*;
use std::cell::Cell;
use std::rc::Rc;

/// A view model that counts its activations (`ActivatingViewModel`): the count goes up on
/// activation and down on deactivation.
pub(crate) struct ActivatingViewModel {
    reactive: ReactiveObject,
    activator: ViewModelActivator,
    is_active_count: Cell<i32>,
}
reactive_object!(ActivatingViewModel);

impl IActivatableViewModel for ActivatingViewModel {
    fn activator(&self) -> &ViewModelActivator {
        &self.activator
    }
}

impl ActivatingViewModel {
    /// A view model that is not active, whose activation block counts.
    pub(crate) fn new() -> Rc<Self> {
        let view_model = Rc::new(Self { reactive: ReactiveObject::new(), activator: ViewModelActivator::new(), is_active_count: Cell::new(0) });
        let weak = Rc::downgrade(&view_model);
        ViewModelActivationMixins::when_activated_register(&*view_model, move |d| {
            if let Some(view_model) = weak.upgrade() {
                view_model.is_active_count.set(view_model.is_active_count.get() + 1);
            }
            let weak = weak.clone();
            d(Disposable::create(move || {
                if let Some(view_model) = weak.upgrade() {
                    view_model.is_active_count.set(view_model.is_active_count.get() - 1);
                }
            }));
        });
        view_model
    }

    /// `IsActiveCount`.
    pub(crate) fn is_active_count(&self) -> i32 {
        self.is_active_count.get()
    }
}
