//! `MergeManyCacheChangeSetsSourceCompare<TObject, TKey, TDestination, TDestinationKey>`
//! (`Cache/Internal/MergeManyCacheChangeSetsSourceCompare.cs`): as `MergeManyCacheChangeSets`,
//! where the value of a key is chosen by the order of the parents first, then of the children.

use super::{transform_items, Cache, ChangeSetCache, ChangeSetMergeTracker, ChangeSetSelector, Comparer, EqualityComparer};
use crate::cache::{Change, ChangeReason, ChangeSet, ICache, IQuery};
use crate::internal::{synchronize_safe_shared, CacheParentSubscription, ParentSubscriptionHandler};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::hash::Hash;
use std::rc::Rc;

/// A child with its parent (`ParentChildEntry`).
#[derive(Clone)]
pub(crate) struct ParentChildEntry<T, D> {
    pub(crate) parent: T,
    pub(crate) child: D,
}

/// The order of the entries (`ParentChildCompare`, or `ParentOnlyCompare` without a comparer of
/// the children).
fn entry_comparer<T: 'static, D: 'static>(parent_compare: Comparer<T>, child_compare: Option<Comparer<D>>) -> Comparer<ParentChildEntry<T, D>> {
    match child_compare {
        Some(child_compare) => Rc::new(move |x: &ParentChildEntry<T, D>, y: &ParentChildEntry<T, D>| match parent_compare(&x.parent, &y.parent) {
            Ordering::Equal => child_compare(&x.child, &y.child),
            ordering => ordering,
        }),
        None => Rc::new(move |x: &ParentChildEntry<T, D>, y: &ParentChildEntry<T, D>| parent_compare(&x.parent, &y.parent)),
    }
}

/// The equality of the entries by their children (`ParentChildEqualityCompare`).
pub(crate) fn child_equality<T: 'static, D: 'static>(equality_comparer: EqualityComparer<D>) -> EqualityComparer<ParentChildEntry<T, D>> {
    Rc::new(move |x: &ParentChildEntry<T, D>, y: &ParentChildEntry<T, D>| equality_comparer(&x.child, &y.child))
}

/// The equality of the record `ParentChildEntry` (`EqualityComparer<ParentChildEntry>.Default`):
/// both the parent and the child equal.
pub(crate) fn entry_equality<T: PartialEq + 'static, D: PartialEq + 'static>() -> EqualityComparer<ParentChildEntry<T, D>> {
    Rc::new(|x: &ParentChildEntry<T, D>, y: &ParentChildEntry<T, D>| x.parent == y.parent && x.child == y.child)
}

type EntryCache<T, D, DK> = ChangeSetCache<ParentChildEntry<T, D>, DK>;

/// The state of one subscription (`Subscription`).
struct Handler<T, D, DK, K> {
    cache: Rc<RefCell<Cache<Rc<EntryCache<T, D, DK>>, K>>>,
    change_set_merge_tracker: ChangeSetMergeTracker<ParentChildEntry<T, D>, DK>,
    reeval_on_refresh: bool,
}

impl<T: Clone + 'static, D: Clone + 'static, DK: Eq + Hash + Clone + 'static, K: Eq + Hash + Clone + 'static>
    ParentSubscriptionHandler<Rc<EntryCache<T, D, DK>>, K, ChangeSet<ParentChildEntry<T, D>, DK>, ChangeSet<ParentChildEntry<T, D>, DK>> for Handler<T, D, DK, K>
{
    fn parent_on_next(
        &self,
        owner: &CacheParentSubscription<Rc<EntryCache<T, D, DK>>, K, ChangeSet<ParentChildEntry<T, D>, DK>, ChangeSet<ParentChildEntry<T, D>, DK>>,
        changes: ChangeSet<Rc<EntryCache<T, D, DK>>, K>,
    ) {
        // All the changes at once, to keep the order of the change set.
        for change in &changes {
            match change.reason() {
                ChangeReason::Add | ChangeReason::Update => {
                    self.cache.borrow_mut().add_or_update(change.current().clone(), change.key().clone());
                    owner.add_child_subscription(change.current().source(), change.key().clone());
                    if let Some(previous) = change.previous() {
                        let key_values = previous.cache().key_values();
                        self.change_set_merge_tracker.remove_items(&key_values, None);
                    }
                }
                ChangeReason::Remove => {
                    self.cache.borrow_mut().remove(change.key());
                    owner.remove_child_subscription(change.key());
                    let key_values = change.current().cache().key_values();
                    self.change_set_merge_tracker.remove_items(&key_values, None);
                }
                ChangeReason::Refresh => {
                    if self.reeval_on_refresh {
                        let keys = change.current().cache().keys();
                        self.change_set_merge_tracker.refresh_items(&keys, None);
                    }
                }
                ChangeReason::Moved => {}
            }
        }
    }

    fn child_on_next(&self, changes: ChangeSet<ParentChildEntry<T, D>, DK>, _parent_key: &K) {
        self.change_set_merge_tracker.process_change_set(&changes, None);
    }

    fn emit_changes(&self, observer: &Rc<dyn IObserver<ChangeSet<ParentChildEntry<T, D>, DK>>>) {
        self.change_set_merge_tracker.emit_changes(observer);
    }
}

/// `MergeManyCacheChangeSetsSourceCompare.Run`: the merge of the entries, then the children of the
/// entries (`TransformImmutable(entry => entry.Child)`).
pub(crate) fn merge_many_cache_change_sets_source_compare<
    T: Clone + 'static,
    K: Eq + Hash + Clone + 'static,
    D: Clone + 'static,
    DK: Eq + Hash + Clone + 'static,
>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    selector: ChangeSetSelector<T, K, D, DK>,
    parent_compare: Comparer<T>,
    equality_comparer: EqualityComparer<ParentChildEntry<T, D>>,
    child_compare: Option<Comparer<D>>,
    reeval_on_refresh: bool,
) -> Rc<dyn IObservable<ChangeSet<D, DK>>> {
    let comparer = entry_comparer(parent_compare, child_compare);
    let entries = Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<ParentChildEntry<T, D>, DK>>>| {
        let cache = Rc::new(RefCell::new(Cache::new()));
        let caches = cache.clone();
        let change_set_merge_tracker = ChangeSetMergeTracker::new(move || caches.borrow().items(), Some(comparer.clone()), equality_comparer.clone());
        let handler = Handler::<T, D, DK, K> { cache, change_set_merge_tracker, reeval_on_refresh };
        let subscription: Rc<CacheParentSubscription<Rc<EntryCache<T, D, DK>>, K, ChangeSet<ParentChildEntry<T, D>, DK>, ChangeSet<ParentChildEntry<T, D>, DK>>> =
            CacheParentSubscription::new(observer, Box::new(handler));
        let queue = subscription.queue().clone();
        let selector = selector.clone();
        // Deviation (DEVIATIONS.md, DynamicData): `IgnoreSameReferenceUpdate` is left out; a generic
        // item has no identity to compare, and an update to the same value changes nothing in the
        // tracker.
        let parent = transform_items(
            source.clone(),
            Rc::new(move |item: &T, key: &K| {
                let parent = item.clone();
                let children = transform_items(selector(item, key), Rc::new(move |child: &D, _: &DK| ParentChildEntry { parent: parent.clone(), child: child.clone() }));
                ChangeSetCache::new(synchronize_safe_shared(children, &queue))
            }),
        );
        subscription.create_parent_subscription(parent);
        let subscription: Rc<dyn IDisposable> = subscription;
        subscription
    });
    entries.select(|changes: ChangeSet<ParentChildEntry<T, D>, DK>| {
        changes
            .iter()
            .map(|change| {
                Change::with_previous(
                    change.reason(),
                    change.key().clone(),
                    change.current().child.clone(),
                    change.previous().map(|previous| previous.child.clone()),
                    change.current_index(),
                    change.previous_index(),
                )
            })
            .collect::<ChangeSet<D, DK>>()
    })
}
