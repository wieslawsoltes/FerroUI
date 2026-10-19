//! `StringToBooleanTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToBooleanTypeConverter.cs`).

use super::value_text::parse_bool;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a text to a `bool` (`StringToBooleanTypeConverter`).
///
/// The text is `true` or `false` in any case, as `Boolean.TryParse` reads it. No text (`None`)
/// fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToBooleanTypeConverter;

impl BindingTypeConverter for StringToBooleanTypeConverter {
    type From = String;
    type To = bool;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<bool> {
        parse_bool(from?)
    }
}
