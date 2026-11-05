//! `StringToGuidTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToGuidTypeConverter.cs`).

use super::value_text::parse_guid;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;
use uuid::Uuid;

/// Converts a text to an identifier, a [`Uuid`] (upstream's `Guid`)
/// (`StringToGuidTypeConverter`).
///
/// The text is any format of `Guid.TryParse`: `N`, `D`, `B`, `P` or `X`. No text (`None`) fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToGuidTypeConverter;

impl BindingTypeConverter for StringToGuidTypeConverter {
    type From = String;
    type To = Uuid;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<Uuid> {
        parse_guid(from?)
    }
}
