//! `DynamicCombiner<TObject, TKey>` (`Cache/Internal/DynamicCombiner.cs`): the items of the caches
//! of an observable list, combined by a logical operator as the caches come and go.

use super::{Cache, CombineOperator, SameReference};
use crate::cache::{ChangeAwareCache, ChangeSet, ICache, IQuery};
use crate::internal::{synchronize_safe_shared, SharedDeliveryQueue};
use crate::list::{IObservableList, ItemChange, ListChangeReason, ObservableListEx};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// A cache of the list, with the copy of its items (`DynamicCombiner.MergeContainer`); equal only
/// to itself, as upstream's class.
struct MergeContainer<T, K>(Rc<MergeContainerInner<T, K>>);

struct MergeContainerInner<T, K> {
    cache: Rc<RefCell<Cache<T, K>>>,
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
}

impl<T, K> Clone for MergeContainer<T, K> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, K> PartialEq for MergeContainer<T, K> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> MergeContainer<T, K> {
    /// `new MergeContainer(source)`: the source copies its changes into the cache of the
    /// container (`Source = source.Do(Clone)`).
    fn new(source: Rc<dyn IObservable<ChangeSet<T, K>>>) -> Self {
        let cache = Rc::new(RefCell::new(Cache::new()));
        let target = cache.clone();
        let source = source.do_(move |changes: &ChangeSet<T, K>| target.borrow_mut().clone_changes(changes));
        Self(Rc::new(MergeContainerInner { cache, source }))
    }

    /// `Source`.
    fn source(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.0.source.clone()
    }

    /// `Cache.KeyValues`.
    fn key_values(&self) -> Vec<(K, T)> {
        self.0.cache.borrow().key_values()
    }

    /// Whether the cache holds `key` (`Cache.Lookup(key).HasValue`).
    fn contains_key(&self, key: &K) -> bool {
        self.0.cache.borrow().dictionary().contains_key(key)
    }
}

/// `DynamicCombiner.Run`: `source` is the sequence of the list of caches, and `connect` makes the
/// sequence of the change sets of each of its items (the identity for a list of sequences, and
/// `Connect` for a list of caches).
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream's forms on a list of caches
/// (`IObservableList<IObservableCache>`, `IObservableList<ISourceCache>`) first make an observable
/// list of their `Connect()` sequences; here `connect` is applied as the containers are made, which
/// subscribes the same sequences in the same order.
pub(crate) fn dynamic_combiner<S: Clone + PartialEq + 'static, T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<crate::list::ChangeSet<S>>>,
    connect: Rc<dyn Fn(&S) -> Rc<dyn IObservable<ChangeSet<T, K>>>>,
    kind: CombineOperator,
    same_reference: SameReference<T>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let queue = SharedDeliveryQueue::new();

        // this is the resulting cache which produces all notifications
        let result_cache = Rc::new(RefCell::new(ChangeAwareCache::<T, K>::new()));

        // Transform to a merge container.
        // This populates a RefTracker when the original source is subscribed to
        let connect = connect.clone();
        let source_lists = synchronize_safe_shared(source.clone(), &queue)
            .transform(move |item: &S| MergeContainer::new(connect(item)))
            .as_observable_list();

        let published = source_lists.connect().publish();
        let shared_lists: Rc<dyn IObservable<crate::list::ChangeSet<MergeContainer<T, K>>>> = published.clone();

        // merge the items back together
        let (lists, target, next_observer, same) = (source_lists.clone(), result_cache.clone(), observer.clone(), same_reference.clone());
        let all_changes = synchronize_safe_shared(shared_lists.merge_many(|container: &MergeContainer<T, K>| container.source()), &queue).subscribe_next(
            move |changes: ChangeSet<T, K>| {
                // Populate result list and check for changes
                let notifications = {
                    let mut target = target.borrow_mut();
                    let sources = lists.items();
                    update_result_list(&mut target, &sources, &changes, kind, &same);
                    target.capture_changes()
                };
                if notifications.count() != 0 {
                    next_observer.on_next(notifications);
                }
            },
        );

        // when an list is removed, need to
        let (lists, target, removed_observer, same) = (source_lists.clone(), result_cache.clone(), observer.clone(), same_reference.clone());
        let removed_item = shared_lists
            .on_item_removed(move |container: &MergeContainer<T, K>| {
                let notifications = {
                    let mut target = target.borrow_mut();
                    let sources = lists.items();
                    // Remove items if required
                    process_changes(&mut target, &sources, container.key_values(), kind, &same);
                    if kind == CombineOperator::And || kind == CombineOperator::Except {
                        let items_to_check: Vec<(K, T)> = sources.iter().flat_map(|other| other.key_values()).collect();
                        process_changes(&mut target, &sources, items_to_check, kind, &same);
                    }
                    target.capture_changes()
                };
                if notifications.count() != 0 {
                    removed_observer.on_next(notifications);
                }
            })
            .subscribe_ignore();

        // when an list is added or removed, need to
        let (lists, target, added_observer, same) = (source_lists.clone(), result_cache.clone(), observer.clone(), same_reference.clone());
        let source_changed = shared_lists
            .where_reasons_are(&[ListChangeReason::Add, ListChangeReason::AddRange])
            .for_each_item_change(move |change: &ItemChange<MergeContainer<T, K>>| {
                let notifications = {
                    let mut target = target.borrow_mut();
                    let sources = lists.items();
                    process_changes(&mut target, &sources, change.current().key_values(), kind, &same);
                    if kind == CombineOperator::And || kind == CombineOperator::Except {
                        let items = target.key_values();
                        process_changes(&mut target, &sources, items, kind, &same);
                    }
                    target.capture_changes()
                };
                if notifications.count() != 0 {
                    added_observer.on_next(notifications);
                }
            })
            .subscribe_ignore();

        let source_lists: Rc<dyn IDisposable> = Rc::new(source_lists);
        let connection = published.connect();
        let queue: Rc<dyn IDisposable> = queue;
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([source_lists, all_changes, removed_item, source_changed, connection, queue]);
        composite
    })
}

/// `MatchesConstraint`: never, while the list is empty.
fn matches_constraint<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(sources: &[MergeContainer<T, K>], key: &K, kind: CombineOperator) -> bool {
    if sources.is_empty() {
        return false;
    }
    match kind {
        CombineOperator::And => sources.iter().all(|source| source.contains_key(key)),
        CombineOperator::Or => sources.iter().any(|source| source.contains_key(key)),
        CombineOperator::Xor => sources.iter().filter(|source| source.contains_key(key)).count() == 1,
        CombineOperator::Except => {
            let first = sources.iter().take(1).any(|source| source.contains_key(key));
            let others = sources.iter().skip(1).any(|source| source.contains_key(key));
            first && !others
        }
    }
}

/// `ProcessChanges`: whether each item should be removed from the result (or in the case of
/// `And`, added).
fn process_changes<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    target: &mut ChangeAwareCache<T, K>,
    source_lists: &[MergeContainer<T, K>],
    items: Vec<(K, T)>,
    kind: CombineOperator,
    same_reference: &SameReference<T>,
) {
    for (key, item) in items {
        process_item(target, source_lists, item, key, kind, same_reference);
    }
}

/// `ProcessItem`.
fn process_item<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    target: &mut ChangeAwareCache<T, K>,
    source_lists: &[MergeContainer<T, K>],
    item: T,
    key: K,
    kind: CombineOperator,
    same_reference: &SameReference<T>,
) {
    let cached = target.lookup(&key);
    let should_be_in_result = matches_constraint(source_lists, &key, kind);
    if should_be_in_result {
        match cached {
            None => target.add_or_update(item, key),
            Some(cached) => {
                if !same_reference(&item, &cached) {
                    target.add_or_update(item, key);
                }
            }
        }
    } else if cached.is_some() {
        target.remove(&key);
    }
}

/// `UpdateResultList`.
fn update_result_list<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    target: &mut ChangeAwareCache<T, K>,
    source_lists: &[MergeContainer<T, K>],
    changes: &ChangeSet<T, K>,
    kind: CombineOperator,
    same_reference: &SameReference<T>,
) {
    for change in changes {
        process_item(target, source_lists, change.current().clone(), change.key().clone(), kind, same_reference);
    }
}
