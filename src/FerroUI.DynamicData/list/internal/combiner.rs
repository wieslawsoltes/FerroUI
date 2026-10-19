//! `Combiner<T>` (`List/Internal/Combiner.cs`): the items of a fixed set of lists, combined by a
//! logical operator.

use super::ReferenceCountTracker;
use crate::cache::internal::CombineOperator;
use crate::list::change_aware_list_with_ref_counts::change_aware_list_with_ref_counts;
use crate::list::{flatten, ChangeAwareList, ChangeSet, ListChangeReason};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// `Combiner.Run`: the items of `source` that `kind` selects. Each subscription counts the items
/// of each list, then subscribes the lists in order.
pub(crate) fn combiner<T: Clone + Eq + Hash + 'static>(source: Vec<Rc<dyn IObservable<ChangeSet<T>>>>, kind: CombineOperator) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let disposable = CompositeDisposable::new();
        let result_list = Rc::new(RefCell::new(change_aware_list_with_ref_counts::<T>()));
        let source_lists = Rc::new(RefCell::new((0..source.len()).map(|_| ReferenceCountTracker::<T>::new()).collect::<Vec<_>>()));
        for (index, item) in source.iter().enumerate() {
            let (source_lists, result_list, observer) = (source_lists.clone(), result_list.clone(), observer.clone());
            // `Synchronize(_locker)` serializes nothing on one thread; upstream subscribes without
            // an error handler: an error of a source is thrown.
            let subscription = item.subscribe_next(move |changes: ChangeSet<T>| {
                let notifications = {
                    let mut trackers = source_lists.borrow_mut();
                    clone_source_list(&mut trackers[index], &changes);
                    update_result_list(&changes, &trackers, &mut result_list.borrow_mut(), kind)
                };
                if notifications.count() != 0 {
                    observer.on_next(notifications);
                }
            });
            disposable.add(subscription);
        }
        let disposable: Rc<dyn IDisposable> = disposable;
        disposable
    })
}

/// `CloneSourceList`: the counts of a list follow its changes.
pub(crate) fn clone_source_list<T: Clone + Eq + Hash>(tracker: &mut ReferenceCountTracker<T>, changes: &ChangeSet<T>) {
    for change in changes {
        match change.reason() {
            ListChangeReason::Add => {
                tracker.add(change.item().current());
            }
            ListChangeReason::AddRange => {
                for item in change.range().iter() {
                    tracker.add(item);
                }
            }
            ListChangeReason::Replace => {
                tracker.remove(change.item().previous().expect("a replacement has a previous item"));
                tracker.add(change.item().current());
            }
            ListChangeReason::Remove => {
                tracker.remove(change.item().current());
            }
            ListChangeReason::RemoveRange | ListChangeReason::Clear => {
                for item in change.range().iter() {
                    tracker.remove(item);
                }
            }
            ListChangeReason::Refresh | ListChangeReason::Moved => {}
        }
    }
}

/// `MatchesConstraint`.
fn matches_constraint<T: Clone + Eq + Hash>(source_lists: &[ReferenceCountTracker<T>], item: &T, kind: CombineOperator) -> bool {
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
fn update_item_membership<T: Clone + Eq + Hash>(item: &T, source_lists: &[ReferenceCountTracker<T>], result_list: &mut ChangeAwareList<T>, kind: CombineOperator) {
    let is_in_result = result_list.contains(item);
    let should_be_in_result = matches_constraint(source_lists, item, kind);
    if should_be_in_result && !is_in_result {
        result_list.add(item.clone());
    } else if !should_be_in_result && is_in_result {
        result_list.remove(item);
    }
}

/// `UpdateResultList`.
fn update_result_list<T: Clone + Eq + Hash>(changes: &ChangeSet<T>, source_lists: &[ReferenceCountTracker<T>], result_list: &mut ChangeAwareList<T>, kind: CombineOperator) -> ChangeSet<T> {
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
            ListChangeReason::AddRange | ListChangeReason::RemoveRange | ListChangeReason::Clear => panic!("Unsupported change type"),
        }
    }
    result_list.capture_changes()
}
