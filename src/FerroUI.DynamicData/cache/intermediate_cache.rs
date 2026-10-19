//! `IntermediateCache<TObject, TKey>` and `IIntermediateCache<TObject, TKey>`
//! (`Cache/IntermediateCache.cs`, `Cache/IIntermediateCache.cs`), with the edit conveniences of
//! the latter (`ObservableCacheEx.AddOrUpdate`, `Clear` and `Remove` on `IIntermediateCache`).

use super::{Change, ChangeSet, ICacheUpdater, IChangeSet, IConnectableCache, IObservableCache, ObservableCache};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable};
use std::hash::Hash;
use std::rc::Rc;

/// A cache whose edits give the keys of the items (`IIntermediateCache<TObject, TKey>`).
pub trait IIntermediateCache<T, K>: IObservableCache<T, K> {
    /// Runs an edit; its changes are delivered as one change set when it ends (`Edit`).
    fn edit(&self, update: &mut dyn FnMut(&mut dyn ICacheUpdater<T, K>));
}

/// A cache without a key selector, whose edits give the keys (`IntermediateCache<TObject,
/// TKey>`): the cache of the operators that keep items under keys of their own. A clone is a
/// handle to the same cache.
pub struct IntermediateCache<T, K> {
    inner: ObservableCache<T, K>,
}

impl<T, K> Clone for IntermediateCache<T, K> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Default for IntermediateCache<T, K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IntermediateCache<T, K> {
    /// An empty cache (`IntermediateCache()`).
    pub fn new() -> Self {
        Self { inner: ObservableCache::without_key_selector() }
    }

    /// A cache that applies the change sets of `source` (`IntermediateCache(IObservable<IChangeSet>)`).
    pub fn from_source<C: IChangeSet<T, K>>(source: Rc<dyn IObservable<C>>) -> Self {
        Self { inner: ObservableCache::from_source(source) }
    }

    /// Runs an edit (`Edit(Action<ICacheUpdater>)`).
    pub fn edit(&self, update: impl FnOnce(&mut dyn ICacheUpdater<T, K>)) {
        self.inner.update_from_intermediate(update);
    }

    /// Holds back the notifications of count changes until the result is disposed (`SuspendCount`).
    pub fn suspend_count(&self) -> Rc<dyn IDisposable> {
        self.inner.suspend_count()
    }

    /// Holds back the notifications of changes until the result is disposed
    /// (`SuspendNotifications`).
    pub fn suspend_notifications(&self) -> Rc<dyn IDisposable> {
        self.inner.suspend_notifications()
    }

    /// Adds `item` under `key`, or replaces the item of `key` (`AddOrUpdate(IIntermediateCache,
    /// TObject, TKey)`).
    pub fn add_or_update(&self, item: T, key: K) {
        self.edit(|updater| updater.add_or_update_with_key(item, key));
    }

    /// Removes every item (`Clear(IIntermediateCache)`).
    pub fn clear(&self) {
        self.edit(|updater| updater.clear());
    }

    /// Removes the item of `key` (`Remove(IIntermediateCache, TKey)`).
    pub fn remove_key(&self, key: K) {
        self.edit(|updater| updater.remove_key(&key));
    }

    /// Removes the items of `keys` (`Remove(IIntermediateCache, IEnumerable<TKey>)`).
    pub fn remove_keys(&self, keys: impl IntoIterator<Item = K>) {
        let keys: Vec<K> = keys.into_iter().collect();
        self.edit(|updater| updater.remove_keys(&mut keys.into_iter()));
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IConnectableCache<T, K> for IntermediateCache<T, K> {
    fn count_changed(&self) -> Rc<dyn IObservable<usize>> {
        self.inner.count_changed()
    }

    fn connect_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>, suppress_empty_change_sets: bool) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.inner.connect_with(predicate, suppress_empty_change_sets)
    }

    fn preview_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.inner.preview_with(predicate)
    }

    fn watch(&self, key: K) -> Rc<dyn IObservable<Change<T, K>>> {
        self.inner.watch(key)
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IObservableCache<T, K> for IntermediateCache<T, K> {
    fn count(&self) -> usize {
        self.inner.count()
    }

    fn items(&self) -> Vec<T> {
        self.inner.items()
    }

    fn keys(&self) -> Vec<K> {
        self.inner.keys()
    }

    fn key_values(&self) -> Dictionary<K, T> {
        self.inner.key_values()
    }

    fn lookup(&self, key: &K) -> Option<T> {
        self.inner.lookup(key)
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IIntermediateCache<T, K> for IntermediateCache<T, K> {
    fn edit(&self, update: &mut dyn FnMut(&mut dyn ICacheUpdater<T, K>)) {
        self.inner.update_from_intermediate(|updater| update(updater));
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IDisposable for IntermediateCache<T, K> {
    fn dispose(&self) {
        self.inner.dispose();
    }
}
