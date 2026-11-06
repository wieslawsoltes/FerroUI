//! Shared fixed-length code-point storage. No SDK array runtime or interning.
use super::KeyDataError;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct CodePointArray(Arc<RwLock<Box<[i32]>>>);
impl PartialEq for CodePointArray {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}
impl Eq for CodePointArray {}
impl CodePointArray {
    pub fn new(values: Vec<i32>) -> Self {
        Self(Arc::new(RwLock::new(values.into_boxed_slice())))
    }
    pub fn len(&self) -> usize {
        self.0.read().len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.read().is_empty()
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn get(&self, index: i32) -> Result<i32, KeyDataError> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.0.read().get(index).copied())
            .ok_or(KeyDataError::ArrayIndexOutOfRange)
    }
    pub fn set(&self, index: i32, value: i32) -> Result<(), KeyDataError> {
        let index = usize::try_from(index).map_err(|_| KeyDataError::ArrayIndexOutOfRange)?;
        let mut values = self.0.write();
        *values
            .get_mut(index)
            .ok_or(KeyDataError::ArrayIndexOutOfRange)? = value;
        Ok(())
    }
    pub fn snapshot(&self) -> Vec<i32> {
        self.0.read().to_vec()
    }
}
