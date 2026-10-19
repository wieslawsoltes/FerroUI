//! `StringToNullableTimeSpanTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToNullableTimeSpanTypeConverter.cs`).

use super::value_text::parse_duration;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;
use std::time::Duration;

/// Converts a text to an `Option` of [`Duration`] (`TimeSpan`)
/// (`StringToNullableTimeSpanTypeConverter`): no text or an empty one converts to `None`.
///
/// The text is read as `TimeSpan.TryParse` reads it: `d`, `[d.]hh:mm[:ss[.fffffff]]` or
/// `d:hh:mm:ss[.fffffff]`.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToNullableTimeSpanTypeConverter;

impl BindingTypeConverter for StringToNullableTimeSpanTypeConverter {
    type From = String;
    type To = Option<Duration>;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<Option<Duration>> {
        let Some(text) = from.filter(|text| !text.is_empty()) else {
            return Some(None);
        };
        parse_duration(text).map(Some)
    }
}
