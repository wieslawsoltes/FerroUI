//! `MergeChangeSets<TObject>` (`List/Internal/MergeChangeSets.cs`): the change sets of several
//! lists merged into those of one.

use super::{merge_many, remove_index, ChangeSetMergeTracker, ClonedListChangeSet};
use crate::internal::synchronize;
use crate::list::{Change, ChangeSet, ListChangeReason};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::rc::Rc;

/// `MergeChangeSets.Run` over a sequence of lists: each list is subscribed as it arrives, without
/// its indexes, and every change set goes through the tracker, which emits the merged change set.
pub(crate) fn merge_change_sets<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<Rc<dyn IObservable<ChangeSet<T>>>>>,
    equality_comparer: Rc<dyn Fn(&T, &T) -> bool>,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        // The tracker of all the changes.
        let change_tracker = ChangeSetMergeTracker::<T>::new();
        let (error_observer, merged_error_observer, completed_observer) = (observer.clone(), observer.clone(), observer.clone());
        // The change set streams merged together, and processed by the tracker, which emits.
        merge_many(
            synchronize(create_cloned_list_observable(source.clone(), equality_comparer.clone())),
            Rc::new(move |cloned_list: &ClonedListChangeSet<T>| {
                let error_observer = error_observer.clone();
                remove_index(cloned_list.source().clone()).do_with_error(|_| {}, move |error| error_observer.on_error(error))
            }),
        )
        .subscribe_all(
            move |changes: ChangeSet<T>| change_tracker.process_change_set(&changes, Some(&observer)),
            move |error| merged_error_observer.on_error(error),
            move || completed_observer.on_completed(),
        )
    })
}

/// A change set with one addition for each new list (`CreateClonedListObservable`, `CreateChange`).
fn create_cloned_list_observable<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<Rc<dyn IObservable<ChangeSet<T>>>>>,
    equality_comparer: Rc<dyn Fn(&T, &T) -> bool>,
) -> Rc<dyn IObservable<ChangeSet<ClonedListChangeSet<T>>>> {
    source.select(move |source: Rc<dyn IObservable<ChangeSet<T>>>| {
        let cloned = ClonedListChangeSet::new(synchronize(source), Some(equality_comparer.clone()));
        ChangeSet::from(vec![Change::new(ListChangeReason::Add, cloned, None)])
    })
}
