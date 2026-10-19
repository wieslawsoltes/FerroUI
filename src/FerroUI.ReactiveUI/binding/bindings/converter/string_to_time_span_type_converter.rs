//! `StringToTimeSpanTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToTimeSpanTypeConverter.cs`).

use super::value_text::parse_duration;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;
use std::time::Duration;

/// Converts a text to a [`Duration`] (`TimeSpan`) (`StringToTimeSpanTypeConverter`).
///
/// The text is read as `TimeSpan.TryParse` reads it: `d`, `[d.]hh:mm[:ss[.fffffff]]` or
/// `d:hh:mm:ss[.fffffff]`. No text (`None`) fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToTimeSpanTypeConverter;

impl BindingTypeConverter for StringToTimeSpanTypeConverter {
    type From = String;
    type To = Duration;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<Duration> {
        parse_duration(from?)
    }
}
