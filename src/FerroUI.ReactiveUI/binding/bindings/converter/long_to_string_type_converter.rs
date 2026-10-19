//! `LongToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/LongToStringTypeConverter.cs`).

use super::number_format::integer_to_string;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts an `i64` (`long`) to its text (`LongToStringTypeConverter`).
///
/// An `i32` conversion hint is the minimum number of digits (`D{width}`), a string hint is a .NET
/// numeric format string (`X`, `N0`, `000`, ...); without a hint the number is written in full.
#[derive(Clone, Copy, Debug, Default)]
pub struct LongToStringTypeConverter;

impl BindingTypeConverter for LongToStringTypeConverter {
    type From = i64;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&i64>, conversion_hint: Option<&dyn Any>) -> Option<String> {
        integer_to_string(i128::from(*from?), 64, conversion_hint)
    }
}
