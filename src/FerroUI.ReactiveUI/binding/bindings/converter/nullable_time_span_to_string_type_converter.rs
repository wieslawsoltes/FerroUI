//! `NullableTimeSpanToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableTimeSpanToStringTypeConverter.cs`).

use super::value_text::format_duration;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;
use std::time::Duration;

/// Converts an `Option` of [`Duration`] (`TimeSpan`) to its text
/// (`NullableTimeSpanToStringTypeConverter`); `None` converts to an empty string.
///
/// The text is `TimeSpan`'s constant format, `[d.]hh:mm:ss[.fffffff]`; the conversion hint is
/// ignored.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableTimeSpanToStringTypeConverter;

impl BindingTypeConverter for NullableTimeSpanToStringTypeConverter {
    type From = Option<Duration>;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&Option<Duration>>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        match from.copied().flatten() {
            // Deviation (DEVIATIONS.md, ReactiveUI): upstream converts `null` to a `null` string; a
            // `String` has no null, so `None` converts to an empty string, which the converter back
            // from text reads as `None`.
            None => Some(String::new()),
            Some(value) => Some(format_duration(value)),
        }
    }
}
