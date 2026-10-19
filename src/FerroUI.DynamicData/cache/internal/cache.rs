//! `Cache<TObject, TKey>` (`Cache/Internal/Cache.cs`): a keyed collection that records no changes.

use crate::cache::{ChangeReason, ChangeSet, ICache, IQuery};
use ferroui_system::collections::generic::Dictionary;
use std::hash::Hash;

/// A keyed collection without change tracking (`Cache<TObject, TKey>`).
#[derive(Clone)]
pub(crate) struct Cache<T, K> {
    data: Dictionary<K, T>,
}

impl<T: Clone, K: Eq + Hash + Clone> Cache<T, K> {
    pub(crate) fn new() -> Self {
        Self { data: Dictionary::new() }
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self { data: Dictionary::with_capacity(capacity) }
    }

    /// A cache over `data` (`Cache(Dictionary<TKey, TObject>)`).
    pub(crate) fn from_dictionary(data: Dictionary<K, T>) -> Self {
        Self { data }
    }

    /// The underlying dictionary (`GetDictionary` of the callers).
    pub(crate) fn dictionary(&self) -> &Dictionary<K, T> {
        &self.data
    }

    pub(crate) fn into_dictionary(self) -> Dictionary<K, T> {
        self.data
    }
}

impl<T: Clone, K: Eq + Hash + Clone> IQuery<T, K> for Cache<T, K> {
    fn count(&self) -> usize {
        self.data.len()
    }

    fn items(&self) -> Vec<T> {
        self.data.values().cloned().collect()
    }

    fn keys(&self) -> Vec<K> {
        self.data.keys().cloned().collect()
    }

    fn key_values(&self) -> Vec<(K, T)> {
        self.data.iter().map(|(key, item)| (key.clone(), item.clone())).collect()
    }

    fn lookup(&self, key: &K) -> Option<T> {
        self.data.get(key).cloned()
    }
}

impl<T: Clone, K: Eq + Hash + Clone> ICache<T, K> for Cache<T, K> {
    fn add_or_update(&mut self, item: T, key: K) {
        self.data.insert(key, item);
    }

    fn clear(&mut self) {
        self.data.clear();
    }

    fn clone_changes(&mut self, changes: &ChangeSet<T, K>) {
        for change in changes {
            match change.reason() {
                ChangeReason::Update | ChangeReason::Add => {
                    self.data.insert(change.key().clone(), change.current().clone());
                }
                ChangeReason::Remove => {
                    self.data.remove(change.key());
                }
                ChangeReason::Refresh | ChangeReason::Moved => {}
            }
        }
    }

    fn refresh_all(&mut self) {}

    fn refresh_keys(&mut self, _keys: &mut dyn Iterator<Item = K>) {}

    fn refresh(&mut self, _key: &K) {}

    fn remove(&mut self, key: &K) {
        self.data.remove(key);
    }

    fn remove_keys(&mut self, keys: &mut dyn Iterator<Item = K>) {
        for key in keys {
            self.data.remove(&key);
        }
    }
}
