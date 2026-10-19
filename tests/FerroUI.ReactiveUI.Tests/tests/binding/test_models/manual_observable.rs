//! `ManualObservable<T>` (`TestModels/ManualObservable.cs`).

use ferroui_reactive::disposables::Disposable;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

/// An observable that keeps its last observer, so that a test drives it by hand
/// (`ManualObservable<T>`).
pub struct ManualObservable<T: 'static> {
    observer: RefCell<Option<Rc<dyn IObserver<T>>>>,
}

#[allow(dead_code)]
impl<T: 'static> ManualObservable<T> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { observer: RefCell::new(None) })
    }

    /// The last observer that subscribed (`Observer`).
    pub fn observer(&self) -> Option<Rc<dyn IObserver<T>>> {
        self.observer.borrow().clone()
    }

    /// Sends `value` to the last observer, if any.
    pub fn on_next(&self, value: T) {
        if let Some(observer) = self.observer() {
            observer.on_next(value);
        }
    }
}

impl<T: 'static> IObservable<T> for ManualObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        *self.observer.borrow_mut() = Some(observer);
        Disposable::empty()
    }
}
