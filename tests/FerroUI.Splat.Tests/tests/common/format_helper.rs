//! `FormatHelper`.

use std::error::Error;
use std::fmt;

/// The exception of the tests (`new Exception()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestException {
    message: String,
}

impl TestException {
    /// An exception with the default message.
    pub fn new() -> Self {
        Self { message: "Exception of type 'Exception' was thrown.".to_owned() }
    }

    /// An exception with `message`.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for TestException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for TestException {}

/// What the tests format with.
pub struct FormatHelper;

impl FormatHelper {
    /// `Environment.NewLine`.
    pub const NEW_LINE: &'static str = "\n";

    /// A new exception (`FormatHelper.Exception`).
    pub fn exception() -> TestException {
        TestException::new()
    }

    /// `message.Trim(FormatHelper.NewLine).Trim()`.
    pub fn trim(message: &str) -> &str {
        message.trim_matches(['\r', '\n']).trim()
    }
}
