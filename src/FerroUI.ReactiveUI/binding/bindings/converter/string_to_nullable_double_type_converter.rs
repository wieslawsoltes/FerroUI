//! `StringToNullableDoubleTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToNullableDoubleTypeConverter.cs`).

use super::number_format::parse_double;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a text to an `Option` of `f64` (`double`) (`StringToNullableDoubleTypeConverter`): no
/// text or an empty one converts to `None`.
///
/// The text is read as `Double.TryParse` reads it with the invariant culture: a sign, group
/// separators, decimals, an exponent, `NaN` and `Infinity`.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToNullableDoubleTypeConverter;

impl BindingTypeConverter for StringToNullableDoubleTypeConverter {
    type From = String;
    type To = Option<f64>;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<Option<f64>> {
        let Some(text) = from.filter(|text| !text.is_empty()) else {
            return Some(None);
        };
        parse_double(text).map(Some)
    }
}
