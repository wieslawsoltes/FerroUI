//! The universal C ABI of FerroUI (#636): one set of `extern "C"` functions over opaque integer
//! handles that every language binds to, the way SQLite, Skia and wgpu-native are consumed. The C
//! header `include/ferroui.h` is generated from this crate by cbindgen (never written by hand),
//! and a unit test fails when it is out of date (`tests/header_tests.rs`). The design, the rules
//! and a hello world are in docs/c-abi.md.
//!
//! Not from upstream: the framework this one is ported from has no C ABI.
//!
//! # The rules, in short
//!
//! - **Handles** ([`FerroHandle`]) are 64-bit integers: a slot of the handle table of the UI
//!   thread and a generation, so a stale handle is a checked error ([`FERRO_BAD_HANDLE`]) and
//!   never a dangling pointer. `0` is no object. A handle counts references
//!   ([`ferro_retain`], [`ferro_release`]); one object has one handle at a time.
//! - **IDs.** Types, properties and events are addressed by integer IDs resolved once by name
//!   ([`ferro_type_id`], [`ferro_property_id`], [`ferro_event_id`]), never by a string on every
//!   call. A name is UTF-8 with a length; a negative length means NUL-terminated.
//! - **Values** cross as scalars: a setter and a getter per kind ([`ferro_set_str`],
//!   [`ferro_set_f64`], [`ferro_set_i64`] (enumerations as integers), [`ferro_set_bool`],
//!   [`ferro_set_handle`]), which cover every property without a function per control.
//! - **Errors.** Functions return a [`FerroStatus`] (`0` is success, errors are negative), and
//!   [`ferro_last_error`] gives the message of the last failure on the calling thread. Nothing
//!   unwinds across the boundary: every exported function catches a panic and returns
//!   [`FERRO_PANIC`]; the shipped profiles abort on a panic instead, and the checks before every
//!   write (handle, thread, type, read-only, validation) turn what would panic into a status.
//! - **Threads.** UI objects belong to the UI thread: the thread that called [`ferro_init`] (or
//!   [`ferro_app_run`], which calls it). A call from another thread returns
//!   [`FERRO_WRONG_THREAD`]; [`ferro_post`] runs work on the UI thread from any thread. The ID
//!   lookups work on any thread.
//! - **Frames** ([`ferro_frame_commit`]): an interface described again on every update as records
//!   (open an element, set scalars, bind events to message tags, close), reconciled into the
//!   retained tree of a root; events call one dispatcher with their tag
//!   ([`ferro_frame_dispatcher`]). For Elm-like bindings (docs/bindings-design.md).
//! - **Callbacks** are a C function pointer and an `int64_t` token ([`FerroEventFn`]), which
//!   every language can make (a language without closures dispatches on the token), or a
//!   function with a `void *` and a release function for it ([`FerroEventDataFn`]).

#![allow(clippy::missing_safety_doc)]

mod app;
// The stable IDs of the API model, generated (scripts/generate_api_model.py).
mod api_ids;
mod error;
mod events;
mod frame;
mod handles;
mod ids;
mod signals;
mod strings;
mod thread;
mod tree;
mod values;

pub use app::{ferro_app_run, ferro_close, ferro_quit, ferro_show};
pub use error::{ferro_clear_error, ferro_last_error, ferro_last_error_code};
pub use events::{
    ferro_raise, ferro_subscribe, ferro_subscribe_data, ferro_subscribe_dispatch, ferro_unsubscribe,
};
pub use frame::{
    ferro_frame_begin, ferro_frame_bind, ferro_frame_bool, ferro_frame_clear, ferro_frame_close, ferro_frame_commit,
    ferro_frame_commit_bytes, ferro_frame_dispatch, ferro_frame_dispatcher, ferro_frame_event_f64,
    ferro_frame_event_text, ferro_frame_f64, ferro_frame_find, ferro_frame_i64, ferro_frame_key_str, ferro_frame_memo,
    ferro_frame_on, ferro_frame_open, ferro_frame_reuse, ferro_frame_str, ferro_frame_str16,
};
pub use handles::{ferro_create, ferro_release, ferro_retain, ferro_type_of, handle_of, object_of};
pub use ids::{ferro_api_fingerprint, ferro_event_id, ferro_property_id, ferro_type_id};
pub use signals::{
    ferro_signal_bind, ferro_signal_free, ferro_signal_get_bool, ferro_signal_get_cstr, ferro_signal_get_f64,
    ferro_signal_get_i64, ferro_signal_new, ferro_signal_set_bool, ferro_signal_set_f64, ferro_signal_set_handle,
    ferro_signal_set_i64, ferro_signal_set_str, ferro_signal_table, ferro_signal_take_changed, ferro_signal_unbind,
    ferro_signal_version, ferro_signal_watch, ferro_signals_apply, ferro_signals_init, FerroSignalTable,
    FERRO_SIGNAL_BOOL, FERRO_SIGNAL_F64, FERRO_SIGNAL_FREE, FERRO_SIGNAL_HANDLE, FERRO_SIGNAL_I64, FERRO_SIGNAL_LAYOUT,
    FERRO_SIGNAL_TEXT,
};
pub use thread::{ferro_init, ferro_is_ui_thread, ferro_post, ferro_post_data, ferro_run_jobs};
pub use tree::{ferro_add_child, ferro_remove_child};
pub use values::{
    ferro_clear, ferro_get_bool, ferro_get_bool_or, ferro_get_cstr, ferro_get_cstr_or, ferro_get_f64, ferro_get_f64_or,
    ferro_get_handle, ferro_get_i64, ferro_get_i64_or, ferro_get_str, ferro_set_bool, ferro_set_f64, ferro_set_handle,
    ferro_set_i64, ferro_set_str, ferro_set_str16,
};

use std::ffi::c_void;

/// An object of the UI thread: a slot of the handle table and its generation. `0` is no object.
pub type FerroHandle = u64;

/// The result of a call: [`FERRO_OK`] (`0`), or a negative error code.
pub type FerroStatus = i32;

/// The version of the ABI this library implements ([`ferro_abi_version`]). Additions keep the
/// version; a change that breaks a binding raises it.
pub const FERRO_ABI_VERSION: i32 = 1;

/// The call succeeded.
pub const FERRO_OK: FerroStatus = 0;
/// A failure that no other code describes; the message says what happened.
pub const FERRO_ERROR: FerroStatus = -1;
/// An argument is out of range (an ID that is not positive, an empty name, a negative capacity).
pub const FERRO_INVALID_ARGUMENT: FerroStatus = -2;
/// A pointer argument that must not be `NULL` is `NULL`.
pub const FERRO_NULL_POINTER: FerroStatus = -3;
/// A string argument is not UTF-8.
pub const FERRO_INVALID_UTF8: FerroStatus = -4;
/// No type, property or event has the name or ID.
pub const FERRO_NOT_FOUND: FerroStatus = -5;
/// The handle is `0`, was released, or was never issued.
pub const FERRO_BAD_HANDLE: FerroStatus = -6;
/// The function was called on a thread that is not the UI thread.
pub const FERRO_WRONG_THREAD: FerroStatus = -7;
/// No thread has been made the UI thread yet ([`ferro_init`], [`ferro_app_run`]).
pub const FERRO_NOT_INITIALIZED: FerroStatus = -8;
/// The value cannot be converted to the type of the property, or the object is not of the kind
/// the function needs (a child that is not a control, a window that is not a window).
pub const FERRO_TYPE_MISMATCH: FerroStatus = -9;
/// The property is read-only.
pub const FERRO_READ_ONLY: FerroStatus = -10;
/// The value is of the right type but the property rejects it (its validation).
pub const FERRO_INVALID_VALUE: FerroStatus = -11;
/// The operation is not available: in this build (`ferro_app_run` without the desktop
/// platform), or for this object (a child for an object that holds none).
pub const FERRO_NOT_SUPPORTED: FerroStatus = -12;
/// The operation does not fit the current state (a second `ferro_app_run`, a content that is
/// already set, a window shown before the application runs).
pub const FERRO_INVALID_STATE: FerroStatus = -13;
/// The buffer given to a getter is too small; the length it needs is written.
pub const FERRO_BUFFER_TOO_SMALL: FerroStatus = -14;
/// The framework panicked inside the call; the message is the panic message. Only builds that
/// unwind report it: the shipped profiles abort on a panic.
pub const FERRO_PANIC: FerroStatus = -15;
/// The start callback of [`ferro_app_run`] returned a value other than `0`.
pub const FERRO_START_FAILED: FerroStatus = -16;

/// The start callback of [`ferro_app_run`]: called on the UI thread once the application is set
/// up, with the token given to `ferro_app_run`. It returns `0` to run the main loop; any other
/// value ends `ferro_app_run` with [`FERRO_START_FAILED`].
pub type FerroStartFn = Option<unsafe extern "C" fn(token: i64) -> i64>;

/// Work posted to the UI thread with [`ferro_post`]: called with its token. The result is
/// ignored (a function that returns `int64_t` is what every language can make).
pub type FerroPostFn = Option<unsafe extern "C" fn(token: i64) -> i64>;

/// Work posted to the UI thread with [`ferro_post_data`]: called once with its user data.
pub type FerroPostDataFn = Option<unsafe extern "C" fn(user_data: *mut c_void)>;

/// The handler of an event ([`ferro_subscribe`]): called on the UI thread with the token of the
/// subscription, the handle of the object the handler is attached to (borrowed for the call:
/// [`ferro_retain`] it to keep it) and an argument (`0` for routed events in ABI version 1). A
/// result other than `0` marks a routed event as handled.
pub type FerroEventFn = Option<unsafe extern "C" fn(token: i64, sender: FerroHandle, arg: i64) -> i64>;

/// The handler of an event with user data ([`ferro_subscribe_data`]): as [`FerroEventFn`], with
/// the user data in place of the token.
pub type FerroEventDataFn = Option<unsafe extern "C" fn(user_data: *mut c_void, sender: FerroHandle, arg: i64) -> i64>;

/// Releases the user data of a subscription ([`ferro_subscribe_data`]) once the handler is
/// removed: by [`ferro_unsubscribe`], or when the object goes away.
pub type FerroReleaseFn = Option<unsafe extern "C" fn(user_data: *mut c_void)>;

/// The version of the ABI ([`FERRO_ABI_VERSION`]). A binding checks it before its first call.
/// Any thread.
#[no_mangle]
pub extern "C" fn ferro_abi_version() -> i32 {
    FERRO_ABI_VERSION
}

#[cfg(test)]
mod tests;
