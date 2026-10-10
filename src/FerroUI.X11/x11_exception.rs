//! The error of the X11 backend (the port of `X11Exception.cs`).

use std::fmt;

/// A failure of the X11 backend. The reference throws it; here it is the
/// payload of the panic that takes its place (see the porting guide on
/// exceptions for programmer and environment errors).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X11Exception {
    message: String,
}

impl X11Exception {
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

impl fmt::Display for X11Exception {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for X11Exception {}
