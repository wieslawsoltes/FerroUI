//! `ObservableBase<T>` (`ObservableBase.cs`).
//!
//! Upstream is an abstract class whose subclasses override `SubscribeCore`; Rust has no
//! inheritance, and a blanket implementation of the foreign trait `IObservable<T>` for the types
//! of a trait is not allowed. The port is a struct that wraps the subscription of the user, given
//! as an [`ObservableCore`] or as a function, and subscribes through it as the base class does.

use crate::internal::try_subscribe_auto_detach;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The subscription of an [`ObservableBase`] (the abstract member `SubscribeCore` of
/// `ObservableBase<T>`).
pub trait ObservableCore<T> {
    /// Subscribes `observer` and returns the disposable that ends the subscription
    /// (`SubscribeCore`); an `Err` is the exception upstream's `SubscribeCore` throws, which is
    /// delivered to the observer as its error.
    fn subscribe_core(&self, observer: Rc<dyn IObserver<T>>) -> Result<Rc<dyn IDisposable>, ObservableError>;
}

/// The core of an observable made of a function.
struct FnCore<F>(F);

impl<T, F> ObservableCore<T> for FnCore<F>
where
    F: Fn(Rc<dyn IObserver<T>>) -> Result<Rc<dyn IDisposable>, ObservableError>,
{
    fn subscribe_core(&self, observer: Rc<dyn IObserver<T>>) -> Result<Rc<dyn IDisposable>, ObservableError> {
        (self.0)(observer)
    }
}

/// An observable whose subscription is the user's [`ObservableCore`] (`ObservableBase<T>`).
///
/// The observer the core receives ignores everything after a terminal notification, and a
/// terminal notification, or a panic while the observer handles a value, disposes the
/// subscription (`AutoDetachObserver<T>`). When no work of the current-thread scheduler is
/// running, the subscription runs through its trampoline, so that work it schedules there runs
/// after the subscription has been set up.
pub struct ObservableBase<T> {
    core: Rc<dyn ObservableCore<T>>,
}

impl<T: 'static> ObservableBase<T> {
    /// An observable whose subscription is `core` (a subclass of `ObservableBase<T>` upstream).
    pub fn new(core: impl ObservableCore<T> + 'static) -> Self {
        Self { core: Rc::new(core) }
    }

    /// An observable whose subscription is the function `subscribe_core`.
    pub fn from_fn(subscribe_core: impl Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> + 'static) -> Self {
        Self::new(FnCore(move |observer: Rc<dyn IObserver<T>>| -> Result<Rc<dyn IDisposable>, ObservableError> {
            Ok(subscribe_core(observer))
        }))
    }

    /// An observable whose subscription is the function `subscribe_core`, which may fail: an `Err`
    /// is delivered to the observer as its error (a `SubscribeCore` that throws upstream).
    pub fn try_from_fn(
        subscribe_core: impl Fn(Rc<dyn IObserver<T>>) -> Result<Rc<dyn IDisposable>, ObservableError> + 'static,
    ) -> Self {
        Self::new(FnCore(subscribe_core))
    }
}

impl<T: 'static> IObservable<T> for ObservableBase<T> {
    /// Subscribes through the core (`Subscribe`): the observer is wrapped in an auto-detaching
    /// observer, which the core receives; an error of the core is delivered to that observer
    /// (`AutoDetachObserver.Fail`), and panics, where upstream rethrows it, when the observer has
    /// stopped already.
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let core = self.core.clone();
        try_subscribe_auto_detach(observer, move |observer: Rc<dyn IObserver<T>>| core.subscribe_core(observer))
    }
}
