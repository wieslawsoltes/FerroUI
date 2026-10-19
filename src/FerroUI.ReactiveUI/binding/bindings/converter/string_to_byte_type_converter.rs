//! `StringToByteTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToByteTypeConverter.cs`).

use super::number_format::parse_integer;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a text to a `u8` (`byte`) (`StringToByteTypeConverter`).
///
/// The text is read as `TryParse` reads it with the invariant culture: white space around it and a
/// leading sign are allowed, and a number out of range fails. No text (`None`) fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToByteTypeConverter;

impl BindingTypeConverter for StringToByteTypeConverter {
    type From = String;
    type To = u8;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<u8> {
        u8::try_from(parse_integer(from?, i128::from(u8::MIN), i128::from(u8::MAX))?).ok()
    }
}
