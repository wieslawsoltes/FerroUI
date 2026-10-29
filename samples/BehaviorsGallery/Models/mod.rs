//! The models of the gallery (namespace `BehaviorsGallery.Models`): one module per reference file
//! of `Models/`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

pub mod drag_item;
pub mod tile;

pub use drag_item::DragItem;
pub use tile::Tile;

/// The classes of this namespace (`X::TYPE`): none.
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The types of this namespace declared with `ferro_markup_type!` (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <DragItem as MarkupTyped>::MARKUP,
    <Tile as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<DragItem>();
    ValueTypes::register_reference::<Tile>();
    ValueTypes::register_display::<DragItem>();
    ValueTypes::register_display::<Tile>();
}
