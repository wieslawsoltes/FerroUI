//! `IGrouping<TObject, TKey, TGroupKey>` (`Cache/IGrouping.cs`).

/// The state of a group at one moment (`IGrouping<TObject, TKey, TGroupKey>`): the items of the
/// change sets of `GroupWithImmutableState` and `GroupOnPropertyWithImmutableState`, which replace
/// a group with a new state whenever its items change.
pub trait IGrouping<T, K, G> {
    /// The number of items (`Count`).
    fn count(&self) -> usize;

    /// The items (`Items`).
    fn items(&self) -> Vec<T>;

    /// The group key (`Key`).
    fn key(&self) -> &G;

    /// The keys of the items (`Keys`).
    fn keys(&self) -> Vec<K>;

    /// The keys with their items (`KeyValues`).
    fn key_values(&self) -> Vec<(K, T)>;

    /// The item of `key` (`Lookup`).
    fn lookup(&self, key: &K) -> Option<T>;
}
