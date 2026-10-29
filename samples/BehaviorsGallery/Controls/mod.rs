//! The controls of the gallery (namespace `BehaviorsGallery.Controls`, which the default XML
//! namespace of the framework includes): one module per reference file of `Controls/`.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

pub mod custom_control;
pub mod editable_item;
pub mod single_selection_tab_control;

pub use custom_control::CustomControl;
pub use editable_item::EditableItem;
pub use single_selection_tab_control::SingleSelectionTabControl;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[
    CustomControl::TYPE,
    EditableItem::TYPE,
    SingleSelectionTabControl::TYPE,
];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&EditableItem::XAML_CLASS];

/// The types of this namespace declared with `ferro_markup_type!`: none.
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of this namespace: nothing.
pub(crate) fn register_value_types() {}
