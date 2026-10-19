//! `RecordingObserver<T>` (`TestModels/RecordingObserver.cs`).

use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Records the values, the error and the completion it receives (`RecordingObserver<T>`).
pub struct RecordingObserver<T> {
    pub values: RefCell<Vec<T>>,
    pub error: RefCell<Option<ObservableError>>,
    pub is_completed: Cell<bool>,
}

#[allow(dead_code)]
impl<T: Clone + 'static> RecordingObserver<T> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { values: RefCell::new(Vec::new()), error: RefCell::new(None), is_completed: Cell::new(false) })
    }

    /// The values received so far.
    pub fn values(&self) -> Vec<T> {
        self.values.borrow().clone()
    }
}

impl<T> IObserver<T> for RecordingObserver<T> {
    fn on_next(&self, value: T) {
        self.values.borrow_mut().push(value);
    }

    fn on_error(&self, error: ObservableError) {
        *self.error.borrow_mut() = Some(error);
    }

    fn on_completed(&self) {
        self.is_completed.set(true);
    }
}
