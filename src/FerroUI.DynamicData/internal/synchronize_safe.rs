//! `SynchronizeSafeExtensions` (`Internal/SynchronizeSafeExtensions.cs`): the replacements of
//! `Synchronize(lock)` that deliver outside of the lock, through a delivery queue, and the merges
//! of sequences whose deliveries are serialized already (`UnsynchronizedMerge`,
//! `UnsynchronizedCombineLatest`).
//!
//! The order of disposal is upstream's:
//! - **Subscription first** (the forms with a shared queue or a caller's lock): disposing the
//!   subscription first lets a last terminal notification flow through the queue, which is
//!   disposed last.
//! - **Queue first** (the form without arguments, of operators with effects at teardown such as
//!   `DisposeMany` and `OnBeingRemoved`): the queue is terminated before the subscription is
//!   disposed, so that no notification is delivered during the teardown.
#![allow(dead_code)]

use super::shared_delivery_queue::SharedDeliveryQueue;
use super::DeliveryQueue;
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{Observable, Observer};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A [`DeliveryQueue`] as the observer of a source (`DeliveryQueue<T>` is an `IObserver<T>`
/// upstream): each notification is enqueued, and delivered when the hold ends.
pub(crate) struct DeliveryQueueObserver<T: 'static> {
    queue: Rc<DeliveryQueue<T>>,
}

impl<T: 'static> DeliveryQueueObserver<T> {
    pub(crate) fn new(queue: Rc<DeliveryQueue<T>>) -> Rc<Self> {
        Rc::new(Self { queue })
    }
}

impl<T: 'static> IObserver<T> for DeliveryQueueObserver<T> {
    /// `DeliveryQueue<T>.OnNext`.
    fn on_next(&self, value: T) {
        let scope = self.queue.acquire();
        scope.enqueue_next(value);
    }

    /// `DeliveryQueue<T>.OnError`.
    fn on_error(&self, error: ObservableError) {
        let scope = self.queue.acquire();
        scope.enqueue_error(error);
    }

    /// `DeliveryQueue<T>.OnCompleted`.
    fn on_completed(&self) {
        let scope = self.queue.acquire();
        scope.enqueue_completed();
    }
}

/// The disposal of a delivery queue, as a disposable (`DeliveryQueue<T>.Dispose`).
pub(crate) fn dispose_queue<T: 'static>(queue: &Rc<DeliveryQueue<T>>) -> Rc<dyn IDisposable> {
    let queue = queue.clone();
    Disposable::create(move || queue.dispose())
}

/// Routes `source` through a sub-queue of `queue`, for sources of different types that share one
/// queue (`SynchronizeSafe(IObservable<T>, SharedDeliveryQueue)`). The subscription is disposed
/// first.
pub(crate) fn synchronize_safe_shared<T: 'static>(source: Rc<dyn IObservable<T>>, queue: &Rc<SharedDeliveryQueue>) -> Rc<dyn IObservable<T>> {
    let queue = queue.clone();
    Observable::create(move |observer: Rc<dyn IObserver<T>>| {
        let sub_queue = queue.create_queue(observer);
        // Subscription first: terminal notifications flow through the sub-queue, still active.
        let subscription = source.subscribe(sub_queue.clone());
        let sub_queue: Rc<dyn IDisposable> = sub_queue;
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([subscription, sub_queue]);
        composite
    })
}

/// Routes `source` through a delivery queue of its own, the replacement of `Synchronize(locker)`
/// (`SynchronizeSafe(IObservable<T>, Lock)`). The subscription is disposed first.
///
/// Deviation (DEVIATIONS.md, DynamicData): a lock that several operators share serializes nothing on
/// one thread, so the caller's lock is not taken: each subscription has the queue of its own, which
/// delivers re-entrant notifications after the current one.
pub(crate) fn synchronize_safe_locked<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    Observable::create(move |observer: Rc<dyn IObserver<T>>| {
        let queue = DeliveryQueue::new(observer);
        // Subscription first: terminal notifications flow through the queue, still active.
        let subscription = source.subscribe(DeliveryQueueObserver::new(queue.clone()));
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([subscription, dispose_queue(&queue)]);
        composite
    })
}

/// Routes `source` through a delivery queue of its own, which is terminated before the
/// subscription is disposed (`SynchronizeSafe(IObservable<T>)`): no notification is delivered
/// while the operators downstream tear down.
pub(crate) fn synchronize_safe<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    Observable::create(move |observer: Rc<dyn IObserver<T>>| {
        let queue = DeliveryQueue::new(observer);
        let queue_disposal = dispose_queue(&queue);
        let subscription = source.subscribe(DeliveryQueueObserver::new(queue));
        // Queue first: the deliveries end before the effects of the teardown run.
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([queue_disposal, subscription]);
        composite
    })
}

/// Merges the sequences without a gate of its own (`UnsynchronizedMerge`): it ends when every
/// sequence has ended, the first error ends it, and the sequences are subscribed in order.
///
/// The deliveries of the sequences must be serialized already, by one
/// [`SharedDeliveryQueue`] through [`synchronize_safe_shared`]; on one thread that holds of any
/// sequences, so this is `Merge` without its gate.
pub(crate) fn unsynchronized_merge<T: 'static>(first: Rc<dyn IObservable<T>>, others: Vec<Rc<dyn IObservable<T>>>) -> Rc<dyn IObservable<T>> {
    Observable::create(move |observer: Rc<dyn IObserver<T>>| {
        let remaining_sources = Rc::new(Cell::new(others.len() + 1));
        let terminated = Rc::new(Cell::new(false));
        let subscriptions = CompositeDisposable::with_capacity(others.len() + 1);
        // Each sequence has an observer of its own, as each ends once.
        let create_inner = || {
            let (next_observer, error_observer, completed_observer) = (observer.clone(), observer.clone(), observer.clone());
            let (next_terminated, error_terminated, completed_terminated) = (terminated.clone(), terminated.clone(), terminated.clone());
            let remaining_sources = remaining_sources.clone();
            Observer::create_all(
                move |value: T| {
                    if !next_terminated.get() {
                        next_observer.on_next(value);
                    }
                },
                move |error| {
                    if !error_terminated.replace(true) {
                        error_observer.on_error(error);
                    }
                },
                move || {
                    remaining_sources.set(remaining_sources.get() - 1);
                    if remaining_sources.get() == 0 && !completed_terminated.replace(true) {
                        completed_observer.on_completed();
                    }
                },
            )
        };
        subscriptions.add(first.subscribe(create_inner()));
        for source in &others {
            subscriptions.add(source.subscribe(create_inner()));
        }
        let subscriptions: Rc<dyn IDisposable> = subscriptions;
        subscriptions
    })
}

/// `CombineLatest` of two sequences without a gate of its own (`UnsynchronizedCombineLatest`): the
/// latest value of each, combined by `result_selector` whenever either produces once both have;
/// the first error ends it, and it ends when both have ended. The deliveries of the two must be
/// serialized already, as for [`unsynchronized_merge`].
pub(crate) fn unsynchronized_combine_latest<A: Clone + 'static, B: Clone + 'static, R: 'static>(
    first: Rc<dyn IObservable<A>>,
    second: Rc<dyn IObservable<B>>,
    result_selector: impl Fn(A, B) -> R + 'static,
) -> Rc<dyn IObservable<R>> {
    let result_selector = Rc::new(result_selector);
    Observable::create(move |observer: Rc<dyn IObserver<R>>| {
        let first_latest: Rc<RefCell<Option<A>>> = Rc::new(RefCell::new(None));
        let second_latest: Rc<RefCell<Option<B>>> = Rc::new(RefCell::new(None));
        let remaining_sources = Rc::new(Cell::new(2usize));
        let terminated = Rc::new(Cell::new(false));
        let on_error = {
            let (observer, terminated) = (observer.clone(), terminated.clone());
            Rc::new(move |error: ObservableError| {
                if !terminated.replace(true) {
                    observer.on_error(error);
                }
            })
        };
        let on_completed = {
            let (observer, terminated, remaining_sources) = (observer.clone(), terminated.clone(), remaining_sources.clone());
            Rc::new(move || {
                remaining_sources.set(remaining_sources.get() - 1);
                if remaining_sources.get() == 0 && !terminated.replace(true) {
                    observer.on_completed();
                }
            })
        };
        let subscriptions = CompositeDisposable::with_capacity(2);
        {
            let (observer, terminated, first_latest, second_latest, result_selector) =
                (observer.clone(), terminated.clone(), first_latest.clone(), second_latest.clone(), result_selector.clone());
            let (on_error, on_completed) = (on_error.clone(), on_completed.clone());
            subscriptions.add(first.subscribe(Observer::create_all(
                move |value: A| {
                    if terminated.get() {
                        return;
                    }
                    *first_latest.borrow_mut() = Some(value.clone());
                    let other = second_latest.borrow().clone();
                    if let Some(other) = other {
                        observer.on_next(result_selector(value, other));
                    }
                },
                move |error| on_error(error),
                move || on_completed(),
            )));
        }
        {
            let (observer, terminated, result_selector) = (observer.clone(), terminated.clone(), result_selector.clone());
            subscriptions.add(second.subscribe(Observer::create_all(
                move |value: B| {
                    if terminated.get() {
                        return;
                    }
                    *second_latest.borrow_mut() = Some(value.clone());
                    let other = first_latest.borrow().clone();
                    if let Some(other) = other {
                        observer.on_next(result_selector(other, value));
                    }
                },
                move |error| on_error(error),
                move || on_completed(),
            )));
        }
        let subscriptions: Rc<dyn IDisposable> = subscriptions;
        subscriptions
    })
}
