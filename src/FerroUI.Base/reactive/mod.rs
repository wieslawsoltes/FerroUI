//! Lightweight reactive primitives: observables, observers and disposables.
//!
//! Everything here is single-threaded and reference counted, matching the
//! thread affinity of the object model.

mod anonymous_observer;
mod composite_disposable;
mod disposable;
mod lightweight_observable_base;
mod lightweight_subject;
mod observable;
mod single_subscriber_observable_base;

pub use anonymous_observer::AnonymousObserver;
pub use composite_disposable::CompositeDisposable;
pub use disposable::{Disposable, IDisposable, SerialDisposable};
pub use lightweight_observable_base::{LightweightObservable, LightweightObservableBase};
pub use lightweight_subject::LightweightSubject;
pub use observable::{Observable, ObservableExt};
pub use single_subscriber_observable_base::SingleSubscriberObservableBase;

use std::rc::Rc;

/// The error type carried by [`IObserver::on_error`].
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
