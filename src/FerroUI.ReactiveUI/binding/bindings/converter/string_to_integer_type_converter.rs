//! `StringToIntegerTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToIntegerTypeConverter.cs`).

use super::number_format::parse_integer;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a text to an `i32` (`int`) (`StringToIntegerTypeConverter`).
///
/// The text is read as `TryParse` reads it with the invariant culture: white space around it and a
/// leading sign are allowed, and a number out of range fails. No text (`None`) fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToIntegerTypeConverter;

impl BindingTypeConverter for StringToIntegerTypeConverter {
    type From = String;
    type To = i32;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<i32> {
        i32::try_from(parse_integer(from?, i128::from(i32::MIN), i128::from(i32::MAX))?).ok()
    }
}
