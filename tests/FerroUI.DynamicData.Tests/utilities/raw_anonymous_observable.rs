//! `RawAnonymousObservable<T>` (`Utilities/RawAnonymousObservable.cs`).

use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

type OnSubscribe<T> = Box<dyn Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable>>;

/// An observable made of its subscription function, without the safeguards of `Observable.Create`
/// (`RawAnonymousObservable<T>`), so that tests can combine it with operators whose misbehaviour
/// those safeguards would hide.
pub struct RawAnonymousObservable<T> {
    on_subscribe: OnSubscribe<T>,
}

impl<T: 'static> RawAnonymousObservable<T> {
    /// The observable of `on_subscribe` (`new RawAnonymousObservable<T>(..)`).
    pub fn new(on_subscribe: impl Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> + 'static) -> Self {
        Self { on_subscribe: Box::new(on_subscribe) }
    }

    /// The observable of `on_subscribe`, as the handle operators take
    /// (`RawAnonymousObservable.Create<T>(..)`).
    pub fn create(on_subscribe: impl Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> + 'static) -> Rc<dyn IObservable<T>> {
        Rc::new(Self::new(on_subscribe))
    }
}

impl<T> IObservable<T> for RawAnonymousObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        (self.on_subscribe)(observer)
    }
}
