//! Native, lazy items-source views over the actual keyboard lists.
//!
//! Core readers remain independent of controls. They publish no collection
//! notifications: direct reads are live, while an items control must replace its
//! source to refresh a changed non-notifying list. No snapshot or identity cache
//! is installed. Fallible callers can use `try_count`/`try_get_at`; the SDK's
//! infallible list contract reports invalid data by panicking with the error.
use ferroboard_core_ferro::{
    KeyReadOnlyList, PerformedActionList, SettingsSubtypeList, StringReadOnlyList,
    SuggestionReadOnlyList, TextKeyReadOnlyList,
};
use ferroui_base::{
    BoxedValue,
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, into_markup_value},
};
use ferroui_controls::{IItemsList, ItemsSource};
use std::{any::Any, rc::Rc};

/// Shared domain collections adapted to FerroUI's native items-list contract.
#[derive(Clone)]
pub enum KeyboardItems {
    Keys(KeyReadOnlyList),
    TextKeys(TextKeyReadOnlyList),
    Predictions(StringReadOnlyList),
    Suggestions(SuggestionReadOnlyList),
    Subtypes(SettingsSubtypeList),
    PerformedActions(PerformedActionList),
}

impl KeyboardItems {
    pub fn try_count(&self) -> Result<usize, MarkupInvokeError> {
        let count = match self {
            Self::Keys(list) => list.count()?,
            Self::TextKeys(list) => list.count()?,
            Self::Predictions(list) => list
                .count()
                .map_err(|e| MarkupInvokeError::Failed(e.to_string()))?,
            Self::Suggestions(list) => list.count()?,
            Self::Subtypes(list) => list.count(),
            Self::PerformedActions(list) => list.count(),
        };
        usize::try_from(count).map_err(|_| {
            MarkupInvokeError::Failed(format!("negative keyboard items count: {count}"))
        })
    }

    pub fn try_get_at(&self, index: usize) -> Result<Option<BoxedValue>, MarkupInvokeError> {
        let count = self.try_count()?;
        if index >= count {
            return Err(MarkupInvokeError::Failed(format!(
                "keyboard item index {index} is outside count {count}"
            )));
        }
        let index = i32::try_from(index)
            .map_err(|_| MarkupInvokeError::Failed("keyboard item index exceeds i32".into()))?;
        Ok(match self {
            Self::Keys(list) => into_markup_value(list.get(index)?),
            Self::TextKeys(list) => into_markup_value(list.get(index)?),
            Self::Suggestions(list) => into_markup_value(list.get(index)?),
            Self::Subtypes(list) => into_markup_value(
                list.get(index)
                    .map_err(|e| MarkupInvokeError::Failed(e.into()))?,
            ),
            Self::PerformedActions(list) => into_markup_value(
                list.get(index)
                    .map_err(|e| MarkupInvokeError::Failed(e.into()))?,
            ),
            Self::Predictions(list) => {
                let value = list
                    .get(index)
                    .map_err(|e| MarkupInvokeError::Failed(e.to_string()))?;
                let value = value
                    .map(|text| text.to_utf8())
                    .transpose()
                    .map_err(|e| MarkupInvokeError::Failed(e.to_string()))?;
                into_markup_value(value)
            }
        })
    }

    pub fn into_source(self) -> ItemsSource {
        ItemsSource::new(Rc::new(self))
    }

    fn storage_ptr(&self) -> *const () {
        match self {
            Self::Keys(list) => list.original().storage_ptr(),
            Self::TextKeys(list) => list.original().storage_ptr(),
            Self::Predictions(list) => list.storage_ptr(),
            Self::Suggestions(list) => list.original().storage_ptr(),
            Self::Subtypes(list) => Rc::as_ptr(&list.original()).cast(),
            Self::PerformedActions(list) => Rc::as_ptr(&list.original()).cast(),
        }
    }
}

impl IItemsList for KeyboardItems {
    fn count(&self) -> usize {
        self.try_count()
            .unwrap_or_else(|error| std::panic::panic_any(error))
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self.try_get_at(index)
            .unwrap_or_else(|error| std::panic::panic_any(error))
    }
    fn identity(&self) -> Option<usize> {
        Some(self.storage_ptr() as usize)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub(crate) fn register_conversions() {
    macro_rules! register {
        ($type:ty, $variant:ident) => {
            ItemsSource::register_binding_conversion_with::<$type>(|list| {
                KeyboardItems::$variant(list.clone()).into_source()
            });
            ValueTypes::register_conversion::<Option<$type>, Option<ItemsSource>>(|list| {
                Some(
                    list.as_ref()
                        .map(|list| KeyboardItems::$variant(list.clone()).into_source()),
                )
            });
        };
    }
    register!(KeyReadOnlyList, Keys);
    register!(TextKeyReadOnlyList, TextKeys);
    register!(StringReadOnlyList, Predictions);
    register!(SuggestionReadOnlyList, Suggestions);
    register!(SettingsSubtypeList, Subtypes);
    register!(PerformedActionList, PerformedActions);
}
