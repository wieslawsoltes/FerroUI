//! Checked markup view retaining the native popup alternatives buffer.
use ferroboard_core::layouts::{PopupKeys, ResourceTextListError, TextKey};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
#[derive(Clone)]
pub struct PopupKeysView(PopupKeys);
impl PartialEq for PopupKeysView {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl Eq for PopupKeysView {}
impl PopupKeysView {
    pub fn new(value: PopupKeys) -> Self {
        Self(value)
    }
    pub fn source(&self) -> PopupKeys {
        self.0.clone()
    }
    pub fn count(&self) -> Result<i32, MarkupInvokeError> {
        i32::try_from(self.0.len()).map_err(|_| {
            MarkupInvokeError::Failed("popup alternative count exceeds markup i32".into())
        })
    }
    pub fn get(&self, index: i32) -> Result<Option<TextKey>, MarkupInvokeError> {
        let index = usize::try_from(index).map_err(|_| {
            crate::resource_error_metadata::failure(ResourceTextListError::IndexOutOfRange)
        })?;
        self.0
            .get_nullable(index)
            .map_err(crate::resource_error_metadata::failure)
    }
}
#[doc(hidden)]
pub struct PopupKeysMetadata;
ferroui_base::ferro_markup_type!(class PopupKeysMetadata as "PopupKeys" {
 namespace:"FerroBoard.Layouts",this:PopupKeysView,handles:[PopupKeysView,Option<PopupKeysView>],
 properties:[Count:i32{try_get:PopupKeysView::count}],indexers:[(i32)->Option<TextKey>{try_get:PopupKeysView::get}]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::popup_keys_metadata::PopupKeysMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(PopupKeysMetadata::MARKUP);
    ValueTypes::register_global(ValueTypes::register_nullable::<PopupKeysView>);
}
