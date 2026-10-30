//! The dialogs of the gallery (namespace `BehaviorsGallery.Dialogs`): one module per reference
//! file of `Dialogs/`.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

pub mod simple_dialog;

pub use simple_dialog::SimpleDialog;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SimpleDialog::TYPE];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&SimpleDialog::XAML_CLASS];

/// The types of this namespace declared with `ferro_markup_type!`: none.
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of this namespace: nothing.
pub(crate) fn register_value_types() {}
