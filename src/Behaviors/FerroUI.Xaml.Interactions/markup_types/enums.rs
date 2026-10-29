//! Markup metadata of the enumerations (`Core/AsyncActionGroup.cs`).

use crate::AsyncActionMode;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_enum;
use ferroui_base::metadata::{MarkupType, MarkupTyped};

ferro_markup_enum!(AsyncActionMode { Sequence, Parallel });

/// The enumerations.
pub(super) const TYPES: &[&MarkupType] = &[<AsyncActionMode as MarkupTyped>::MARKUP];

pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<AsyncActionMode>();
}
