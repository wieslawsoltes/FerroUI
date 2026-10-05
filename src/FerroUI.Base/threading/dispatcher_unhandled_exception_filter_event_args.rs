use std::any::Any;
use std::cell::Cell;
use std::sync::Arc;

use super::dispatcher_unhandled_exception_event_args::panic_message;
use super::{Dispatcher, DispatcherEventArgs};

/// Provides data for the [`Dispatcher::unhandled_exception_filter`] event.
pub struct DispatcherUnhandledExceptionFilterEventArgs<'a> {
    dispatcher: &'a Arc<Dispatcher>,
    exception: &'a (dyn Any + Send),
    request_catch: Cell<bool>,
}

impl<'a> DispatcherUnhandledExceptionFilterEventArgs<'a> {
    pub(crate) fn new(dispatcher: &'a Arc<Dispatcher>, exception: &'a (dyn Any + Send), request_catch: bool) -> Self {
        Self { dispatcher, exception, request_catch: Cell::new(request_catch) }
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

    /// Whether the exception should be caught and the
    /// `unhandled_exception` handlers called.
    ///
    /// A filter handler can set this to `false`, which stops the exception
    /// from being caught and handed to the `unhandled_exception` handlers.
    pub fn request_catch(&self) -> bool {
        self.request_catch.get()
    }

    pub fn set_request_catch(&self, value: bool) {
        // Only allow to be set false.
        if !value {
            self.request_catch.set(value);
        }
    }
}

impl DispatcherEventArgs for DispatcherUnhandledExceptionFilterEventArgs<'_> {
    fn dispatcher(&self) -> &Arc<Dispatcher> {
        self.dispatcher
    }
}
