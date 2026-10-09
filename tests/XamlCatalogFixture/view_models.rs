//! The view models of the pages of the fixture (`ControlCatalog.ViewModels`): the source
//! files of the sample, compiled here as they are.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};

#[path = "../../samples/ControlCatalog/ViewModels/random.rs"]
mod random;
// Public: the list of the items (`WrapPanelItemList`) is a type generated code names, and
// the sample declares it next to the view model.
#[path = "../../samples/ControlCatalog/ViewModels/wrap_panel_page_view_model.rs"]
pub mod wrap_panel_page_view_model;

pub use wrap_panel_page_view_model::{WrapPanelItemViewModel, WrapPanelPageViewModel};

/// The types with markup metadata of this namespace.
pub(crate) const MARKUP_TYPES: &[&MarkupType] =
    &[<WrapPanelItemViewModel as MarkupTyped>::MARKUP, <WrapPanelPageViewModel as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this namespace, as the
/// sample registers it.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<WrapPanelItemViewModel>();
    ValueTypes::register_reference::<WrapPanelPageViewModel>();
}

/// Makes the typed list of the view model known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {
    wrap_panel_page_view_model::WrapPanelItemList::register();
}
