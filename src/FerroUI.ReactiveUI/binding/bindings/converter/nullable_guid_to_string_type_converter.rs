//! `NullableGuidToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableGuidToStringTypeConverter.cs`).

use super::value_text::format_guid;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;
use uuid::Uuid;

/// Converts an `Option` of an identifier, a [`Uuid`] (upstream's `Guid`), to its text
/// (`NullableGuidToStringTypeConverter`); `None` converts to an empty string.
///
/// The text is the `D` format, `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, in lower case; the
/// conversion hint is ignored.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableGuidToStringTypeConverter;

impl BindingTypeConverter for NullableGuidToStringTypeConverter {
    type From = Option<Uuid>;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&Option<Uuid>>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        match from.copied().flatten() {
            // Deviation (DEVIATIONS.md, ReactiveUI): upstream converts `null` to a `null` string; a
            // `String` has no null, so `None` converts to an empty string, which the converter back
            // from text reads as `None`.
            None => Some(String::new()),
            Some(value) => Some(format_guid(value)),
        }
    }
}
