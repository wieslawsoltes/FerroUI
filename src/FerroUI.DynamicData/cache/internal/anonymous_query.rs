//! `AnonymousQuery<TObject, TKey>` (`Cache/Internal/AnonymousQuery.cs`).

use super::Cache;
use crate::cache::IQuery;
use std::hash::Hash;

/// A snapshot of the items of a cache, to query (`AnonymousQuery<TObject, TKey>`): it copies the
/// cache, so later changes do not reach it.
pub(crate) struct AnonymousQuery<T, K> {
    cache: Cache<T, K>,
}

impl<T: Clone, K: Eq + Hash + Clone> AnonymousQuery<T, K> {
    pub(crate) fn new(cache: &Cache<T, K>) -> Self {
        Self { cache: cache.clone() }
    }
}

impl<T: Clone, K: Eq + Hash + Clone> IQuery<T, K> for AnonymousQuery<T, K> {
    fn count(&self) -> usize {
        self.cache.count()
    }

    fn items(&self) -> Vec<T> {
        self.cache.items()
    }

    fn keys(&self) -> Vec<K> {
        self.cache.keys()
    }

    fn key_values(&self) -> Vec<(K, T)> {
        self.cache.key_values()
    }

    fn lookup(&self, key: &K) -> Option<T> {
        self.cache.lookup(key)
    }
}
