//! `KeyValueCollection<TObject, TKey>` (`Cache/Internal/KeyValueCollection.cs`).

use super::KeyValueComparer;
use crate::cache::{IKeyValueCollection, SortOptimisations, SortReason};
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// The sorted pairs of a key and an item of a sorted change set (`KeyValueCollection<TObject,
/// TKey>`), with the comparer that sorted them and the reason of the sort.
///
/// Deviation (DEVIATIONS.md, DynamicData): a pair is the tuple `(key, item)`, upstream's
/// `KeyValuePair<TKey, TObject>`; `.0` is `Key` and `.1` is `Value`. The pairs are shared, so
/// cloning a collection is cheap.
pub struct KeyValueCollection<T, K> {
    items: Rc<[(K, T)]>,
    comparer: KeyValueComparer<T, K>,
    sort_reason: SortReason,
    optimisations: SortOptimisations,
}

impl<T, K> Clone for KeyValueCollection<T, K> {
    fn clone(&self) -> Self {
        Self { items: self.items.clone(), comparer: self.comparer.clone(), sort_reason: self.sort_reason, optimisations: self.optimisations }
    }
}

impl<T, K> Default for KeyValueCollection<T, K> {
    /// An empty collection, compared by the keys (`KeyValueCollection()`).
    fn default() -> Self {
        Self::empty()
    }
}

impl<T, K> KeyValueCollection<T, K> {
    /// A collection of `items`, sorted by `comparer` for `sort_reason`
    /// (`KeyValueCollection(IReadOnlyCollection<KeyValuePair<TKey, TObject>>, IComparer, SortReason,
    /// SortOptimisations)`).
    pub fn new(items: Vec<(K, T)>, comparer: KeyValueComparer<T, K>, sort_reason: SortReason, optimisations: SortOptimisations) -> Self {
        Self { items: Rc::from(items), comparer, sort_reason, optimisations }
    }

    /// An empty collection (`KeyValueCollection()`): no optimisations, compared by the keys.
    pub fn empty() -> Self {
        Self::new(Vec::new(), KeyValueComparer::new(None), SortReason::InitialLoad, SortOptimisations::NONE)
    }

    /// The comparer of the pairs (`Comparer`).
    pub fn comparer(&self) -> &KeyValueComparer<T, K> {
        &self.comparer
    }

    /// The number of pairs (`Count`).
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// The optimisations of the sort (`Optimisations`).
    pub fn optimisations(&self) -> SortOptimisations {
        self.optimisations
    }

    /// The reason of the sort (`SortReason`).
    pub fn sort_reason(&self) -> SortReason {
        self.sort_reason
    }

    /// The pairs, in order.
    pub fn items(&self) -> &[(K, T)] {
        &self.items
    }

    /// The pairs as a list of their own (`ToList`).
    pub fn to_vec(&self) -> Vec<(K, T)>
    where
        T: Clone,
        K: Clone,
    {
        self.items.to_vec()
    }

    /// The items, in order (`Select(kv => kv.Value)`).
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.items.iter().map(|(_, value)| value)
    }

    /// The keys, in order (`Select(kv => kv.Key)`).
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.items.iter().map(|(key, _)| key)
    }

    /// Whether `self` and `other` share their pairs (`ReferenceEquals` of upstream's collections).
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.items, &other.items)
    }
}

impl<T, K> Deref for KeyValueCollection<T, K> {
    type Target = [(K, T)];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<'a, T, K> IntoIterator for &'a KeyValueCollection<T, K> {
    type Item = &'a (K, T);
    type IntoIter = std::slice::Iter<'a, (K, T)>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<T: PartialEq, K: PartialEq> PartialEq for KeyValueCollection<T, K> {
    /// The same pairs in the same order (`SequenceEqual`).
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

impl<T: fmt::Debug, K: fmt::Debug> fmt::Debug for KeyValueCollection<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.items.iter()).finish()
    }
}

impl<T, K> IKeyValueCollection<T, K> for KeyValueCollection<T, K> {
    fn comparer(&self) -> &KeyValueComparer<T, K> {
        &self.comparer
    }

    fn optimisations(&self) -> SortOptimisations {
        self.optimisations
    }

    fn sort_reason(&self) -> SortReason {
        self.sort_reason
    }

    fn items(&self) -> &[(K, T)] {
        &self.items
    }
}
