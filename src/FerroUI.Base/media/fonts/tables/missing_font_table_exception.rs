// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

use std::fmt;

/// The error font loading reports when it finds a required table is missing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingFontTableException {
    message: String,
    table: String,
}

impl MissingFontTableException {
    /// Creates the error from the message that describes it and the table
    /// where it originated.
    pub fn new(message: impl Into<String>, table: impl Into<String>) -> Self {
        Self { message: message.into(), table: table.into() }
    }

    /// Gets the message that describes the error.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Gets the table where the error originated.
    pub fn table(&self) -> &str {
        &self.table
    }
}

impl fmt::Display for MissingFontTableException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MissingFontTableException {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_message_and_table() {
        let error = MissingFontTableException::new("no table", "cmap");

        assert_eq!(error.message(), "no table");
        assert_eq!(error.table(), "cmap");
        assert_eq!(error.to_string(), "no table");
    }
}
