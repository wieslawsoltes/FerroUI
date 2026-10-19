//! `MergeMany<TObject, TKey, TDestination>` (`Cache/Internal/MergeMany.cs`): the values of a
//! sequence chosen for each item, merged, for as long as the item is in the cache.

use super::subscribe_many;
use crate::cache::ChangeSet;
use crate::internal::{dispose_queue, DeliveryQueue, DeliveryQueueObserver};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::hash::Hash;
use std::rc::Rc;

/// The selector of the sequence of an item (`Func<TObject, TKey, IObservable<TDestination>>`).
pub(crate) type ObservableSelector<T, K, D> = Rc<dyn Fn(&T, &K) -> Rc<dyn IObservable<D>>>;

/// The count of the source and the sequences not ended, and the queue that delivers the merge.
struct Completion<D: 'static> {
    counter: Cell<isize>,
    queue: Rc<DeliveryQueue<D>>,
    queue_observer: Rc<DeliveryQueueObserver<D>>,
}

impl<D: 'static> Completion<D> {
    /// `CheckCompleted`: the last of the source and the sequences to end completes the merge.
    fn check_completed(&self) {
        self.counter.set(self.counter.get() - 1);
        if self.counter.get() == 0 && !self.queue.is_terminated() {
            self.queue_observer.on_completed();
        }
    }
}

/// `MergeMany.Run`: each sequence is subscribed while its item is in the cache; the values are
/// delivered through a queue. The merge completes when the source and every sequence have
/// completed; an error of the source ends it, those of the sequences are ignored.
pub(crate) fn merge_many<T: Clone + 'static, K: Eq + Hash + Clone + 'static, D: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    observable_selector: ObservableSelector<T, K, D>,
) -> Rc<dyn IObservable<D>> {
    Observable::create(move |observer: Rc<dyn IObserver<D>>| {
        let queue = DeliveryQueue::new(observer.clone());
        let completion = Rc::new(Completion { counter: Cell::new(1), queue: queue.clone(), queue_observer: DeliveryQueueObserver::new(queue.clone()) });
        let source_completion = completion.clone();
        let observable_selector = observable_selector.clone();
        let error_observer = observer.clone();
        let subscription = subscribe_many(
            source.do_with_completed(|_| {}, move || source_completion.check_completed()).concat(Observable::never()),
            Rc::new(move |item: &T, key: &K| {
                completion.counter.set(completion.counter.get() + 1);
                let finally_completion = completion.clone();
                let next_observer = completion.queue_observer.clone();
                observable_selector(item, key)
                    .finally(move || finally_completion.check_completed())
                    .subscribe_next_error(move |value| next_observer.on_next(value), |_| {})
            }),
        )
        .subscribe_next_error(|_| {}, move |error| error_observer.on_error(error));
        // Queue first: terminated before the subscriptions are disposed, so that the `Finally` of
        // the teardown delivers no completion.
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([dispose_queue(&queue), subscription]);
        composite
    })
}
