//! `NullableIntegerToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableIntegerToStringTypeConverter.cs`).

use super::number_format::integer_to_string;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts an `Option` of `i32` (`int`) to its text (`NullableIntegerToStringTypeConverter`);
/// `None` converts to an empty string.
///
/// An `i32` conversion hint is the minimum number of digits (`D{width}`), a string hint is a .NET
/// numeric format string (`X`, `N0`, `000`, ...); without a hint the number is written in full.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableIntegerToStringTypeConverter;

impl BindingTypeConverter for NullableIntegerToStringTypeConverter {
    type From = Option<i32>;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&Option<i32>>, conversion_hint: Option<&dyn Any>) -> Option<String> {
        match from.copied().flatten() {
            // Deviation (DEVIATIONS.md, ReactiveUI): upstream converts `null` to a `null` string; a
            // `String` has no null, so `None` converts to an empty string, which the converter back
            // from text reads as `None`.
            None => Some(String::new()),
            Some(value) => integer_to_string(i128::from(value), 32, conversion_hint),
        }
    }
}
