//! `Defer` (`Linq/Observable/Defer.cs`).

use super::creation::IdentitySink;
use crate::internal::subscribe_sink;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

type Factory<T> = Rc<dyn Fn() -> Result<Rc<dyn IObservable<T>>, ObservableError>>;

/// `Defer<TValue>`: each subscription calls the factory and subscribes to the sequence it returns;
/// a factory that fails ends the sequence with its error.
struct DeferObservable<T> {
    factory: Factory<T>,
}

impl<T: 'static> IObservable<T> for DeferObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let factory = self.factory.clone();
        subscribe_sink(observer, IdentitySink::new, move |sink: &Rc<IdentitySink<T>>| match factory() {
            Ok(source) => sink.run(source),
            Err(error) => sink.sink.forward_on_error(error),
        })
    }
}

impl crate::linq::Observable {
    /// A sequence that calls `factory` on each subscription and subscribes to the sequence it
    /// returns (`Defer<TResult>(Func<IObservable<TResult>>)`).
    pub fn defer<T: 'static>(factory: impl Fn() -> Rc<dyn IObservable<T>> + 'static) -> Rc<dyn IObservable<T>> {
        Rc::new(DeferObservable { factory: Rc::new(move || Ok(factory())) })
    }

    /// [`defer`](Self::defer) with a factory that may fail; a failure ends the sequence with its
    /// error (`Defer<TResult>(Func<IObservable<TResult>>)` with a factory that throws).
    pub fn try_defer<T: 'static>(factory: impl Fn() -> Result<Rc<dyn IObservable<T>>, ObservableError> + 'static) -> Rc<dyn IObservable<T>> {
        Rc::new(DeferObservable { factory: Rc::new(factory) })
    }
}
