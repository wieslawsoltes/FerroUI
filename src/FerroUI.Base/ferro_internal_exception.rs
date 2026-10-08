use std::fmt;

/// Exception signifying an internal logic error in the framework.
///
/// The framework itself reports such an error with a panic (porting guide:
/// programmer errors panic); the type names the error where it is passed on
/// as a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FerroInternalException {
    message: String,
}

impl FerroInternalException {
    /// Creates the exception with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    /// The message that describes the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for FerroInternalException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FerroInternalException {}
