//! Strings across the boundary: UTF-8 with a length, borrowed for the duration of the call. A
//! negative length means a NUL-terminated string (the SQLite convention), which is what a language
//! whose strings are C strings passes.

use crate::error::{Error, Result};
use crate::{FERRO_INVALID_ARGUMENT, FERRO_INVALID_UTF8, FERRO_NULL_POINTER};
use std::ffi::{c_char, CStr};

/// The string `text` points at: `length` bytes, or up to the NUL when `length` is negative.
/// `NULL` with a length of `0` is the empty string.
///
/// # Safety
/// `text` is `NULL` or points at `length` readable bytes (at a NUL-terminated string when `length`
/// is negative) that stay unchanged for `'a`.
pub(crate) unsafe fn borrowed<'a>(text: *const c_char, length: i64) -> Result<&'a str> {
    if text.is_null() {
        return if length == 0 { Ok("") } else { Err(Error::new(FERRO_NULL_POINTER, "the string is NULL")) };
    }
    let bytes = if length < 0 {
        // SAFETY: the caller passes a NUL-terminated string for a negative length.
        unsafe { CStr::from_ptr(text) }.to_bytes()
    } else {
        let length = usize::try_from(length).map_err(|_| Error::new(FERRO_INVALID_ARGUMENT, "the string is too long"))?;
        // SAFETY: the caller passes `length` readable bytes.
        unsafe { std::slice::from_raw_parts(text.cast::<u8>(), length) }
    };
    std::str::from_utf8(bytes).map_err(|error| Error::new(FERRO_INVALID_UTF8, format!("the string is not UTF-8: {error}")))
}
