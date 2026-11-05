//! `GuidToStringTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/GuidToStringTypeConverter.cs`).

use super::value_text::format_guid;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;
use uuid::Uuid;

/// Converts an identifier, a [`Uuid`] (upstream's `Guid`), to its text
/// (`GuidToStringTypeConverter`).
///
/// The text is the `D` format, `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, in lower case; the
/// conversion hint is ignored.
#[derive(Clone, Copy, Debug, Default)]
pub struct GuidToStringTypeConverter;

impl BindingTypeConverter for GuidToStringTypeConverter {
    type From = Uuid;
    type To = String;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&Uuid>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        Some(format_guid(*from?))
    }
}
