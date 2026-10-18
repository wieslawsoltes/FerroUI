//! `ObserveOn` (`Concurrency/Synchronization.ObserveOn.cs`, `ObserveOn<TSource>.Scheduler`, and its
//! sink `ObserveOnObserverNew<T>` of `Internal/ScheduledObserver.cs`).
//!
//! The values of the source are queued and delivered on the scheduler, one per scheduled step,
//! each step scheduling the next while work is pending (the "work in progress" count of upstream).
//! An error ends the delivery at the next step, dropping the values still queued; the end of the
//! sequence is delivered once the queue is empty.

use crate::concurrency::IScheduler;
use crate::disposables::{Disposable, MultipleAssignmentDisposableValue, SingleAssignmentDisposable};
use crate::internal::{subscribe_sink, OperatorSink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// The sink of `ObserveOn` (`ObserveOnObserverNew<T>`).
struct ObserveOnSink<T> {
    this: Weak<ObserveOnSink<T>>,
    sink: Sink<T>,
    scheduler: Rc<dyn IScheduler>,
    queue: RefCell<VecDeque<T>>,
    /// The scheduled drain step (`_task`).
    task: MultipleAssignmentDisposableValue,
    /// The number of signals not yet drained (`_wip`).
    wip: Cell<usize>,
    done: Cell<bool>,
    error: RefCell<Option<ObservableError>>,
    disposed: Cell<bool>,
}

impl<T: 'static> ObserveOnSink<T> {
    fn schedule(&self) {
        let wip = self.wip.get() + 1;
        self.wip.set(wip);
        if wip == 1 {
            let new_task = SingleAssignmentDisposable::new();
            if !self.task.is_disposed() {
                self.task.set_disposable(Some(new_task.clone()));
                let this = self.this.upgrade().expect("the sink is alive while it is notified");
                new_task.set_disposable(Some(self.scheduler.schedule(Box::new(move |scheduler| this.drain_short_running(scheduler)))));
            }
            // A cancellation clears the queue (`if (Volatile.Read(ref _disposed)) Clear(_queue)`).
            if self.disposed.get() {
                self.queue.borrow_mut().clear();
            }
        }
    }

    /// One step of the delivery, and the next step while work is pending (`DrainShortRunning`).
    fn drain_short_running(self: Rc<Self>, recursive_scheduler: &Rc<dyn IScheduler>) -> Rc<dyn IDisposable> {
        self.drain_step();
        let wip = self.wip.get() - 1;
        self.wip.set(wip);
        if wip != 0 {
            // The disposable of the next step replaces this one instead of being returned, so
            // that a long delivery does not chain its steps for disposal.
            let this = self.clone();
            let next = recursive_scheduler.schedule(Box::new(move |scheduler| this.drain_short_running(scheduler)));
            if !self.task.is_disposed() {
                self.task.set_disposable(Some(next));
            } else {
                next.dispose();
            }
        }
        Disposable::empty()
    }

    /// Delivers one signal (`DrainStep`).
    fn drain_step(&self) {
        if self.disposed.get() {
            self.queue.borrow_mut().clear();
            return;
        }
        let done = self.done.get();
        if done {
            let error = self.error.borrow().clone();
            if let Some(error) = error {
                self.disposed.set(true);
                self.sink.forward_on_error(error);
                return;
            }
        }
        let next = self.queue.borrow_mut().pop_front();
        if let Some(value) = next {
            self.sink.forward_on_next(value);
            return;
        }
        if done {
            self.disposed.set(true);
            self.sink.forward_on_completed();
        }
    }
}

impl<T: 'static> OperatorSink<T, T> for ObserveOnSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static> IObserver<T> for ObserveOnSink<T> {
    fn on_next(&self, value: T) {
        self.queue.borrow_mut().push_back(value);
        self.schedule();
    }

    fn on_error(&self, error: ObservableError) {
        *self.error.borrow_mut() = Some(error);
        self.done.set(true);
        self.schedule();
    }

    fn on_completed(&self) {
        self.done.set(true);
        self.schedule();
    }
}

impl<T: 'static> IDisposable for ObserveOnSink<T> {
    fn dispose(&self) {
        self.disposed.set(true);
        self.sink.dispose();
        self.task.dispose();
        self.queue.borrow_mut().clear();
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
                Rc::new_cyclic(|this| ObserveOnSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    scheduler,
                    queue: RefCell::new(VecDeque::new()),
                    task: MultipleAssignmentDisposableValue::new(),
                    wip: Cell::new(0),
                    done: Cell::new(false),
                    error: RefCell::new(None),
                    disposed: Cell::new(false),
                })
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
