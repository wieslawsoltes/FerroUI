//! `StringToNullableBooleanTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/StringToNullableBooleanTypeConverter.cs`).

use super::value_text::parse_bool;
use crate::{BindingAffinity, BindingTypeConverter};
use std::any::Any;

/// Converts a text to an `Option` of `bool` (`StringToNullableBooleanTypeConverter`): no text or an
/// empty one converts to `None`.
///
/// The text is `true` or `false` in any case, as `Boolean.TryParse` reads it.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringToNullableBooleanTypeConverter;

impl BindingTypeConverter for StringToNullableBooleanTypeConverter {
    type From = String;
    type To = Option<bool>;

    fn affinity(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<Option<bool>> {
        let Some(text) = from.filter(|text| !text.is_empty()) else {
            return Some(None);
        };
        parse_bool(text).map(Some)
    }
}
