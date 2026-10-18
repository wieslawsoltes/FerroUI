//! `AnonymousObserver<T>` and `Observer` (`AnonymousObserver.cs`, `Observer.Extensions.cs`).

use crate::notification::Notification;
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
    fn check_access(&self) {
        match self.state.replace(CheckedState::Busy) {
            CheckedState::Busy => panic!("This observer is currently busy with a notification."),
            CheckedState::Done => panic!("This observer has already terminated."),
            CheckedState::Idle => {}
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
