//! Port of `XamlLoadException.cs`.

use std::fmt;
use std::rc::Rc;

/// The error raised when a markup document cannot be loaded, or when a
/// member of the markup runtime fails on its input.
#[derive(Clone, Debug, Default)]
pub struct XamlLoadException {
    message: String,
    inner: Option<Rc<dyn std::error::Error>>,
}

impl XamlLoadException {
    /// An error without a message.
    pub fn new() -> Self {
        Self::default()
    }

    /// An error with a message.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self { message: message.into(), inner: None }
    }

    /// An error with a message and the error that caused it.
    pub fn with_inner(message: impl Into<String>, inner_exception: impl std::error::Error + 'static) -> Self {
        Self { message: message.into(), inner: Some(Rc::new(inner_exception)) }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// The error that caused this one.
    pub fn inner_exception(&self) -> Option<&Rc<dyn std::error::Error>> {
        self.inner.as_ref()
    }
}

impl fmt::Display for XamlLoadException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for XamlLoadException {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.as_deref()
    }
}

impl PartialEq for XamlLoadException {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn carries_message_and_inner_error() {
        assert_eq!(XamlLoadException::new().message(), "");
        let e = XamlLoadException::with_message("boom");
        assert_eq!(e.to_string(), "boom");
        assert!(e.inner_exception().is_none());

        let outer = XamlLoadException::with_inner("outer", e.clone());
        assert_eq!(outer.message(), "outer");
        assert_eq!(outer.source().unwrap().to_string(), "boom");
    }
}
