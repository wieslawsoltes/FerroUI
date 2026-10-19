//! `ManagedGroup<TObject, TKey, TGroupKey>` (`Cache/Internal/ManagedGroup.cs`).

use crate::cache::{ChangeSet, ICacheUpdater, IGroup, IObservableCache, ObservableCache};
use ferroui_system::IDisposable;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Inner<T, K, G> {
    key: G,
    cache: ObservableCache<T, K>,
}

/// A group of the items of a cache that share a group key (`ManagedGroup<TObject, TKey,
/// TGroupKey>`, the `IGroup` of the change sets of `Group`): the grouping operators edit its cache,
/// and its observers watch it. A clone is the same group.
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream's class is internal and its change sets hold
/// the interface `IGroup`; here the change sets hold this type, so it is public, as `Group`.
pub struct Group<T, K, G> {
    inner: Rc<Inner<T, K, G>>,
}

impl<T, K, G> Clone for Group<T, K, G> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, G> Group<T, K, G> {
    /// An empty group of `key` (`ManagedGroup(TGroupKey)`), whose cache gives the keys with its
    /// items (`IntermediateCache`).
    pub(crate) fn new(key: G) -> Self {
        Self { inner: Rc::new(Inner { key, cache: ObservableCache::without_key_selector() }) }
    }

    /// The group key (`Key`).
    pub fn key(&self) -> &G {
        &self.inner.key
    }

    /// The items of the group (`Cache`).
    pub fn cache(&self) -> Rc<dyn IObservableCache<T, K>> {
        Rc::new(self.inner.cache.clone())
    }

    /// The cache of the group, as its concrete type.
    pub(crate) fn observable_cache(&self) -> &ObservableCache<T, K> {
        &self.inner.cache
    }

    /// The number of items (`Count`).
    pub(crate) fn count(&self) -> usize {
        self.inner.cache.count()
    }

    /// Holds back the notifications of the cache of the group until the result is disposed
    /// (`SuspendNotifications`).
    pub(crate) fn suspend_notifications(&self) -> Rc<dyn IDisposable> {
        self.inner.cache.suspend_notifications()
    }

    /// The items of the group as additions (`GetInitialUpdates`).
    pub(crate) fn get_initial_updates(&self) -> ChangeSet<T, K> {
        self.inner.cache.get_initial_updates(None)
    }

    /// Edits the cache of the group (`Update`, `IntermediateCache.Edit`).
    pub(crate) fn update(&self, update_action: impl FnOnce(&mut dyn ICacheUpdater<T, K>)) {
        self.inner.cache.update_from_intermediate(update_action);
    }

    /// Whether `self` and `other` are the same group (`ReferenceEquals`).
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, G> IGroup<T, K, G> for Group<T, K, G> {
    fn key(&self) -> &G {
        &self.inner.key
    }

    fn cache(&self) -> Rc<dyn IObservableCache<T, K>> {
        Rc::new(self.inner.cache.clone())
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, G> IDisposable for Group<T, K, G> {
    /// Ends the cache of the group, which completes its observers (`Dispose`).
    fn dispose(&self) {
        self.inner.cache.dispose();
    }
}

impl<T, K, G: PartialEq> PartialEq for Group<T, K, G> {
    /// Groups of the same key are equal (`Equals`).
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner) || self.inner.key == other.inner.key
    }
}

impl<T, K, G: Eq> Eq for Group<T, K, G> {}

impl<T, K, G: Hash> Hash for Group<T, K, G> {
    /// The hash of the group key (`GetHashCode`).
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.key.hash(state);
    }
}

impl<T, K, G: fmt::Display> fmt::Display for Group<T, K, G> {
    /// `ToString`: `Group: {Key}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Group: {}", self.inner.key)
    }
}

impl<T, K, G: fmt::Debug> fmt::Debug for Group<T, K, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Group: {:?}", self.inner.key)
    }
}
