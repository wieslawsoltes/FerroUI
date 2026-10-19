//! `UnchangingPropertyObservable<T>`.

use ferroui_reactive::disposables::Disposable;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// A value that does not change: produced on subscription, then nothing, never completing
/// (`UnchangingPropertyObservable<T>`).
pub struct UnchangingPropertyObservable<T> {
    value: T,
}

impl<T: Clone + 'static> UnchangingPropertyObservable<T> {
    /// The sequence of `value`.
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: Clone + 'static> IObservable<T> for UnchangingPropertyObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        observer.on_next(self.value.clone());
        Disposable::empty()
    }
}
