//! `InteractionAncestorViewModel` (`ReactiveUI.Tests/Mocks/InteractionAncestorViewModel.cs`).

use super::InteractionBindViewModel;
use ferroui_reactiveui::{reactive_object, reactive_properties, ReactiveObject};
use std::rc::Rc;

reactive_properties! {
    /// A view model that holds a view model with an interaction (`InteractionAncestorViewModel`).
    #[derive(Default)]
    pub struct InteractionAncestorViewModel {
        reactive: ReactiveObject,
        #[reactive]
        interaction_view_model: Rc<InteractionBindViewModel>,
    }
}
reactive_object!(InteractionAncestorViewModel);

impl InteractionAncestorViewModel {
    /// A view model holding a new [`InteractionBindViewModel`].
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}

/// View models compare by identity, as .NET compares references.
impl PartialEq for InteractionAncestorViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
