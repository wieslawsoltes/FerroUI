//! `StringToSingleTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToSingleTypeConverter.cs`).

use super::number_format::parse_single;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a text to an `f32` (`float`) (`StringToSingleTypeConverter`).
///
/// The text is read as `Single.TryParse` reads it with the invariant culture: a sign, group
/// separators, decimals, an exponent, `NaN` and `Infinity`. No text (`None`) fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToSingleTypeConverter;

impl BindingTypeConverter for StringToSingleTypeConverter {
    type From = String;
    type To = f32;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<f32> {
        parse_single(from?)
    }
}
