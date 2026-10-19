//! `DeliveryQueueMergeExtensions` (`Internal/DeliveryQueueMergeExtensions.cs`): a merge of sequences
//! of one type that owns a [`DeliveryQueue`], which serializes the notifications of every input and
//! delivers them outside of its hold.
#![allow(dead_code)]

use super::synchronize_safe::{dispose_queue, DeliveryQueueObserver};
use super::DeliveryQueue;
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, Observer};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::rc::Rc;

/// Merges `first` and `others` through a delivery queue (`DeliveryQueueMerge`): as
/// `Observable.Merge`, it ends after every input has ended, the first error ends it, and the
/// inputs are subscribed in order. A notification that an observer raises during a delivery is
/// delivered after the current one.
pub(crate) fn delivery_queue_merge<T: 'static>(first: Rc<dyn IObservable<T>>, others: Vec<Rc<dyn IObservable<T>>>) -> Rc<dyn IObservable<T>> {
    Observable::create(move |observer: Rc<dyn IObserver<T>>| {
        let queue = DeliveryQueue::new(observer);
        let queue_observer = DeliveryQueueObserver::new(queue.clone());
        let remaining_sources = Rc::new(Cell::new(others.len() + 1));
        let subscriptions = CompositeDisposable::with_capacity(others.len() + 2);
        // Each input has an observer of its own, as each ends once; only the completion of the
        // last input ends the merge.
        let create_inner = || {
            let (next_target, error_target, completed_target) = (queue_observer.clone(), queue_observer.clone(), queue_observer.clone());
            let remaining_sources = remaining_sources.clone();
            Observer::create_all(
                move |value: T| next_target.on_next(value),
                move |error| error_target.on_error(error),
                move || {
                    remaining_sources.set(remaining_sources.get() - 1);
                    if remaining_sources.get() == 0 {
                        completed_target.on_completed();
                    }
                },
            )
        };
        subscriptions.add(first.subscribe(create_inner()));
        for source in &others {
            subscriptions.add(source.subscribe(create_inner()));
        }
        // The subscriptions first, so that a terminal notification of their disposal still flows
        // through the queue; the queue last, as cleanup.
        subscriptions.add(dispose_queue(&queue));
        let subscriptions: Rc<dyn IDisposable> = subscriptions;
        subscriptions
    })
}
