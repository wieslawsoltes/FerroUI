//! Status codes, the message of the last failure of a thread, and the boundary every exported
//! function goes through: a framework error becomes a status, and a panic never unwinds into the
//! caller.

use crate::{FerroStatus, FERRO_OK, FERRO_PANIC};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

ferroui_base::ferro_thread_local! {
    /// The code of the last failure on this thread ([`ferro_last_error_code`]).
    static LAST_CODE: Cell<FerroStatus> = const { Cell::new(FERRO_OK) };
    /// The message of the last failure on this thread ([`ferro_last_error`]) with its NUL, in a
    /// buffer kept from one failure to the next: clearing it allocates nothing.
    static LAST_MESSAGE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// A failure of a call: its status code and a message for [`ferro_last_error`].
#[derive(Debug)]
pub(crate) struct Error {
    pub(crate) code: FerroStatus,
    pub(crate) message: Cow<'static, str>,
}

impl Error {
    #[cold]
    pub(crate) fn new(code: FerroStatus, message: impl Into<Cow<'static, str>>) -> Self {
        Self { code, message: message.into() }
    }

    /// Records the failure as the last one of the thread and returns its code.
    #[cold]
    pub(crate) fn record(self) -> FerroStatus {
        record(self.code, &self.message);
        self.code
    }
}

pub(crate) type Result<T> = std::result::Result<T, Error>;

/// Records `code` and `message` as the last failure of the thread.
#[cold]
fn record(code: FerroStatus, message: &str) {
    // A message cannot hold a NUL in C: one inside the text ends it there.
    let text = message.split('\0').next().unwrap_or_default();
    let _ = LAST_CODE.try_with(|last| last.set(code));
    let _ = LAST_MESSAGE.try_with(|last| {
        let mut last = last.borrow_mut();
        last.clear();
        last.extend_from_slice(text.as_bytes());
        last.push(0);
    });
}

/// The text of a panic payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        format!("the framework panicked: {text}")
    } else if let Some(text) = payload.downcast_ref::<String>() {
        format!("the framework panicked: {text}")
    } else {
        "the framework panicked".to_string()
    }
}

/// Runs the body of an exported function: a panic is recorded as [`FERRO_PANIC`] and `on_panic`
/// is returned in place of the result, so nothing unwinds across the boundary.
#[inline]
pub(crate) fn guard<R>(on_panic: R, body: impl FnOnce() -> R) -> R {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(payload) => {
            record(FERRO_PANIC, &panic_message(&*payload));
            on_panic
        }
    }
}

/// [`guard`] for a function that returns a status: `Ok` is [`FERRO_OK`], an error is recorded and
/// its code returned.
#[inline]
pub(crate) fn status(body: impl FnOnce() -> Result<()>) -> FerroStatus {
    guard(FERRO_PANIC, || match body() {
        Ok(()) => FERRO_OK,
        Err(error) => error.record(),
    })
}

/// [`guard`] for a function that returns a positive integer (an ID, a token) or a negative status.
#[inline]
pub(crate) fn status_or_value(body: impl FnOnce() -> Result<i64>) -> i64 {
    guard(i64::from(FERRO_PANIC), || match body() {
        Ok(value) => value,
        Err(error) => i64::from(error.record()),
    })
}

/// The message of the last failure of the calling thread.
pub(crate) fn last_message() -> String {
    LAST_MESSAGE
        .try_with(|last| String::from_utf8_lossy(last.borrow().strip_suffix(&[0]).unwrap_or_default()).into_owned())
        .unwrap_or_default()
}

/// The message of the last call that failed on the calling thread, as UTF-8 with a NUL at the end;
/// an empty string when none has failed. Successful calls leave it as it is (as `errno`). The text
/// stays valid until the next failure on the thread. Any thread.
#[no_mangle]
pub extern "C" fn ferro_last_error() -> *const c_char {
    static EMPTY: &[u8] = b"\0";
    LAST_MESSAGE
        .try_with(|last| {
            let last = last.borrow();
            if last.is_empty() {
                EMPTY.as_ptr()
            } else {
                last.as_ptr()
            }
        })
        .unwrap_or(EMPTY.as_ptr())
        .cast()
}

/// The status code of the last call that failed on the calling thread ([`FERRO_OK`] when none has
/// failed): the code of a function that returns a handle, which is `0` on a failure. Any thread.
#[no_mangle]
pub extern "C" fn ferro_last_error_code() -> FerroStatus {
    LAST_CODE.try_with(Cell::get).unwrap_or(FERRO_OK)
}

/// Forgets the last failure of the calling thread: [`ferro_last_error_code`] is [`FERRO_OK`] and
/// [`ferro_last_error`] empty until the next failure. A binding whose language reports errors as
/// values (Osprey's `Result`) clears it before a call whose result does not tell a failure apart
/// (a getter with a fallback) and reads the code after it. Any thread.
#[no_mangle]
pub extern "C" fn ferro_clear_error() {
    let _ = LAST_CODE.try_with(|last| last.set(FERRO_OK));
    let _ = LAST_MESSAGE.try_with(|last| last.borrow_mut().clear());
}
