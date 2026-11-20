use std::any::Any;
use std::cell::{Cell, OnceCell};
use std::error::Error;
use std::sync::Arc;

use super::dispatcher_unhandled_exception_event_args::{exception_error, exception_text, panic_message};
use super::{Dispatcher, DispatcherEventArgs};

/// Provides data for the [`Dispatcher::unhandled_exception_filter`] event.
pub struct DispatcherUnhandledExceptionFilterEventArgs<'a> {
    dispatcher: &'a Arc<Dispatcher>,
    exception: &'a (dyn Any + Send),
    request_catch: Cell<bool>,
    message: OnceCell<Option<String>>,
}

impl<'a> DispatcherUnhandledExceptionFilterEventArgs<'a> {
    pub(crate) fn new(dispatcher: &'a Arc<Dispatcher>, exception: &'a (dyn Any + Send), request_catch: bool) -> Self {
        Self { dispatcher, exception, request_catch: Cell::new(request_catch), message: OnceCell::new() }
    }

    /// The exception that was raised while executing code by way of a
    /// dispatcher: a panic payload, or for an error raised with
    /// [`Dispatcher::raise_unhandled_exception`] a `Box<dyn Error + Send + Sync>`.
    pub fn exception(&self) -> &(dyn Any + Send) {
        self.exception
    }

    /// The error, when the exception was raised as a value with
    /// [`Dispatcher::raise_unhandled_exception`] (#567); `None` for a panic.
    pub fn error(&self) -> Option<&(dyn Error + Send + Sync + 'static)> {
        exception_error(self.exception)
    }

    /// The message: of the panic when its payload is a string, or of the
    /// [`error`](Self::error).
    pub fn exception_message(&self) -> Option<&str> {
        if self.error().is_none() {
            return panic_message(self.exception);
        }
        self.message.get_or_init(|| exception_text(self.exception)).as_deref()
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
