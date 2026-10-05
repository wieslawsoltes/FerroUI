//! The error of the expression parser.

use std::error::Error;
use std::fmt;

/// The error raised when a composition expression cannot be parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionParseException {
    message: String,
    position: i32,
}

impl ExpressionParseException {
    /// Creates the error with a message and the position (in UTF-16 code
    /// units from the start of the expression) it was detected at.
    pub fn new(message: impl Into<String>, position: i32) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }

    /// The message of the error.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The position the error was detected at.
    pub fn position(&self) -> i32 {
        self.position
    }
}

impl fmt::Display for ExpressionParseException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ExpressionParseException {}
