//! The error handler of the connection (the port of `XError.cs`).

use crate::x11_exception::X11Exception;
use crate::xlib::{self, XErrorEvent};
use std::sync::{Mutex, PoisonError};

/// What the last error event of the server said. The reference keeps the
/// whole event; the display pointer of the event is left out here, which
/// nothing reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LastError {
    pub serial: u64,
    pub error_code: u8,
    pub request_code: u8,
    pub minor_code: u8,
    pub resourceid: u64,
}

static LAST_ERROR: Mutex<LastError> =
    Mutex::new(LastError { serial: 0, error_code: 0, request_code: 0, minor_code: 0, resourceid: 0 });

/// The error handling of the backend.
pub struct XError;

impl XError {
    /// `LastError`: the last error the server reported.
    pub fn last_error() -> LastError {
        *LAST_ERROR.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Forgets the last error, so that a later [`last_error`](Self::last_error)
    /// reports only what happened since.
    pub fn clear_last_error() {
        *LAST_ERROR.lock().unwrap_or_else(PoisonError::into_inner) = LastError::default();
    }

    unsafe extern "C" fn handler(_display: *mut x11_dl::xlib::Display, error: *mut XErrorEvent) -> i32 {
        // SAFETY: Xlib passes a valid error event for the duration of the
        // call (null is tolerated all the same).
        if let Some(error) = unsafe { error.as_ref() } {
            *LAST_ERROR.lock().unwrap_or_else(PoisonError::into_inner) = LastError {
                serial: error.serial as u64,
                error_code: error.error_code,
                request_code: error.request_code,
                minor_code: error.minor_code,
                resourceid: error.resourceid as u64,
            };
        }
        0
    }

    /// Fails with `desc` and the code of the last error, which it clears.
    pub fn throw_last_error(desc: &str) -> ! {
        let err = std::mem::take(&mut *LAST_ERROR.lock().unwrap_or_else(PoisonError::into_inner));
        X11Exception::new(Self::last_error_message(desc, err)).throw()
    }

    fn last_error_message(desc: &str, err: LastError) -> String {
        if err.error_code == 0 {
            desc.to_string()
        } else {
            format!("{desc}: {}", err.error_code)
        }
    }

    /// Installs the handler: errors of the server are recorded and do not
    /// end the process, which is what the default handler of Xlib does.
    pub fn init() {
        xlib::x_set_error_handler(Self::handler);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_message_names_the_error_code_when_there_is_one() {
        assert_eq!(XError::last_error_message("XCreateWindow failed", LastError::default()), "XCreateWindow failed");
        let error = LastError { error_code: 8, ..LastError::default() };
        assert_eq!(XError::last_error_message("XCreateWindow failed", error), "XCreateWindow failed: 8");
    }
}
