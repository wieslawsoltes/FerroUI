//! `NullableDoubleToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableDoubleToStringTypeConverter.cs`).

use super::number_format::double_to_string;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts an `Option` of `f64` (`double`) to its text (`NullableDoubleToStringTypeConverter`);
/// `None` converts to an empty string.
///
/// An `i32` conversion hint is the number of decimals (`F{places}`), a string hint is a .NET
/// numeric format string (`E2`, `G`, `R`, `0.0000`, ...); without a hint the number is written with
/// the shortest digits that read back to it.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableDoubleToStringTypeConverter;

impl BindingTypeConverter for NullableDoubleToStringTypeConverter {
    type From = Option<f64>;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&Option<f64>>, conversion_hint: Option<&dyn Any>) -> Option<String> {
        match from.copied().flatten() {
            // Deviation (DEVIATIONS.md, ReactiveUI): upstream converts `null` to a `null` string; a
            // `String` has no null, so `None` converts to an empty string, which the converter back
            // from text reads as `None`.
            None => Some(String::new()),
            Some(value) => double_to_string(value, conversion_hint),
        }
    }
}
