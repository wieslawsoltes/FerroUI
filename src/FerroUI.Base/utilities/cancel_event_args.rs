use std::cell::Cell;
use std::rc::Rc;

/// The data of a cancellable event (the counterpart of the platform's
/// cancel event args).
///
/// The args are a reference object: a copy shares the cancel flag with the
/// args it was made from, so that a handler given a copy cancels the event,
/// and copies compare by identity.
#[derive(Clone, Debug, Default)]
pub struct CancelEventArgs {
    cancel: Rc<Cell<bool>>,
}

impl PartialEq for CancelEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.cancel, &other.cancel)
    }
}

impl CancelEventArgs {
    /// Creates args of an event that is not cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the event should be cancelled.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value)
    }
}
