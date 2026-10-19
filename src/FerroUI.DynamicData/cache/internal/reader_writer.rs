//! `ReaderWriter<TObject, TKey>` (`Cache/Internal/ReaderWriter.cs`): the items of an observable
//! cache, and the edits that change them.

use super::{Cache, CacheUpdater, KeySelector};
use crate::cache::{Change, ChangeAwareCache, ChangeReason, ChangeSet, ICache, ISourceUpdater};
use ferroui_system::collections::generic::Dictionary;
use std::cell::RefCell;
use std::hash::Hash;

/// The cache that an edit in progress changes: one that records the changes, or one that does not
/// (when nothing observes them).
enum ActiveCache<T, K> {
    Tracking(RefCell<ChangeAwareCache<T, K>>),
    Plain(RefCell<Cache<T, K>>),
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> ActiveCache<T, K> {
    fn cell(&self) -> &RefCell<dyn ICache<T, K>> {
        match self {
            ActiveCache::Tracking(cache) => cache,
            ActiveCache::Plain(cache) => cache,
        }
    }

    fn dictionary_len(&self) -> usize {
        match self {
            ActiveCache::Tracking(cache) => cache.borrow().dictionary().len(),
            ActiveCache::Plain(cache) => cache.borrow().dictionary().len(),
        }
    }
}

/// The items and the edits (`ReaderWriter<TObject, TKey>`).
pub(crate) struct ReaderWriter<T, K> {
    key_selector: Option<KeySelector<T, K>>,
    data: RefCell<Dictionary<K, T>>,
    /// The edit in progress (`_activeUpdater`): its cache holds the items meanwhile, as upstream's
    /// shares the dictionary.
    active: RefCell<Option<std::rc::Rc<ActiveCache<T, K>>>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> ReaderWriter<T, K> {
    pub(crate) fn new(key_selector: Option<KeySelector<T, K>>) -> Self {
        Self { key_selector, data: RefCell::new(Dictionary::new()), active: RefCell::new(None) }
    }

    pub(crate) fn key_selector(&self) -> Option<&KeySelector<T, K>> {
        self.key_selector.as_ref()
    }

    /// Reads the items: those of the edit in progress, if there is one.
    fn read<R>(&self, read: impl FnOnce(&Dictionary<K, T>) -> R) -> R {
        let active = self.active.borrow().clone();
        match active.as_deref() {
            Some(ActiveCache::Tracking(cache)) => read(cache.borrow().dictionary()),
            Some(ActiveCache::Plain(cache)) => read(cache.borrow().dictionary()),
            None => read(&self.data.borrow()),
        }
    }

    pub(crate) fn count(&self) -> usize {
        match self.active.borrow().as_deref() {
            Some(active) => active.dictionary_len(),
            None => self.data.borrow().len(),
        }
    }

    pub(crate) fn items(&self) -> Vec<T> {
        self.read(|data| data.values().cloned().collect())
    }

    pub(crate) fn keys(&self) -> Vec<K> {
        self.read(|data| data.keys().cloned().collect())
    }

    pub(crate) fn key_values(&self) -> Dictionary<K, T> {
        self.read(|data| data.clone())
    }

    pub(crate) fn lookup(&self, key: &K) -> Option<T> {
        self.read(|data| data.get(key).cloned())
    }

    /// The items as additions, those that `filter` accepts (`GetInitialUpdates`).
    pub(crate) fn get_initial_updates(&self, filter: Option<&dyn Fn(&T) -> bool>) -> ChangeSet<T, K> {
        self.read(|data| {
            data.iter()
                .filter(|(_, item)| filter.is_none_or(|filter| filter(item)))
                .map(|(key, item)| Change::new(ChangeReason::Add, key.clone(), item.clone()))
                .collect()
        })
    }

    /// Applies a change set (`Write(IChangeSet, ..)`).
    pub(crate) fn write_changes(&self, changes: &ChangeSet<T, K>, preview: Option<&dyn Fn(&ChangeSet<T, K>)>, collect_changes: bool) -> ChangeSet<T, K> {
        self.write(&mut |updater| updater.clone_changes(changes), preview, collect_changes)
    }

    /// Runs an edit (`Write(Action<ISourceUpdater>, ..)`, `DoUpdate`): the changes it made, when
    /// they are previewed or collected; the preview sees the items as they were before the edit.
    pub(crate) fn write(
        &self,
        update: &mut dyn FnMut(&mut dyn ISourceUpdater<T, K>),
        preview: Option<&dyn Fn(&ChangeSet<T, K>)>,
        collect_changes: bool,
    ) -> ChangeSet<T, K> {
        let data = std::mem::take(&mut *self.data.borrow_mut());
        let copy = preview.map(|_| data.clone());
        let active = std::rc::Rc::new(if preview.is_some() || collect_changes {
            ActiveCache::Tracking(RefCell::new(ChangeAwareCache::from_dictionary(data)))
        } else {
            ActiveCache::Plain(RefCell::new(Cache::from_dictionary(data)))
        });
        *self.active.borrow_mut() = Some(active.clone());
        // The items come back from the edit's cache however the edit ends.
        struct Restore<'a, T: Clone + 'static, K: Eq + Hash + Clone + 'static> {
            owner: &'a ReaderWriter<T, K>,
        }
        impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Drop for Restore<'_, T, K> {
            fn drop(&mut self) {
                let active = self.owner.active.borrow_mut().take();
                if let Some(active) = active {
                    let data = match std::rc::Rc::try_unwrap(active) {
                        Ok(ActiveCache::Tracking(cache)) => cache.into_inner().into_dictionary(),
                        Ok(ActiveCache::Plain(cache)) => cache.into_inner().into_dictionary(),
                        Err(shared) => match &*shared {
                            ActiveCache::Tracking(cache) => cache.borrow().dictionary().clone(),
                            ActiveCache::Plain(cache) => cache.borrow().dictionary().clone(),
                        },
                    };
                    *self.owner.data.borrow_mut() = data;
                }
            }
        }
        let changes = {
            let restore = Restore { owner: self };
            {
                let mut updater = CacheUpdater::new(active.cell(), self.key_selector.as_ref());
                update(&mut updater);
            }
            let changes = match &*active {
                ActiveCache::Tracking(cache) => cache.borrow_mut().capture_changes(),
                ActiveCache::Plain(_) => ChangeSet::empty(),
            };
            drop(active);
            drop(restore);
            changes
        };
        if let (Some(preview), Some(copy)) = (preview, copy) {
            let new_data = std::mem::replace(&mut *self.data.borrow_mut(), copy);
            preview(&changes);
            *self.data.borrow_mut() = new_data;
        }
        changes
    }

    /// Runs an edit inside the edit in progress (`WriteNested`).
    ///
    /// # Panics
    /// When no edit is in progress (`InvalidOperationException`).
    pub(crate) fn write_nested(&self, update: &mut dyn FnMut(&mut dyn ISourceUpdater<T, K>)) {
        let active = self.active.borrow().clone();
        let Some(active) = active else {
            panic!("WriteNested can only be used if another write is already in progress.");
        };
        let mut updater = CacheUpdater::new(active.cell(), self.key_selector.as_ref());
        update(&mut updater);
    }
}
