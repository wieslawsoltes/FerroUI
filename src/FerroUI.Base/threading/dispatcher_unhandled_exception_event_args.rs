use std::any::Any;
use std::cell::Cell;
use std::sync::Arc;

use super::{Dispatcher, DispatcherEventArgs};

/// Returns the message of a panic payload when it is a string, which is the
/// case for every `panic!` with a message.
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> Option<&str> {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        Some(message)
    } else {
        payload.downcast_ref::<String>().map(String::as_str)
    }
}

/// Provides data for the [`Dispatcher::unhandled_exception`] event.
pub struct DispatcherUnhandledExceptionEventArgs<'a> {
    dispatcher: &'a Arc<Dispatcher>,
    exception: &'a (dyn Any + Send),
    handled: Cell<bool>,
}

impl<'a> DispatcherUnhandledExceptionEventArgs<'a> {
    pub(crate) fn new(dispatcher: &'a Arc<Dispatcher>, exception: &'a (dyn Any + Send), handled: bool) -> Self {
        Self { dispatcher, exception, handled: Cell::new(handled) }
    }

    /// The panic payload that was raised while executing code by way of a
    /// dispatcher.
    pub fn exception(&self) -> &(dyn Any + Send) {
        self.exception
    }

    /// The panic message, when the payload is a string.
    pub fn exception_message(&self) -> Option<&str> {
        panic_message(self.exception)
    }

    /// Whether the exception event has been handled.
    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    pub fn set_handled(&self, value: bool) {
        // Only allow to be set true.
        if value {
            self.handled.set(value);
        }
    }
}

impl DispatcherEventArgs for DispatcherUnhandledExceptionEventArgs<'_> {
    fn dispatcher(&self) -> &Arc<Dispatcher> {
        self.dispatcher
    }
}
