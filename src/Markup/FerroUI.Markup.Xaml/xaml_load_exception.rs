//! Port of `XamlLoadException.cs`.

use std::fmt;
use std::rc::Rc;

/// The error raised when a markup document cannot be loaded, or when a
/// member of the markup runtime fails on its input.
///
/// Generated factories propagate this error through every fallible instruction.
/// Keep the success ABI small and allocate the diagnostic only on failure.
#[derive(Clone, Default)]
pub struct XamlLoadError(Rc<ErrorDetails>);

#[derive(Default)]
struct ErrorDetails {
    message: String,
    inner: Option<Rc<dyn std::error::Error>>,
}

/// The name of [`XamlLoadError`] before 0.5.
#[deprecated(since = "0.5.0", note = "use `XamlLoadError`")]
pub type XamlLoadException = XamlLoadError;

impl XamlLoadError {
    /// An error without a message.
    pub fn new() -> Self {
        Self::default()
    }

    /// An error with a message.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self(Rc::new(ErrorDetails { message: message.into(), inner: None }))
    }

    /// An error with a message and the error that caused it.
    pub fn with_inner(message: impl Into<String>, inner_exception: impl std::error::Error + 'static) -> Self {
        Self(Rc::new(ErrorDetails { message: message.into(), inner: Some(Rc::new(inner_exception)) }))
    }

    pub fn message(&self) -> &str {
        &self.0.message
    }

    /// The error that caused this one.
    pub fn inner_exception(&self) -> Option<&Rc<dyn std::error::Error>> {
        self.0.inner.as_ref()
    }
}

impl fmt::Debug for XamlLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XamlLoadError").field("message", &self.0.message).field("inner", &self.0.inner).finish()
    }
}

impl fmt::Display for XamlLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.message)
    }
}

impl std::error::Error for XamlLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.inner.as_deref()
    }
}

impl PartialEq for XamlLoadError {
    fn eq(&self, other: &Self) -> bool {
        self.message() == other.message()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn carries_message_and_inner_error() {
        assert_eq!(XamlLoadError::new().message(), "");
        let e = XamlLoadError::with_message("boom");
        assert_eq!(e.to_string(), "boom");
        assert!(e.inner_exception().is_none());

        let outer = XamlLoadError::with_inner("outer", e.clone());
        assert_eq!(outer.message(), "outer");
        assert_eq!(outer.source().unwrap().to_string(), "boom");
        let clone = outer.clone();
        assert!(Rc::ptr_eq(outer.inner_exception().unwrap(), clone.inner_exception().unwrap()));
        assert_eq!(clone, XamlLoadError::with_message("outer"));
    }
}
