//! The models of the catalog (namespace `ControlCatalog.Models`): one module per upstream file.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod catalog_theme;
mod home_section;
mod page_item;
mod person;
mod state_data;

pub use catalog_theme::CatalogTheme;
pub use home_section::HomeSection;
pub use page_item::PageItem;
pub use person::Person;
pub use state_data::StateData;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <CatalogTheme as MarkupTyped>::MARKUP,
    <HomeSection as MarkupTyped>::MARKUP,
    <PageItem as MarkupTyped>::MARKUP,
    <Person as MarkupTyped>::MARKUP,
    <StateData as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<HomeSection>();
    ValueTypes::register_reference::<PageItem>();
    ValueTypes::register_reference::<Person>();
    ValueTypes::register_reference::<StateData>();
    // `ToString()` of a state: its name.
    ValueTypes::register_display::<StateData>();
}
