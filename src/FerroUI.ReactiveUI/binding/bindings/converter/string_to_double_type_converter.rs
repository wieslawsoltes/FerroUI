//! `StringToDoubleTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToDoubleTypeConverter.cs`).

use super::number_format::parse_double;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a text to an `f64` (`double`) (`StringToDoubleTypeConverter`).
///
/// The text is read as `Double.TryParse` reads it with the invariant culture: a sign, group
/// separators, decimals, an exponent, `NaN` and `Infinity`. No text (`None`) fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToDoubleTypeConverter;

impl BindingTypeConverter for StringToDoubleTypeConverter {
    type From = String;
    type To = f64;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<f64> {
        parse_double(from?)
    }
}
