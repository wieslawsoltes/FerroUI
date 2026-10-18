//! Markup metadata of the enumerations of this crate that markup names: the sort direction of a
//! column (`System.ComponentModel.ListSortDirection` of the reference) and the drop positions and
//! effects of a move of rows.

use crate::models::ListSortDirection;
use crate::RowDropPosition;
use ferroui_base::ferro_markup_enum;

ferro_markup_enum!(ListSortDirection { Ascending, Descending }, { namespace: "System.ComponentModel" });
ferro_markup_enum!(RowDropPosition { None, Before, After, Inside }, { namespace: "TreeDataGridCore" });

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};

/// The types declared in this file.
const TYPES: &[&MarkupType] = &[<ListSortDirection as MarkupTyped>::MARKUP, <RowDropPosition as MarkupTyped>::MARKUP];

/// Registers the markup metadata of the enumerations of the crate and their nullable forms with
/// the untyped value conversions. Cheap and idempotent; the control crate calls it from its
/// `register_types`.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| MarkupType::register_all(TYPES));
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<ListSortDirection>();
        ValueTypes::register_nullable::<RowDropPosition>();
    });
}
