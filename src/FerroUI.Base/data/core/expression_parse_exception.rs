//! The error reported when an expression string cannot be parsed.

use std::fmt;

/// Error returned when the provided binding expression string could not be parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionParseError {
    column: i32,
    message: String,
}

/// The name of [`ExpressionParseError`] before 0.5.
#[deprecated(since = "0.5.0", note = "use `ExpressionParseError`")]
pub type ExpressionParseException = ExpressionParseError;

impl ExpressionParseError {
    /// Initializes a new instance of the [`ExpressionParseError`] type.
    ///
    /// * `column` - The column position of the error.
    /// * `message` - The error message.
    pub fn new(column: i32, message: impl Into<String>) -> Self {
        Self {
            column,
            message: message.into(),
        }
    }

    /// Gets the column position at which the error occurred.
    #[inline]
    pub fn column(&self) -> i32 {
        self.column
    }

    /// Gets the error message.
    #[inline]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ExpressionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ExpressionParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_column_and_message() {
        let e = ExpressionParseError::new(3, "Expected ')'.");
        assert_eq!(e.column(), 3);
        assert_eq!(e.message(), "Expected ')'.");
        assert_eq!(e.to_string(), "Expected ')'.");
        let _: &dyn std::error::Error = &e;
    }
}
