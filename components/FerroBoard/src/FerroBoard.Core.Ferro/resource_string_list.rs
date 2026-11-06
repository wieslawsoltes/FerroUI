//! Native IReadOnlyList<string> views over the original resource list or caller's actual interface.
use ferroboard_core::{
    layouts::{ResourceReadOnlyTextList, ResourceTexts},
    text::{PredictionSource, SuggestionError, SuggestionResult, Utf16Text},
};
use std::rc::Rc;
struct ResourceListObject(ResourceTexts);
impl PartialEq for ResourceListObject {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl PredictionSource for ResourceListObject {
    fn len(&self) -> SuggestionResult<usize> {
        Ok(self.0.len())
    }
    fn get(&self, index: usize) -> SuggestionResult<Option<Utf16Text>> {
        self.0.get(index).map_err(SuggestionError::Collection)
    }
    fn iter(&self) -> SuggestionResult<ferroboard_core::text::PredictionIter<'_>> {
        Ok(Box::new(self.0.iter().map(Ok)))
    }
}
fn resource_view(original: ResourceTexts) -> crate::StringReadOnlyList {
    crate::native_string_list(Rc::new(ResourceListObject(original)))
}
pub(crate) fn same_storage(a: &crate::StringReadOnlyList, b: &crate::StringReadOnlyList) -> bool {
    match (
        a.downcast::<ResourceListObject>(),
        b.downcast::<ResourceListObject>(),
    ) {
        (Some(a), Some(b)) => a.0.ptr_eq(&b.0),
        _ => false,
    }
}
pub(crate) fn storage_ptr(value: &crate::StringReadOnlyList) -> *const () {
    match value.downcast::<ResourceListObject>() {
        Some(resource) => resource.0.storage_ptr(),
        None => Rc::as_ptr(&value.interface()).cast(),
    }
}
pub(crate) fn native(value: ResourceReadOnlyTextList) -> crate::StringReadOnlyList {
    match value {
        ResourceReadOnlyTextList::Concrete(value) => resource_view(value),
        ResourceReadOnlyTextList::Custom(value) => crate::native_string_list(value),
    }
}
pub(crate) fn original(value: crate::StringReadOnlyList) -> ResourceReadOnlyTextList {
    if let Some(resource) = value.downcast::<ResourceListObject>() {
        resource.0.clone().into()
    } else {
        ResourceReadOnlyTextList::Custom(value.interface())
    }
}
