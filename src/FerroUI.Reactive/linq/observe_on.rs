//! `ObserveOn` (`Concurrency/Synchronization.ObserveOn.cs`, `Internal/ScheduledObserver.cs`; the
//! operator of `Linq/Observable.Concurrency.cs`).
//!
//! The notifications of the source are queued and delivered on the scheduler, one value per
//! scheduled step, in the order they arrived, as `ObserveOnObserver<T>` does with the scheduled
//! observer it is built on. The port has no long-running schedulers, so the form of upstream for
//! `ISchedulerLongRunning` does not apply. `ObserveOn(SynchronizationContext)` is not ported yet
//! (phase 4).
//!
//! Deviation (DEVIATIONS.md, Reactive): the producer of upstream for a scheduler is the lock-free
//! `ObserveOnObserverNew<T>`; the port uses the scheduled observer of `ObserveOnObserver<T>`, which
//! needs no work-in-progress counter on one thread. The values are delivered at the same times;
//! the differences are that an error is delivered after the values queued before it (the lock-free
//! form drops them and delivers the error at once), and that the delivery ends with one scheduled
//! step that finds the queue empty.

use crate::concurrency::IScheduler;
use crate::internal::{subscribe_sink, IScheduledObserver, OperatorSink, ScheduledObserver, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// The sink of `ObserveOn` (`ObserveOnObserver<T>`): it hands the notifications of the source to a
/// [`ScheduledObserver`], which delivers them on the scheduler to the observer downstream.
struct ObserveOnSink<T> {
    sink: Sink<T>,
    scheduled: Rc<ScheduledObserver<T>>,
    deliver: Rc<Deliver<T>>,
}

impl<T: 'static> OperatorSink<T, T> for ObserveOnSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static> IDisposable for ObserveOnSink<T> {
    /// Disposes the subscription to the source and stops the delivery (`ObserveOnObserver.Dispose`).
    fn dispose(&self) {
        self.sink.dispose();
        self.scheduled.dispose();
        // The delivery no longer reaches the sink, which frees it.
        self.deliver.parent.borrow_mut().take();
    }
}

impl<T: 'static> IObserver<T> for ObserveOnSink<T> {
    fn on_next(&self, value: T) {
        self.scheduled.on_next(value);
        self.scheduled.ensure_active();
    }

    fn on_error(&self, error: ObservableError) {
        self.scheduled.on_error(error);
        self.scheduled.ensure_active();
    }

    fn on_completed(&self) {
        self.scheduled.on_completed();
        self.scheduled.ensure_active();
    }
}

/// What the scheduled observer delivers to: the observer downstream, through the sink, so that a
/// terminal notification disposes the subscription to the source. It keeps the sink alive while a
/// delivery is pending, as the closure of upstream does, until the sink is disposed.
struct Deliver<T> {
    parent: RefCell<Option<Rc<ObserveOnSink<T>>>>,
}

impl<T: 'static> Deliver<T> {
    fn parent(&self) -> Option<Rc<ObserveOnSink<T>>> {
        self.parent.borrow().clone()
    }
}

impl<T: 'static> IObserver<T> for Deliver<T> {
    fn on_next(&self, value: T) {
        if let Some(parent) = self.parent() {
            parent.sink.forward_on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        if let Some(parent) = self.parent() {
            parent.sink.forward_on_error(error);
            parent.dispose();
        }
    }

    fn on_completed(&self) {
        if let Some(parent) = self.parent() {
            parent.sink.forward_on_completed();
            parent.dispose();
        }
    }
}

/// The observable of `ObserveOn` (`ObserveOn<TSource>.Scheduler`).
struct ObserveOnObservable<T> {
    source: Rc<dyn IObservable<T>>,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for ObserveOnObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            move |observer| {
                let deliver = Rc::new(Deliver { parent: RefCell::new(None) });
                let target: Rc<dyn IObserver<T>> = deliver.clone();
                let sink = Rc::new(ObserveOnSink { sink: Sink::new(observer), scheduled: ScheduledObserver::new(scheduler, target), deliver });
                *sink.deliver.parent.borrow_mut() = Some(sink.clone());
                sink
            },
            move |sink: &Rc<ObserveOnSink<T>>| {
                let upstream = source.subscribe(sink.clone());
                sink.sink.set_upstream(upstream);
            },
        )
    }
}

pub(crate) fn observe_on<T: 'static>(source: Rc<dyn IObservable<T>>, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(ObserveOnObservable { source, scheduler })
}

/// The methods of `ObserveOn` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! observe_on_methods {
    () => {
        /// Delivers the notifications of the sequence on `scheduler`, in order
        /// (`ObserveOn(IScheduler)`).
        fn observe_on(
            &self,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::observe_on::observe_on(self.observable(), scheduler)
        }
    };
}
pub(crate) use observe_on_methods;
