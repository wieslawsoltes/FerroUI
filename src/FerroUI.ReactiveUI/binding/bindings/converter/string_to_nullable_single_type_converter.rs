//! `StringToNullableSingleTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToNullableSingleTypeConverter.cs`).

use super::number_format::parse_single;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a text to an `Option` of `f32` (`float`) (`StringToNullableSingleTypeConverter`): no
/// text or an empty one converts to `None`.
///
/// The text is read as `Single.TryParse` reads it with the invariant culture: a sign, group
/// separators, decimals, an exponent, `NaN` and `Infinity`.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToNullableSingleTypeConverter;

impl BindingTypeConverter for StringToNullableSingleTypeConverter {
    type From = String;
    type To = Option<f32>;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<Option<f32>> {
        let Some(text) = from.filter(|text| !text.is_empty()) else {
            return Some(None);
        };
        parse_single(text).map(Some)
    }
}
