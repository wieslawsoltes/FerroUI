//! `LockFreeObservableCache<TObject, TKey>` (`Cache/Internal/LockFreeObservableCache.cs`, public
//! upstream), with its edit convenience (`ObservableCacheEx.Clear(LockFreeObservableCache)`).

use super::internal::{filter_static, CacheUpdater};
use super::{Change, ChangeAwareCache, ChangeReason, ChangeSet, ICache, ICacheUpdater, IChangeSet, IConnectableCache, IObservableCache, IQuery, ObservableCacheEx};
use crate::kernel::return_with;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

struct Inner<T, K> {
    changes: Rc<Subject<ChangeSet<T, K>>>,
    changes_preview: Rc<Subject<ChangeSet<T, K>>>,
    count_changed: Rc<Subject<usize>>,
    inner_cache: Rc<RefCell<ChangeAwareCache<T, K>>>,
    clean_up: RefCell<Option<Rc<dyn IDisposable>>>,
}

/// An observable cache for a single writer (`LockFreeObservableCache<TObject, TKey>`): an edit
/// delivers its change set as soon as it ends. A clone is a handle to the same cache.
///
/// As upstream's, the count and the preview are never notified after the subscription: the
/// count is the number of items when it is observed, and the preview never has a change set.
pub struct LockFreeObservableCache<T, K> {
    inner: Rc<Inner<T, K>>,
}

impl<T, K> Clone for LockFreeObservableCache<T, K> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Default for LockFreeObservableCache<T, K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> LockFreeObservableCache<T, K> {
    fn create() -> Self {
        Self {
            inner: Rc::new(Inner {
                changes: Subject::new(),
                changes_preview: Subject::new(),
                count_changed: Subject::new(),
                inner_cache: Rc::new(RefCell::new(ChangeAwareCache::new())),
                clean_up: RefCell::new(None),
            }),
        }
    }

    /// An empty cache (`LockFreeObservableCache()`).
    pub fn new() -> Self {
        let cache = Self::create();
        let (changes, count_changed) = (cache.inner.changes.clone(), cache.inner.count_changed.clone());
        *cache.inner.clean_up.borrow_mut() = Some(Disposable::create(move || {
            changes.on_completed();
            count_changed.on_completed();
        }));
        cache
    }

    /// A cache that applies the change sets of `source` (`LockFreeObservableCache(IObservable<IChangeSet>)`).
    pub fn from_source<C: IChangeSet<T, K>>(source: Rc<dyn IObservable<C>>) -> Self {
        let cache = Self::create();
        let inner_cache = cache.inner.inner_cache.clone();
        let loader = source
            .change_sets()
            .select(move |changes: ChangeSet<T, K>| {
                let mut inner_cache = inner_cache.borrow_mut();
                inner_cache.clone_changes(&changes);
                inner_cache.capture_changes()
            })
            .subscribe_safe(cache.inner.changes.as_observer());
        let (changes_preview, changes, count_changed) = (cache.inner.changes_preview.clone(), cache.inner.changes.clone(), cache.inner.count_changed.clone());
        *cache.inner.clean_up.borrow_mut() = Some(Disposable::create(move || {
            loader.dispose();
            changes_preview.on_completed();
            changes.on_completed();
            count_changed.on_completed();
        }));
        cache
    }

    /// Runs an edit, then delivers its changes (`Edit(Action<ICacheUpdater>)`).
    pub fn edit(&self, edit_action: impl FnOnce(&mut dyn ICacheUpdater<T, K>)) {
        let changes = {
            let cache: &RefCell<dyn ICache<T, K>> = &*self.inner.inner_cache;
            let mut updater = CacheUpdater::new(cache, None);
            edit_action(&mut updater);
            self.inner.inner_cache.borrow_mut().capture_changes()
        };
        self.inner.changes.on_next(changes);
    }

    /// Removes every item (`Clear(LockFreeObservableCache)`).
    pub fn clear(&self) {
        self.edit(|updater| updater.clear());
    }

    fn get_initial_updates(inner_cache: &RefCell<ChangeAwareCache<T, K>>, predicate: Option<&dyn Fn(&T) -> bool>) -> ChangeSet<T, K> {
        inner_cache
            .borrow()
            .key_values()
            .into_iter()
            .filter(|(_, item)| predicate.is_none_or(|predicate| predicate(item)))
            .map(|(key, item)| Change::new(ChangeReason::Add, key, item))
            .collect()
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IConnectableCache<T, K> for LockFreeObservableCache<T, K> {
    fn count_changed(&self) -> Rc<dyn IObservable<usize>> {
        let count = self.inner.inner_cache.borrow().count();
        self.inner.count_changed.as_observable().start_with([count]).distinct_until_changed()
    }

    fn connect_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>, suppress_empty_change_sets: bool) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        let (inner_cache, changes) = (self.inner.inner_cache.clone(), self.inner.changes.clone());
        Observable::defer(move || {
            let (inner_cache, filter) = (inner_cache.clone(), predicate.clone());
            let initial = return_with(move || Self::get_initial_updates(&inner_cache, filter.as_deref()));
            let changes = initial.concat(changes.as_observable());
            match &predicate {
                Some(predicate) => filter_static(changes, predicate.clone(), suppress_empty_change_sets),
                None if suppress_empty_change_sets => changes.not_empty(),
                None => changes,
            }
        })
    }

    fn preview_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        let preview = self.inner.changes_preview.as_observable();
        match predicate {
            None => preview,
            Some(predicate) => filter_static(preview, predicate, true),
        }
    }

    fn watch(&self, key: K) -> Rc<dyn IObservable<Change<T, K>>> {
        let (inner_cache, changes) = (self.inner.inner_cache.clone(), self.inner.changes.clone());
        Observable::create(move |observer: Rc<dyn IObserver<Change<T, K>>>| {
            let initial = inner_cache.borrow().lookup(&key);
            if let Some(initial) = initial {
                observer.on_next(Change::new(ChangeReason::Add, key.clone(), initial));
            }
            let key = key.clone();
            changes.as_observable().subscribe_next(move |changes: ChangeSet<T, K>| {
                for change in &changes {
                    if *change.key() == key {
                        observer.on_next(change.clone());
                    }
                }
            })
        })
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IObservableCache<T, K> for LockFreeObservableCache<T, K> {
    fn count(&self) -> usize {
        self.inner.inner_cache.borrow().count()
    }

    fn items(&self) -> Vec<T> {
        self.inner.inner_cache.borrow().items()
    }

    fn keys(&self) -> Vec<K> {
        self.inner.inner_cache.borrow().keys()
    }

    fn key_values(&self) -> Dictionary<K, T> {
        self.inner.inner_cache.borrow().dictionary().clone()
    }

    fn lookup(&self, key: &K) -> Option<T> {
        self.inner.inner_cache.borrow().lookup(key)
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IDisposable for LockFreeObservableCache<T, K> {
    fn dispose(&self) {
        let clean_up = self.inner.clean_up.borrow_mut().take();
        if let Some(clean_up) = clean_up {
            clean_up.dispose();
        }
    }
}
