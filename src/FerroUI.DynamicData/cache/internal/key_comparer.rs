//! `KeyComparer<TObject, TKey>` (`Cache/Internal/KeyComparer.cs`): the equality of pairs by their
//! keys, with the set operations of LINQ that the paging operators run with it.

use std::collections::HashSet;
use std::hash::Hash;

/// The equality of pairs of a key and an item by their keys (`KeyComparer<TObject, TKey>`).
pub(crate) struct KeyComparer;

impl KeyComparer {
    /// Whether two pairs have the same key (`Equals`).
    pub(crate) fn equals<T, K: Eq>(x: &(K, T), y: &(K, T)) -> bool {
        x.0 == y.0
    }

    /// The pairs of `first` whose key is not in `second`, each key once, in the order of `first`
    /// (`first.Except(second, keyComparer)`).
    pub(crate) fn except<T: Clone, K: Eq + Hash + Clone>(first: &[(K, T)], second: &[(K, T)]) -> Vec<(K, T)> {
        let mut seen: HashSet<&K> = second.iter().map(|(key, _)| key).collect();
        first.iter().filter(|(key, _)| seen.insert(key)).cloned().collect()
    }

    /// The pairs of `first` whose key is in `second`, each key once, in the order of `first`
    /// (`first.Intersect(second, keyComparer)`).
    pub(crate) fn intersect<T: Clone, K: Eq + Hash + Clone>(first: &[(K, T)], second: &[(K, T)]) -> Vec<(K, T)> {
        let mut remaining: HashSet<&K> = second.iter().map(|(key, _)| key).collect();
        first.iter().filter(|(key, _)| remaining.remove(key)).cloned().collect()
    }

    /// The index of the pair of the key of `item` in `list` (`IndexOf`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): upstream finds a pair by the default equality of
    /// `KeyValuePair` (the key and the item); a sorted cache holds one pair per key, whose item
    /// is the one of the change, so the port finds it by the key and needs no equality of items.
    pub(crate) fn index_of<T, K: Eq>(list: &[(K, T)], item: &(K, T)) -> Option<usize> {
        list.iter().position(|candidate| candidate.0 == item.0)
    }
}
