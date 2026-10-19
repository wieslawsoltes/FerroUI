//! `StringToNullableGuidTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToNullableGuidTypeConverter.cs`).

use super::value_text::parse_guid;
use crate::{BindingAffinity, BindingTypeConverter};
use ferroui_splat::Guid;
use std::any::Any;

/// Converts a text to an `Option` of [`Guid`] (`StringToNullableGuidTypeConverter`): no text or an
/// empty one converts to `None`.
///
/// The text is any format of `Guid.TryParse`: `N`, `D`, `B`, `P` or `X`.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToNullableGuidTypeConverter;

impl BindingTypeConverter for StringToNullableGuidTypeConverter {
    type From = String;
    type To = Option<Guid>;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<Option<Guid>> {
        let Some(text) = from.filter(|text| !text.is_empty()) else {
            return Some(None);
        };
        parse_guid(text).map(Some)
    }
}
