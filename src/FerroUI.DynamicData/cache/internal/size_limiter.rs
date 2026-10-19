//! `SizeLimiter<TObject, TKey>` (`Cache/Internal/SizeLimiter.cs`).

use super::ExpirableItem;
use crate::cache::{Change, ChangeAwareCache, ChangeSet, ICache, IQuery};
use std::cmp::Reverse;
use std::hash::Hash;

/// Keeps the newest `size` items (`SizeLimiter<TObject, TKey>`).
pub(crate) struct SizeLimiter<T, K> {
    cache: ChangeAwareCache<ExpirableItem<T, K>, K>,
    size: usize,
}

impl<T: Clone, K: Eq + Hash + Clone> SizeLimiter<T, K> {
    pub(crate) fn new(size: usize) -> Self {
        Self { cache: ChangeAwareCache::new(), size }
    }

    /// `Change`: applies the updates and removes the oldest items over the size; the changes of
    /// the items.
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): the items are ordered by the order in which they
    /// were added or updated, where upstream orders them by `DateTime.Now` at that moment and leaves
    /// the items stamped within the same tick in the order of its dictionary.
    pub(crate) fn change(&mut self, updates: &ChangeSet<ExpirableItem<T, K>, K>) -> ChangeSet<T, K> {
        self.cache.clone_changes(updates);
        let mut key_values = self.cache.key_values();
        key_values.sort_by_key(|(_, item)| Reverse(item.index));
        let items_to_expire: Vec<K> = key_values.into_iter().skip(self.size).map(|(key, _)| key).collect();
        if !items_to_expire.is_empty() {
            self.cache.remove_keys(&mut items_to_expire.into_iter());
        }
        let notifications = self.cache.capture_changes();
        notifications
            .iter()
            .map(|update| Change::with_previous(update.reason(), update.key().clone(), update.current().value.clone(), update.previous().map(|previous| previous.value.clone()), None, None))
            .collect()
    }

    /// `CloneAndReturnExpiredOnly`: applies the updates; the items over the size, the oldest by
    /// their order of addition, which stay in the limiter.
    pub(crate) fn clone_and_return_expired_only(&mut self, updates: &ChangeSet<ExpirableItem<T, K>, K>) -> Vec<(K, T)> {
        self.cache.clone_changes(updates);
        // The changes are cleared.
        self.cache.capture_changes();
        let mut key_values = self.cache.key_values();
        key_values.sort_by_key(|(_, item)| Reverse(item.index));
        key_values.into_iter().skip(self.size).map(|(key, item)| (key, item.value)).collect()
    }
}

/// `LimitSizeTo(ISourceCache, int, IScheduler?)`: the oldest items of the source over
/// `size_limit` are removed from it, on `scheduler`; the removed items with their keys.
///
/// # Panics
/// For a size of zero (`ArgumentException`).
pub(crate) fn limit_size_to_source<T: Clone + 'static, K: Eq + Hash + Clone + 'static, S: crate::cache::ISourceCache<T, K> + Clone + 'static>(
    source: S,
    size_limit: usize,
    scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<(K, T)>>> {
    use crate::cache::ISourceUpdater;
    use ferroui_reactive::{Observable, ObservableExt};
    use ferroui_system::IObserver;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    assert!(size_limit > 0, "Size limit must be greater than zero");
    let scheduler = scheduler.unwrap_or_else(ferroui_reactive::concurrency::SchedulerDefaults::time_based_operations);
    Observable::create(move |observer: Rc<dyn IObserver<Vec<(K, T)>>>| {
        let order_item_was_added = Cell::new(0u64);
        let size_limiter = RefCell::new(SizeLimiter::<T, K>::new(size_limit));
        let transformed = RefCell::new(ChangeAwareCache::new());
        let completion = observer.clone();
        let editor = source.clone();
        source
            .connect()
            .finally(move || completion.on_completed())
            .observe_on(scheduler.clone())
            .select(move |changes: ChangeSet<T, K>| {
                let expirable = super::transform_to_expirable(&mut transformed.borrow_mut(), &changes, &order_item_was_added);
                size_limiter.borrow_mut().clone_and_return_expired_only(&expirable)
            })
            .where_(|expired: &Vec<(K, T)>| !expired.is_empty())
            .subscribe_next(move |to_remove: Vec<(K, T)>| {
                let keys: Vec<K> = to_remove.iter().map(|(key, _)| key.clone()).collect();
                let mut keys = Some(keys);
                editor.edit(&mut |updater: &mut dyn ISourceUpdater<T, K>| {
                    if let Some(keys) = keys.take() {
                        updater.remove_keys(&mut keys.into_iter());
                    }
                });
                observer.on_next(to_remove);
            })
    })
}
