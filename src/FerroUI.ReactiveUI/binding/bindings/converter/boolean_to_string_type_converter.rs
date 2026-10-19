//! `BooleanToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/BooleanToStringTypeConverter.cs`).

use super::value_text::format_bool;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a `bool` to its text (`BooleanToStringTypeConverter`).
///
/// The text is `True` or `False`; the conversion hint is ignored.
#[derive(Clone, Copy, Debug, Default)]
pub struct BooleanToStringTypeConverter;

impl BindingTypeConverter for BooleanToStringTypeConverter {
    type From = bool;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&bool>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        Some(format_bool(*from?))
    }
}
