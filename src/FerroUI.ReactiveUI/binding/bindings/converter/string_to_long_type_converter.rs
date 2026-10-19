//! `StringToLongTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToLongTypeConverter.cs`).

use super::number_format::parse_integer;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a text to an `i64` (`long`) (`StringToLongTypeConverter`).
///
/// The text is read as `TryParse` reads it with the invariant culture: white space around it and a
/// leading sign are allowed, and a number out of range fails. No text (`None`) fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToLongTypeConverter;

impl BindingTypeConverter for StringToLongTypeConverter {
    type From = String;
    type To = i64;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<i64> {
        i64::try_from(parse_integer(from?, i128::from(i64::MIN), i128::from(i64::MAX))?).ok()
    }
}
