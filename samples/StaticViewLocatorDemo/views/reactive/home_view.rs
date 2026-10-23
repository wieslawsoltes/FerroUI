//! Port of `Views/HomeView` (markup) of the ReactiveUI demo: a view of ReactiveUI for its view model
//! (`ReactiveUserControl<HomeViewModel>`), which the generator discovers through `IControlViewFor`.

use crate::class_prelude::*;
use crate::located_view;

use crate::view_models::reactive::HomeViewModel;
use crate::view_models::ViewModelBase;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use std::rc::Rc;

located_view!(HomeView, HomeViewModel, home);

/// The view is a view of ReactiveUI for its view model (`ReactiveUserControl<HomeViewModel>`): its view
/// model is its data context. An `impl` the generator reads: it is an item of the sources, not the
/// expansion of a macro.
impl ferroui_reactiveui_controls::IControlViewFor for HomeView {
    type ViewModel = HomeViewModel;

    fn view_model_of(&self) -> Option<Rc<HomeViewModel>> {
        self.view_model()
    }

    fn set_view_model_of(&self, view_model: Option<Rc<HomeViewModel>>) {
        self.set_data_context(view_model.map(|view_model| view_model as ferroui_base::BoxedValue));
    }
}

fn home(view_model: &Rc<HomeViewModel>) -> Element {
    stack_panel()
        .spacing(8.0)
        .child(text_block().text(view_model.title()).font_size(20.0).font_weight(FontWeight::SemiBold))
        .child(text_block().text(view_model.description()).text_wrapping(TextWrapping::Wrap))
        .into()
}
