//! The views of the demo (namespace `TreeDataGridDemo.Views`): the pages of the main window,
//! which the ControlCatalog shows too.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod countries_page;
mod realized_count;

pub use countries_page::CountriesPage;
pub use realized_count::update_realized_count;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[CountriesPage::TYPE];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&CountriesPage::XAML_CLASS];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {}
