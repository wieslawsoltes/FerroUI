//! `AnonymousObserver<T>` and `Observer` (`AnonymousObserver.cs`, `Observer.Extensions.cs`), and
//! the observers of `Synchronize` (`Internal/SynchronizedObserver.cs`,
//! `Internal/AsyncLockObserver.cs`).

use crate::concurrency::AsyncLock;
use crate::notification::Notification;
use crate::observer_base::{ObserverBase, ObserverCore};
use ferroui_system::{IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

type OnError = Box<dyn Fn(ObservableError)>;

/// An observer made of functions (`AnonymousObserver<T>`). It ignores everything after a terminal
/// notification. Without a function for errors, an error panics, as upstream rethrows it.
pub struct AnonymousObserver<T> {
    on_next: Box<dyn Fn(T)>,
    on_error: OnError,
    on_completed: Box<dyn Fn()>,
    is_stopped: Cell<bool>,
}

impl<T> AnonymousObserver<T> {
    /// An observer of the three functions.
    pub fn new(
        on_next: impl Fn(T) + 'static,
        on_error: impl Fn(ObservableError) + 'static,
        on_completed: impl Fn() + 'static,
    ) -> Self {
        Self { on_next: Box::new(on_next), on_error: Box::new(on_error), on_completed: Box::new(on_completed), is_stopped: Cell::new(false) }
    }
}

/// The error of an observer that has no function for errors (`Stubs.Throw`): it panics with the
/// error, which is how an unhandled error of a sequence surfaces.
pub(crate) fn throw(error: ObservableError) {
    panic!("unhandled error of an observable sequence: {error}");
}

impl<T> IObserver<T> for AnonymousObserver<T> {
    fn on_next(&self, value: T) {
        if !self.is_stopped.get() {
            (self.on_next)(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        if !self.is_stopped.replace(true) {
            (self.on_error)(error);
        }
    }

    fn on_completed(&self) {
        if !self.is_stopped.replace(true) {
            (self.on_completed)();
        }
    }
}

/// The factory of observers (`Observer`).
pub struct Observer;

impl Observer {
    /// An observer of values; an error panics (`Create(Action<T>)`).
    pub fn create<T: 'static>(on_next: impl Fn(T) + 'static) -> Rc<dyn IObserver<T>> {
        Rc::new(AnonymousObserver::new(on_next, throw, || {}))
    }

    /// An observer of values and errors (`Create(Action<T>, Action<Exception>)`).
    pub fn create_with_error<T: 'static>(
        on_next: impl Fn(T) + 'static,
        on_error: impl Fn(ObservableError) + 'static,
    ) -> Rc<dyn IObserver<T>> {
        Rc::new(AnonymousObserver::new(on_next, on_error, || {}))
    }

    /// An observer of values and the end; an error panics (`Create(Action<T>, Action)`).
    pub fn create_with_completed<T: 'static>(on_next: impl Fn(T) + 'static, on_completed: impl Fn() + 'static) -> Rc<dyn IObserver<T>> {
        Rc::new(AnonymousObserver::new(on_next, throw, on_completed))
    }

    /// An observer of all three notifications (`Create(Action<T>, Action<Exception>, Action)`).
    pub fn create_all<T: 'static>(
        on_next: impl Fn(T) + 'static,
        on_error: impl Fn(ObservableError) + 'static,
        on_completed: impl Fn() + 'static,
    ) -> Rc<dyn IObserver<T>> {
        Rc::new(AnonymousObserver::new(on_next, on_error, on_completed))
    }

    /// An observer that hands each notification, as a value, to `handler` (`ToObserver(Action<Notification<T>>)`).
    pub fn to_observer<T: 'static>(handler: impl Fn(Notification<T>) + 'static) -> Rc<dyn IObserver<T>> {
        let handler = Rc::new(handler);
        let (on_error, on_completed) = (handler.clone(), handler.clone());
        Rc::new(AnonymousObserver::new(
            move |value| handler(Notification::OnNext(value)),
            move |error| on_error(Notification::OnError(error)),
            move || on_completed(Notification::OnCompleted),
        ))
    }
}

/// The extension methods of `Observer` on an observer (`Observer.Extensions.cs`).
pub trait ObserverExt<T> {
    /// The observer as a function of notifications (`ToNotifier`).
    fn to_notifier(&self) -> Box<dyn Fn(Notification<T>)>;

    /// An observer that hides the identity of this one (`AsObserver`).
    fn as_observer(&self) -> Rc<dyn IObserver<T>>;

    /// An observer that checks the grammar of the notifications it receives (`Checked`): it panics,
    /// where upstream throws `InvalidOperationException`, on a notification after a terminal one
    /// and on a notification while another is being handled.
    fn checked(&self) -> Rc<dyn IObserver<T>>;

    /// An observer whose notifications reach this one under a lock of its own
    /// (`Synchronize(IObserver<T>)`, a `SynchronizedObserver<T>`): it ignores everything after a
    /// terminal notification. On one thread the notifications are serialized already, and the lock
    /// of upstream (a monitor) is re-entrant: a notification that re-enters the observer from
    /// inside another reaches this one at once.
    fn synchronize(&self) -> Rc<dyn IObserver<T>>;

    /// An observer whose notifications reach this one one at a time
    /// (`Synchronize(IObserver<T>, bool preventReentrancy)`): with `prevent_reentrancy`, through an
    /// [`AsyncLock`] of its own, so that a notification that re-enters the observer from inside
    /// another is queued and reaches this one when that one has returned (`AsyncLockObserver<T>`);
    /// without, as [`synchronize`](Self::synchronize).
    fn synchronize_prevent_reentrancy(&self, prevent_reentrancy: bool) -> Rc<dyn IObserver<T>>;

    /// An observer whose notifications reach this one under the lock `gate`
    /// (`Synchronize(IObserver<T>, object gate)`), which observers may share; as
    /// [`synchronize`](Self::synchronize), the lock is re-entrant and has nothing to serialize on
    /// one thread. The observer keeps the gate.
    fn synchronize_with_gate(&self, gate: Rc<dyn std::any::Any>) -> Rc<dyn IObserver<T>>;

    /// An observer whose notifications reach this one through `async_lock`
    /// (`Synchronize(IObserver<T>, AsyncLock)`, an `AsyncLockObserver<T>`), which observers may
    /// share: a notification that arrives while an action of the lock runs (a re-entrant one) is
    /// queued and reaches this one after it.
    fn synchronize_with_lock(&self, async_lock: Rc<AsyncLock>) -> Rc<dyn IObserver<T>>;

    /// An observer that delivers the notifications to this one on `scheduler`, in order
    /// (`NotifyOn(IScheduler)`).
    fn notify_on(&self, scheduler: Rc<dyn crate::concurrency::IScheduler>) -> Rc<dyn IObserver<T>>;
}

impl<T: 'static> ObserverExt<T> for Rc<dyn IObserver<T>> {
    fn to_notifier(&self) -> Box<dyn Fn(Notification<T>)> {
        let observer = self.clone();
        Box::new(move |notification| notification.accept(&*observer))
    }

    fn as_observer(&self) -> Rc<dyn IObserver<T>> {
        let (next, error, completed) = (self.clone(), self.clone(), self.clone());
        Rc::new(AnonymousObserver::new(move |v| next.on_next(v), move |e| error.on_error(e), move || completed.on_completed()))
    }

    fn checked(&self) -> Rc<dyn IObserver<T>> {
        Rc::new(CheckedObserver { observer: self.clone(), state: Cell::new(CheckedState::Idle) })
    }

    fn synchronize(&self) -> Rc<dyn IObserver<T>> {
        synchronized(self.clone(), Rc::new(()))
    }

    fn synchronize_prevent_reentrancy(&self, prevent_reentrancy: bool) -> Rc<dyn IObserver<T>> {
        if prevent_reentrancy {
            return Rc::new(ObserverBase::new(AsyncLockCore { observer: self.clone(), gate: Rc::new(AsyncLock::new()) }));
        }
        synchronized(self.clone(), Rc::new(()))
    }

    fn synchronize_with_gate(&self, gate: Rc<dyn std::any::Any>) -> Rc<dyn IObserver<T>> {
        synchronized(self.clone(), gate)
    }

    fn synchronize_with_lock(&self, async_lock: Rc<AsyncLock>) -> Rc<dyn IObserver<T>> {
        Rc::new(ObserverBase::new(AsyncLockCore { observer: self.clone(), gate: async_lock }))
    }

    fn notify_on(&self, scheduler: Rc<dyn crate::concurrency::IScheduler>) -> Rc<dyn IObserver<T>> {
        Rc::new(NotifyOnObserver { inner: crate::internal::ScheduledObserver::new(scheduler, self.clone()) })
    }
}

/// `SynchronizedObserver<T>` on `observer` with the lock `gate`.
fn synchronized<T: 'static>(observer: Rc<dyn IObserver<T>>, gate: Rc<dyn std::any::Any>) -> Rc<dyn IObserver<T>> {
    Rc::new(ObserverBase::new(SynchronizedCore { observer, _gate: gate }))
}

/// The core of `SynchronizedObserver<T>` (an `ObserverBase<T>`): each notification reaches the
/// observer inside `lock (gate)`. A monitor is re-entrant and the notifications of one thread
/// cannot overlap, so the notification reaches it at once; the gate is kept, as upstream keeps it.
struct SynchronizedCore<T> {
    observer: Rc<dyn IObserver<T>>,
    _gate: Rc<dyn std::any::Any>,
}

impl<T> ObserverCore<T> for SynchronizedCore<T> {
    fn on_next_core(&self, value: T) {
        self.observer.on_next(value);
    }

    fn on_error_core(&self, error: ObservableError) {
        self.observer.on_error(error);
    }

    fn on_completed_core(&self) {
        self.observer.on_completed();
    }
}

/// The core of `AsyncLockObserver<T>` (an `ObserverBase<T>`): each notification reaches the
/// observer through `gate.Wait`, so that one that arrives while another runs is queued after it.
struct AsyncLockCore<T> {
    observer: Rc<dyn IObserver<T>>,
    gate: Rc<AsyncLock>,
}

impl<T: 'static> ObserverCore<T> for AsyncLockCore<T> {
    fn on_next_core(&self, value: T) {
        let observer = self.observer.clone();
        self.gate.wait(move || observer.on_next(value));
    }

    fn on_error_core(&self, error: ObservableError) {
        let observer = self.observer.clone();
        self.gate.wait(move || observer.on_error(error));
    }

    fn on_completed_core(&self) {
        let observer = self.observer.clone();
        self.gate.wait(move || observer.on_completed());
    }
}

/// The observer of `NotifyOn` (`ObserveOnObserver<T>` as an observer): each notification is
/// queued and the queue is delivered on the scheduler.
struct NotifyOnObserver<T: 'static> {
    inner: Rc<crate::internal::ScheduledObserver<T>>,
}

impl<T: 'static> IObserver<T> for NotifyOnObserver<T> {
    fn on_next(&self, value: T) {
        use crate::internal::IScheduledObserver;
        self.inner.on_next(value);
        self.inner.ensure_active();
    }

    fn on_error(&self, error: ObservableError) {
        use crate::internal::IScheduledObserver;
        self.inner.on_error(error);
        self.inner.ensure_active();
    }

    fn on_completed(&self) {
        use crate::internal::IScheduledObserver;
        self.inner.on_completed();
        self.inner.ensure_active();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CheckedState {
    Idle,
    Busy,
    Done,
}

/// `CheckedObserver<T>`.
struct CheckedObserver<T> {
    observer: Rc<dyn IObserver<T>>,
    state: Cell<CheckedState>,
}

impl<T> CheckedObserver<T> {
    /// Moves from idle to busy; any other state is left as it is and panics (`CheckAccess`, which
    /// only moves from `Idle`).
    fn check_access(&self) {
        match self.state.get() {
            CheckedState::Busy => panic!("This observer is currently busy with a notification."),
            CheckedState::Done => panic!("This observer has already terminated."),
            CheckedState::Idle => self.state.set(CheckedState::Busy),
        }
    }
}

impl<T> IObserver<T> for CheckedObserver<T> {
    fn on_next(&self, value: T) {
        self.check_access();
        self.observer.on_next(value);
        self.state.set(CheckedState::Idle);
    }

    fn on_error(&self, error: ObservableError) {
        self.check_access();
        self.observer.on_error(error);
        self.state.set(CheckedState::Done);
    }

    fn on_completed(&self) {
        self.check_access();
        self.observer.on_completed();
        self.state.set(CheckedState::Done);
    }
}
