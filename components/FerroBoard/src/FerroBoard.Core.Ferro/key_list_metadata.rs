//! Native shared key list view for domain properties and XAML indexers.
use ferroboard_core::layouts::{Key, ResourceReadOnlyList, ResourceTextListError};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
#[derive(Clone)]
pub struct KeyReadOnlyList(ResourceReadOnlyList<Key>);
impl PartialEq for KeyReadOnlyList {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl Eq for KeyReadOnlyList {}
impl KeyReadOnlyList {
    pub fn from_original(value: ResourceReadOnlyList<Key>) -> Self {
        Self(value)
    }
    pub fn original(&self) -> ResourceReadOnlyList<Key> {
        self.0.clone()
    }
    pub fn count(&self) -> Result<i32, MarkupInvokeError> {
        self.0.count().map_err(collection_failure)
    }
    pub fn get(&self, index: i32) -> Result<Option<Key>, MarkupInvokeError> {
        self.0.get(index).map_err(collection_failure)
    }
}
pub(crate) fn collection_failure(error: ResourceTextListError) -> MarkupInvokeError {
    MarkupInvokeError::Failed(error.to_string())
}
#[doc(hidden)]
pub struct KeyReadOnlyListMetadata;
ferroui_base::ferro_markup_type!(class KeyReadOnlyListMetadata as "KeyList" {
 namespace:"FerroBoard.Layouts",this:KeyReadOnlyList,handles:[KeyReadOnlyList,Option<KeyReadOnlyList>],
 properties:[Count:i32{try_get:KeyReadOnlyList::count}],indexers:[(i32)->Option<Key>{try_get:KeyReadOnlyList::get}]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::KeyReadOnlyListMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(KeyReadOnlyListMetadata::MARKUP);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<KeyReadOnlyList>();
        ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<KeyReadOnlyList>(),"::ferroboard_core_ferro::KeyReadOnlyList")]);}
    });
}
