//! Shared resource values; snapshots release locks before extension code runs.
use super::ResourceTextListError;
use parking_lot::RwLock;
use std::sync::Arc;
#[derive(Clone)]
pub struct ResourceObjectList<T>(Arc<RwLock<Vec<T>>>);
impl<T> Default for ResourceObjectList<T> {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(Vec::new())))
    }
}
impl<T> ResourceObjectList<T> {
    pub fn new(values: Vec<T>) -> Self {
        Self(Arc::new(RwLock::new(values)))
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn storage_ptr(&self) -> *const () {
        Arc::as_ptr(&self.0).cast()
    }
    pub fn len(&self) -> usize {
        self.0.read().len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.read().is_empty()
    }
    pub fn push(&self, value: T) {
        self.0.write().push(value);
    }
    pub fn clear(&self) {
        let previous = std::mem::take(&mut *self.0.write());
        drop(previous);
    }
    pub fn set(&self, index: usize, value: T) -> Result<(), ResourceTextListError> {
        let mut values = self.0.write();
        let slot = values
            .get_mut(index)
            .ok_or(ResourceTextListError::IndexOutOfRange)?;
        let previous = std::mem::replace(slot, value);
        drop(values);
        drop(previous);
        Ok(())
    }
    pub fn insert(&self, index: usize, value: T) -> Result<(), ResourceTextListError> {
        let mut values = self.0.write();
        if index > values.len() {
            return Err(ResourceTextListError::IndexOutOfRange);
        }
        values.insert(index, value);
        Ok(())
    }
    pub fn remove(&self, index: usize) -> Result<T, ResourceTextListError> {
        let mut values = self.0.write();
        if index >= values.len() {
            return Err(ResourceTextListError::IndexOutOfRange);
        }
        Ok(values.remove(index))
    }
}
impl<T: Clone> ResourceObjectList<T> {
    pub fn snapshot(&self) -> Vec<T> {
        self.0.read().clone()
    }
    pub fn get(&self, index: usize) -> Result<T, ResourceTextListError> {
        self.0
            .read()
            .get(index)
            .cloned()
            .ok_or(ResourceTextListError::IndexOutOfRange)
    }
    pub fn iter(&self) -> std::vec::IntoIter<T> {
        self.snapshot().into_iter()
    }
}
