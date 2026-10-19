//! `StartWithObservable<T>` (`Shared/Platform/StartWithObservable.cs`).

use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// A seed value at subscription, then the source (`StartWithObservable<T>`): `StartWith(value)`
/// without an operator in between.
pub struct StartWithObservable<T> {
    source: Rc<dyn IObservable<T>>,
    value: T,
}

impl<T: Clone + 'static> StartWithObservable<T> {
    /// `value`, then `source` (`StartWithObservable(IObservable<T>, T)`).
    pub fn new(source: Rc<dyn IObservable<T>>, value: T) -> Rc<Self> {
        Rc::new(Self { source, value })
    }
}

impl<T: Clone + 'static> IObservable<T> for StartWithObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        observer.on_next(self.value.clone());
        self.source.subscribe(observer)
    }
}
