//! Native shared textkey list view for domain properties and XAML indexers.
use ferroboard_core::layouts::{ResourceReadOnlyList, ResourceTextListError, TextKey};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
#[derive(Clone)]
pub struct TextKeyReadOnlyList(ResourceReadOnlyList<TextKey>);
impl PartialEq for TextKeyReadOnlyList {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl Eq for TextKeyReadOnlyList {}
impl TextKeyReadOnlyList {
    pub fn from_original(value: ResourceReadOnlyList<TextKey>) -> Self {
        Self(value)
    }
    pub fn original(&self) -> ResourceReadOnlyList<TextKey> {
        self.0.clone()
    }
    pub fn count(&self) -> Result<i32, MarkupInvokeError> {
        self.0.count().map_err(collection_failure)
    }
    pub fn get(&self, index: i32) -> Result<Option<TextKey>, MarkupInvokeError> {
        self.0.get(index).map_err(collection_failure)
    }
    pub fn as_keys(&self) -> crate::KeyReadOnlyList {
        crate::KeyReadOnlyList::from_original(crate::text_key_covariance::list(&self.0))
    }
}
pub(crate) fn collection_failure(error: ResourceTextListError) -> MarkupInvokeError {
    MarkupInvokeError::Failed(error.to_string())
}
#[doc(hidden)]
pub struct TextKeyReadOnlyListMetadata;
ferroui_base::ferro_markup_type!(class TextKeyReadOnlyListMetadata as "TextKeyList" {
 namespace:"FerroBoard.Layouts",this:TextKeyReadOnlyList,handles:[TextKeyReadOnlyList,Option<TextKeyReadOnlyList>],
 properties:[Count:i32{try_get:TextKeyReadOnlyList::count}],indexers:[(i32)->Option<TextKey>{try_get:TextKeyReadOnlyList::get}]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::TextKeyReadOnlyListMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(TextKeyReadOnlyListMetadata::MARKUP);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<TextKeyReadOnlyList>();
        ValueTypes::register_cast::<TextKeyReadOnlyList, crate::KeyReadOnlyList>(
            TextKeyReadOnlyList::as_keys,
        );
        ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<TextKeyReadOnlyList>(),"::ferroboard_core_ferro::TextKeyReadOnlyList")]);}
    });
}
