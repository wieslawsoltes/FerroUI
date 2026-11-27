//! The UI thread of the tests, and helpers to call the ABI with Rust strings.

use crate::*;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::mpsc::{channel, Sender};
use std::sync::{LazyLock, Mutex};

type Job = Box<dyn FnOnce() + Send>;

/// The UI thread: it made itself the UI thread with `ferro_init` before its first job.
static UI: LazyLock<Mutex<Sender<Job>>> = LazyLock::new(|| {
    let (sender, receiver) = channel::<Job>();
    std::thread::Builder::new()
        .name("ferroui-capi-tests-ui".into())
        .spawn(move || {
            assert_eq!(ferro_init(), FERRO_OK, "ferro_init: {}", last_error());
            for job in receiver {
                job();
            }
        })
        .expect("the UI thread of the tests starts");
    Mutex::new(sender)
});

/// Runs `body` on the UI thread of the tests and returns its result; a panic of `body` (a failed
/// assertion) is the panic of the caller.
pub(crate) fn ui<R: Send + 'static>(body: impl FnOnce() -> R + Send + 'static) -> R {
    let (sender, receiver) = channel();
    let job: Job = Box::new(move || {
        let _ = sender.send(catch_unwind(AssertUnwindSafe(body)));
    });
    UI.lock().unwrap_or_else(|error| error.into_inner()).send(job).expect("the UI thread runs");
    match receiver.recv().expect("the UI thread answers") {
        Ok(result) => result,
        Err(panic) => resume_unwind(panic),
    }
}

/// The message of the last failure of the calling thread.
pub(crate) fn last_error() -> String {
    // SAFETY: `ferro_last_error` returns a NUL-terminated string.
    unsafe { std::ffi::CStr::from_ptr(ferro_last_error()) }.to_string_lossy().into_owned()
}

pub(crate) fn type_id(name: &str) -> i64 {
    // SAFETY: a pointer and the length of a Rust string.
    unsafe { ferro_type_id(name.as_ptr().cast(), name.len() as i64) }
}

pub(crate) fn property_id(name: &str) -> i64 {
    // SAFETY: a pointer and the length of a Rust string.
    unsafe { ferro_property_id(name.as_ptr().cast(), name.len() as i64) }
}

pub(crate) fn event_id(name: &str) -> i64 {
    // SAFETY: a pointer and the length of a Rust string.
    unsafe { ferro_event_id(name.as_ptr().cast(), name.len() as i64) }
}

/// Creates an object of the type `name`, on the UI thread.
pub(crate) fn create(name: &str) -> FerroHandle {
    let handle = ferro_create(type_id(name));
    assert_ne!(handle, 0, "ferro_create({name}): {}", last_error());
    handle
}

pub(crate) fn set_str(object: FerroHandle, property: &str, text: &str) -> FerroStatus {
    // SAFETY: a pointer and the length of a Rust string.
    unsafe { ferro_set_str(object, property_id(property), text.as_ptr().cast(), text.len() as i64) }
}

/// The text of a property, read through `ferro_get_str`.
pub(crate) fn get_str(object: FerroHandle, property: &str) -> Result<String, FerroStatus> {
    let id = property_id(property);
    let mut length = 0i64;
    // SAFETY: a NULL buffer with a capacity of 0 asks for the length.
    let code = unsafe { ferro_get_str(object, id, std::ptr::null_mut(), 0, &mut length) };
    if code != FERRO_BUFFER_TOO_SMALL {
        return Err(code);
    }
    let mut buffer = vec![0u8; length as usize + 1];
    // SAFETY: the buffer has the capacity given.
    let code = unsafe { ferro_get_str(object, id, buffer.as_mut_ptr().cast(), buffer.len() as i64, &mut length) };
    if code != FERRO_OK {
        return Err(code);
    }
    buffer.truncate(length as usize);
    Ok(String::from_utf8(buffer).expect("UTF-8"))
}

pub(crate) fn get_f64(object: FerroHandle, property: &str) -> Result<f64, FerroStatus> {
    let mut value = 0.0;
    // SAFETY: a pointer to a local.
    match unsafe { ferro_get_f64(object, property_id(property), &mut value) } {
        FERRO_OK => Ok(value),
        code => Err(code),
    }
}

pub(crate) fn get_i64(object: FerroHandle, property: &str) -> Result<i64, FerroStatus> {
    let mut value = 0;
    // SAFETY: a pointer to a local.
    match unsafe { ferro_get_i64(object, property_id(property), &mut value) } {
        FERRO_OK => Ok(value),
        code => Err(code),
    }
}

pub(crate) fn get_bool(object: FerroHandle, property: &str) -> Result<bool, FerroStatus> {
    let mut value = false;
    // SAFETY: a pointer to a local.
    match unsafe { ferro_get_bool(object, property_id(property), &mut value) } {
        FERRO_OK => Ok(value),
        code => Err(code),
    }
}

pub(crate) fn get_handle(object: FerroHandle, property: &str) -> Result<FerroHandle, FerroStatus> {
    let mut value = 0;
    // SAFETY: a pointer to a local.
    match unsafe { ferro_get_handle(object, property_id(property), &mut value) } {
        FERRO_OK => Ok(value),
        code => Err(code),
    }
}
