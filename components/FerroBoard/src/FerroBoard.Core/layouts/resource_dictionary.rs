//! Shared insertion-ordered resource maps with Rust snapshot iteration.
use super::{ResourceItems, ResourceReadOnlyList, ResourceTextListError};
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::{hash::Hash, rc::Rc, sync::Arc};
type Result<T> = std::result::Result<T, ResourceTextListError>;
pub trait ResourceDictionaryKey: Clone + Eq + Hash + 'static {}
impl<T: Clone + Eq + Hash + 'static> ResourceDictionaryKey for T {}
#[derive(Clone)]
pub struct ResourceDictionaryEntry<K, V> {
    pub key: Option<K>,
    pub value: Option<V>,
}
pub trait ResourceSequence<T: Clone + 'static> {
    fn iter(&self) -> Result<ResourceItems<T>>;
}
pub type ResourceEnumerable<T> = Rc<dyn ResourceSequence<T>>;
impl<T: Clone + 'static> ResourceSequence<T> for ResourceReadOnlyList<T> {
    fn iter(&self) -> Result<ResourceItems<T>> {
        self.iter()
    }
}
/// Custom popup maps retain synchronous lookup and native fallible iteration.
pub trait IResourceReadOnlyDictionary<K: ResourceDictionaryKey, V: Clone + 'static> {
    fn count(&self) -> Result<i32>;
    fn get(&self, key: Option<&K>) -> Result<Option<V>>;
    fn contains_key(&self, key: Option<&K>) -> Result<bool>;
    fn try_get_value(&self, key: Option<&K>) -> Result<(bool, Option<V>)>;
    fn keys(&self) -> Result<Option<ResourceEnumerable<K>>>;
    fn values(&self) -> Result<Option<ResourceEnumerable<V>>>;
    fn iter(&self) -> Result<ResourceItems<ResourceDictionaryEntry<K, V>>>;
}
type SharedMap<K, V> = Arc<RwLock<IndexMap<K, Option<V>>>>;
#[derive(Clone)]
pub struct ResourceDictionary<K: ResourceDictionaryKey, V: Clone + 'static>(SharedMap<K, V>);
impl<K: ResourceDictionaryKey, V: Clone + 'static> Default for ResourceDictionary<K, V> {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(IndexMap::new())))
    }
}
impl<K: ResourceDictionaryKey, V: Clone + 'static> ResourceDictionary<K, V> {
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn count(&self) -> Result<i32> {
        i32::try_from(self.0.read().len()).map_err(|_| ResourceTextListError::IndexOutOfRange)
    }
    pub fn try_get_value(&self, key: Option<&K>) -> Result<(bool, Option<V>)> {
        let key = key.ok_or(ResourceTextListError::ArgumentNull)?;
        Ok(match self.0.read().get(key) {
            Some(v) => (true, v.clone()),
            None => (false, None),
        })
    }
    pub fn get(&self, key: Option<&K>) -> Result<Option<V>> {
        let (found, value) = self.try_get_value(key)?;
        if found {
            Ok(value)
        } else {
            Err(ResourceTextListError::KeyNotFound)
        }
    }
    pub fn contains_key(&self, key: Option<&K>) -> Result<bool> {
        Ok(self
            .0
            .read()
            .contains_key(key.ok_or(ResourceTextListError::ArgumentNull)?))
    }
    pub fn set(&self, key: K, value: Option<V>) {
        let previous = self.0.write().insert(key, value);
        drop(previous);
    }
    pub fn add(&self, key: K, value: Option<V>) -> Result<()> {
        let mut values = self.0.write();
        if values.contains_key(&key) {
            return Err(ResourceTextListError::DuplicateKey);
        }
        values.insert(key, value);
        Ok(())
    }
    pub fn remove(&self, key: Option<&K>) -> Result<bool> {
        let previous = self
            .0
            .write()
            .shift_remove(key.ok_or(ResourceTextListError::ArgumentNull)?);
        Ok(previous.is_some())
    }
    pub fn clear(&self) {
        let previous = std::mem::take(&mut *self.0.write());
        drop(previous);
    }
    pub fn keys(&self) -> ResourceEnumerable<K> {
        Rc::new(MapKeys(self.clone()))
    }
    pub fn values(&self) -> ResourceEnumerable<V> {
        Rc::new(MapValues(self.clone()))
    }
    pub fn iter(&self) -> Result<ResourceItems<ResourceDictionaryEntry<K, V>>> {
        let snapshot: Vec<_> = self
            .0
            .read()
            .iter()
            .map(|(k, v)| {
                Ok(Some(ResourceDictionaryEntry {
                    key: Some(k.clone()),
                    value: v.clone(),
                }))
            })
            .collect();
        Ok(Box::new(snapshot.into_iter()))
    }
}
struct MapKeys<K: ResourceDictionaryKey, V: Clone + 'static>(ResourceDictionary<K, V>);
struct MapValues<K: ResourceDictionaryKey, V: Clone + 'static>(ResourceDictionary<K, V>);
impl<K: ResourceDictionaryKey, V: Clone + 'static> ResourceSequence<K> for MapKeys<K, V> {
    fn iter(&self) -> Result<ResourceItems<K>> {
        let values: Vec<_> = self
            .0
            .0
            .read()
            .keys()
            .cloned()
            .map(|v| Ok(Some(v)))
            .collect();
        Ok(Box::new(values.into_iter()))
    }
}
impl<K: ResourceDictionaryKey, V: Clone + 'static> ResourceSequence<V> for MapValues<K, V> {
    fn iter(&self) -> Result<ResourceItems<V>> {
        let values: Vec<_> = self.0.0.read().values().cloned().map(Ok).collect();
        Ok(Box::new(values.into_iter()))
    }
}
#[derive(Clone)]
enum Storage<K: ResourceDictionaryKey, V: Clone + 'static> {
    Concrete(ResourceDictionary<K, V>),
    Custom(Rc<dyn IResourceReadOnlyDictionary<K, V>>),
}
#[derive(Clone)]
pub struct ResourceReadOnlyDictionary<K: ResourceDictionaryKey, V: Clone + 'static>(Storage<K, V>);
impl<K: ResourceDictionaryKey, V: Clone + 'static> Default for ResourceReadOnlyDictionary<K, V> {
    fn default() -> Self {
        ResourceDictionary::default().into()
    }
}
impl<K: ResourceDictionaryKey, V: Clone + 'static> From<ResourceDictionary<K, V>>
    for ResourceReadOnlyDictionary<K, V>
{
    fn from(value: ResourceDictionary<K, V>) -> Self {
        Self(Storage::Concrete(value))
    }
}
impl<K: ResourceDictionaryKey, V: Clone + 'static> ResourceReadOnlyDictionary<K, V> {
    pub fn from_custom(value: Rc<dyn IResourceReadOnlyDictionary<K, V>>) -> Self {
        Self(Storage::Custom(value))
    }
    pub fn as_concrete(&self) -> Option<ResourceDictionary<K, V>> {
        if let Storage::Concrete(v) = &self.0 {
            Some(v.clone())
        } else {
            None
        }
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Storage::Concrete(a), Storage::Concrete(b)) => a.ptr_eq(b),
            (Storage::Custom(a), Storage::Custom(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
    pub fn count(&self) -> Result<i32> {
        match &self.0 {
            Storage::Concrete(v) => v.count(),
            Storage::Custom(v) => v.count(),
        }
    }
    pub fn try_get_value(&self, key: Option<&K>) -> Result<(bool, Option<V>)> {
        match &self.0 {
            Storage::Concrete(v) => v.try_get_value(key),
            Storage::Custom(v) => v.try_get_value(key),
        }
    }
    pub fn get(&self, key: Option<&K>) -> Result<Option<V>> {
        match &self.0 {
            Storage::Concrete(v) => v.get(key),
            Storage::Custom(v) => v.get(key),
        }
    }
    pub fn contains_key(&self, key: Option<&K>) -> Result<bool> {
        match &self.0 {
            Storage::Concrete(v) => v.contains_key(key),
            Storage::Custom(v) => v.contains_key(key),
        }
    }
    pub fn keys(&self) -> Result<Option<ResourceEnumerable<K>>> {
        match &self.0 {
            Storage::Concrete(v) => Ok(Some(v.keys())),
            Storage::Custom(v) => v.keys(),
        }
    }
    pub fn values(&self) -> Result<Option<ResourceEnumerable<V>>> {
        match &self.0 {
            Storage::Concrete(v) => Ok(Some(v.values())),
            Storage::Custom(v) => v.values(),
        }
    }
    pub fn iter(&self) -> Result<ResourceItems<ResourceDictionaryEntry<K, V>>> {
        match &self.0 {
            Storage::Concrete(v) => v.iter(),
            Storage::Custom(v) => v.iter(),
        }
    }
}
