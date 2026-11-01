//! Port of `Vnc/VncException.cs` of RemoteViewing (BSD-2-Clause, Copyright
//! (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

use super::VncFailureReason;
use std::error::Error;
use std::fmt;

/// The error of a problem with the VNC client-server communication.
#[derive(Debug)]
pub struct VncError {
    message: String,
    reason: VncFailureReason,
    inner_exception: Option<Box<dyn Error + Send + Sync>>,
}

/// The name of [`VncError`] before 0.5.
#[deprecated(since = "0.5.0", note = "use `VncError`")]
pub type VncException = VncError;

impl VncError {
    /// Initializes a new instance of the [`VncError`] class, with a
    /// `reason` and optionally the error that caused it.
    pub fn new(
        message: impl Into<String>,
        reason: VncFailureReason,
        inner_exception: Option<Box<dyn Error + Send + Sync>>,
    ) -> Self {
        Self {
            message: message.into(),
            reason,
            inner_exception,
        }
    }

    /// The error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Gets the reason the exception was raised.
    pub fn reason(&self) -> VncFailureReason {
        self.reason
    }
}

impl Default for VncError {
    /// The exception without a reason: "VNC error.".
    fn default() -> Self {
        Self::new("VNC error.", VncFailureReason::Unknown, None)
    }
}

impl fmt::Display for VncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for VncError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner_exception
            .as_deref()
            .map(|inner| inner as &(dyn Error + 'static))
    }
}
