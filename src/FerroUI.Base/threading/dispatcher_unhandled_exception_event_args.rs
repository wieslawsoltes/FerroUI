use std::any::Any;
use std::cell::{Cell, OnceCell};
use std::error::Error;
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

/// The error of an exception that application code raised as a value with
/// [`Dispatcher::raise_unhandled_exception`], if it is one.
pub(crate) fn exception_error(payload: &(dyn Any + Send)) -> Option<&(dyn Error + Send + Sync + 'static)> {
    payload.downcast_ref::<Box<dyn Error + Send + Sync>>().map(|error| &**error)
}

/// The text of an exception: the message of a panic, or the error raised as a
/// value.
pub(crate) fn exception_text(payload: &(dyn Any + Send)) -> Option<String> {
    match exception_error(payload) {
        Some(error) => Some(error.to_string()),
        None => panic_message(payload).map(str::to_owned),
    }
}

/// Provides data for the [`Dispatcher::unhandled_exception`] event.
pub struct DispatcherUnhandledExceptionEventArgs<'a> {
    dispatcher: &'a Arc<Dispatcher>,
    exception: &'a (dyn Any + Send),
    handled: Cell<bool>,
    message: OnceCell<Option<String>>,
}

impl<'a> DispatcherUnhandledExceptionEventArgs<'a> {
    pub(crate) fn new(dispatcher: &'a Arc<Dispatcher>, exception: &'a (dyn Any + Send), handled: bool) -> Self {
        Self { dispatcher, exception, handled: Cell::new(handled), message: OnceCell::new() }
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
