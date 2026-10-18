//! The models of the demo (namespace `TreeDataGridDemo.Models`): one module per reference file.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod countries;
mod country;

pub use countries::Countries;
pub use country::Country;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[<Country as MarkupTyped>::MARKUP, <Countries as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<Country>();
}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {}
