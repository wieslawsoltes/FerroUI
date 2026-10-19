//! `MergeManyListChangeSets<TObject, TKey, TDestination>`
//! (`Cache/Internal/MergeManyListChangeSets.cs`): the change sets of a list chosen for each item of
//! a cache, merged into those of one list.

use super::transform_items;
use crate::cache::{ChangeReason, ChangeSet};
use crate::internal::{synchronize_safe_shared, CacheParentSubscription, ParentSubscriptionHandler};
use crate::list::internal::{remove_index, ChangeSetMergeTracker, ClonedListChangeSet};
use ferroui_reactive::Observable;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::hash::Hash;
use std::rc::Rc;

type ListChangeSet<D> = crate::list::ChangeSet<D>;

/// The state of one subscription (`Subscription`).
struct Handler<D> {
    change_set_merge_tracker: ChangeSetMergeTracker<D>,
}

impl<D: Clone + PartialEq + 'static, K: Eq + Hash + Clone + 'static> ParentSubscriptionHandler<ClonedListChangeSet<D>, K, ListChangeSet<D>, ListChangeSet<D>> for Handler<D> {
    fn parent_on_next(&self, owner: &CacheParentSubscription<ClonedListChangeSet<D>, K, ListChangeSet<D>, ListChangeSet<D>>, changes: ChangeSet<ClonedListChangeSet<D>, K>) {
        // All the changes at once, to keep the order of the change set.
        for change in &changes {
            match change.reason() {
                // Ends the subscription of the key (if any) and makes the new one; the items of the
                // replaced list are removed.
                ChangeReason::Add | ChangeReason::Update => {
                    owner.add_child_subscription(change.current().source().clone(), change.key().clone());
                    if let Some(previous) = change.previous() {
                        self.change_set_merge_tracker.remove_items(&previous.list(), None);
                    }
                }
                // Ends the subscription and removes the items of the list.
                ChangeReason::Remove => {
                    owner.remove_child_subscription(change.key());
                    self.change_set_merge_tracker.remove_items(&change.current().list(), None);
                }
                ChangeReason::Refresh | ChangeReason::Moved => {}
            }
        }
    }

    fn child_on_next(&self, child: ListChangeSet<D>, _parent_key: &K) {
        self.change_set_merge_tracker.process_change_set(&child, None);
    }

    fn emit_changes(&self, observer: &Rc<dyn IObserver<ListChangeSet<D>>>) {
        self.change_set_merge_tracker.emit_changes(observer);
    }
}

/// `MergeManyListChangeSets.Run`.
pub(crate) fn merge_many_list_change_sets<T: Clone + 'static, K: Eq + Hash + Clone + 'static, D: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    selector: Rc<dyn Fn(&T, &K) -> Rc<dyn IObservable<ListChangeSet<D>>>>,
    equality_comparer: Rc<dyn Fn(&D, &D) -> bool>,
) -> Rc<dyn IObservable<ListChangeSet<D>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ListChangeSet<D>>>| {
        let subscription: Rc<CacheParentSubscription<ClonedListChangeSet<D>, K, ListChangeSet<D>, ListChangeSet<D>>> =
            CacheParentSubscription::new(observer, Box::new(Handler { change_set_merge_tracker: ChangeSetMergeTracker::new() }));
        let queue = subscription.queue().clone();
        let (selector, equality_comparer) = (selector.clone(), equality_comparer.clone());
        // The indexes are removed before the queue; the copy of the list is made after it.
        let parent = transform_items(
            source.clone(),
            Rc::new(move |item: &T, key: &K| ClonedListChangeSet::new(synchronize_safe_shared(remove_index(selector(item, key)), &queue), Some(equality_comparer.clone()))),
        );
        subscription.create_parent_subscription(parent);
        let subscription: Rc<dyn IDisposable> = subscription;
        subscription
    })
}
