//! `MergeManyListChangeSets<TObject, TDestination>` (`List/Internal/MergeManyListChangeSets.cs`):
//! the change sets of a list chosen for each item of the source list, merged into those of one.

use super::{merge_many, on_item_removed, remove_index, transform_items, ChangeSetMergeTracker, ClonedListChangeSet};
use crate::internal::synchronize;
use crate::list::ChangeSet;
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::rc::Rc;

/// `MergeManyListChangeSets.Run`: the list of the child lists is shared; the changes of the
/// children go through the tracker, which emits them at once unless the parent is being updated,
/// and the removal of a parent removes its children, then emits.
pub(crate) fn merge_many_list_change_sets<T: Clone + PartialEq + 'static, D: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    selector: Rc<dyn Fn(&T) -> Rc<dyn IObservable<ChangeSet<D>>>>,
    equality_comparer: Rc<dyn Fn(&D, &D) -> bool>,
) -> Rc<dyn IObservable<ChangeSet<D>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D>>>| {
        let parent_update = Rc::new(Cell::new(false));
        // The tracker of all the changes.
        let change_tracker = Rc::new(ChangeSetMergeTracker::<D>::new());
        // The change sets of the child lists, serialized and shared.
        let (selector, equality_comparer, update_flag) = (selector.clone(), equality_comparer.clone(), parent_update.clone());
        let shared = synchronize(transform_items(source.clone(), Rc::new(move |item: &T| ClonedListChangeSet::new(synchronize(selector(item)), Some(equality_comparer.clone())))))
            .do_(move |_| update_flag.set(true))
            .publish();
        let shared_source: Rc<dyn IObservable<ChangeSet<ClonedListChangeSet<D>>>> = shared.clone();
        // The changes of the children, merged and applied to the tracker.
        let (tracker, flag, next_observer) = (change_tracker.clone(), parent_update.clone(), observer.clone());
        let (error_observer, completed_observer) = (observer.clone(), observer.clone());
        let sub_merge_many = merge_many(shared_source.clone(), Rc::new(|cloned_list: &ClonedListChangeSet<D>| remove_index(cloned_list.source().clone())))
            .subscribe_all(
                move |changes: ChangeSet<D>| tracker.process_change_set(&changes, if flag.get() { None } else { Some(&next_observer) }),
                move |error| error_observer.on_error(error),
                move || completed_observer.on_completed(),
            );
        // The removal of a parent removes its children.
        let removal_tracker = change_tracker.clone();
        let (emit_tracker, emit_flag) = (change_tracker, parent_update);
        let sub_remove = on_item_removed(shared_source, Rc::new(move |cloned_list: &ClonedListChangeSet<D>| removal_tracker.remove_items(&cloned_list.list(), None)), false)
            .do_(move |_| {
                emit_tracker.emit_changes(&observer);
                emit_flag.set(false);
            })
            .subscribe_ignore();
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([shared.connect(), sub_merge_many, sub_remove]);
        composite
    })
}
