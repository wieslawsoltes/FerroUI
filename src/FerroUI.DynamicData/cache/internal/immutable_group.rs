//! `ImmutableGroup<TObject, TKey, TGroupKey>` (`Cache/Internal/ImmutableGroup.cs`).

use super::Cache;
use crate::cache::{ICache, IGrouping, IQuery};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Inner<T, K, G> {
    key: G,
    cache: Cache<T, K>,
}

/// The state of a group at one moment (`ImmutableGroup<TObject, TKey, TGroupKey>`, the
/// `IGrouping` of the change sets of `GroupWithImmutableState`): a copy of its items that does not
/// change. A clone is the same state.
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream's class is internal and its change sets hold
/// the interface `IGrouping`; here they hold this type, so it is public.
pub struct ImmutableGroup<T, K, G> {
    inner: Rc<Inner<T, K, G>>,
}

impl<T, K, G> Clone for ImmutableGroup<T, K, G> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone, K: Eq + Hash + Clone, G> ImmutableGroup<T, K, G> {
    /// The state of the group `key` with a copy of the items of `cache` (`ImmutableGroup(TGroupKey,
    /// ICache<TObject, TKey>)`).
    pub(crate) fn new(key: G, cache: &dyn IQuery<T, K>) -> Self {
        let mut copy = Cache::with_capacity(cache.count());
        for (item_key, item) in cache.key_values() {
            copy.add_or_update(item, item_key);
        }
        Self { inner: Rc::new(Inner { key, cache: copy }) }
    }

    /// Whether `self` and `other` are the same state (`ReferenceEquals`).
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T: Clone, K: Eq + Hash + Clone, G> IGrouping<T, K, G> for ImmutableGroup<T, K, G> {
    fn count(&self) -> usize {
        self.inner.cache.count()
    }

    fn items(&self) -> Vec<T> {
        self.inner.cache.items()
    }

    fn key(&self) -> &G {
        &self.inner.key
    }

    fn keys(&self) -> Vec<K> {
        self.inner.cache.keys()
    }

    fn key_values(&self) -> Vec<(K, T)> {
        self.inner.cache.key_values()
    }

    fn lookup(&self, key: &K) -> Option<T> {
        self.inner.cache.lookup(key)
    }
}

impl<T, K, G: PartialEq> PartialEq for ImmutableGroup<T, K, G> {
    /// States of the same group key are equal (`Equals`).
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner) || self.inner.key == other.inner.key
    }
}

impl<T, K, G: Eq> Eq for ImmutableGroup<T, K, G> {}

impl<T, K, G: Hash> Hash for ImmutableGroup<T, K, G> {
    /// The hash of the group key (`GetHashCode`).
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.key.hash(state);
    }
}

impl<T: Clone, K: Eq + std::hash::Hash + Clone, G: fmt::Display> fmt::Display for ImmutableGroup<T, K, G> {
    /// `ToString`: `Grouping for: {Key} ({Count} items)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Grouping for: {} ({} items)", self.inner.key, self.inner.cache.dictionary().len())
    }
}

impl<T: Clone, K: Eq + std::hash::Hash + Clone, G: fmt::Debug> fmt::Debug for ImmutableGroup<T, K, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Grouping for: {:?} ({} items)", self.inner.key, self.inner.cache.dictionary().len())
    }
}
