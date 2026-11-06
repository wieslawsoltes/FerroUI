//! Native markup view over the actual shared suggestion result; no CLR collection facets.
use ferroboard_core::text::{Suggestion, SuggestionError, SuggestionList};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::rc::Rc;
pub(crate) fn failure(error: SuggestionError) -> MarkupInvokeError {
    MarkupInvokeError::Failed(match error {
        SuggestionError::Custom(message) => message,
        other => format!("{other:?}"),
    })
}
#[derive(Clone)]
pub struct SuggestionReadOnlyList(SuggestionList);
impl PartialEq for SuggestionReadOnlyList {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl Eq for SuggestionReadOnlyList {}
impl SuggestionReadOnlyList {
    pub fn from_original(value: SuggestionList) -> Self {
        Self(value)
    }
    pub fn original(&self) -> SuggestionList {
        self.0.clone()
    }
    pub fn count(&self) -> Result<i32, MarkupInvokeError> {
        i32::try_from(self.0.len().map_err(failure)?)
            .map_err(|_| MarkupInvokeError::Failed("suggestion count exceeds markup i32".into()))
    }
    pub fn get(&self, index: i32) -> Result<Option<Rc<Suggestion>>, MarkupInvokeError> {
        self.0
            .get(
                usize::try_from(index)
                    .map_err(|_| MarkupInvokeError::Failed("negative suggestion index".into()))?,
            )
            .map_err(failure)
    }
}
#[doc(hidden)]
pub struct SuggestionReadOnlyListMetadata;
ferroui_base::ferro_markup_type!(class SuggestionReadOnlyListMetadata as "SuggestionList" {
 namespace:"FerroBoard.Text",this:SuggestionReadOnlyList,handles:[SuggestionReadOnlyList,Option<SuggestionReadOnlyList>],
 properties:[Count:i32{try_get:SuggestionReadOnlyList::count}],indexers:[(i32)->Option<Rc<Suggestion>>{try_get:SuggestionReadOnlyList::get}]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::SuggestionReadOnlyListMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(SuggestionReadOnlyListMetadata::MARKUP);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<SuggestionReadOnlyList>();
        ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<SuggestionReadOnlyList>(),"::ferroboard_core_ferro::SuggestionReadOnlyList")]);}
    });
}
