//! `Dictionary<TKey, TValue>`, with the order of enumeration of .NET.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

/// A map from keys to values that enumerates its entries in the order of .NET's
/// `Dictionary<TKey, TValue>`: in the order they were added, except that an entry added after a
/// removal takes the place of the entry removed last (the free list of .NET), and `clear` starts
/// over.
///
/// Code ported from .NET that enumerates a dictionary (its keys, values, or a `Select` over it)
/// produces the same sequence with this type; `HashMap` would give an arbitrary one.
#[derive(Clone)]
pub struct Dictionary<K, V> {
    /// The entries by slot; a removed entry leaves a hole that the free list reuses.
    entries: Vec<Option<(K, V)>>,
    /// The slots of the removed entries, the one removed last at the end (`_freeList`).
    free: Vec<usize>,
    /// The slot of each key (`_buckets`).
    index: HashMap<K, usize>,
}

impl<K, V> Default for Dictionary<K, V> {
    fn default() -> Self {
        Self { entries: Vec::new(), free: Vec::new(), index: HashMap::new() }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Dictionary<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V> Dictionary<K, V> {
    /// The entries in the order of enumeration.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().filter_map(|entry| entry.as_ref().map(|(key, value)| (key, value)))
    }

    /// The entries in the order of enumeration, with their values to change.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.entries.iter_mut().filter_map(|entry| entry.as_mut().map(|(key, value)| (&*key, value)))
    }

    /// The keys in the order of enumeration (`Keys`).
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(key, _)| key)
    }

    /// The values in the order of enumeration (`Values`).
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, value)| value)
    }
}

impl<K: Eq + Hash + Clone, V> Dictionary<K, V> {
    /// An empty dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty dictionary with room for `capacity` entries.
    pub fn with_capacity(capacity: usize) -> Self {
        Self { entries: Vec::with_capacity(capacity), free: Vec::new(), index: HashMap::with_capacity(capacity) }
    }

    /// The number of entries (`Count`).
    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Whether the dictionary has `key` (`ContainsKey`).
    pub fn contains_key<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.index.contains_key(key)
    }

    /// The value of `key` (`TryGetValue`).
    pub fn get<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        let slot = *self.index.get(key)?;
        self.entries[slot].as_ref().map(|(_, value)| value)
    }

    /// The value of `key`, to change.
    pub fn get_mut<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        let slot = *self.index.get(key)?;
        self.entries[slot].as_mut().map(|(_, value)| value)
    }

    /// Sets the value of `key` (the indexer's setter, `this[key] = value`): an existing entry keeps
    /// its place; the previous value is returned.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(&slot) = self.index.get(&key) {
            let entry = self.entries[slot].as_mut().expect("an indexed slot holds its entry");
            return Some(std::mem::replace(&mut entry.1, value));
        }
        let slot = match self.free.pop() {
            Some(slot) => {
                self.entries[slot] = Some((key.clone(), value));
                slot
            }
            None => {
                self.entries.push(Some((key.clone(), value)));
                self.entries.len() - 1
            }
        };
        self.index.insert(key, slot);
        None
    }

    /// Adds an entry unless the key is there already (`TryAdd`); false when it is.
    pub fn try_add(&mut self, key: K, value: V) -> bool {
        if self.index.contains_key(&key) {
            return false;
        }
        self.insert(key, value);
        true
    }

    /// Adds an entry (`Add`).
    ///
    /// # Panics
    /// When the key is there already (`ArgumentException`).
    pub fn add(&mut self, key: K, value: V) {
        assert!(self.try_add(key, value), "An item with the same key has already been added.");
    }

    /// Removes the entry of `key` and gives its value (`Remove(TKey, out TValue)`).
    pub fn remove<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        let slot = self.index.remove(key)?;
        let (_, value) = self.entries[slot].take().expect("an indexed slot holds its entry");
        self.free.push(slot);
        Some(value)
    }

    /// Removes every entry (`Clear`); the next entry is the first again.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.free.clear();
        self.index.clear();
    }

}

impl<K: Eq + Hash + Clone, V> FromIterator<(K, V)> for Dictionary<K, V> {
    /// The entries in order; a key given twice keeps its first place and its last value.
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut dictionary = Self::new();
        for (key, value) in iter {
            dictionary.insert(key, value);
        }
        dictionary
    }
}

impl<K: Eq + Hash + Clone, V> Extend<(K, V)> for Dictionary<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

impl<K: Eq + Hash + Clone, V: PartialEq> PartialEq for Dictionary<K, V> {
    /// The same entries, whatever their order.
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(key, value)| other.get(key) == Some(value))
    }
}
