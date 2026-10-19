//! `ValueRecordingObserver<T>` (`Utilities/ValueRecordingObserver.cs`).

use super::RecordingObserverBase;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::{IObserver, ObservableError};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// Records the values of a sequence until it ends (`ValueRecordingObserver<T>`); the error, the
/// end and the notifications are those of [`RecordingObserverBase`], which it dereferences to.
pub struct ValueRecordingObserver<T> {
    base: RecordingObserverBase<T>,
    recorded_values: RefCell<Vec<T>>,
}

impl<T: Clone + 'static> ValueRecordingObserver<T> {
    /// An observer that stamps the notifications with the time of `scheduler`.
    pub fn new(scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Rc::new(Self { base: RecordingObserverBase::new(scheduler), recorded_values: RefCell::new(Vec::new()) })
    }

    /// The values received before the end (`RecordedValues`).
    pub fn recorded_values(&self) -> Vec<T> {
        self.recorded_values.borrow().clone()
    }
}

impl<T> Deref for ValueRecordingObserver<T> {
    type Target = RecordingObserverBase<T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T: Clone + 'static> IObserver<T> for ValueRecordingObserver<T> {
    fn on_next(&self, value: T) {
        self.base.record_next(&value);
        if !self.base.has_finalized() {
            self.recorded_values.borrow_mut().push(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.base.record_error(error);
    }

    fn on_completed(&self) {
        self.base.record_completed();
    }
}
