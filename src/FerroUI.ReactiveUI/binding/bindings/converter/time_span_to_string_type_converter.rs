//! `TimeSpanToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/TimeSpanToStringTypeConverter.cs`).

use super::value_text::format_duration;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;
use std::time::Duration;

/// Converts a [`Duration`] (`TimeSpan`) to its text (`TimeSpanToStringTypeConverter`).
///
/// The text is `TimeSpan`'s constant format, `[d.]hh:mm:ss[.fffffff]`; the conversion hint is
/// ignored.
#[derive(Clone, Copy, Debug, Default)]
pub struct TimeSpanToStringTypeConverter;

impl BindingTypeConverter for TimeSpanToStringTypeConverter {
    type From = Duration;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&Duration>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        Some(format_duration(*from?))
    }
}
