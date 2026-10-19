//! `StringToNullableByteTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToNullableByteTypeConverter.cs`).

use super::number_format::parse_integer;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a text to an `Option` of `u8` (`byte`) (`StringToNullableByteTypeConverter`): no text
/// or an empty one converts to `None`.
///
/// The text is read as `TryParse` reads it with the invariant culture: white space around it and a
/// leading sign are allowed, and a number out of range fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToNullableByteTypeConverter;

impl BindingTypeConverter for StringToNullableByteTypeConverter {
    type From = String;
    type To = Option<u8>;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<Option<u8>> {
        let Some(text) = from.filter(|text| !text.is_empty()) else {
            return Some(None);
        };
        u8::try_from(parse_integer(text, i128::from(u8::MIN), i128::from(u8::MAX))?).ok().map(Some)
    }
}
