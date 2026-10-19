//! `InteractionAncestorView` (`ReactiveUI.Tests/Mocks/InteractionAncestorView.cs`).

use super::InteractionAncestorViewModel;
use ferroui_reactiveui::{reactive_object, reactive_properties, view_for, ReactiveObject};
use std::rc::Rc;

reactive_properties! {
    /// A reactive view of an [`InteractionAncestorViewModel`] (`InteractionAncestorView`).
    #[derive(Default)]
    pub struct InteractionAncestorView {
        reactive: ReactiveObject,
        #[reactive]
        view_model: Option<Rc<InteractionAncestorViewModel>>,
    }
}
reactive_object!(InteractionAncestorView);
view_for!(InteractionAncestorView: INotifyPropertyChanged, InteractionAncestorViewModel);

impl InteractionAncestorView {
    /// A view without a view model.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
