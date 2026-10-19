//! `ObserveOn` (`Concurrency/Synchronization.ObserveOn.cs`, `ObserveOn<TSource>.Scheduler`, and its
//! sink `ObserveOnObserverNew<T>` of `Internal/ScheduledObserver.cs`).
//!
//! The values of the source are queued and delivered on the scheduler, one per scheduled step,
//! each step scheduling the next while work is pending (the "work in progress" count of upstream).
//! An error ends the delivery at the next step, dropping the values still queued; the end of the
//! sequence is delivered once the queue is empty.
//!
//! `ObserveOn(SynchronizationContext)` (`ObserveOn<TSource>.Context`) posts each notification to
//! the context instead, and tells the context that an operation runs from the subscription to its
//! disposal (`OperationStarted`, `OperationCompleted`).
//!
//! Deviation (DEVIATIONS.md, Reactive): the notifications are not `Send`, so they are posted with
//! [`SynchronizationContext::post_local`](ferroui_system::threading::SynchronizationContext::post_local):
//! the source notifies on the thread of the context, and each notification reaches the observer
//! when the context runs the work it was posted, later on that thread. Upstream posts from the
//! thread of the source, which may be any thread.

use crate::concurrency::IScheduler;
use crate::disposables::{Disposable, MultipleAssignmentDisposableValue, SingleAssignmentDisposable};
use crate::internal::{subscribe_sink, OperatorSink, Sink};
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::sync::Arc;

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

/// The sink of `ObserveOn(SynchronizationContext)` (`ObserveOn<TSource>.Context._`, an
/// `IdentitySink<TSource>`).
struct ObserveOnContextSink<T> {
    this: Weak<ObserveOnContextSink<T>>,
    sink: Sink<T>,
    context: Arc<dyn SynchronizationContext>,
    /// Whether the operation of the subscription has been completed (`Dispose(bool)` runs once).
    operation_completed: Cell<bool>,
}

impl<T: 'static> ObserveOnContextSink<T> {
    fn this(&self) -> Rc<Self> {
        self.this.upgrade().expect("the sink is alive while it is notified")
    }

    /// `Run`: the operation of the subscription starts, then the source is subscribed to.
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        self.context.operation_started();
        let upstream = source.subscribe(self.clone());
        self.sink.set_upstream(upstream);
    }
}

impl<T: 'static> OperatorSink<T, T> for ObserveOnContextSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static> IObserver<T> for ObserveOnContextSink<T> {
    fn on_next(&self, value: T) {
        let this = self.this();
        self.context.post_local(Box::new(move || this.sink.forward_on_next(value)));
    }

    fn on_error(&self, error: ObservableError) {
        let this = self.this();
        self.context.post_local(Box::new(move || {
            this.sink.forward_on_error(error);
            this.dispose();
        }));
    }

    fn on_completed(&self) {
        let this = self.this();
        self.context.post_local(Box::new(move || {
            this.sink.forward_on_completed();
            this.dispose();
        }));
    }
}

impl<T: 'static> IDisposable for ObserveOnContextSink<T> {
    /// `Dispose(bool)`: the operation of the subscription completes, then the sink is disposed.
    fn dispose(&self) {
        if !self.operation_completed.replace(true) {
            self.context.operation_completed();
        }
        self.sink.dispose();
    }
}

/// The observable of `ObserveOn(SynchronizationContext)` (`ObserveOn<TSource>.Context`).
struct ObserveOnContextObservable<T> {
    source: Rc<dyn IObservable<T>>,
    context: Arc<dyn SynchronizationContext>,
}

impl<T: 'static> IObservable<T> for ObserveOnContextObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let context = self.context.clone();
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new_cyclic(|this| ObserveOnContextSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    context,
                    operation_completed: Cell::new(false),
                })
            },
            move |sink: &Rc<ObserveOnContextSink<T>>| sink.run(&source),
        )
    }
}

pub(crate) fn observe_on_context<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    context: Arc<dyn SynchronizationContext>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(ObserveOnContextObservable { source, context })
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

        /// Delivers the notifications of the sequence through `context`, each posted to it in order
        /// (`ObserveOn(SynchronizationContext)`). The sequence notifies on the thread of the context.
        fn observe_on_context(
            &self,
            context: std::sync::Arc<dyn ferroui_system::threading::SynchronizationContext>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::observe_on::observe_on_context(self.observable(), context)
        }
    };
}
pub(crate) use observe_on_methods;
