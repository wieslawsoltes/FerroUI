//! `MergeChangeSets<TObject, TKey>` (`Cache/Internal/MergeChangeSets.cs`): the change sets of
//! several caches merged into those of one, with a value for each key chosen among the caches.

use super::{merge_many, Cache, ChangeSetCache, ChangeSetMergeTracker, Comparer, EqualityComparer};
use crate::cache::{Change, ChangeReason, ChangeSet, ICache, IQuery};
use crate::internal::{synchronize_safe_shared, SharedDeliveryQueue};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// `MergeChangeSets.Run` over a sequence of caches: each cache is subscribed as it arrives, and
/// every change set goes through the tracker, which emits the merged change set. The merge
/// completes when the sequence of caches and every cache have completed.
pub(crate) fn merge_change_sets<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<Rc<dyn IObservable<ChangeSet<T, K>>>>>,
    equality_comparer: EqualityComparer<T>,
    comparer: Option<Comparer<T>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let queue = SharedDeliveryQueue::new();
        let cache = Rc::new(RefCell::new(Cache::<Rc<ChangeSetCache<T, K>>, usize>::new()));
        // The tracker of all the changes.
        let caches = cache.clone();
        let change_tracker = ChangeSetMergeTracker::new(move || caches.borrow().items(), comparer.clone(), equality_comparer.clone());
        // A change set of caches, serialized, the copy of which is updated, and the sequences of
        // the caches merged.
        let target = cache.clone();
        let (error_observer, merged_error_observer, completed_observer) = (observer.clone(), observer.clone(), observer.clone());
        let containers = synchronize_safe_shared(create_container_observable(source.clone(), &queue), &queue)
            .do_(move |changes: &ChangeSet<Rc<ChangeSetCache<T, K>>, usize>| target.borrow_mut().clone_changes(changes));
        let subscription = merge_many(
            containers,
            Rc::new(move |container: &Rc<ChangeSetCache<T, K>>, _: &usize| {
                let error_observer = error_observer.clone();
                container.source().do_with_error(|_| {}, move |error| error_observer.on_error(error))
            }),
        )
        .subscribe_all(
            move |changes: ChangeSet<T, K>| change_tracker.process_change_set(&changes, Some(&observer)),
            move |error| merged_error_observer.on_error(error),
            move || completed_observer.on_completed(),
        );
        let queue: Rc<dyn IDisposable> = queue;
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([subscription, queue]);
        composite
    })
}

/// The sequence of the caches of an enumerable, which completes after them unless `completable`
/// is false (`CreateObservable`): enumerated on `scheduler`, or on the default of `ToObservable`.
pub(crate) fn sources_observable<S: Clone + 'static>(sources: Vec<S>, completable: bool, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<dyn IObservable<S>> {
    let observable = match scheduler {
        Some(scheduler) => Observable::to_observable_on(sources, scheduler),
        None => Observable::to_observable(sources),
    };
    if completable {
        observable
    } else {
        observable.concat(Observable::never())
    }
}

/// A change set with one addition for each new cache (`CreateContainerObservable`), keyed by its
/// position; the cache is serialized by the queue (`CreateChange`).
fn create_container_observable<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<Rc<dyn IObservable<ChangeSet<T, K>>>>>,
    queue: &Rc<SharedDeliveryQueue>,
) -> Rc<dyn IObservable<ChangeSet<Rc<ChangeSetCache<T, K>>, usize>>> {
    let queue = queue.clone();
    source.select_with_index(move |source: Rc<dyn IObservable<ChangeSet<T, K>>>, index| {
        // Deviation (DEVIATIONS.md, DynamicData): `IgnoreSameReferenceUpdate` is left out; a
        // generic item has no identity to compare, and an update to the same value changes
        // nothing in the tracker.
        let container = ChangeSetCache::new(synchronize_safe_shared(source, &queue));
        ChangeSet::from(vec![Change::new(ChangeReason::Add, index, container)])
    })
}

