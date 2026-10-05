use super::AccessorListener;
use crate::BoxedValue;
use std::cell::RefCell;

/// The listener bookkeeping shared by property accessors.
#[derive(Default)]
pub struct PropertyAccessorBase {
    listener: RefCell<Option<AccessorListener>>,
}

impl PropertyAccessorBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_subscribed(&self) -> bool {
        self.listener.borrow().is_some()
    }

    /// Stores the listener. Panics if already subscribed: a member accessor
    /// can be subscribed to only once.
    pub fn set_listener(&self, listener: AccessorListener) {
        let mut slot = self.listener.borrow_mut();
        if slot.is_some() {
            panic!("A member accessor can be subscribed to only once.");
        }
        *slot = Some(listener);
    }

    /// Clears the listener. Panics if not subscribed.
    pub fn clear_listener(&self) {
        if self.listener.borrow_mut().take().is_none() {
            panic!("The member accessor was not subscribed.");
        }
    }

    /// Publishes a value to the listener.
    pub fn publish_value(&self, value: Option<BoxedValue>) {
        let listener = self.listener.borrow().clone();
        if let Some(listener) = listener {
            listener(value);
        }
    }
}
