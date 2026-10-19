//! `DoubleToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/DoubleToStringTypeConverter.cs`).

use super::number_format::double_to_string;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts an `f64` (`double`) to its text (`DoubleToStringTypeConverter`).
///
/// An `i32` conversion hint is the number of decimals (`F{places}`), a string hint is a .NET
/// numeric format string (`E2`, `G`, `R`, `0.0000`, ...); without a hint the number is written with
/// the shortest digits that read back to it.
#[derive(Clone, Copy, Debug, Default)]
pub struct DoubleToStringTypeConverter;

impl BindingTypeConverter for DoubleToStringTypeConverter {
    type From = f64;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&f64>, conversion_hint: Option<&dyn Any>) -> Option<String> {
        double_to_string(*from?, conversion_hint)
    }
}
