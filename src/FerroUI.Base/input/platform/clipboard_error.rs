use std::error::Error;
use std::fmt;

/// The category of a failed clipboard operation.
///
/// The reference reports a failed clipboard operation with an exception
/// whose type tells what happened; the kinds mirror the exception types the
/// platform clipboards raise:
///
/// | Kind | Reference exception |
/// |---|---|
/// | [`Timeout`](Self::Timeout) | `TimeoutException` |
/// | [`Canceled`](Self::Canceled) | `OperationCanceledException` (and `TaskCanceledException`) |
/// | [`AccessDenied`](Self::AccessDenied) | `UnauthorizedAccessException` |
/// | [`Platform`](Self::Platform) | `COMException` (an `ExternalException` carrying a result code) |
/// | [`Other`](Self::Other) | every other exception |
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClipboardErrorKind {
    /// The clipboard could not be opened or did not answer in time, usually
    /// because another application holds it.
    Timeout,
    /// The operation was canceled before it completed.
    Canceled,
    /// The platform denied access to the clipboard.
    AccessDenied,
    /// A call of the platform clipboard interface failed with a result
    /// code (see [`ClipboardError::code`]).
    Platform,
    /// Any other failure: one a platform clipboard is not expected to
    /// report.
    Other,
}

impl ClipboardErrorKind {
    fn description(self) -> &'static str {
        match self {
            ClipboardErrorKind::Timeout => "The clipboard operation has timed out.",
            ClipboardErrorKind::Canceled => "The clipboard operation was canceled.",
            ClipboardErrorKind::AccessDenied => "Access to the clipboard was denied.",
            ClipboardErrorKind::Platform => "A platform clipboard call failed.",
            ClipboardErrorKind::Other => "The clipboard operation failed.",
        }
    }
}

/// The failure of a clipboard operation: what a platform clipboard reports
/// instead of a result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardError {
    kind: ClipboardErrorKind,
    message: String,
    code: Option<i32>,
}

impl ClipboardError {
    /// Creates an error of a kind with a message.
    pub fn new(kind: ClipboardErrorKind, message: impl Into<String>) -> ClipboardError {
        ClipboardError { kind, message: message.into(), code: None }
    }

    /// Creates an error of a kind with the default message of the kind.
    pub fn from_kind(kind: ClipboardErrorKind) -> ClipboardError {
        ClipboardError::new(kind, kind.description())
    }

    /// The clipboard could not be opened or did not answer in time.
    pub fn timeout(message: impl Into<String>) -> ClipboardError {
        ClipboardError::new(ClipboardErrorKind::Timeout, message)
    }

    /// The operation was canceled.
    pub fn canceled(message: impl Into<String>) -> ClipboardError {
        ClipboardError::new(ClipboardErrorKind::Canceled, message)
    }

    /// The platform denied access to the clipboard.
    pub fn access_denied(message: impl Into<String>) -> ClipboardError {
        ClipboardError::new(ClipboardErrorKind::AccessDenied, message)
    }

    /// A call of the platform clipboard interface failed with the result
    /// code `code`.
    pub fn platform(code: i32, message: impl Into<String>) -> ClipboardError {
        ClipboardError { kind: ClipboardErrorKind::Platform, message: message.into(), code: Some(code) }
    }

    /// A failure a platform clipboard is not expected to report.
    pub fn other(message: impl Into<String>) -> ClipboardError {
        ClipboardError::new(ClipboardErrorKind::Other, message)
    }

    /// The category of the failure.
    pub fn kind(&self) -> ClipboardErrorKind {
        self.kind
    }

    /// The message describing the failure.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The result code of the platform call that failed, for
    /// [`ClipboardErrorKind::Platform`] errors.
    pub fn code(&self) -> Option<i32> {
        self.code
    }
}

impl From<ClipboardErrorKind> for ClipboardError {
    fn from(kind: ClipboardErrorKind) -> ClipboardError {
        ClipboardError::from_kind(kind)
    }
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        if let Some(code) = self.code {
            write!(f, " (0x{:08X})", code as u32)?;
        }
        Ok(())
    }
}

impl Error for ClipboardError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_set_the_kind_the_message_and_the_code() {
        let cases = [
            (ClipboardError::timeout("a"), ClipboardErrorKind::Timeout),
            (ClipboardError::canceled("a"), ClipboardErrorKind::Canceled),
            (ClipboardError::access_denied("a"), ClipboardErrorKind::AccessDenied),
            (ClipboardError::other("a"), ClipboardErrorKind::Other),
        ];
        for (error, kind) in cases {
            assert_eq!(kind, error.kind());
            assert_eq!("a", error.message());
            assert_eq!(None, error.code());
            assert_eq!("a", error.to_string());
        }

        let error = ClipboardError::platform(0x8000_4005_u32 as i32, "Native call failed");
        assert_eq!(ClipboardErrorKind::Platform, error.kind());
        assert_eq!(Some(0x8000_4005_u32 as i32), error.code());
        assert_eq!("Native call failed (0x80004005)", error.to_string());
    }

    #[test]
    fn an_error_of_a_kind_has_the_message_of_the_kind() {
        let error: ClipboardError = ClipboardErrorKind::Timeout.into();
        assert_eq!(ClipboardErrorKind::Timeout, error.kind());
        assert_eq!("The clipboard operation has timed out.", error.message());
        assert_eq!(error, ClipboardError::from_kind(ClipboardErrorKind::Timeout));
        assert_ne!(error, ClipboardError::from_kind(ClipboardErrorKind::Canceled));

        let error: &dyn Error = &error;
        assert!(error.source().is_none());
    }
}
