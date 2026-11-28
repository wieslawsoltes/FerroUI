/// The error of an operation on an automation element that is not enabled.
///
/// Provider members that act on an element return it as the error of their
/// result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementNotEnabledError {
    message: String,
}

impl ElementNotEnabledError {
    /// Creates the error with the default message.
    pub fn new() -> Self {
        Self::with_message("Element not enabled.")
    }

    /// Creates the error with a message.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    /// The message of the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Default for ElementNotEnabledError {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ElementNotEnabledError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ElementNotEnabledError {}
