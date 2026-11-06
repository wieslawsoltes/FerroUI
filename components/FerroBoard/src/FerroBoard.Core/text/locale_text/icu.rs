use super::{CollationError, Ordering, length, pointer};
use std::{
    ffi::{CString, c_void},
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/locale_icu.rs"));
}
#[cfg(target_os = "emscripten")]
#[path = "icu_data.rs"]
mod data;

struct Handle(NonNull<c_void>);
impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: this is the unique owner of a successful ICU allocation.
        unsafe { ffi::close(self.0.as_ptr()) }
    }
}
struct Search(NonNull<c_void>);
impl Drop for Search {
    fn drop(&mut self) {
        // SAFETY: the local search is destroyed before its borrowed text and collator.
        unsafe { ffi::search_close(self.0.as_ptr()) }
    }
}
fn check(code: i32, operation: &'static str) -> Result<(), CollationError> {
    // ICU documents negative fallback/default warnings as successful results.
    if code > 0 {
        Err(CollationError::Native { operation, code })
    } else {
        Ok(())
    }
}
fn open(locale: &CString, strength: i32) -> Result<Handle, CollationError> {
    let mut code = 0;
    // SAFETY: locale is NUL-terminated and status is writable through this call.
    let raw = unsafe { ffi::open(locale.as_ptr(), &mut code) };
    let handle = NonNull::new(raw).map(Handle);
    check(code, "ucol_open")?;
    let handle = handle.ok_or(CollationError::Native {
        operation: "ucol_open",
        code,
    })?;
    // SAFETY: live uniquely owned collator; strength is one of ICU's documented constants.
    unsafe {
        ffi::set_strength(handle.0.as_ptr(), strength);
    }
    Ok(handle)
}
pub(super) struct Collator {
    sensitive: Handle,
    insensitive: Handle,
    // No shared mutable native handle or implied cross-thread synchronization.
    _owner_thread: PhantomData<Rc<()>>,
}
impl Collator {
    pub(super) fn new(locale: &str) -> Result<Self, CollationError> {
        #[cfg(target_os = "emscripten")]
        data::install()?;
        let locale = CString::new(locale).map_err(|_| CollationError::InvalidLocale)?;
        Ok(Self {
            sensitive: open(&locale, 2)?,
            insensitive: open(&locale, 1)?,
            _owner_thread: PhantomData,
        })
    }
    fn handle(&self, ignore_case: bool) -> *mut c_void {
        if ignore_case {
            self.insensitive.0.as_ptr()
        } else {
            self.sensitive.0.as_ptr()
        }
    }
    pub(super) fn compare(
        &self,
        left: &[u16],
        right: &[u16],
        ignore_case: bool,
    ) -> Result<Ordering, CollationError> {
        let (left_len, right_len) = (length(left)?, length(right)?);
        // SAFETY: checked explicit lengths; buffers and immutable collator stay live for the call.
        let result = unsafe {
            ffi::compare(
                self.handle(ignore_case),
                pointer(left),
                left_len,
                pointer(right),
                right_len,
            )
        };
        Ok(result.cmp(&0))
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
        let mut code = 0;
        // SAFETY: ICU borrows these buffers/collator until Search drops before this method returns.
        // Null breaker selects ICU's standard search; no PAL/custom boundary rules are installed.
        let raw = unsafe {
            ffi::search_open(
                pointer(pattern),
                pattern_len,
                pointer(text),
                text_len,
                self.handle(ignore_case),
                std::ptr::null_mut(),
                &mut code,
            )
        };
        let search = NonNull::new(raw).map(Search);
        check(code, "usearch_openFromCollator")?;
        let search = search.ok_or(CollationError::Native {
            operation: "usearch_openFromCollator",
            code,
        })?;
        // SAFETY: the search, text, pattern and collator are all live through this call.
        let found = unsafe { ffi::search_first(search.0.as_ptr(), &mut code) };
        check(code, "usearch_first")?;
        Ok(found >= 0)
    }
}
