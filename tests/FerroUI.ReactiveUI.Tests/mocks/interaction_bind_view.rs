//! `InteractionBindView` (`ReactiveUI.Tests/Mocks/InteractionBindView.cs`).

use super::InteractionBindViewModel;
use ferroui_reactiveui::{reactive_object, reactive_properties, view_for, ReactiveObject};
use std::rc::Rc;

reactive_properties! {
    /// A reactive view of an [`InteractionBindViewModel`] (`InteractionBindView`).
    #[derive(Default)]
    pub struct InteractionBindView {
        reactive: ReactiveObject,
        #[reactive]
        view_model: Option<Rc<InteractionBindViewModel>>,
    }
}
reactive_object!(InteractionBindView);
view_for!(InteractionBindView: INotifyPropertyChanged, InteractionBindViewModel);

impl InteractionBindView {
    /// A view without a view model.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
