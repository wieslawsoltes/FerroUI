//! `IndexedItem<TObject, TKey>` (`Cache/IndexedItem.cs`).

use std::fmt;

/// An item of a sorted collection with its key and its index (`IndexedItem<TObject, TKey>`). Two
/// indexed items are equal when their keys, items and indexes are.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndexedItem<T, K> {
    value: T,
    key: K,
    index: usize,
}

impl<T, K> IndexedItem<T, K> {
    /// The item `value` of `key` at `index` (`IndexedItem(TObject value, TKey key, int index)`).
    pub fn new(value: T, key: K, index: usize) -> Self {
        Self { value, key, index }
    }

    /// The index (`Index`).
    pub fn index(&self) -> usize {
        self.index
    }

    /// The key (`Key`).
    pub fn key(&self) -> &K {
        &self.key
    }

    /// The item (`Value`).
    pub fn value(&self) -> &T {
        &self.value
    }
}

impl<T: fmt::Display, K: fmt::Display> fmt::Display for IndexedItem<T, K> {
    /// `ToString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Value: {}, Key: {}, CurrentIndex: {}", self.value, self.key, self.index)
    }
}
