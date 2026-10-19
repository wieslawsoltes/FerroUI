//! `TestObserver` (`ReactiveUI.Test.Utilities/TestObserver.cs`).

use ferroui_system::{IObserver, ObservableError};
use std::rc::Rc;

/// Observers of the tests (`TestObserver`).
pub struct TestObserver;

impl TestObserver {
    /// An observer that calls `on_next` for each value and panics on an error (`Create<T>`).
    pub fn create<T: 'static>(on_next: impl Fn(T) + 'static) -> Rc<dyn IObserver<T>> {
        Rc::new(DelegateObserver { on_next: Box::new(on_next) })
    }
}

struct DelegateObserver<T> {
    on_next: Box<dyn Fn(T)>,
}

impl<T> IObserver<T> for DelegateObserver<T> {
    fn on_next(&self, value: T) {
        (self.on_next)(value);
    }

    fn on_error(&self, error: ObservableError) {
        panic!("{error}");
    }

    fn on_completed(&self) {
        // No completion handling needed for a test observer.
    }
}
