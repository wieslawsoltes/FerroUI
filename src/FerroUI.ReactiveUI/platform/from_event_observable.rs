//! `FromEventObservable<T>` (`Shared/Platform/FromEventObservable.cs`).

use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// The handler that a subscription hooks to an event: it pushes the value of a raise.
pub type EventSink<T> = Rc<dyn Fn(T)>;

/// A bridge from an event to an observable (`FromEventObservable<T>`), with no reflection, no
/// scheduler and no queue: each subscription calls `subscribe` with the function that pushes a
/// value to the observer, and `subscribe` hooks it to the event and returns what unhooks it.
pub struct FromEventObservable<T> {
    subscribe: Rc<dyn Fn(EventSink<T>) -> Rc<dyn IDisposable>>,
}

impl<T: 'static> FromEventObservable<T> {
    /// A bridge that hooks the event with `subscribe` (`FromEventObservable(Func<Action<T>, IDisposable>)`).
    pub fn new(subscribe: impl Fn(EventSink<T>) -> Rc<dyn IDisposable> + 'static) -> Rc<Self> {
        Rc::new(Self { subscribe: Rc::new(subscribe) })
    }
}

impl<T: 'static> IObservable<T> for FromEventObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        (self.subscribe)(Rc::new(move |value: T| observer.on_next(value)))
    }
}
