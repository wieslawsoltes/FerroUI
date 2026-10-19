//! `ChangeSetMergeTracker<TObject, TKey>` (`Cache/Internal/ChangeSetMergeTracker.cs`): the merge of
//! the change sets of several caches into one, which chooses the value of each key among the
//! caches that have it.
//!
//! Without a comparer, the first value of a key stays until it is removed or replaced; with one,
//! the value downstream is the least by the comparer, the first of the caches among equals.

use super::ChangeSetCache;
use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache, IQuery};
use ferroui_system::IObserver;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::hash::Hash;
use std::rc::Rc;

/// An equality comparer of items (`IEqualityComparer<T>`).
pub(crate) type EqualityComparer<T> = Rc<dyn Fn(&T, &T) -> bool>;
/// A comparer of items (`IComparer<T>`).
pub(crate) type Comparer<T> = Rc<dyn Fn(&T, &T) -> Ordering>;

/// The equality comparer of `PartialEq` (`EqualityComparer<T>.Default`).
pub(crate) fn default_equality_comparer<T: PartialEq + 'static>() -> EqualityComparer<T> {
    Rc::new(|left: &T, right: &T| left == right)
}

/// The tracker (`ChangeSetMergeTracker<TObject, TKey>`).
pub(crate) struct ChangeSetMergeTracker<T, K> {
    select_caches: Box<dyn Fn() -> Vec<Rc<ChangeSetCache<T, K>>>>,
    comparer: Option<Comparer<T>>,
    equality_comparer: EqualityComparer<T>,
    result_cache: RefCell<ChangeAwareCache<T, K>>,
    has_completed: Cell<bool>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> ChangeSetMergeTracker<T, K> {
    /// A tracker of the caches that `select_caches` gives (`ChangeSetMergeTracker(Func<IEnumerable
    /// <ChangeSetCache>>, IComparer<TObject>?, IEqualityComparer<TObject>?)`).
    pub(crate) fn new(
        select_caches: impl Fn() -> Vec<Rc<ChangeSetCache<T, K>>> + 'static,
        comparer: Option<Comparer<T>>,
        equality_comparer: EqualityComparer<T>,
    ) -> Self {
        Self { select_caches: Box::new(select_caches), comparer, equality_comparer, result_cache: RefCell::new(ChangeAwareCache::new()), has_completed: Cell::new(false) }
    }

    /// Removes the items, choosing another value for their keys (`RemoveItems`).
    pub(crate) fn remove_items(&self, items: &[(K, T)], observer: Option<&Rc<dyn IObserver<ChangeSet<T, K>>>>) {
        let source_caches = (self.select_caches)();
        for (key, item) in items {
            self.on_item_removed(&source_caches, item, key);
        }
        if let Some(observer) = observer {
            self.emit_changes(observer);
        }
    }

    /// Chooses the value of the keys again (`RefreshItems`).
    pub(crate) fn refresh_items(&self, keys: &[K], observer: Option<&Rc<dyn IObserver<ChangeSet<T, K>>>>) {
        let source_caches = (self.select_caches)();
        for key in keys {
            self.force_evaluate(&source_caches, key);
        }
        if let Some(observer) = observer {
            self.emit_changes(observer);
        }
    }

    /// Applies the change set of one of the caches (`ProcessChangeSet`).
    pub(crate) fn process_change_set(&self, changes: &ChangeSet<T, K>, observer: Option<&Rc<dyn IObserver<ChangeSet<T, K>>>>) {
        let source_caches = (self.select_caches)();
        for change in changes {
            match change.reason() {
                ChangeReason::Add => self.on_item_added(change.current(), change.key()),
                ChangeReason::Remove => self.on_item_removed(&source_caches, change.current(), change.key()),
                ChangeReason::Update => self.on_item_updated(&source_caches, change.current(), change.key(), change.previous()),
                ChangeReason::Refresh => self.on_item_refreshed(&source_caches, change.current(), change.key()),
                ChangeReason::Moved => {}
            }
        }
        if let Some(observer) = observer {
            self.emit_changes(observer);
        }
    }

    /// Emits the changes since the last emission, and the completion once marked (`EmitChanges`).
    pub(crate) fn emit_changes(&self, observer: &Rc<dyn IObserver<ChangeSet<T, K>>>) {
        let change_set = self.result_cache.borrow_mut().capture_changes();
        if change_set.count() != 0 {
            observer.on_next(change_set);
        }
        if self.has_completed.get() {
            observer.on_completed();
        }
    }

    fn lookup(&self, key: &K) -> Option<T> {
        self.result_cache.borrow().lookup(key)
    }

    fn on_item_added(&self, item: &T, key: &K) {
        match self.lookup(key) {
            // No value yet: add it.
            None => self.result_cache.borrow_mut().add(item.clone(), key.clone()),
            Some(cached) => {
                if self.should_replace(item, &cached) {
                    self.result_cache.borrow_mut().add_or_update(item.clone(), key.clone());
                }
            }
        }
    }

    fn on_item_removed(&self, source_caches: &[Rc<ChangeSetCache<T, K>>], item: &T, key: &K) {
        let cached = self.lookup(key);
        // The key has been seen, and its value downstream is the one removed: choose another, or
        // remove it.
        if let Some(value) = &cached {
            if self.check_equality(item, value) {
                self.update_to_best_value(source_caches, key, cached.as_ref());
            }
        }
    }

    fn on_item_updated(&self, source_caches: &[Rc<ChangeSetCache<T, K>>], item: &T, key: &K, previous: Option<&T>) {
        let Some(cached) = self.lookup(key) else {
            // An update of a key not seen yet: take the value.
            self.result_cache.borrow_mut().add(item.clone(), key.clone());
            return;
        };
        // Whether the value downstream is the one replaced.
        let is_updating_current = previous.is_none_or(|previous| self.check_equality(previous, &cached));
        if self.comparer.is_none() {
            // The value downstream is replaced by a different one.
            if is_updating_current && !self.check_equality(item, &cached) {
                self.result_cache.borrow_mut().add_or_update(item.clone(), key.clone());
            }
        } else if is_updating_current {
            // The best value has been replaced: choose again among all of them.
            self.update_to_best_value(source_caches, key, Some(&cached));
        } else if self.should_replace(item, &cached) {
            // Another value changed: it replaces the one downstream only when it is better.
            self.result_cache.borrow_mut().add_or_update(item.clone(), key.clone());
        }
    }

    fn on_item_refreshed(&self, source_caches: &[Rc<ChangeSetCache<T, K>>], item: &T, key: &K) {
        // A refresh of a key not seen yet changes nothing.
        let Some(cached) = self.lookup(key) else { return };
        // With a comparer, a refresh may change which value is the best, so choose again; when that
        // changed nothing and the value downstream is the one refreshed, emit the refresh.
        let reselected = self.comparer.is_some() && self.update_to_best_value(source_caches, key, Some(&cached));
        if !reselected && self.check_equality(&cached, item) {
            self.result_cache.borrow_mut().refresh(key);
        }
    }

    fn force_evaluate(&self, source_caches: &[Rc<ChangeSetCache<T, K>>], key: &K) {
        let cached = self.lookup(key);
        if cached.is_none() {
            return;
        }
        self.update_to_best_value(source_caches, key, cached.as_ref());
    }

    /// Makes the value downstream the best one; true when it changed (`UpdateToBestValue`).
    fn update_to_best_value(&self, source_caches: &[Rc<ChangeSetCache<T, K>>], key: &K, current: Option<&T>) -> bool {
        match self.lookup_best_value(source_caches, key) {
            Some(candidate) => match current {
                None => {
                    self.result_cache.borrow_mut().add(candidate, key.clone());
                    true
                }
                Some(current) if !self.check_equality(current, &candidate) => {
                    self.result_cache.borrow_mut().add_or_update(candidate, key.clone());
                    true
                }
                // The value downstream is the one it should be.
                Some(_) => false,
            },
            None => {
                // No value is left.
                self.result_cache.borrow_mut().remove(key);
                true
            }
        }
    }

    /// The value of `key` downstream: the first of the caches, or the least by the comparer and
    /// the first among equals, as a stable `OrderBy` gives (`LookupBestValue`).
    fn lookup_best_value(&self, source_caches: &[Rc<ChangeSetCache<T, K>>], key: &K) -> Option<T> {
        let mut values = source_caches.iter().filter_map(|source| source.cache().lookup(key));
        match &self.comparer {
            None => values.next(),
            Some(comparer) => values.fold(None, |best: Option<T>, value| match best {
                Some(best) if comparer(&value, &best) != Ordering::Less => Some(best),
                _ => Some(value),
            }),
        }
    }

    fn check_equality(&self, left: &T, right: &T) -> bool {
        (self.equality_comparer)(left, right)
    }

    /// Whether `candidate` should replace `current` downstream (`ShouldReplace`).
    fn should_replace(&self, candidate: &T, current: &T) -> bool {
        self.comparer.as_ref().is_some_and(|comparer| comparer(candidate, current) == Ordering::Less)
    }
}
