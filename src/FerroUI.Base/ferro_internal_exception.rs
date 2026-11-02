use std::fmt;

/// Exception signifying an internal logic error in the framework.
///
/// The framework itself reports such an error with a panic (porting guide:
/// programmer errors panic); the type names the error where it is passed on
/// as a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FerroInternalError {
    message: String,
}

/// The name of [`FerroInternalError`] before 0.5.
#[deprecated(since = "0.5.0", note = "use `FerroInternalError`")]
pub type FerroInternalException = FerroInternalError;

impl FerroInternalError {
    /// Creates the exception with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    /// The message that describes the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for FerroInternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FerroInternalError {}
