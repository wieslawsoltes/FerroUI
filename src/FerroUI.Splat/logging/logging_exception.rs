//! `LoggingException`.

use std::error::Error;
use std::fmt;

/// The failure of the logging infrastructure (`LoggingException`): a log manager or a logger that
/// the resolver does not have.
///
/// Where upstream throws it, the port panics with it as the payload
/// (`std::panic::panic_any`), so that a caller that catches the panic can tell it apart.
#[derive(Debug)]
pub struct LoggingException {
    message: String,
    inner: Option<Box<dyn Error + Send + Sync>>,
}

impl LoggingException {
    /// The exception with the default message (`LoggingException()`).
    pub fn new() -> Self {
        Self { message: "Exception of type 'LoggingException' was thrown.".to_owned(), inner: None }
    }

    /// The exception with `message` (`LoggingException(string)`).
    pub fn with_message(message: impl Into<String>) -> Self {
        Self { message: message.into(), inner: None }
    }

    /// The exception with `message`, caused by `inner_exception`
    /// (`LoggingException(string, Exception)`).
    pub fn with_inner(message: impl Into<String>, inner_exception: Box<dyn Error + Send + Sync>) -> Self {
        Self { message: message.into(), inner: Some(inner_exception) }
    }

    /// The message (`Message`).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The exception that caused this one (`InnerException`).
    pub fn inner_exception(&self) -> Option<&(dyn Error + Send + Sync + 'static)> {
        self.inner.as_deref()
    }
}

impl Default for LoggingException {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LoggingException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for LoggingException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.as_deref().map(|inner| inner as &(dyn Error + 'static))
    }
}
