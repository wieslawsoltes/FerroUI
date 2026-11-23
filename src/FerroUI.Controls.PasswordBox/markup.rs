//! Markup metadata of the enumeration of this crate.

use crate::PasswordRevealMode;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_enum;
use ferroui_base::metadata::{MarkupType, MarkupTyped};

ferro_markup_enum!(PasswordRevealMode { Peek, Hidden, Visible }, { namespace: "FerroUI.Controls" });

const TYPES: &[&MarkupType] = &[<PasswordRevealMode as MarkupTyped>::MARKUP];

pub(crate) fn register() {
    MarkupType::register_all(TYPES);
    ValueTypes::register_used::<PasswordRevealMode>();
}

impl ferroui_base::data::core::ValueTypeRegistration for PasswordRevealMode {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<PasswordRevealMode>();
    }
}
