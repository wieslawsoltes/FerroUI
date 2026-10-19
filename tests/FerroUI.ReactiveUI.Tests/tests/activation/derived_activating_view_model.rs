//! `DerivedActivatingViewModel` (`ReactiveUI.Tests/Activation/DerivedActivatingViewModel.cs`).

use super::ActivatingViewModel;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::*;
use std::cell::Cell;
use std::rc::Rc;

/// A view model built on [`ActivatingViewModel`] that registers a block of its own on the same
/// activator (`DerivedActivatingViewModel`). The derivation of upstream is a field holding the
/// base view model.
pub(crate) struct DerivedActivatingViewModel {
    base: Rc<ActivatingViewModel>,
    is_active_count_also: Cell<i32>,
}

impl IActivatableViewModel for DerivedActivatingViewModel {
    fn activator(&self) -> &ViewModelActivator {
        self.base.activator()
    }
}

impl DerivedActivatingViewModel {
    /// A view model whose base and own blocks both count.
    pub(crate) fn new() -> Rc<Self> {
        let view_model = Rc::new(Self { base: ActivatingViewModel::new(), is_active_count_also: Cell::new(0) });
        let weak = Rc::downgrade(&view_model);
        ViewModelActivationMixins::when_activated_register(&*view_model, move |d| {
            if let Some(view_model) = weak.upgrade() {
                view_model.is_active_count_also.set(view_model.is_active_count_also.get() + 1);
            }
            let weak = weak.clone();
            d(Disposable::create(move || {
                if let Some(view_model) = weak.upgrade() {
                    view_model.is_active_count_also.set(view_model.is_active_count_also.get() - 1);
                }
            }));
        });
        view_model
    }

    /// `IsActiveCount`, the count of the base view model.
    pub(crate) fn is_active_count(&self) -> i32 {
        self.base.is_active_count()
    }

    /// `IsActiveCountAlso`.
    pub(crate) fn is_active_count_also(&self) -> i32 {
        self.is_active_count_also.get()
    }
}
