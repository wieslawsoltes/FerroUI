//! The errors the example reports where the reference throws a .NET exception.
//!
//! Not an upstream file: the reference throws `InvalidOperationException` (a failing command, a
//! missing registration) and relies on `PlatformNotSupportedException` of `Process`.

use ferroui_system::ObservableError;
use std::fmt;
use std::rc::Rc;

/// An operation that is not valid in the current state (`InvalidOperationException`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidOperationError {
    message: String,
}

impl InvalidOperationError {
    /// The error with `message` (`new InvalidOperationException(message)`).
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    /// The error as the error of an observable or a task.
    pub fn into_error(self) -> ObservableError {
        Rc::new(self)
    }

    /// The message of the error (`Message`).
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for InvalidOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for InvalidOperationError {}

/// A measurement the platform cannot take (`PlatformNotSupportedException`, which `Process` throws
/// where it is not supported).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformNotSupportedError {
    message: String,
}

impl PlatformNotSupportedError {
    /// The error with `message`.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    /// The error as the error of an observable or a task.
    pub fn into_error(self) -> ObservableError {
        Rc::new(self)
    }
}

impl fmt::Display for PlatformNotSupportedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PlatformNotSupportedError {}
