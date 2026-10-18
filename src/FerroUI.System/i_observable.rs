//! `System.IObservable<T>` and `System.IObserver<T>`: the interfaces of push-based notification.

use crate::IDisposable;
use std::rc::Rc;

/// The error type carried by [`IObserver::on_error`]: a shared error, so that it can be delivered to
/// several observers and kept by the operators that replay it. Errors compare by identity where the
/// tests of the reactive libraries compare exceptions.
pub type ObservableError = Rc<dyn std::error::Error>;

/// Receives notifications from an [`IObservable`].
pub trait IObserver<T> {
    fn on_next(&self, value: T);
    fn on_error(&self, _error: ObservableError) {}
    fn on_completed(&self) {}
}

/// A push-based source of values.
pub trait IObservable<T> {
    /// Subscribes `observer`; disposing the returned handle unsubscribes it.
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable>;
}
