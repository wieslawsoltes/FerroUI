//! The subscription overloads of `ObservableExtensions` (`Observable.Extensions.cs`) and the
//! operator trait.

use crate::observer::{throw, AnonymousObserver};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The operators of an observable sequence, as methods of its handle (`Observable`'s extension
/// methods and `ObservableExtensions`).
pub trait ObservableExt<T: 'static> {
    /// Subscribes an observer, with the safeguards of a subscription (`SubscribeSafe`): an observer
    /// that panics while it handles a notification disposes the subscription first.
    fn subscribe_safe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable>;

    /// Subscribes and ignores every notification but errors, which panic (`Subscribe()`).
    fn subscribe_ignore(&self) -> Rc<dyn IDisposable>;

    /// Subscribes a function of values; an error panics (`Subscribe(Action<T>)`).
    fn subscribe_next(&self, on_next: impl Fn(T) + 'static) -> Rc<dyn IDisposable>;

    /// Subscribes functions of values and errors (`Subscribe(Action<T>, Action<Exception>)`).
    fn subscribe_next_error(&self, on_next: impl Fn(T) + 'static, on_error: impl Fn(ObservableError) + 'static) -> Rc<dyn IDisposable>;

    /// Subscribes functions of values and the end; an error panics (`Subscribe(Action<T>, Action)`).
    fn subscribe_next_completed(&self, on_next: impl Fn(T) + 'static, on_completed: impl Fn() + 'static) -> Rc<dyn IDisposable>;

    /// Subscribes functions of all three notifications (`Subscribe(Action<T>, Action<Exception>, Action)`).
    fn subscribe_all(
        &self,
        on_next: impl Fn(T) + 'static,
        on_error: impl Fn(ObservableError) + 'static,
        on_completed: impl Fn() + 'static,
    ) -> Rc<dyn IDisposable>;
}

impl<T: 'static> ObservableExt<T> for Rc<dyn IObservable<T>> {
    fn subscribe_safe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        // Every observable of the crate subscribes through the safeguards of a sink or of user code
        // (`SubscribeSafe` short-cuts producers the same way upstream).
        self.subscribe(observer)
    }

    fn subscribe_ignore(&self) -> Rc<dyn IDisposable> {
        self.subscribe(Rc::new(AnonymousObserver::new(|_| {}, throw, || {})))
    }

    fn subscribe_next(&self, on_next: impl Fn(T) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe(Rc::new(AnonymousObserver::new(on_next, throw, || {})))
    }

    fn subscribe_next_error(&self, on_next: impl Fn(T) + 'static, on_error: impl Fn(ObservableError) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe(Rc::new(AnonymousObserver::new(on_next, on_error, || {})))
    }

    fn subscribe_next_completed(&self, on_next: impl Fn(T) + 'static, on_completed: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe(Rc::new(AnonymousObserver::new(on_next, throw, on_completed)))
    }

    fn subscribe_all(
        &self,
        on_next: impl Fn(T) + 'static,
        on_error: impl Fn(ObservableError) + 'static,
        on_completed: impl Fn() + 'static,
    ) -> Rc<dyn IDisposable> {
        self.subscribe(Rc::new(AnonymousObserver::new(on_next, on_error, on_completed)))
    }
}
