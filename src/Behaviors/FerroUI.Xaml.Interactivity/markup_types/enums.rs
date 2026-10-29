//! Markup metadata of the enumerations (`Contract/ComparisonConditionType.cs`).

use crate::ComparisonConditionType;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_enum;
use ferroui_base::metadata::{MarkupType, MarkupTyped};

ferro_markup_enum!(ComparisonConditionType { Equal, NotEqual, LessThan, LessThanOrEqual, GreaterThan, GreaterThanOrEqual });

/// The enumerations.
pub(super) const TYPES: &[&MarkupType] = &[<ComparisonConditionType as MarkupTyped>::MARKUP];

pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<ComparisonConditionType>();
}
