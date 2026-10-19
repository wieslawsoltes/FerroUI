//! `DynamicCombiner<T>` (`List/Internal/DynamicCombiner.cs`): the items of the lists of an
//! observable list, combined by a logical operator as the lists come and go.

use super::{clone_source_list, ReferenceCountTracker};
use crate::cache::internal::CombineOperator;
use crate::internal::synchronize;
use crate::list::change_aware_list_with_ref_counts::change_aware_list_with_ref_counts;
use crate::list::{flatten, ChangeAwareList, ChangeSet, IObservableList, ItemChange, ListChangeReason, ObservableListEx};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// A list of the observable list, with the counts of its items (`DynamicCombiner.MergeContainer`);
/// equal only to itself, as upstream's class.
struct MergeContainer<T>(Rc<MergeContainerInner<T>>);

struct MergeContainerInner<T> {
    tracker: Rc<RefCell<ReferenceCountTracker<T>>>,
    source: Rc<dyn IObservable<ChangeSet<T>>>,
}

impl<T> Clone for MergeContainer<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for MergeContainer<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: Clone + Eq + Hash + 'static> MergeContainer<T> {
    /// `new MergeContainer(source)`: the source counts its items into the tracker of the container
    /// (`Source = source.Do(Clone)`).
    fn new(source: Rc<dyn IObservable<ChangeSet<T>>>) -> Self {
        let tracker = Rc::new(RefCell::new(ReferenceCountTracker::new()));
        let target = tracker.clone();
        let source = source.do_(move |changes: &ChangeSet<T>| clone_source_list(&mut target.borrow_mut(), changes));
        Self(Rc::new(MergeContainerInner { tracker, source }))
    }

    /// `Source`.
    fn source(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        self.0.source.clone()
    }

    /// `Tracker.Items`.
    fn items(&self) -> Vec<T> {
        self.0.tracker.borrow().items()
    }

    /// `Tracker.Contains(item)`.
    fn contains(&self, item: &T) -> bool {
        self.0.tracker.borrow().contains(item)
    }
}

/// `DynamicCombiner.Run`: `source` is the sequence of the list of lists, and `connect` makes the
/// sequence of the change sets of each of its items (the identity for a list of sequences, and
/// `Connect` for a list of lists).
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream's forms on a list of lists
/// (`IObservableList<IObservableList>`, `IObservableList<ISourceList>`) first make an observable
/// list of their `Connect()` sequences; here `connect` is applied as the containers are made, which
/// subscribes the same sequences in the same order.
pub(crate) fn dynamic_combiner<S: Clone + PartialEq + 'static, T: Clone + Eq + Hash + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S>>>,
    connect: Rc<dyn Fn(&S) -> Rc<dyn IObservable<ChangeSet<T>>>>,
    kind: CombineOperator,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        // this is the resulting list which produces all notifications
        let result_list = Rc::new(RefCell::new(change_aware_list_with_ref_counts::<T>()));

        // Transform to a merge container.
        // This populates a RefTracker when the original source is subscribed to
        let connect = connect.clone();
        let source_lists = synchronize(source.clone()).transform(move |item: &S| MergeContainer::new(connect(item))).as_observable_list();

        // merge the items back together
        let (lists, target, next_observer) = (source_lists.clone(), result_list.clone(), observer.clone());
        let all_changes = synchronize(source_lists.connect().merge_many(|container: &MergeContainer<T>| container.source())).subscribe_next(move |changes: ChangeSet<T>| {
            // Populate result list and check for changes
            let notifications = update_result_list(&lists.items(), &mut target.borrow_mut(), &changes, kind);
            if notifications.count() != 0 {
                next_observer.on_next(notifications);
            }
        });

        // When a list is removed, update all items that were in that list
        let (lists, target, removed_observer) = (source_lists.clone(), result_list.clone(), observer.clone());
        let removed_item = source_lists
            .connect()
            .on_item_removed(move |container: &MergeContainer<T>| {
                // Remove items if required
                let sources = lists.items();
                let notifications = update_item_set_memberships(&sources, &mut target.borrow_mut(), container.items(), kind);
                if notifications.count() != 0 {
                    removed_observer.on_next(notifications);
                }

                // On some operators, items not in the removed list can also be affected.
                if kind == CombineOperator::And || kind == CombineOperator::Except {
                    let items_to_check: Vec<T> = sources.iter().flat_map(MergeContainer::items).collect();
                    let notification2 = update_item_set_memberships(&sources, &mut target.borrow_mut(), items_to_check, kind);
                    if notification2.count() != 0 {
                        removed_observer.on_next(notification2);
                    }
                }
            })
            .subscribe_ignore();

        // When a list is added, update all items that are in that list
        let (lists, target, added_observer) = (source_lists.clone(), result_list.clone(), observer);
        let source_changed = source_lists
            .connect()
            .where_reasons_are(&[ListChangeReason::Add, ListChangeReason::AddRange])
            .for_each_item_change(move |change: &ItemChange<MergeContainer<T>>| {
                let sources = lists.items();
                let notifications = update_item_set_memberships(&sources, &mut target.borrow_mut(), change.current().items(), kind);
                if notifications.count() != 0 {
                    added_observer.on_next(notifications);
                }

                // On some operators, items not in the new list can also be affected.
                if kind == CombineOperator::And || kind == CombineOperator::Except {
                    let items = target.borrow().items().to_vec();
                    let notification2 = update_item_set_memberships(&sources, &mut target.borrow_mut(), items, kind);
                    if notification2.count() != 0 {
                        added_observer.on_next(notification2);
                    }
                }
            })
            .subscribe_ignore();

        let source_lists: Rc<dyn IDisposable> = Rc::new(source_lists);
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([source_lists, all_changes, removed_item, source_changed]);
        composite
    })
}

/// `MatchesConstraint`: never, while the list is empty.
fn matches_constraint<T: Clone + Eq + Hash + 'static>(source_lists: &[MergeContainer<T>], item: &T, kind: CombineOperator) -> bool {
    if source_lists.is_empty() {
        return false;
    }
    match kind {
        CombineOperator::And => source_lists.iter().all(|list| list.contains(item)),
        CombineOperator::Or => source_lists.iter().any(|list| list.contains(item)),
        CombineOperator::Xor => source_lists.iter().filter(|list| list.contains(item)).count() == 1,
        CombineOperator::Except => {
            let first = source_lists[0].contains(item);
            let others = source_lists.iter().skip(1).any(|list| list.contains(item));
            first && !others
        }
    }
}

/// `UpdateItemMembership`.
fn update_item_membership<T: Clone + Eq + Hash + 'static>(item: &T, source_lists: &[MergeContainer<T>], result_list: &mut ChangeAwareList<T>, kind: CombineOperator) {
    let is_in_result = result_list.contains(item);
    let should_be_in_result = matches_constraint(source_lists, item, kind);
    if should_be_in_result && !is_in_result {
        result_list.add(item.clone());
    } else if !should_be_in_result && is_in_result {
        result_list.remove(item);
    }
}

/// `UpdateItemSetMemberships`.
fn update_item_set_memberships<T: Clone + Eq + Hash + 'static>(source_lists: &[MergeContainer<T>], resulting_list: &mut ChangeAwareList<T>, items: Vec<T>, kind: CombineOperator) -> ChangeSet<T> {
    for item in &items {
        update_item_membership(item, source_lists, resulting_list, kind);
    }
    resulting_list.capture_changes()
}

/// `UpdateResultList`.
fn update_result_list<T: Clone + Eq + Hash + 'static>(source_lists: &[MergeContainer<T>], result_list: &mut ChangeAwareList<T>, changes: &ChangeSet<T>, kind: CombineOperator) -> ChangeSet<T> {
    // child caches have been updated before we reached this point.
    for change in flatten(changes) {
        match change.reason() {
            ListChangeReason::Add | ListChangeReason::Remove => update_item_membership(change.current(), source_lists, result_list, kind),
            ListChangeReason::Replace => {
                update_item_membership(change.previous().expect("a replacement has a previous item"), source_lists, result_list, kind);
                update_item_membership(change.current(), source_lists, result_list, kind);
            }
            // Pass through refresh changes:
            ListChangeReason::Refresh => {
                result_list.refresh(change.current());
            }
            // A move does not affect contents and so can be ignored:
            ListChangeReason::Moved => {}
            // These should not occur as they are replaced by the Flatten operator:
            ListChangeReason::AddRange | ListChangeReason::RemoveRange | ListChangeReason::Clear => panic!("Unsupported change type"),
        }
    }
    result_list.capture_changes()
}
