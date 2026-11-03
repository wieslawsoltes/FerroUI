use super::{CollationError, Ordering, length, pointer};
use windows_sys::Win32::{
    Foundation::GetLastError,
    Globalization::{
        CompareStringEx, FIND_FROMSTART, FindNLSStringEx, LINGUISTIC_IGNORECASE,
        NORM_LINGUISTIC_CASING,
    },
};

pub(super) struct Collator {
    locale: Vec<u16>,
}
impl Collator {
    pub(super) fn new(locale: &str) -> Result<Self, CollationError> {
        let mut name: Vec<_> = locale.encode_utf16().collect();
        name.push(0);
        let result = Self { locale: name };
        // Validate with the actual NLS API before publishing this service.
        result.compare(&[], &[], false)?;
        Ok(result)
    }
    pub(super) fn compare(
        &self,
        left: &[u16],
        right: &[u16],
        ignore_case: bool,
    ) -> Result<Ordering, CollationError> {
        let (left_len, right_len) = (length(left)?, length(right)?);
        let flags = if ignore_case {
            LINGUISTIC_IGNORECASE | NORM_LINGUISTIC_CASING
        } else {
            0
        };
        // SAFETY: NUL-terminated locale, checked UTF-16 lengths and live buffers; optional parameters are null/zero.
        let result = unsafe {
            CompareStringEx(
                self.locale.as_ptr(),
                flags,
                pointer(left),
                left_len,
                pointer(right),
                right_len,
                std::ptr::null(),
                std::ptr::null(),
                0,
            )
        };
        match result {
            1 => Ok(Ordering::Less),
            2 => Ok(Ordering::Equal),
            3 => Ok(Ordering::Greater),
            _ => Err(native_error("CompareStringEx")),
        }
    }
    pub(super) fn contains(
        &self,
        text: &[u16],
        pattern: &[u16],
        ignore_case: bool,
    ) -> Result<bool, CollationError> {
        if pattern.is_empty() || self.compare(pattern, &[], ignore_case)? == Ordering::Equal {
            return Ok(true);
        }
        if text.is_empty() {
            return Ok(false);
        }
        let (text_len, pattern_len) = (length(text)?, length(pattern)?);
        let flags = FIND_FROMSTART
            | if ignore_case {
                LINGUISTIC_IGNORECASE | NORM_LINGUISTIC_CASING
            } else {
                0
            };
        // SAFETY: all buffers and the locale remain live; optional outputs/context are null.
        let result = unsafe {
            FindNLSStringEx(
                self.locale.as_ptr(),
                flags,
                pointer(text),
                text_len,
                pointer(pattern),
                pattern_len,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                0,
            )
        };
        if result >= 0 {
            return Ok(true);
        }
        // NLS explicitly clears LastError when there is no match. Other errors remain failures.
        let error = native_error("FindNLSStringEx");
        if matches!(error, CollationError::Native { code: 0, .. }) {
            Ok(false)
        } else {
            Err(error)
        }
    }
}
fn native_error(operation: &'static str) -> CollationError {
    // SAFETY: read this thread's error immediately after the documented failing API call.
    CollationError::Native {
        operation,
        code: unsafe { GetLastError() } as i32,
    }
}
