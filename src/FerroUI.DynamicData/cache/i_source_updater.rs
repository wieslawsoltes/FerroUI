//! `ICacheUpdater<TObject, TKey>` and `ISourceUpdater<TObject, TKey>` (`Cache/ICacheUpdater.cs`,
//! `Cache/ISourceUpdater.cs`): what an edit of a cache can do.
//!
//! The overloads of upstream become methods of their own: `_item`/`_items` take items whose key
//! the cache's key selector gives, `_key`/`_keys` take keys, `_pair`/`_pairs` take keys with items.
//! An equality comparer (`IEqualityComparer<TObject>`) is a closure.

use super::{ChangeSet, IQuery};

/// An edit of a cache whose items carry their keys (`ICacheUpdater<TObject, TKey>`).
pub trait ICacheUpdater<T, K>: IQuery<T, K> {
    /// Adds or replaces the items of the keys (`AddOrUpdate(IEnumerable<KeyValuePair>)`).
    fn add_or_update_pairs(&mut self, pairs: &mut dyn Iterator<Item = (K, T)>);
    /// Adds or replaces the item of `key` (`AddOrUpdate(TObject, TKey)`,
    /// `AddOrUpdate(KeyValuePair)`).
    fn add_or_update_with_key(&mut self, item: T, key: K);
    /// Removes every item (`Clear`).
    fn clear(&mut self);
    /// Applies a change set (`Clone(IChangeSet)`, `Update(IChangeSet)`).
    fn clone_changes(&mut self, changes: &ChangeSet<T, K>);
    /// The key of `item` (`GetKey`).
    ///
    /// # Panics
    /// When the cache has no key selector (`KeySelectorException`).
    fn get_key(&self, item: &T) -> K;
    /// The keys of the items, with the items (`GetKeyValues`).
    fn get_key_values(&self, items: &mut dyn Iterator<Item = T>) -> Vec<(K, T)>;
    /// Refreshes every item (`Refresh()`).
    fn refresh_all(&mut self);
    /// Refreshes the items of the keys (`Refresh(IEnumerable<TKey>)`).
    fn refresh_keys(&mut self, keys: &mut dyn Iterator<Item = K>);
    /// Refreshes the item of `key` (`Refresh(TKey)`).
    fn refresh_key(&mut self, key: &K);
    /// Removes the items of the keys (`Remove(IEnumerable<TKey>)`, `RemoveKeys`).
    fn remove_keys(&mut self, keys: &mut dyn Iterator<Item = K>);
    /// Removes the item of `key` (`Remove(TKey)`, `RemoveKey`, `Remove(KeyValuePair)`).
    fn remove_key(&mut self, key: &K);
    /// Removes the items of the pairs' keys (`Remove(IEnumerable<KeyValuePair>)`).
    fn remove_pairs(&mut self, pairs: &mut dyn Iterator<Item = (K, T)>);
}

/// An edit of a cache that knows the keys of its items (`ISourceUpdater<TObject, TKey>`).
pub trait ISourceUpdater<T, K>: ICacheUpdater<T, K> {
    /// Adds or replaces the items (`AddOrUpdate(IEnumerable<TObject>)`).
    fn add_or_update_items(&mut self, items: &mut dyn Iterator<Item = T>);
    /// Adds or replaces the items, leaving out an item equal to the item of its key
    /// (`AddOrUpdate(IEnumerable<TObject>, IEqualityComparer<TObject>)`).
    fn add_or_update_items_with(&mut self, items: &mut dyn Iterator<Item = T>, comparer: &dyn Fn(&T, &T) -> bool);
    /// Adds or replaces the item (`AddOrUpdate(TObject)`).
    fn add_or_update(&mut self, item: T);
    /// Adds or replaces the item unless it equals the item of its key
    /// (`AddOrUpdate(TObject, IEqualityComparer<TObject>)`).
    fn add_or_update_with(&mut self, item: T, comparer: &dyn Fn(&T, &T) -> bool);
    /// Replaces every item with the items (`Load`).
    fn load(&mut self, items: &mut dyn Iterator<Item = T>);
    /// The item of the key of `item` (`Lookup(TObject)`).
    fn lookup_item(&self, item: &T) -> Option<T>;
    /// Refreshes the items (`Refresh(IEnumerable<TObject>)`).
    fn refresh_items(&mut self, items: &mut dyn Iterator<Item = T>);
    /// Refreshes the item (`Refresh(TObject)`).
    fn refresh_item(&mut self, item: &T);
    /// Removes the items (`Remove(IEnumerable<TObject>)`).
    fn remove_items(&mut self, items: &mut dyn Iterator<Item = T>);
    /// Removes the item (`Remove(TObject)`).
    fn remove_item(&mut self, item: &T);
}
