//! `RawAnonymousObserver<T>` (`Utilities/RawAnonymousObserver.cs`).

use ferroui_system::{IObserver, ObservableError};
use std::rc::Rc;

/// An observer made of three functions, without the safeguards of `Observer.Create`
/// (`RawAnonymousObserver<T>`): it forwards every notification, also after an end.
pub struct RawAnonymousObserver<T> {
    on_next: Box<dyn Fn(T)>,
    on_error: Box<dyn Fn(ObservableError)>,
    on_completed: Box<dyn Fn()>,
}

impl<T: 'static> RawAnonymousObserver<T> {
    /// The observer of the three functions (`new RawAnonymousObserver<T>(..)`).
    pub fn new(on_next: impl Fn(T) + 'static, on_error: impl Fn(ObservableError) + 'static, on_completed: impl Fn() + 'static) -> Self {
        Self { on_next: Box::new(on_next), on_error: Box::new(on_error), on_completed: Box::new(on_completed) }
    }

    /// The observer of the three functions, as the handle observables take
    /// (`RawAnonymousObserver.Create<T>(..)`).
    pub fn create(on_next: impl Fn(T) + 'static, on_error: impl Fn(ObservableError) + 'static, on_completed: impl Fn() + 'static) -> Rc<dyn IObserver<T>> {
        Rc::new(Self::new(on_next, on_error, on_completed))
    }
}

impl<T> IObserver<T> for RawAnonymousObserver<T> {
    fn on_next(&self, value: T) {
        (self.on_next)(value);
    }

    fn on_error(&self, error: ObservableError) {
        (self.on_error)(error);
    }

    fn on_completed(&self) {
        (self.on_completed)();
    }
}
