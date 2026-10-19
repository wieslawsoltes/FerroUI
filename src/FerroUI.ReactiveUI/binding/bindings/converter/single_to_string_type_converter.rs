//! `SingleToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/SingleToStringTypeConverter.cs`).

use super::number_format::single_to_string;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts an `f32` (`float`) to its text (`SingleToStringTypeConverter`).
///
/// An `i32` conversion hint is the number of decimals (`F{places}`), a string hint is a .NET
/// numeric format string (`E2`, `G`, `R`, `0.0000`, ...); without a hint the number is written with
/// the shortest digits that read back to it.
#[derive(Clone, Copy, Debug, Default)]
pub struct SingleToStringTypeConverter;

impl BindingTypeConverter for SingleToStringTypeConverter {
    type From = f32;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&f32>, conversion_hint: Option<&dyn Any>) -> Option<String> {
        single_to_string(*from?, conversion_hint)
    }
}
