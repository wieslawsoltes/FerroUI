//! `ViewThreadObservable<T>`.

use super::NotificationKind;
use crate::binding::bindings::BindingSchedulers;
use crate::binding::interfaces::IViewThreadInvoker;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Where a [`ViewThreadObservable`] delivers.
#[derive(Clone)]
enum Delivery {
    /// On the thread that owns `target`, through `invoker`.
    Invoker { target: Rc<dyn Any>, invoker: Rc<dyn IViewThreadInvoker> },
    /// On a scheduler, never at once.
    Sequencer(Rc<dyn IScheduler>),
}

/// Delivers each notification on the thread that owns the object a binding writes to, or on a
/// scheduler (`ViewThreadObservable<T>`).
///
/// A notification raised on the owning thread, while nothing waits, is delivered at once. Otherwise
/// it waits for the owning thread, and both forms deliver only the latest value that waits: a
/// two-way binding that names a scheduler writes each side once with the newest value, rather than
/// replaying every value the other side raised in between. A delivery on a scheduler never runs at
/// once.
pub struct ViewThreadObservable<T: 'static> {
    source: Rc<dyn IObservable<T>>,
    delivery: Delivery,
}

impl<T: 'static> ViewThreadObservable<T> {
    /// The stage that delivers on the thread that owns `target`, through `invoker`
    /// (`ViewThreadObservable(IObservable<T>, object, IViewThreadInvoker)`).
    pub fn new(source: Rc<dyn IObservable<T>>, target: Rc<dyn Any>, invoker: Rc<dyn IViewThreadInvoker>) -> Self {
        Self { source, delivery: Delivery::Invoker { target, invoker } }
    }

    /// The stage that delivers on `sequencer` (`ViewThreadObservable(IObservable<T>, ISequencer)`).
    pub fn on_sequencer(source: Rc<dyn IObservable<T>>, sequencer: Rc<dyn IScheduler>) -> Self {
        Self { source, delivery: Delivery::Sequencer(sequencer) }
    }
}

impl<T: 'static> IObservable<T> for ViewThreadObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new_cyclic(|this| Sink {
            this: this.clone(),
            observer,
            delivery: self.delivery.clone(),
            upstream: RefCell::new(None),
            waiting: RefCell::new(Waiting { value: None, error: None, completed: false }),
            scheduled: Cell::new(false),
            disposed: Cell::new(false),
        });
        let upstream = self.source.subscribe(sink.clone());
        sink.attach(upstream);
        sink
    }
}

/// What waits for the owning thread.
struct Waiting<T> {
    value: Option<T>,
    error: Option<ObservableError>,
    completed: bool,
}

/// The subscription that delivers at once on the owning thread, and holds the latest value for it
/// otherwise.
struct Sink<T: 'static> {
    this: Weak<Sink<T>>,
    observer: Rc<dyn IObserver<T>>,
    delivery: Delivery,
    /// The subscription to the source, `None` once disposed.
    upstream: RefCell<Option<Rc<dyn IDisposable>>>,
    waiting: RefCell<Waiting<T>>,
    /// Whether a drain is scheduled or running.
    scheduled: Cell<bool>,
    disposed: Cell<bool>,
}

impl<T: 'static> Sink<T> {
    /// Takes the subscription to the source; disposes it at once when the sink was disposed while
    /// it subscribed (`Attach`).
    fn attach(&self, upstream: Rc<dyn IDisposable>) {
        if self.disposed.get() {
            upstream.dispose();
            return;
        }
        *self.upstream.borrow_mut() = Some(upstream);
    }

    /// Whether the caller delivers the notification now; otherwise holds it for the owning thread
    /// and schedules the drain if none is (`Admit`).
    fn admit(&self, kind: NotificationKind, value: Option<T>, error: Option<ObservableError>) -> Option<(Option<T>, Option<ObservableError>)> {
        if self.disposed.get() {
            return None;
        }
        if !self.scheduled.get() {
            if let Delivery::Invoker { target, invoker } = &self.delivery {
                if invoker.check_access(target) {
                    return Some((value, error));
                }
            }
        }
        self.hold(kind, value, error);
        if !self.scheduled.replace(true) {
            self.schedule_drain();
        }
        None
    }

    /// Holds a notification until the drain runs; a newer value replaces a waiting one, so that an
    /// echo of an earlier write never writes an old value back (`Hold`).
    fn hold(&self, kind: NotificationKind, value: Option<T>, error: Option<ObservableError>) {
        let mut waiting = self.waiting.borrow_mut();
        match kind {
            NotificationKind::Next => waiting.value = value,
            NotificationKind::Error => waiting.error = error,
            NotificationKind::Completed => waiting.completed = true,
        }
    }

    /// Schedules the drain on the scheduler of the stage, on the main thread the host set, or
    /// through the invoker (`ScheduleDrain`).
    fn schedule_drain(&self) {
        let Some(sink) = self.this.upgrade() else {
            return;
        };
        match &self.delivery {
            Delivery::Sequencer(sequencer) => Self::schedule_on(sequencer, sink),
            Delivery::Invoker { target, invoker } => match BindingSchedulers::main_thread() {
                Some(main_thread) => Self::schedule_on(&main_thread, sink),
                None => invoker.post(target, Box::new(move || sink.drain())),
            },
        }
    }

    /// Schedules the drain of `sink` on `scheduler`.
    fn schedule_on(scheduler: &Rc<dyn IScheduler>, sink: Rc<Sink<T>>) {
        let _ = scheduler.schedule(Box::new(move |_| {
            sink.drain();
            Disposable::empty()
        }));
    }

    /// Delivers what waits until nothing is left, including what is held while a write runs
    /// (`Drain`).
    fn drain(&self) {
        loop {
            let (value, error, completed) = {
                let mut waiting = self.waiting.borrow_mut();
                if waiting.value.is_none() && waiting.error.is_none() && !waiting.completed {
                    self.scheduled.set(false);
                    return;
                }
                (waiting.value.take(), waiting.error.take(), std::mem::replace(&mut waiting.completed, false))
            };
            if self.disposed.get() {
                continue;
            }
            self.deliver(value, error, completed);
        }
    }

    /// Hands the waiting value, then a terminal notification, to the observer (`Deliver`).
    fn deliver(&self, value: Option<T>, error: Option<ObservableError>, completed: bool) {
        if let Some(value) = value {
            self.observer.on_next(value);
        }
        if let Some(error) = error {
            self.observer.on_error(error);
        } else if completed {
            self.observer.on_completed();
        }
    }
}

impl<T: 'static> IObserver<T> for Sink<T> {
    fn on_next(&self, value: T) {
        if let Some((Some(value), _)) = self.admit(NotificationKind::Next, Some(value), None) {
            self.observer.on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        if let Some((_, Some(error))) = self.admit(NotificationKind::Error, None, Some(error)) {
            self.observer.on_error(error);
        }
    }

    fn on_completed(&self) {
        if self.admit(NotificationKind::Completed, None, None).is_some() {
            self.observer.on_completed();
        }
    }
}

impl<T: 'static> IDisposable for Sink<T> {
    /// Disposes the subscription to the source on the first call; later calls do nothing.
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let upstream = self.upstream.borrow_mut().take();
        if let Some(upstream) = upstream {
            upstream.dispose();
        }
    }
}
