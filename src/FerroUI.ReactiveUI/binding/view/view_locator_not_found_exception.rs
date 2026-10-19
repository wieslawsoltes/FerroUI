//! `ViewLocatorNotFoundException` (ReactiveUI.Binding, `View/ViewLocatorNotFoundException.cs`).

use std::error::Error;
use std::fmt;

/// The error of [`ViewLocator::current`](crate::ViewLocator::current) when no
/// [`IViewLocator`](crate::IViewLocator) is registered (`ViewLocatorNotFoundException`); it is the
/// payload of the panic (`std::panic::panic_any`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the inner exception is kept as its message, so that the
/// error can be the payload of a panic, which has to be `Send`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewLocatorNotFoundException {
    message: String,
    inner_exception: Option<String>,
}

impl Default for ViewLocatorNotFoundException {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewLocatorNotFoundException {
    /// The exception with a message that names the builder calls registering the default locator
    /// (`ViewLocatorNotFoundException()`).
    pub fn new() -> Self {
        Self::with_message(
            "No IViewLocator is registered. Call RxBindingBuilder::create_reactive_ui_binding_builder().with_core_services().build_app() to register default services.",
        )
    }

    /// The exception with `message` (`ViewLocatorNotFoundException(string)`).
    pub fn with_message(message: impl Into<String>) -> Self {
        Self { message: message.into(), inner_exception: None }
    }

    /// The exception with `message`, caused by `inner_exception`
    /// (`ViewLocatorNotFoundException(string, Exception)`).
    pub fn with_inner(message: impl Into<String>, inner_exception: &dyn Error) -> Self {
        Self { message: message.into(), inner_exception: Some(inner_exception.to_string()) }
    }

    /// The message (`Message`).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The message of the error that caused this one (`InnerException.Message`).
    pub fn inner_exception(&self) -> Option<&str> {
        self.inner_exception.as_deref()
    }
}

impl fmt::Display for ViewLocatorNotFoundException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ViewLocatorNotFoundException {}
