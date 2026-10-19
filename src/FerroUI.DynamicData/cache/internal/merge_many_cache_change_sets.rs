//! `MergeManyCacheChangeSets<TObject, TKey, TDestination, TDestinationKey>`
//! (`Cache/Internal/MergeManyCacheChangeSets.cs`): the change sets of a cache chosen for each item
//! of the source, merged into those of one cache.

use super::{transform_items, Cache, ChangeSetCache, ChangeSetMergeTracker, Comparer, EqualityComparer};
use crate::cache::{ChangeReason, ChangeSet, ICache, IQuery};
use crate::internal::{synchronize_safe_shared, CacheParentSubscription, ParentSubscriptionHandler};
use ferroui_reactive::Observable;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The selector of the change sets of an item.
pub(crate) type ChangeSetSelector<T, K, D, DK> = Rc<dyn Fn(&T, &K) -> Rc<dyn IObservable<ChangeSet<D, DK>>>>;

/// The state of one subscription (`Subscription`).
struct Handler<D, DK, K> {
    cache: Rc<RefCell<Cache<Rc<ChangeSetCache<D, DK>>, K>>>,
    change_set_merge_tracker: ChangeSetMergeTracker<D, DK>,
}

impl<D: Clone + 'static, DK: Eq + Hash + Clone + 'static, K: Eq + Hash + Clone + 'static> Handler<D, DK, K> {
    fn new(comparer: Option<Comparer<D>>, equality_comparer: EqualityComparer<D>) -> Self {
        let cache = Rc::new(RefCell::new(Cache::new()));
        let caches = cache.clone();
        let change_set_merge_tracker = ChangeSetMergeTracker::new(move || caches.borrow().items(), comparer, equality_comparer);
        Self { cache, change_set_merge_tracker }
    }
}

impl<D: Clone + 'static, DK: Eq + Hash + Clone + 'static, K: Eq + Hash + Clone + 'static>
    ParentSubscriptionHandler<Rc<ChangeSetCache<D, DK>>, K, ChangeSet<D, DK>, ChangeSet<D, DK>> for Handler<D, DK, K>
{
    fn parent_on_next(
        &self,
        owner: &CacheParentSubscription<Rc<ChangeSetCache<D, DK>>, K, ChangeSet<D, DK>, ChangeSet<D, DK>>,
        changes: ChangeSet<Rc<ChangeSetCache<D, DK>>, K>,
    ) {
        // All the changes at once, to keep the order of the change set.
        for change in &changes {
            match change.reason() {
                // Ends the subscription of the key (if any) and makes the new one, which updates
                // the cache and emits the changes.
                ChangeReason::Add | ChangeReason::Update => {
                    self.cache.borrow_mut().add_or_update(change.current().clone(), change.key().clone());
                    owner.add_child_subscription(change.current().source(), change.key().clone());
                    if let Some(previous) = change.previous() {
                        let key_values = previous.cache().key_values();
                        self.change_set_merge_tracker.remove_items(&key_values, None);
                    }
                }
                // Ends the subscription and removes the cache.
                ChangeReason::Remove => {
                    self.cache.borrow_mut().remove(change.key());
                    owner.remove_child_subscription(change.key());
                    let key_values = change.current().cache().key_values();
                    self.change_set_merge_tracker.remove_items(&key_values, None);
                }
                ChangeReason::Refresh | ChangeReason::Moved => {}
            }
        }
    }

    fn child_on_next(&self, changes: ChangeSet<D, DK>, _parent_key: &K) {
        self.change_set_merge_tracker.process_change_set(&changes, None);
    }

    fn emit_changes(&self, observer: &Rc<dyn IObserver<ChangeSet<D, DK>>>) {
        self.change_set_merge_tracker.emit_changes(observer);
    }
}

/// `MergeManyCacheChangeSets.Run`.
pub(crate) fn merge_many_cache_change_sets<T: Clone + 'static, K: Eq + Hash + Clone + 'static, D: Clone + 'static, DK: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    change_set_selector: ChangeSetSelector<T, K, D, DK>,
    equality_comparer: EqualityComparer<D>,
    comparer: Option<Comparer<D>>,
) -> Rc<dyn IObservable<ChangeSet<D, DK>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D, DK>>>| {
        let subscription: Rc<CacheParentSubscription<Rc<ChangeSetCache<D, DK>>, K, ChangeSet<D, DK>, ChangeSet<D, DK>>> =
            CacheParentSubscription::new(observer, Box::new(Handler::<D, DK, K>::new(comparer.clone(), equality_comparer.clone())));
        // The child sequence goes into the `ChangeSetCache`, delivered through the queue.
        let queue = subscription.queue().clone();
        let selector = change_set_selector.clone();
        // Deviation (DEVIATIONS.md, DynamicData): `IgnoreSameReferenceUpdate` is left out; a generic
        // item has no identity to compare, and an update to the same value changes nothing in the
        // tracker.
        let parent = transform_items(source.clone(), Rc::new(move |item: &T, key: &K| ChangeSetCache::new(synchronize_safe_shared(selector(item, key), &queue))));
        subscription.create_parent_subscription(parent);
        let subscription: Rc<dyn IDisposable> = subscription;
        subscription
    })
}
