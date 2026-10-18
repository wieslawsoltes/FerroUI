//! The view models of the demo (namespace `TreeDataGridDemo.ViewModels`): one module per
//! reference file.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod countries_page_view_model;
mod main_window_view_model;
// The pseudo-random number generator of the runtime library (not a file of the demo).
mod random;

pub use countries_page_view_model::CountriesPageViewModel;
pub use main_window_view_model::MainWindowViewModel;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] =
    &[<CountriesPageViewModel as MarkupTyped>::MARKUP, <MainWindowViewModel as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<CountriesPageViewModel>();
    ValueTypes::register_reference::<MainWindowViewModel>();
}

/// Makes the typed lists of the view models known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {}
