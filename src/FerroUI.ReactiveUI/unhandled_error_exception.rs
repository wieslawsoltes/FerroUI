//! `UnhandledErrorException` (`ReactiveUI.Core/UnhandledErrorException.cs`).

use std::error::Error;
use std::fmt;

/// The error of a pipeline that broke because nothing handled its error
/// (`UnhandledErrorException`): what [`RxState::default_exception_handler`](crate::RxState::default_exception_handler)
/// panics with when an object implementing
/// [`IHandleObservableErrors`](crate::IHandleObservableErrors) errors and nothing subscribes to
/// its `thrown_exceptions`.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the inner exception is kept as its message. The error is
/// the payload of a panic (`std::panic::panic_any`), which has to be `Send`, and the errors of the
/// reactive libraries (`Rc<dyn Error>`) are not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnhandledErrorException {
    message: String,
    inner_exception: Option<String>,
}

impl Default for UnhandledErrorException {
    fn default() -> Self {
        Self::new()
    }
}

impl UnhandledErrorException {
    /// The exception with the default message of .NET (`UnhandledErrorException()`).
    pub fn new() -> Self {
        Self { message: "Exception of type 'ReactiveUI.UnhandledErrorException' was thrown.".to_owned(), inner_exception: None }
    }

    /// The exception with `message` (`UnhandledErrorException(string)`).
    pub fn with_message(message: impl Into<String>) -> Self {
        Self { message: message.into(), inner_exception: None }
    }

    /// The exception with `message`, caused by `inner_exception`
    /// (`UnhandledErrorException(string, Exception)`).
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

impl fmt::Display for UnhandledErrorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner_exception {
            Some(inner) => write!(f, "{} ({inner})", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl Error for UnhandledErrorException {}
