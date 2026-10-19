//! `NullableBooleanToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableBooleanToStringTypeConverter.cs`).

use super::value_text::format_bool;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts an `Option` of `bool` to its text (`NullableBooleanToStringTypeConverter`); `None`
/// converts to an empty string.
///
/// The text is `True` or `False`; the conversion hint is ignored.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableBooleanToStringTypeConverter;

impl BindingTypeConverter for NullableBooleanToStringTypeConverter {
    type From = Option<bool>;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&Option<bool>>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        match from.copied().flatten() {
            // Deviation (DEVIATIONS.md, ReactiveUI): upstream converts `null` to a `null` string; a
            // `String` has no null, so `None` converts to an empty string, which the converter back
            // from text reads as `None`.
            None => Some(String::new()),
            Some(value) => Some(format_bool(value)),
        }
    }
}
