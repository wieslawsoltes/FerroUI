//! Shared resource text values. UTF-16 and nullable entries preserve keyboard data formats.
//! Handles share an Arc; iteration snapshots the values before caller code runs.
use crate::text::Utf16Text;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceTextListError {
    IndexOutOfRange,
    NotSupported,
    Modified,
    NullReference,
    ArgumentNull,
    KeyNotFound,
    DuplicateKey,
}
impl std::fmt::Display for ResourceTextListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::IndexOutOfRange => "index was out of range",
            Self::NotSupported => "resource operation is unsupported",
            Self::Modified => "resource data changed while being read",
            Self::NullReference => "object reference was null",
            Self::ArgumentNull => "argument was null",
            Self::KeyNotFound => "key was not present",
            Self::DuplicateKey => "key was already present",
        })
    }
}
impl std::error::Error for ResourceTextListError {}
type Result<T> = std::result::Result<T, ResourceTextListError>;

pub type SharedResourceTexts = Arc<RwLock<Vec<Option<Utf16Text>>>>;

#[derive(Clone, Default)]
pub struct ResourceTexts(SharedResourceTexts);
impl PartialEq for ResourceTexts {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}
impl Eq for ResourceTexts {}
impl ResourceTexts {
    pub fn new(values: Vec<Utf16Text>) -> Self {
        Self::from_nullable(values.into_iter().map(Some).collect())
    }
    pub fn from_nullable(values: Vec<Option<Utf16Text>>) -> Self {
        Self::from_shared(Arc::new(RwLock::new(values)))
    }
    pub fn from_shared(values: SharedResourceTexts) -> Self {
        Self(values)
    }
    pub fn shared_values(&self) -> SharedResourceTexts {
        self.0.clone()
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    /// Comparison only; valid while the shared allocation is retained.
    pub fn storage_ptr(&self) -> *const () {
        Arc::as_ptr(&self.0).cast()
    }
    pub fn len(&self) -> usize {
        self.0.read().len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.read().is_empty()
    }
    pub fn snapshot(&self) -> Vec<Option<Utf16Text>> {
        self.0.read().clone()
    }
    pub fn get(&self, index: usize) -> Result<Option<Utf16Text>> {
        self.0
            .read()
            .get(index)
            .cloned()
            .ok_or(ResourceTextListError::IndexOutOfRange)
    }
    pub fn set(&self, index: usize, value: Option<Utf16Text>) -> Result<()> {
        let mut values = self.0.write();
        let slot = values
            .get_mut(index)
            .ok_or(ResourceTextListError::IndexOutOfRange)?;
        let previous = std::mem::replace(slot, value);
        drop(values);
        drop(previous);
        Ok(())
    }
    pub fn push(&self, value: Option<Utf16Text>) {
        self.0.write().push(value);
    }
    pub fn insert(&self, index: usize, value: Option<Utf16Text>) -> Result<()> {
        let mut values = self.0.write();
        if index > values.len() {
            return Err(ResourceTextListError::IndexOutOfRange);
        }
        values.insert(index, value);
        Ok(())
    }
    pub fn remove_value(&self, value: &Option<Utf16Text>) -> bool {
        let mut values = self.0.write();
        let Some(index) = values.iter().position(|v| v == value) else {
            return false;
        };
        let previous = values.remove(index);
        drop(values);
        drop(previous);
        true
    }
    pub fn remove_at(&self, index: usize) -> Result<Option<Utf16Text>> {
        let mut values = self.0.write();
        if index >= values.len() {
            return Err(ResourceTextListError::IndexOutOfRange);
        }
        Ok(values.remove(index))
    }
    pub fn clear(&self) {
        let previous = std::mem::take(&mut *self.0.write());
        drop(previous);
    }
    pub fn position(&self, value: &Option<Utf16Text>) -> Option<usize> {
        self.0.read().iter().position(|v| v == value)
    }
    pub fn contains(&self, value: &Option<Utf16Text>) -> bool {
        self.position(value).is_some()
    }
    /// A coherent snapshot; no read lock spans callbacks or user iteration.
    pub fn iter(&self) -> std::vec::IntoIter<Option<Utf16Text>> {
        self.snapshot().into_iter()
    }
}
