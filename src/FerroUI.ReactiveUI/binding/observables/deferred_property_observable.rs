//! `DeferredPropertyObservable<TSource, TValue>`.

use ferroui_reactive::disposables::Disposable;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// A property read on each subscription and produced once, then nothing, never completing
/// (`DeferredPropertyObservable<TSource, TValue>`).
pub struct DeferredPropertyObservable<S, V> {
    source: S,
    getter: Rc<dyn Fn(&S) -> V>,
}

impl<S: 'static, V: 'static> DeferredPropertyObservable<S, V> {
    /// The value of `getter` on `source`, read on each subscription.
    pub fn new(source: S, getter: impl Fn(&S) -> V + 'static) -> Self {
        Self { source, getter: Rc::new(getter) }
    }
}

impl<S: 'static, V: 'static> IObservable<V> for DeferredPropertyObservable<S, V> {
    fn subscribe(&self, observer: Rc<dyn IObserver<V>>) -> Rc<dyn IDisposable> {
        observer.on_next((self.getter)(&self.source));
        Disposable::empty()
    }
}
