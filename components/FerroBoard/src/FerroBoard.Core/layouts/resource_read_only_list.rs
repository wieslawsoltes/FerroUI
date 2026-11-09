//! Extensible resource readers using fallible Rust iterators and RAII.
use super::{ResourceObjectList, ResourceTextListError};
use std::{rc::Rc, sync::Arc};
type Result<T> = std::result::Result<T, ResourceTextListError>;
pub type ResourceItems<T> = Box<dyn Iterator<Item = Result<Option<T>>>>;
/// Source name retained for custom layouts. Iterators own cleanup through Drop.
pub trait IResourceReadOnlyList<T: Clone + 'static> {
    fn count(&self) -> Result<i32>;
    fn get(&self, index: i32) -> Result<Option<T>>;
    fn iter(&self) -> Result<ResourceItems<T>>;
}
#[derive(Clone)]
enum Storage<T: Clone + 'static> {
    Concrete(ResourceObjectList<T>),
    Nullable(ResourceObjectList<Option<T>>),
    Custom(Rc<dyn IResourceReadOnlyList<T>>),
    Slice(Arc<[Option<T>]>),
}
#[derive(Clone)]
pub struct ResourceReadOnlyList<T: Clone + 'static>(Storage<T>);
impl<T: Clone + 'static> PartialEq for ResourceReadOnlyList<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}
impl<T: Clone + 'static> Eq for ResourceReadOnlyList<T> {}
impl<T: Clone + 'static> Default for ResourceReadOnlyList<T> {
    fn default() -> Self {
        ResourceObjectList::default().into()
    }
}
impl<T: Clone + 'static> From<ResourceObjectList<T>> for ResourceReadOnlyList<T> {
    fn from(value: ResourceObjectList<T>) -> Self {
        Self(Storage::Concrete(value))
    }
}
impl<T: Clone + 'static> ResourceReadOnlyList<T> {
    /// Immutable native values for library constants; no mutable storage is exposed.
    pub fn from_values(values: Vec<T>) -> Self {
        Self(Storage::Slice(values.into_iter().map(Some).collect()))
    }
    pub fn from_nullable(value: ResourceObjectList<Option<T>>) -> Self {
        Self(Storage::Nullable(value))
    }
    pub fn from_custom(value: Rc<dyn IResourceReadOnlyList<T>>) -> Self {
        Self(Storage::Custom(value))
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Storage::Concrete(a), Storage::Concrete(b)) => a.ptr_eq(b),
            (Storage::Nullable(a), Storage::Nullable(b)) => a.ptr_eq(b),
            (Storage::Custom(a), Storage::Custom(b)) => Rc::ptr_eq(a, b),
            (Storage::Slice(a), Storage::Slice(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
    pub fn storage_ptr(&self) -> *const () {
        match &self.0 {
            Storage::Concrete(v) => v.storage_ptr(),
            Storage::Nullable(v) => v.storage_ptr(),
            Storage::Custom(v) => Rc::as_ptr(v).cast(),
            Storage::Slice(v) => v.as_ptr().cast(),
        }
    }
    pub fn as_concrete(&self) -> Option<ResourceObjectList<T>> {
        if let Storage::Concrete(v) = &self.0 {
            Some(v.clone())
        } else {
            None
        }
    }
    pub fn as_nullable_concrete(&self) -> Option<ResourceObjectList<Option<T>>> {
        if let Storage::Nullable(v) = &self.0 {
            Some(v.clone())
        } else {
            None
        }
    }
    /// Signed counts remain at the source layout extension boundary.
    pub fn count(&self) -> Result<i32> {
        let len = match &self.0 {
            Storage::Concrete(v) => v.len(),
            Storage::Nullable(v) => v.len(),
            Storage::Slice(v) => v.len(),
            Storage::Custom(v) => return v.count(),
        };
        i32::try_from(len).map_err(|_| ResourceTextListError::IndexOutOfRange)
    }
    pub fn get(&self, index: i32) -> Result<Option<T>> {
        match &self.0 {
            Storage::Concrete(v) => v
                .get(usize::try_from(index).map_err(|_| ResourceTextListError::IndexOutOfRange)?)
                .map(Some),
            Storage::Nullable(v) => {
                v.get(usize::try_from(index).map_err(|_| ResourceTextListError::IndexOutOfRange)?)
            }
            Storage::Custom(v) => v.get(index),
            Storage::Slice(v) => usize::try_from(index)
                .ok()
                .and_then(|i| v.get(i))
                .cloned()
                .ok_or(ResourceTextListError::IndexOutOfRange),
        }
    }
    pub fn iter(&self) -> Result<ResourceItems<T>> {
        Ok(match &self.0 {
            Storage::Concrete(v) => Box::new(v.iter().map(|v| Ok(Some(v)))),
            Storage::Nullable(v) => Box::new(v.iter().map(Ok)),
            Storage::Slice(values) => {
                let values = values.clone();
                let mut index = 0;
                Box::new(std::iter::from_fn(move || {
                    let value = values.get(index)?.clone();
                    index += 1;
                    Some(Ok(value))
                }))
            }
            Storage::Custom(v) => return v.iter(),
        })
    }
    pub fn visit<E: From<ResourceTextListError>>(
        &self,
        mut body: impl FnMut(Option<T>) -> std::result::Result<(), E>,
    ) -> std::result::Result<(), E> {
        for value in self.iter()? {
            body(value?)?;
        }
        Ok(())
    }
    pub(crate) fn append_to(&self, output: &mut Vec<Option<T>>) -> Result<()> {
        self.visit(|value| {
            output.push(value);
            Ok(())
        })
    }
}
