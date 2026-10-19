//! `ShortToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/ShortToStringTypeConverter.cs`).

use super::number_format::integer_to_string;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts an `i16` (`short`) to its text (`ShortToStringTypeConverter`).
///
/// An `i32` conversion hint is the minimum number of digits (`D{width}`), a string hint is a .NET
/// numeric format string (`X`, `N0`, `000`, ...); without a hint the number is written in full.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShortToStringTypeConverter;

impl BindingTypeConverter for ShortToStringTypeConverter {
    type From = i16;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&i16>, conversion_hint: Option<&dyn Any>) -> Option<String> {
        integer_to_string(i128::from(*from?), 16, conversion_hint)
    }
}
