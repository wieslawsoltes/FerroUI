//! `ChangeAwareCache<TObject, TKey>` (`Cache/ChangeAwareCache.cs`).

use super::{Change, ChangeReason, ChangeSet, ICache, IQuery};
use ferroui_system::collections::generic::Dictionary;
use std::hash::Hash;

/// A keyed collection that records its changes until they are captured (`ChangeAwareCache<TObject,
/// TKey>`): the building block of the cache operators.
pub struct ChangeAwareCache<T, K> {
    data: Dictionary<K, T>,
    changes: Vec<Change<T, K>>,
}

impl<T: Clone, K: Eq + Hash + Clone> Default for ChangeAwareCache<T, K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, K: Eq + Hash + Clone> ChangeAwareCache<T, K> {
    /// An empty cache.
    pub fn new() -> Self {
        Self { data: Dictionary::new(), changes: Vec::new() }
    }

    /// An empty cache with room for `capacity` items.
    pub fn with_capacity(capacity: usize) -> Self {
        Self { data: Dictionary::with_capacity(capacity), changes: Vec::with_capacity(capacity) }
    }

    /// A cache of the items of `data`, without changes (`ChangeAwareCache(Dictionary<TKey,
    /// TObject>)`).
    pub fn from_dictionary(data: Dictionary<K, T>) -> Self {
        Self { data, changes: Vec::new() }
    }

    /// The underlying dictionary (`GetDictionary`).
    pub fn dictionary(&self) -> &Dictionary<K, T> {
        &self.data
    }

    /// The items, giving up the cache.
    pub fn into_dictionary(self) -> Dictionary<K, T> {
        self.data
    }

    /// Adds an item (`Add`).
    ///
    /// # Panics
    /// When the key is there already (`ArgumentException` of the dictionary).
    pub fn add(&mut self, item: T, key: K) {
        self.changes.push(Change::new(ChangeReason::Add, key.clone(), item.clone()));
        self.data.add(key, item);
    }

    /// The changes since the last capture, and a clean slate (`CaptureChanges`).
    pub fn capture_changes(&mut self) -> ChangeSet<T, K> {
        if self.changes.is_empty() {
            return ChangeSet::empty();
        }
        ChangeSet::from(std::mem::take(&mut self.changes))
    }

    /// Whether there are changes not captured yet.
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }
}

impl<T: Clone, K: Eq + Hash + Clone> IQuery<T, K> for ChangeAwareCache<T, K> {
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

impl<T: Clone, K: Eq + Hash + Clone> ICache<T, K> for ChangeAwareCache<T, K> {
    fn add_or_update(&mut self, item: T, key: K) {
        let change = match self.data.get(&key) {
            Some(existing) => Change::update(key.clone(), item.clone(), existing.clone()),
            None => Change::new(ChangeReason::Add, key.clone(), item.clone()),
        };
        self.changes.push(change);
        self.data.insert(key, item);
    }

    fn clear(&mut self) {
        let removed = self.data.iter().map(|(key, item)| Change::new(ChangeReason::Remove, key.clone(), item.clone()));
        self.changes.extend(removed);
        self.data.clear();
    }

    fn clone_changes(&mut self, changes: &ChangeSet<T, K>) {
        for change in changes {
            match change.reason() {
                ChangeReason::Add | ChangeReason::Update => self.add_or_update(change.current().clone(), change.key().clone()),
                ChangeReason::Remove => self.remove(change.key()),
                ChangeReason::Refresh => self.refresh(change.key()),
                ChangeReason::Moved => {}
            }
        }
    }

    fn refresh_all(&mut self) {
        let refreshed = self.data.iter().map(|(key, item)| Change::new(ChangeReason::Refresh, key.clone(), item.clone()));
        self.changes.extend(refreshed);
    }

    fn refresh_keys(&mut self, keys: &mut dyn Iterator<Item = K>) {
        for key in keys {
            self.refresh(&key);
        }
    }

    fn refresh(&mut self, key: &K) {
        if let Some(existing) = self.data.get(key) {
            self.changes.push(Change::new(ChangeReason::Refresh, key.clone(), existing.clone()));
        }
    }

    fn remove(&mut self, key: &K) {
        if let Some(existing) = self.data.remove(key) {
            self.changes.push(Change::new(ChangeReason::Remove, key.clone(), existing));
        }
    }

    fn remove_keys(&mut self, keys: &mut dyn Iterator<Item = K>) {
        for key in keys {
            self.remove(&key);
        }
    }
}
