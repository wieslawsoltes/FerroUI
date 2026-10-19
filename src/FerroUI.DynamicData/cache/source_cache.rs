//! `SourceCache<TObject, TKey>` and `ISourceCache<TObject, TKey>` (`Cache/SourceCache.cs`,
//! `Cache/ISourceCache.cs`).

use super::internal::KeySelector;
use super::{Change, ChangeSet, IConnectableCache, IObservableCache, ISourceUpdater, ObservableCache};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable};
use std::hash::Hash;
use std::rc::Rc;

/// A cache whose items are edited by its owner (`ISourceCache<TObject, TKey>`).
pub trait ISourceCache<T, K>: IObservableCache<T, K> {
    /// The key selector of the items (`KeySelector`).
    fn key_selector(&self) -> KeySelector<T, K>;
    /// Runs an edit; its changes are delivered as one change set when it ends (`Edit`).
    fn edit(&self, update: &mut dyn FnMut(&mut dyn ISourceUpdater<T, K>));
}

/// The source of a reactive keyed collection (`SourceCache<TObject, TKey>`): edits make change
/// sets, which `connect` delivers to the operators.
pub struct SourceCache<T, K> {
    inner: ObservableCache<T, K>,
    key_selector: KeySelector<T, K>,
}

impl<T, K> Clone for SourceCache<T, K> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), key_selector: self.key_selector.clone() }
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> SourceCache<T, K> {
    /// An empty cache whose items are keyed by `key_selector`.
    pub fn new(key_selector: impl Fn(&T) -> K + 'static) -> Self {
        let key_selector: KeySelector<T, K> = Rc::new(key_selector);
        let selector = key_selector.clone();
        Self { inner: ObservableCache::new(move |item| selector(item)), key_selector }
    }

    /// Runs an edit (`Edit(Action<ISourceUpdater>)`).
    pub fn edit(&self, update: impl FnOnce(&mut dyn ISourceUpdater<T, K>)) {
        self.inner.update_from_source(update);
    }

    /// Holds back the notifications of count changes (`SuspendCount`).
    pub fn suspend_count(&self) -> Rc<dyn IDisposable> {
        self.inner.suspend_count()
    }

    /// Holds back the notifications of changes (`SuspendNotifications`).
    pub fn suspend_notifications(&self) -> Rc<dyn IDisposable> {
        self.inner.suspend_notifications()
    }

    /// The observable cache this source edits.
    pub fn as_observable_cache_inner(&self) -> &ObservableCache<T, K> {
        &self.inner
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IConnectableCache<T, K> for SourceCache<T, K> {
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

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IObservableCache<T, K> for SourceCache<T, K> {
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

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> ISourceCache<T, K> for SourceCache<T, K> {
    fn key_selector(&self) -> KeySelector<T, K> {
        self.key_selector.clone()
    }

    fn edit(&self, update: &mut dyn FnMut(&mut dyn ISourceUpdater<T, K>)) {
        self.inner.update_from_source(|updater| update(updater));
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IDisposable for SourceCache<T, K> {
    fn dispose(&self) {
        self.inner.dispose();
    }
}
