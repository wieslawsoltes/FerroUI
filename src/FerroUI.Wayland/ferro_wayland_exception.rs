//! The errors of the Wayland backend (the port of the exception classes of
//! the reference, which form a hierarchy: a poll, a network and a protocol
//! error are Wayland errors; a flush and a read error are network errors).

use std::fmt;

/// A failure of the Wayland backend. The reference throws it; here it is
/// the error value of the calls that can fail, and the payload of the panic
/// that takes the place of an exception nobody catches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FerroWaylandError {
    kind: FerroWaylandErrorKind,
    message: String,
    inner: Option<String>,
}

/// The name of [`FerroWaylandError`] before 0.5.
#[deprecated(since = "0.5.0", note = "use `FerroWaylandError`")]
pub type FerroWaylandException = FerroWaylandError;

/// Which class of the reference an error is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FerroWaylandErrorKind {
    /// The base class.
    General,
    /// `poll` failed.
    Poll,
    /// The base class of the errors of the socket.
    Network,
    /// The requests could not be written to the socket.
    Flush,
    /// The events could not be read from the socket.
    Read,
    /// The compositor reported a protocol error.
    ProtocolError { error_code: u32, error_message: Option<String> },
}

/// The name of [`FerroWaylandErrorKind`] before 0.5.
#[deprecated(since = "0.5.0", note = "use `FerroWaylandErrorKind`")]
pub type FerroWaylandExceptionKind = FerroWaylandErrorKind;

impl FerroWaylandError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { kind: FerroWaylandErrorKind::General, message: message.into(), inner: None }
    }

    /// An error with the error that caused it.
    pub fn with_inner(message: impl Into<String>, inner: impl fmt::Display) -> Self {
        Self { kind: FerroWaylandErrorKind::General, message: message.into(), inner: Some(inner.to_string()) }
    }

    fn of_kind(kind: FerroWaylandErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into(), inner: None }
    }

    pub fn kind(&self) -> &FerroWaylandErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// The message of the error that caused this one.
    pub fn inner(&self) -> Option<&str> {
        self.inner.as_deref()
    }

    /// Whether the error is of the network classes (`FerroWaylandNetworkException` and the two
    /// derived from it).
    pub fn is_network(&self) -> bool {
        matches!(
            self.kind,
            FerroWaylandErrorKind::Network | FerroWaylandErrorKind::Flush | FerroWaylandErrorKind::Read
        )
    }

    /// Fails with this error.
    pub fn throw(self) -> ! {
        panic!("{self}")
    }
}

impl fmt::Display for FerroWaylandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        if let Some(inner) = &self.inner {
            write!(f, " ---> {inner}")?;
        }
        Ok(())
    }
}

impl std::error::Error for FerroWaylandError {}

/// `poll` failed.
pub struct FerroWaylandPollException;

impl FerroWaylandPollException {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> FerroWaylandError {
        FerroWaylandError::of_kind(FerroWaylandErrorKind::Poll, "poll failed")
    }
}

/// An error of the socket.
pub struct FerroWaylandNetworkException;

impl FerroWaylandNetworkException {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(message: impl Into<String>) -> FerroWaylandError {
        FerroWaylandError::of_kind(FerroWaylandErrorKind::Network, message)
    }
}

/// The requests could not be written to the socket.
pub struct FerroWaylandFlushException;

impl FerroWaylandFlushException {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(message: impl Into<String>) -> FerroWaylandError {
        FerroWaylandError::of_kind(FerroWaylandErrorKind::Flush, message)
    }

    /// The error for an error number of the C library.
    pub fn from_errno(errno: i32) -> FerroWaylandError {
        Self::new(format!("wl_display_flush failed, errno: {errno}"))
    }
}

/// The events could not be read from the socket.
pub struct FerroWaylandReadException;

impl FerroWaylandReadException {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(message: impl Into<String>) -> FerroWaylandError {
        FerroWaylandError::of_kind(FerroWaylandErrorKind::Read, message)
    }

    /// The error for an error number of the C library.
    pub fn from_errno(errno: i32) -> FerroWaylandError {
        Self::new(format!("wl_display_read_events failed, errno: {errno}"))
    }
}

/// The compositor reported a protocol error.
pub struct FerroWaylandProtocolErrorException;

impl FerroWaylandProtocolErrorException {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> FerroWaylandError {
        FerroWaylandError::of_kind(
            FerroWaylandErrorKind::ProtocolError { error_code: 0, error_message: None },
            "protocol error",
        )
    }

    /// The error with the code and the message of the compositor.
    pub fn with_code(error_code: u32, error_message: impl Into<String>) -> FerroWaylandError {
        let error_message = error_message.into();
        FerroWaylandError::of_kind(
            FerroWaylandErrorKind::ProtocolError { error_code, error_message: Some(error_message.clone()) },
            format!("protocol error: {error_code} {error_message}"),
        )
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_classes_keep_their_messages_and_their_place_in_the_hierarchy() {
        assert_eq!(FerroWaylandPollException::new().message(), "poll failed");
        assert!(!FerroWaylandPollException::new().is_network());
        assert_eq!(FerroWaylandFlushException::from_errno(32).message(), "wl_display_flush failed, errno: 32");
        assert!(FerroWaylandFlushException::from_errno(32).is_network());
        assert_eq!(FerroWaylandReadException::from_errno(104).message(), "wl_display_read_events failed, errno: 104");
        assert!(FerroWaylandReadException::from_errno(104).is_network());
        assert!(FerroWaylandNetworkException::new("x").is_network());
        assert_eq!(FerroWaylandProtocolErrorException::new().message(), "protocol error");
        let error = FerroWaylandProtocolErrorException::with_code(3, "bad surface");
        assert_eq!(error.message(), "protocol error: 3 bad surface");
        assert_eq!(
            error.kind(),
            &FerroWaylandErrorKind::ProtocolError { error_code: 3, error_message: Some("bad surface".to_string()) }
        );
    }

    #[test]
    fn an_error_shows_what_caused_it() {
        let error = FerroWaylandError::with_inner("Unable to connect to Wayland display", "no socket");
        assert_eq!(error.to_string(), "Unable to connect to Wayland display ---> no socket");
        assert_eq!(error.inner(), Some("no socket"));
    }
}
