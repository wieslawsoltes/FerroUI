//! `IQuery<TObject, TKey>` and `ICache<TObject, TKey>` (`Cache/IQuery.cs`, `Cache/ICache.cs`).

use super::ChangeSet;

/// Read access to a keyed collection (`IQuery<TObject, TKey>`).
pub trait IQuery<T, K> {
    /// The number of items (`Count`).
    fn count(&self) -> usize;
    /// The items, in the order of enumeration (`Items`).
    fn items(&self) -> Vec<T>;
    /// The keys, in the order of enumeration (`Keys`).
    fn keys(&self) -> Vec<K>;
    /// The keys and items, in the order of enumeration (`KeyValues`).
    fn key_values(&self) -> Vec<(K, T)>;
    /// The item of `key` (`Lookup`).
    fn lookup(&self, key: &K) -> Option<T>;
}

/// A keyed collection that can be edited (`ICache<TObject, TKey>`).
pub trait ICache<T, K>: IQuery<T, K> {
    /// Adds the item, or replaces the item of its key (`AddOrUpdate(TObject, TKey)`).
    fn add_or_update(&mut self, item: T, key: K);
    /// Removes every item (`Clear`).
    fn clear(&mut self);
    /// Applies a change set (`Clone(IChangeSet<TObject, TKey>)`).
    fn clone_changes(&mut self, changes: &ChangeSet<T, K>);
    /// Refreshes every item (`Refresh()`).
    fn refresh_all(&mut self);
    /// Refreshes the items of the keys (`Refresh(IEnumerable<TKey>)`).
    fn refresh_keys(&mut self, keys: &mut dyn Iterator<Item = K>);
    /// Refreshes the item of `key` (`Refresh(TKey)`).
    fn refresh(&mut self, key: &K);
    /// Removes the item of `key` (`Remove(TKey)`).
    fn remove(&mut self, key: &K);
    /// Removes the items of the keys (`Remove(IEnumerable<TKey>)`).
    fn remove_keys(&mut self, keys: &mut dyn Iterator<Item = K>);
}
