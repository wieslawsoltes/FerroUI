//! `CacheUpdater<TObject, TKey>` (`Cache/Internal/CacheUpdater.cs`).
//!
//! The updater reaches the cache it edits through a [`RefCell`] that each of its methods borrows
//! for that method only: an edit that starts another edit of the same cache from inside its
//! function (a nested edit, `ReaderWriter.WriteNested`) edits the same cache, as upstream's does.

use crate::cache::{ChangeSet, ICache, ICacheUpdater, IQuery, ISourceUpdater};
use std::cell::RefCell;
use std::rc::Rc;

/// The key selector of a cache (`Func<TObject, TKey>`).
pub type KeySelector<T, K> = Rc<dyn Fn(&T) -> K>;

/// The edit of a cache (`CacheUpdater<TObject, TKey>`).
pub(crate) struct CacheUpdater<'a, T, K> {
    cache: &'a RefCell<dyn ICache<T, K>>,
    key_selector: Option<&'a KeySelector<T, K>>,
}

impl<'a, T, K> CacheUpdater<'a, T, K> {
    pub(crate) fn new(cache: &'a RefCell<dyn ICache<T, K>>, key_selector: Option<&'a KeySelector<T, K>>) -> Self {
        Self { cache, key_selector }
    }

    fn key_of(&self, item: &T) -> K {
        match self.key_selector {
            Some(selector) => selector(item),
            None => panic!("A key selector must be specified"),
        }
    }

    fn with<R>(&self, action: impl FnOnce(&mut dyn ICache<T, K>) -> R) -> R {
        action(&mut *self.cache.borrow_mut())
    }
}

impl<T, K> IQuery<T, K> for CacheUpdater<'_, T, K> {
    fn count(&self) -> usize {
        self.cache.borrow().count()
    }

    fn items(&self) -> Vec<T> {
        self.cache.borrow().items()
    }

    fn keys(&self) -> Vec<K> {
        self.cache.borrow().keys()
    }

    fn key_values(&self) -> Vec<(K, T)> {
        self.cache.borrow().key_values()
    }

    fn lookup(&self, key: &K) -> Option<T> {
        self.cache.borrow().lookup(key)
    }
}

impl<T, K> ICacheUpdater<T, K> for CacheUpdater<'_, T, K> {
    fn add_or_update_pairs(&mut self, pairs: &mut dyn Iterator<Item = (K, T)>) {
        for (key, item) in pairs {
            self.with(|cache| cache.add_or_update(item, key));
        }
    }

    fn add_or_update_with_key(&mut self, item: T, key: K) {
        self.with(|cache| cache.add_or_update(item, key));
    }

    fn clear(&mut self) {
        self.with(|cache| cache.clear());
    }

    fn clone_changes(&mut self, changes: &ChangeSet<T, K>) {
        self.with(|cache| cache.clone_changes(changes));
    }

    fn get_key(&self, item: &T) -> K {
        self.key_of(item)
    }

    fn get_key_values(&self, items: &mut dyn Iterator<Item = T>) -> Vec<(K, T)> {
        items.map(|item| (self.key_of(&item), item)).collect()
    }

    fn refresh_all(&mut self) {
        self.with(|cache| cache.refresh_all());
    }

    fn refresh_keys(&mut self, keys: &mut dyn Iterator<Item = K>) {
        for key in keys {
            self.with(|cache| cache.refresh(&key));
        }
    }

    fn refresh_key(&mut self, key: &K) {
        self.with(|cache| cache.refresh(key));
    }

    fn remove_keys(&mut self, keys: &mut dyn Iterator<Item = K>) {
        for key in keys {
            self.with(|cache| cache.remove(&key));
        }
    }

    fn remove_key(&mut self, key: &K) {
        self.with(|cache| cache.remove(key));
    }

    fn remove_pairs(&mut self, pairs: &mut dyn Iterator<Item = (K, T)>) {
        for (key, _) in pairs {
            self.with(|cache| cache.remove(&key));
        }
    }
}

impl<T, K> ISourceUpdater<T, K> for CacheUpdater<'_, T, K> {
    fn add_or_update_items(&mut self, items: &mut dyn Iterator<Item = T>) {
        for item in items {
            self.add_or_update(item);
        }
    }

    fn add_or_update_items_with(&mut self, items: &mut dyn Iterator<Item = T>, comparer: &dyn Fn(&T, &T) -> bool) {
        for item in items {
            self.add_or_update_with(item, comparer);
        }
    }

    fn add_or_update(&mut self, item: T) {
        let key = self.key_of(&item);
        self.with(|cache| cache.add_or_update(item, key));
    }

    fn add_or_update_with(&mut self, item: T, comparer: &dyn Fn(&T, &T) -> bool) {
        let key = self.key_of(&item);
        let old = self.cache.borrow().lookup(&key);
        if let Some(old) = old {
            if comparer(&old, &item) {
                return;
            }
        }
        self.with(|cache| cache.add_or_update(item, key));
    }

    fn load(&mut self, items: &mut dyn Iterator<Item = T>) {
        self.with(|cache| cache.clear());
        self.add_or_update_items(items);
    }

    fn lookup_item(&self, item: &T) -> Option<T> {
        let key = self.key_of(item);
        self.cache.borrow().lookup(&key)
    }

    fn refresh_items(&mut self, items: &mut dyn Iterator<Item = T>) {
        for item in items {
            self.refresh_item(&item);
        }
    }

    fn refresh_item(&mut self, item: &T) {
        let key = self.key_of(item);
        self.with(|cache| cache.refresh(&key));
    }

    fn remove_items(&mut self, items: &mut dyn Iterator<Item = T>) {
        for item in items {
            self.remove_item(&item);
        }
    }

    fn remove_item(&mut self, item: &T) {
        let key = self.key_of(item);
        self.with(|cache| cache.remove(&key));
    }
}
