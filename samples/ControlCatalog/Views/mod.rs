//! The views of the catalog (namespace `ControlCatalog.Views`): one module per upstream file.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {}
