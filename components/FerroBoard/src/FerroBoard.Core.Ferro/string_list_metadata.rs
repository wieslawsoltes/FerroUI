//! Native prediction-list binding over the actual Core Rc, without runtime collection facades.
use ferroboard_core::text::{PredictionList, PredictionSource};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::rc::Rc;
#[derive(Clone)]
pub struct StringReadOnlyList(PredictionList);
impl StringReadOnlyList {
    pub fn count(&self) -> Result<i32, MarkupInvokeError> {
        i32::try_from(
            self.0
                .len()
                .map_err(crate::suggestion_list_metadata::failure)?,
        )
        .map_err(|_| MarkupInvokeError::Failed("prediction count exceeds markup i32".into()))
    }
    pub fn get(
        &self,
        index: i32,
    ) -> Result<Option<ferroboard_core::text::Utf16Text>, MarkupInvokeError> {
        let index = usize::try_from(index)
            .map_err(|_| MarkupInvokeError::Failed("negative prediction index".into()))?;
        self.0
            .get(index)
            .map_err(crate::suggestion_list_metadata::failure)
    }
    pub fn interface(&self) -> PredictionList {
        self.0.clone()
    }
    /// Identity of the actual retained resource storage or custom reader.
    pub fn storage_ptr(&self) -> *const () {
        crate::resource_string_list::storage_ptr(self)
    }
    pub fn downcast<T: PredictionSource>(&self) -> Option<Rc<T>> {
        let value: Rc<dyn std::any::Any> = self.0.clone();
        value.downcast().ok()
    }
}
impl std::ops::Deref for StringReadOnlyList {
    type Target = dyn PredictionSource;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl PartialEq for StringReadOnlyList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || crate::resource_string_list::same_storage(self, other)
    }
}
impl Eq for StringReadOnlyList {}
pub fn native_string_list(original: PredictionList) -> StringReadOnlyList {
    StringReadOnlyList(original)
}
#[doc(hidden)]
pub struct StringReadOnlyListMetadata;
ferroui_base::ferro_markup_type!(class StringReadOnlyListMetadata as "PredictionList" {
 namespace:"FerroBoard.Text",this:StringReadOnlyList,handles:[StringReadOnlyList,Option<StringReadOnlyList>],
 properties:[Count:i32{try_get:|v:&StringReadOnlyList|v.count()}],
 indexers:[(i32)->Option<String>{try_get:|v:&StringReadOnlyList,index|{let value=v.get(index)?;crate::utf16_metadata::to_native_optional(value).map_err(MarkupInvokeError::Failed)}}]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::StringReadOnlyListMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(StringReadOnlyListMetadata::MARKUP);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<StringReadOnlyList>();
        ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<StringReadOnlyList>(),"::ferroboard_core_ferro::StringReadOnlyList")]);}
    });
}
