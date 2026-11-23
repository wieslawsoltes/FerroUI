//! The C boundary of an Osprey application compiled as a library (`--target=macos`), and the
//! two imports its runtime calls back: `native_slice`, text the host holds for the application,
//! and `native_shape`, the scene primitives a resident application streams outside the envelope
//! (docs/osprey-native-host.md, section 2).
//!
//! Every export is called from the UI thread, one call at a time: `osprey_main` once, then the
//! application's `start` and `dispatch`. Strings passed in are borrowed for the call; a returned
//! string is copied at once and released with `osprey_release_string`.

use std::collections::HashMap;
use std::ffi::CString;
use std::sync::Mutex;

/// The calls a host makes into an application: the linked archive, or a stand-in in tests.
pub trait Boundary {
    /// The start envelope for the launch `context` (JSON).
    fn start(&mut self, context: &str) -> Result<String, String>;
    /// The next envelope for the saved `state` and one `event` (JSON).
    fn dispatch(&mut self, state: &str, event: &str) -> Result<String, String>;
}

/// The texts the host holds under their keys until the application writes them again.
static SLICES: Mutex<Option<HashMap<String, CString>>> = Mutex::new(None);

/// Holds `text` under `key` for the runtime's next `native_slice` call.
pub fn put_slice(key: &str, text: &str) -> Result<(), String> {
    let text =
        CString::new(text).map_err(|_| format!("the text of slice {key} holds a NUL byte"))?;
    let mut slices = SLICES
        .lock()
        .map_err(|_| "the slice store was poisoned".to_owned())?;
    slices
        .get_or_insert_with(HashMap::new)
        .insert(key.to_owned(), text);
    Ok(())
}

/// The text held under `key`, for a capability that reads a slice (`store-write`).
pub fn get_slice(key: &str) -> Option<String> {
    let slices = SLICES.lock().ok()?;
    slices
        .as_ref()?
        .get(key)
        .and_then(|text| text.to_str().ok())
        .map(str::to_owned)
}

/// One rect, disc or sprite streamed by a resident application during the current call: the
/// scene code, a sprite's atlas index, five values, the packed colour and the depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Streamed {
    pub code: i64,
    pub index: i64,
    pub values: [f64; 5],
    pub colour: i64,
    pub depth: i64,
}

/// The shapes streamed since the last `start` or `dispatch` began, in the order they came.
static STREAM: Mutex<Vec<Streamed>> = Mutex::new(Vec::new());

/// Appends one streamed shape (what `native_shape` does).
pub fn push_shape(shape: Streamed) {
    if let Ok(mut stream) = STREAM.lock() {
        stream.push(shape);
    }
}

/// Takes every shape streamed during the last call.
pub fn take_stream() -> Vec<Streamed> {
    STREAM
        .lock()
        .map(|mut stream| std::mem::take(&mut *stream))
        .unwrap_or_default()
}

#[cfg(osprey_app)]
pub use linked::Linked;

#[cfg(osprey_app)]
mod linked {
    use super::{push_shape, Boundary, Streamed, SLICES};
    use std::ffi::{c_char, CStr, CString};
    use std::sync::atomic::{AtomicBool, Ordering};

    extern "C" {
        /// Runs the application's top-level statements and `fn main`; called first.
        fn osprey_main() -> i32;
        /// The application's `start` export.
        fn osprey_app_start(source: *const c_char) -> *const c_char;
        /// The application's `dispatch` export.
        fn osprey_app_dispatch(saved: *const c_char, event: *const c_char) -> *const c_char;
        /// Releases one string an export returned.
        fn osprey_release_string(value: *const c_char);
        /// 1: the runtime copies a borrowed import string before the import returns.
        fn osprey_host_abi_version() -> i32;
    }

    /// The empty answer for a slice the host never held: never a null pointer.
    static EMPTY: &CStr = c"";

    /// The runtime's slice import: the text held under `key`, borrowed until the call returns
    /// (ABI 1 copies it at once). A missing, null or non-UTF-8 key reads as empty text.
    #[no_mangle]
    pub extern "C" fn native_slice(key: *const c_char) -> *const c_char {
        if key.is_null() {
            return EMPTY.as_ptr();
        }
        // SAFETY: the runtime passes a NUL-terminated string that outlives the call.
        let Ok(name) = unsafe { CStr::from_ptr(key) }.to_str() else {
            return EMPTY.as_ptr();
        };
        let Ok(slices) = SLICES.lock() else {
            return EMPTY.as_ptr();
        };
        // The text stays in the store, unchanged, until the export that asked returns: slices are
        // only written between calls, on the same thread.
        slices
            .as_ref()
            .and_then(|slices| slices.get(name))
            .map_or(EMPTY.as_ptr(), |text| text.as_ptr())
    }

    /// The runtime's scene-stream import: appends one primitive and answers 0.
    #[no_mangle]
    #[allow(
        clippy::too_many_arguments,
        reason = "the import's C signature has one scalar per field"
    )]
    pub extern "C" fn native_shape(
        code: i64,
        index: i64,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        colour: i64,
        depth: i64,
    ) -> i64 {
        push_shape(Streamed {
            code,
            index,
            values: [a, b, c, d, e],
            colour,
            depth,
        });
        0
    }

    static INITIALIZED: AtomicBool = AtomicBool::new(false);

    /// The application linked into this binary.
    pub struct Linked(());

    impl Linked {
        /// Checks the ownership ABI and runs `osprey_main` once.
        pub fn new() -> Result<Self, String> {
            // SAFETY: a scalar export without side effects.
            let version = unsafe { osprey_host_abi_version() };
            if version != 1 {
                return Err(format!(
                    "the application speaks import ownership ABI {version}; this host speaks 1"
                ));
            }
            if !INITIALIZED.swap(true, Ordering::SeqCst) {
                // SAFETY: called once, before any other export, on the thread that makes every call.
                let status = unsafe { osprey_main() };
                if status != 0 {
                    return Err(format!("osprey_main returned {status}"));
                }
            }
            Ok(Self(()))
        }
    }

    fn held(text: &str) -> Result<CString, String> {
        CString::new(text).map_err(|_| "text for the application holds a NUL byte".to_owned())
    }

    /// Copies and releases a string an export returned.
    fn copied(value: *const c_char) -> Result<String, String> {
        if value.is_null() {
            return Err("the application returned no envelope".to_owned());
        }
        // SAFETY: a non-null result is a NUL-terminated string valid until it is released.
        let text = unsafe { CStr::from_ptr(value) }
            .to_str()
            .map(str::to_owned)
            .map_err(|problem| {
                format!("the application returned text that is not UTF-8: {problem}")
            });
        // SAFETY: each non-null result hands the host exactly one reference.
        unsafe { osprey_release_string(value) };
        text
    }

    impl Boundary for Linked {
        fn start(&mut self, context: &str) -> Result<String, String> {
            let context = held(context)?;
            super::take_stream();
            // SAFETY: the argument outlives the call and the result is copied at once.
            copied(unsafe { osprey_app_start(context.as_ptr()) })
        }

        fn dispatch(&mut self, state: &str, event: &str) -> Result<String, String> {
            let state = held(state)?;
            let event = held(event)?;
            super::take_stream();
            // SAFETY: both arguments outlive the call and the result is copied at once.
            copied(unsafe { osprey_app_dispatch(state.as_ptr(), event.as_ptr()) })
        }
    }
}
