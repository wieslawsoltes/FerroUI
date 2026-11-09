//! Ownership of one scheduled keyboard callback.
use std::{cell::RefCell, rc::Rc};

struct Cancellation(RefCell<Option<Box<dyn FnOnce()>>>);
impl Cancellation {
    fn cancel(&self) {
        // A cancellation callback may reenter this handle or drop another clone.
        // Publish cancellation and release the borrow before running caller code.
        let callback = self.0.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
    }
}
impl Drop for Cancellation {
    fn drop(&mut self) {
        if let Some(callback) = self.0.get_mut().take() {
            callback();
        }
    }
}

/// Retain this guard until the callback finishes or should be cancelled.
/// Clones share cancellation; `cancel` or the final drop cancels exactly once.
/// The scheduler must not retain a strong clone of its own guard.
#[must_use = "dropping the final scheduled-action guard cancels the callback"]
#[derive(Clone)]
pub struct ScheduledAction(Rc<Cancellation>);
impl ScheduledAction {
    pub fn new(cancel: impl FnOnce() + 'static) -> Self {
        Self(Rc::new(Cancellation(RefCell::new(Some(Box::new(cancel))))))
    }
    pub fn cancel(&self) {
        self.0.cancel();
    }
    #[cfg(test)]
    pub(crate) fn test_identity(&self) -> std::rc::Weak<dyn std::any::Any> {
        let value: Rc<dyn std::any::Any> = self.0.clone();
        Rc::downgrade(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn final_drop_cancels_once_and_releases_captures() {
        let calls = Rc::new(Cell::new(0));
        let payload = Rc::new(());
        let weak = Rc::downgrade(&payload);
        let count = calls.clone();
        let guard = ScheduledAction::new(move || {
            count.set(count.get() + 1);
            drop(payload);
        });
        let clone = guard.clone();
        drop(guard);
        assert_eq!(calls.get(), 0);
        assert!(weak.upgrade().is_some());
        drop(clone);
        assert_eq!(calls.get(), 1);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn cancellation_publishes_before_reentrant_caller_code() {
        let slot = Rc::new(RefCell::new(None::<ScheduledAction>));
        let weak = Rc::downgrade(&slot);
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let guard = ScheduledAction::new(move || {
            count.set(count.get() + 1);
            weak.upgrade().unwrap().borrow().as_ref().unwrap().cancel();
        });
        *slot.borrow_mut() = Some(guard.clone());
        guard.cancel();
        guard.cancel();
        slot.borrow_mut().take();
        drop(guard);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn original_cancellation_panic_propagates_without_retry_on_drop() {
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let guard = ScheduledAction::new(move || {
            count.set(count.get() + 1);
            std::panic::panic_any("keyboard cancellation failure");
        });
        let failure =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| guard.cancel())).unwrap_err();
        assert_eq!(
            failure.downcast_ref::<&str>(),
            Some(&"keyboard cancellation failure")
        );
        guard.cancel();
        drop(guard);
        assert_eq!(calls.get(), 1);
    }
}
