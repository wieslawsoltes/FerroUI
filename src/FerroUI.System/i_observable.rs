//! `System.IObservable<T>` and `System.IObserver<T>`: the interfaces of push-based notification.

use crate::{IDisposable, Subscription};
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

/// Subscribes and returns the [`Subscription`] guard instead of the handle (C3b of #423,
/// `docs/idiomatic-rust-disposal.md`, step D5).
///
/// [`IObservable::subscribe`] keeps returning the `Rc<dyn IDisposable>` of the Rx contract, which
/// the operators compose and which keeps the subscription alive when it is dropped. An owner that
/// keeps a subscription only to end it later subscribes with
/// [`subscribe_guard`](SubscribeGuard::subscribe_guard) and holds the guard: dropping it
/// unsubscribes, once, and [`Subscription::detach`] gives the handle back where the subscription
/// is meant to last as long as its source.
///
/// The trait is implemented for every observable, sized or not, so it applies to a concrete
/// observable, to `dyn IObservable<T>` and, through deref, to `Rc<dyn IObservable<T>>`.
pub trait SubscribeGuard<T> {
    /// Subscribes `observer`; dropping the returned guard unsubscribes it.
    fn subscribe_guard(&self, observer: Rc<dyn IObserver<T>>) -> Subscription;
}

impl<T, O: IObservable<T> + ?Sized> SubscribeGuard<T> for O {
    fn subscribe_guard(&self, observer: Rc<dyn IObserver<T>>) -> Subscription {
        Subscription::new(self.subscribe(observer))
    }
}

/// Sequences compare by identity (reference equality), so that they can be the items of a list.
impl<T> PartialEq for dyn IObservable<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
