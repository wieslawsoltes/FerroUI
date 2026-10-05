//! String marshalling: Rust strings handed to native code as COM string
//! objects, and helpers to read the strings native code returns.

use crate::interop::*;
use ferroui_microcom::{ComPtr, HResult};
use std::ffi::{c_void, CString};

/// A Rust string exposed to native code as an `IFrnString`.
///
/// The UTF-8 bytes are owned by the object and NUL-terminated, so the
/// pointer native code obtains stays valid until the last reference is
/// released.
pub struct FrnString {
    string: String,
    /// The NUL-terminated UTF-8 copy handed to native code.
    native: Vec<u8>,
}

impl FrnString {
    pub fn new(s: &str) -> Self {
        let mut native = Vec::with_capacity(s.len() + 1);
        native.extend_from_slice(s.as_bytes());
        native.push(0);
        Self { string: s.to_owned(), native }
    }

    /// The string.
    pub fn string(&self) -> &str {
        &self.string
    }

    /// The UTF-8 bytes of the string.
    pub fn bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }
}

impl IFrnStringImpl for FrnString {
    fn pointer(&self) -> Result<*mut c_void, HResult> {
        // An empty string has no native buffer in the reference
        // implementation either.
        if self.string.is_empty() {
            return Ok(std::ptr::null_mut());
        }
        Ok(self.native.as_ptr() as *mut c_void)
    }

    fn length(&self) -> Result<i32, HResult> {
        Ok(self.string.len() as i32)
    }
}

/// A list of Rust strings exposed to native code as an `IFrnStringArray`.
pub struct FrnStringArray {
    items: Vec<(String, ComPtr<IFrnString>)>,
}

impl FrnStringArray {
    pub fn new<S: AsRef<str>>(items: impl IntoIterator<Item = S>) -> Self {
        Self {
            items: items
                .into_iter()
                .map(|s| (s.as_ref().to_owned(), IFrnString::from_impl(FrnString::new(s.as_ref()))))
                .collect(),
        }
    }

    /// The strings of the array.
    pub fn to_string_array(&self) -> Vec<String> {
        self.items.iter().map(|(s, _)| s.clone()).collect()
    }
}

impl IFrnStringArrayImpl for FrnStringArray {
    fn get_count(&self) -> u32 {
        self.items.len() as u32
    }

    fn get(&self, index: u32) -> Result<Option<ComPtr<IFrnString>>, HResult> {
        match self.items.get(index as usize) {
            Some((_, item)) => Ok(Some(item.clone())),
            None => Err(HResult::INVALIDARG),
        }
    }
}

/// The UTF-8 bytes of a native string; empty when it has no buffer.
pub fn frn_string_bytes(s: &IFrnString) -> Vec<u8> {
    let (Ok(ptr), Ok(len)) = (s.pointer(), s.length()) else {
        return Vec::new();
    };
    if ptr.is_null() || len <= 0 {
        return Vec::new();
    }
    // SAFETY: an `IFrnString` owns `length` bytes at `pointer` for as long as
    // it is alive; the bytes are copied before the borrow of `s` ends.
    unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) }.to_vec()
}

/// The content of a native string, or `None` when it has no buffer.
pub fn frn_string_to_string(s: &IFrnString) -> Option<String> {
    match s.pointer() {
        Ok(ptr) if !ptr.is_null() => Some(String::from_utf8_lossy(&frn_string_bytes(s)).into_owned()),
        _ => None,
    }
}

/// The content of a native string array; strings without a buffer are
/// empty.
pub fn frn_string_array_to_vec(array: &IFrnStringArray) -> Vec<String> {
    (0..array.get_count())
        .map(|index| match array.get(index) {
            Ok(Some(s)) => frn_string_to_string(&s).unwrap_or_default(),
            _ => String::new(),
        })
        .collect()
}

/// A C string for a `const char*` parameter. Native code stops at the first
/// NUL, so that is where the string is cut.
pub(crate) fn to_c_string(s: &str) -> CString {
    let end = s.bytes().position(|b| b == 0).unwrap_or(s.len());
    CString::new(&s.as_bytes()[..end]).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_round_trips_through_its_com_interface() {
        let s = IFrnString::from_impl(FrnString::new("héllo ⌘"));
        assert_eq!(s.length(), Ok("héllo ⌘".len() as i32));
        assert_eq!(frn_string_to_string(&s).as_deref(), Some("héllo ⌘"));
        assert_eq!(frn_string_bytes(&s), "héllo ⌘".as_bytes());
        // The buffer is NUL-terminated.
        let ptr = s.pointer().unwrap() as *const u8;
        // SAFETY: the buffer is `length + 1` bytes long.
        assert_eq!(unsafe { *ptr.add("héllo ⌘".len()) }, 0);
    }

    #[test]
    fn empty_string_has_no_buffer() {
        let s = IFrnString::from_impl(FrnString::new(""));
        assert!(s.pointer().unwrap().is_null());
        assert_eq!(s.length(), Ok(0));
        assert_eq!(frn_string_to_string(&s), None);
        assert!(frn_string_bytes(&s).is_empty());
    }

    #[test]
    fn string_array_round_trips() {
        let source = FrnStringArray::new(["a", "", "ccc"]);
        assert_eq!(source.to_string_array(), ["a", "", "ccc"]);
        let array = IFrnStringArray::from_impl(source);
        assert_eq!(array.get_count(), 3);
        assert_eq!(frn_string_array_to_vec(&array), ["a", "", "ccc"]);
        assert_eq!(array.get(3).unwrap_err(), HResult::INVALIDARG);
    }

    #[test]
    fn c_strings_stop_at_the_first_nul() {
        assert_eq!(to_c_string("title").as_bytes(), b"title");
        assert_eq!(to_c_string("a\0b").as_bytes(), b"a");
        assert_eq!(to_c_string("").as_bytes(), b"");
    }
}
