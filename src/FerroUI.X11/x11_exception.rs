//! The error of the X11 backend (the port of `X11Exception.cs`).

use std::fmt;

/// A failure of the X11 backend. The reference throws it; here it is the
/// payload of the panic that takes its place (see the porting guide on
/// exceptions for programmer and environment errors).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X11Error {
    message: String,
}

/// The name of [`X11Error`] before 0.5.
#[deprecated(since = "0.5.0", note = "use `X11Error`")]
pub type X11Exception = X11Error;

impl X11Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// Fails with this error.
    pub fn throw(self) -> ! {
        panic!("{}", self.message)
    }
}

impl fmt::Display for X11Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for X11Error {}
