//! `MergeManyCacheChangeSets<TObject, TDestination, TDestinationKey>`
//! (`List/Internal/MergeManyCacheChangeSets.cs`): the change sets of a cache chosen for each item
//! of a list, merged into those of one cache.

use super::{merge_many, on_item_removed, transform_items};
use crate::cache::internal::{ChangeSetCache, ChangeSetMergeTracker, Comparer, EqualityComparer};
use crate::cache::IQuery;
use crate::internal::synchronize;
use crate::list::{clone_changes, ChangeSet};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

/// A child cache, equal only to itself in the list of the children (upstream's class).
#[derive(Clone)]
pub(crate) struct ChildCache<D, DK>(pub(crate) Rc<ChangeSetCache<D, DK>>);

impl<D, DK> PartialEq for ChildCache<D, DK> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// `MergeManyCacheChangeSets.Run`: the list of the child caches is shared, and copied; the changes
/// of the children go through the tracker, which emits them at once unless the parent is being
/// updated, and the removal of a parent removes its children, then emits.
pub(crate) fn merge_many_cache_change_sets<T: Clone + PartialEq + 'static, D: Clone + 'static, DK: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    change_set_selector: Rc<dyn Fn(&T) -> Rc<dyn IObservable<crate::cache::ChangeSet<D, DK>>>>,
    equality_comparer: EqualityComparer<D>,
    comparer: Option<Comparer<D>>,
) -> Rc<dyn IObservable<crate::cache::ChangeSet<D, DK>>> {
    Observable::create(move |observer: Rc<dyn IObserver<crate::cache::ChangeSet<D, DK>>>| {
        let list = Rc::new(RefCell::new(Vec::<ChildCache<D, DK>>::new()));
        let parent_update = Rc::new(Cell::new(false));
        // The tracker of all the changes.
        let caches = list.clone();
        let change_tracker = Rc::new(ChangeSetMergeTracker::new(
            move || caches.borrow().iter().map(|child| child.0.clone()).collect(),
            comparer.clone(),
            equality_comparer.clone(),
        ));
        // The list of the child caches, serialized, copied and shared.
        let (selector, target, update_flag) = (change_set_selector.clone(), list.clone(), parent_update.clone());
        let shared = synchronize(transform_items(source.clone(), Rc::new(move |item: &T| ChildCache(ChangeSetCache::new(synchronize(selector(item)))))))
            .do_(move |changes: &ChangeSet<ChildCache<D, DK>>| clone_changes(&mut *target.borrow_mut(), changes))
            .do_(move |_| update_flag.set(true))
            .publish();
        let shared_source: Rc<dyn IObservable<ChangeSet<ChildCache<D, DK>>>> = shared.clone();
        // The changes of the children, merged and applied to the tracker.
        let (tracker, flag, next_observer) = (change_tracker.clone(), parent_update.clone(), observer.clone());
        let (error_observer, completed_observer) = (observer.clone(), observer.clone());
        let sub_merge_many = merge_many(shared_source.clone(), Rc::new(|child: &ChildCache<D, DK>| child.0.source()))
            .subscribe_all(
                move |changes: crate::cache::ChangeSet<D, DK>| tracker.process_change_set(&changes, if flag.get() { None } else { Some(&next_observer) }),
                move |error| error_observer.on_error(error),
                move || completed_observer.on_completed(),
            );
        // The removal of a parent removes its children.
        let removal_tracker = change_tracker.clone();
        let (emit_tracker, emit_flag) = (change_tracker, parent_update);
        let sub_remove = on_item_removed(
            shared_source,
            Rc::new(move |child: &ChildCache<D, DK>| {
                let key_values = child.0.cache().key_values();
                removal_tracker.remove_items(&key_values, None);
            }),
            false,
        )
        .do_(move |_| {
            emit_tracker.emit_changes(&observer);
            emit_flag.set(false);
        })
        .subscribe_ignore();
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([shared.connect(), sub_merge_many, sub_remove]);
        composite
    })
}
