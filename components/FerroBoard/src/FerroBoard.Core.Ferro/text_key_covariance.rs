//! Native trait upcast over shared text keys.
use ferroboard_core::layouts::{
    IResourceReadOnlyList, Key, ResourceItems, ResourceReadOnlyList, ResourceTextListError, TextKey,
};
use std::rc::Rc;
type Result<T> = std::result::Result<T, ResourceTextListError>;
fn upcast(value: Option<TextKey>) -> Option<Key> {
    value.map(|v| v as Key)
}
struct List(ResourceReadOnlyList<TextKey>);
impl IResourceReadOnlyList<Key> for List {
    fn count(&self) -> Result<i32> {
        self.0.count()
    }
    fn get(&self, index: i32) -> Result<Option<Key>> {
        self.0.get(index).map(upcast)
    }
    fn iter(&self) -> Result<ResourceItems<Key>> {
        Ok(Box::new(self.0.iter()?.map(|v| v.map(upcast))))
    }
}
pub(crate) fn list(text: &ResourceReadOnlyList<TextKey>) -> ResourceReadOnlyList<Key> {
    ResourceReadOnlyList::from_custom(Rc::new(List(text.clone())))
}
