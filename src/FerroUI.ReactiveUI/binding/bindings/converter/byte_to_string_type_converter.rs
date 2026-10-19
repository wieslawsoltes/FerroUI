//! `ByteToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/ByteToStringTypeConverter.cs`).

use super::number_format::integer_to_string;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a `u8` (`byte`) to its text (`ByteToStringTypeConverter`).
///
/// An `i32` conversion hint is the minimum number of digits (`D{width}`), a string hint is a .NET
/// numeric format string (`X`, `N0`, `000`, ...); without a hint the number is written in full.
#[derive(Clone, Copy, Debug, Default)]
pub struct ByteToStringTypeConverter;

impl BindingTypeConverter for ByteToStringTypeConverter {
    type From = u8;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&u8>, conversion_hint: Option<&dyn Any>) -> Option<String> {
        integer_to_string(i128::from(*from?), 8, conversion_hint)
    }
}
