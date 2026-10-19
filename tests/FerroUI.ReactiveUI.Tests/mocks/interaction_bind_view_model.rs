//! `InteractionBindViewModel` (`ReactiveUI.Tests/Mocks/InteractionBindViewModel.cs`).

use ferroui_reactiveui::{reactive_object, reactive_properties, Interaction, ReactiveObject};
use std::rc::Rc;

reactive_properties! {
    /// A view model with an interaction (`InteractionBindViewModel`).
    #[derive(Default)]
    pub struct InteractionBindViewModel {
        reactive: ReactiveObject,
        #[reactive]
        interaction1: Interaction<String, bool>,
    }
}
reactive_object!(InteractionBindViewModel);

impl InteractionBindViewModel {
    /// A view model with a new interaction.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}

/// View models compare by identity, as .NET compares references.
impl PartialEq for InteractionBindViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
