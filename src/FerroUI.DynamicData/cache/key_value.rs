//! `IKeyValue<TObject, TKey>` and `IKeyValueCollection<TObject, TKey>` (`Cache/IKeyValue.cs`,
//! `Cache/IKeyValueCollection.cs`).

use super::{KeyValueComparer, SortOptimisations, SortReason};

/// A key and its item (`IKeyValue<TObject, TKey>`, with `IKey<TKey>`).
pub trait IKeyValue<T, K> {
    /// The key (`Key`).
    fn key(&self) -> &K;
    /// The item (`Value`).
    fn value(&self) -> &T;
}

impl<T, K> IKeyValue<T, K> for (K, T) {
    /// `KeyValuePair<TKey, TObject>.Key`.
    fn key(&self) -> &K {
        &self.0
    }

    /// `KeyValuePair<TKey, TObject>.Value`.
    fn value(&self) -> &T {
        &self.1
    }
}

/// The sorted pairs of a key and an item of a sorted change set (`IKeyValueCollection<TObject,
/// TKey>`, a read-only list of `KeyValuePair<TKey, TObject>`).
pub trait IKeyValueCollection<T, K> {
    /// The comparer of the pairs (`Comparer`).
    fn comparer(&self) -> &KeyValueComparer<T, K>;
    /// The optimisations of the sort (`Optimisations`).
    fn optimisations(&self) -> SortOptimisations;
    /// The reason of the sort (`SortReason`).
    fn sort_reason(&self) -> SortReason;
    /// The pairs, in order (the `IReadOnlyList`).
    fn items(&self) -> &[(K, T)];
}
