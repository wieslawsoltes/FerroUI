//! Native command-label sorting/filtering, using one ICU collation profile.
//! No general string/comparison/runtime API. UTF-16 slices stay at the FFI boundary.
use std::{
    cell::RefCell,
    cmp::Ordering,
    ffi::{CString, c_char, c_void},
    ptr::NonNull,
    rc::Rc,
};
#[path = "command_profile.rs"]
mod command_profile;
#[derive(Debug)]
pub enum CollationError {
    LocaleUnavailable,
    TextTooLong,
    AllocationFailed,
    Icu {
        operation: &'static str,
        status: i32,
    },
}
impl std::fmt::Display for CollationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native command collation: {self:?}")
    }
}
impl std::error::Error for CollationError {}
#[repr(C)]
struct UCollator {
    _opaque: [u8; 0],
}
#[repr(C)]
struct UStringSearch {
    _opaque: [u8; 0],
}
#[cfg_attr(target_os = "macos", link(name = "icucore"))]
#[cfg_attr(target_os = "windows", link(name = "icu", kind = "raw-dylib"))]
unsafe extern "C" {
    #[cfg_attr(target_os = "emscripten", link_name = "ucol_open_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_ucol_open")]
    fn ucol_open(locale: *const c_char, status: *mut i32) -> *mut UCollator;
    #[cfg_attr(target_os = "emscripten", link_name = "ucol_close_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_ucol_close")]
    fn ucol_close(collator: *mut UCollator);
    #[cfg_attr(target_os = "emscripten", link_name = "ucol_getRules_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_ucol_getRules")]
    fn ucol_getRules(collator: *const UCollator, length: *mut i32) -> *const u16;
    #[cfg_attr(target_os = "emscripten", link_name = "ucol_openRules_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_ucol_openRules")]
    fn ucol_openRules(
        rules: *const u16,
        length: i32,
        normalization: i32,
        strength: i32,
        parse_error: *mut c_void,
        status: *mut i32,
    ) -> *mut UCollator;
    #[cfg_attr(target_os = "emscripten", link_name = "ucol_strcoll_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_ucol_strcoll")]
    fn ucol_strcoll(
        collator: *const UCollator,
        left: *const u16,
        left_len: i32,
        right: *const u16,
        right_len: i32,
    ) -> i32;
    #[cfg_attr(target_os = "emscripten", link_name = "usearch_openFromCollator_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_usearch_openFromCollator")]
    fn usearch_openFromCollator(
        pattern: *const u16,
        pattern_len: i32,
        text: *const u16,
        text_len: i32,
        collator: *const UCollator,
        break_iterator: *mut c_void,
        status: *mut i32,
    ) -> *mut UStringSearch;
    #[cfg_attr(target_os = "emscripten", link_name = "usearch_first_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_usearch_first")]
    fn usearch_first(search: *mut UStringSearch, status: *mut i32) -> i32;
    #[cfg_attr(target_os = "emscripten", link_name = "usearch_close_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_usearch_close")]
    fn usearch_close(search: *mut UStringSearch);
    #[cfg_attr(target_os = "emscripten", link_name = "u_getVersion_68")]
    #[cfg_attr(target_os = "linux", link_name = "fr_icu_u_getVersion")]
    fn u_getVersion(version: *mut u8);
}
struct Collator(NonNull<UCollator>);
impl Drop for Collator {
    fn drop(&mut self) {
        unsafe { ucol_close(self.0.as_ptr()) };
    }
}
struct Search(NonNull<UStringSearch>);
impl Drop for Search {
    fn drop(&mut self) {
        unsafe { usearch_close(self.0.as_ptr()) };
    }
}
fn status(operation: &'static str, value: i32) -> Result<(), CollationError> {
    if value > 0 {
        Err(CollationError::Icu {
            operation,
            status: value,
        })
    } else {
        Ok(())
    }
}
fn length(value: &[u16]) -> Result<i32, CollationError> {
    value
        .len()
        .try_into()
        .map_err(|_| CollationError::TextTooLong)
}
pub struct SearchLabelCollator {
    handle: Rc<Collator>,
}
// Bounded native resource reuse for interactive typing. This cache contains no
// controls, models, generic comparer options or runtime object identities.
ferroui_base::ferro_thread_local! {static LAST_COMMAND_PROFILE:RefCell<Option<(String,Rc<Collator>)>>=const{RefCell::new(None)};}
impl SearchLabelCollator {
    pub fn new(culture: &str) -> Result<Self, CollationError> {
        #[cfg(target_os = "emscripten")]
        status("load browser ICU data", crate::browser_icu::initialize())?;
        if let Some(handle) = LAST_COMMAND_PROFILE.with_borrow(|last| {
            last.as_ref()
                .filter(|(name, _)| name == culture)
                .map(|(_, handle)| handle.clone())
        }) {
            return Ok(Self { handle });
        }
        let name = CString::new(culture).map_err(|_| CollationError::LocaleUnavailable)?;
        let mut error = 0;
        // SAFETY: ICU copies the locale; status and CString are live. Acquire any
        // returned handle immediately, including an error path, for RAII cleanup.
        let base = unsafe { ucol_open(name.as_ptr(), &mut error) };
        let base = NonNull::new(base).map(Collator);
        status("open locale", error)?;
        let base = base.ok_or(CollationError::AllocationFailed)?;
        let mut count = 0;
        // SAFETY: borrowed locale rules remain valid until base is closed below.
        let rules = unsafe { ucol_getRules(base.0.as_ptr(), &mut count) };
        let count = usize::try_from(count).map_err(|_| CollationError::Icu {
            operation: "read locale rules",
            status: 1,
        })?;
        let mut combined = Vec::new();
        if count > 0 {
            if rules.is_null() {
                return Err(CollationError::Icu {
                    operation: "read locale rules",
                    status: 1,
                });
            }
            combined.extend_from_slice(unsafe { std::slice::from_raw_parts(rules, count) });
        }
        combined.extend_from_slice(command_profile::PROFILE);
        error = 0;
        // SAFETY: ICU compiles/copies these rules. -1 is UCOL_DEFAULT,1 is
        // UCOL_SECONDARY. Only this fixed product profile is supported.
        let handle = unsafe {
            ucol_openRules(
                combined.as_ptr(),
                length(&combined)?,
                -1,
                1,
                std::ptr::null_mut(),
                &mut error,
            )
        };
        let handle = NonNull::new(handle).map(Collator);
        status("open command profile", error)?;
        let handle = handle.ok_or(CollationError::AllocationFailed)?;
        let handle = Rc::new(handle);
        let old = LAST_COMMAND_PROFILE
            .with(|last| last.replace(Some((culture.to_owned(), handle.clone()))));
        drop(old);
        Ok(Self { handle })
    }
    pub fn compare(&self, left: &[u16], right: &[u16]) -> Result<Ordering, CollationError> {
        let a = length(left)?;
        let b = length(right)?;
        // SAFETY: const collator and borrowed slices live across the call. ICU
        // accepts explicit zero lengths and handles UTF-16, including surrogates.
        Ok(
            unsafe { ucol_strcoll(self.handle.0.as_ptr(), left.as_ptr(), a, right.as_ptr(), b) }
                .cmp(&0),
        )
    }
    pub fn contains(&self, text: &[u16], query: &[u16]) -> Result<bool, CollationError> {
        let n = length(text)?;
        let q = length(query)?;
        // ICU search rejects empty text/patterns; a collation-ignorable query
        // matches even empty text in the source command filter.
        if q == 0 || self.compare(query, &[])?.is_eq() {
            return Ok(true);
        }
        if n == 0 {
            return Ok(false);
        }
        let mut error = 0;
        // SAFETY: search borrows the collator and both slices only within this
        // function. Its RAII guard closes it before any borrowed data can expire.
        let search = unsafe {
            usearch_openFromCollator(
                query.as_ptr(),
                q,
                text.as_ptr(),
                n,
                self.handle.0.as_ptr(),
                std::ptr::null_mut(),
                &mut error,
            )
        };
        let search = NonNull::new(search).map(Search);
        status("open command search", error)?;
        let search = search.ok_or(CollationError::AllocationFailed)?;
        let found = unsafe { usearch_first(search.0.as_ptr(), &mut error) };
        status("find command text", error)?;
        Ok(found >= 0)
    }
    pub fn runtime_version() -> [u8; 4] {
        let mut version = [0; 4];
        unsafe { u_getVersion(version.as_mut_ptr()) };
        version
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locale_cache_keeps_only_last_profile_and_preserves_live_callers() {
        LAST_COMMAND_PROFILE.with(|last| last.replace(None));
        let english = SearchLabelCollator::new("en-US").unwrap();
        let same = SearchLabelCollator::new("en-US").unwrap();
        assert!(Rc::ptr_eq(&english.handle, &same.handle));
        let old = Rc::downgrade(&english.handle);
        drop(same);

        let turkish = SearchLabelCollator::new("tr-TR").unwrap();
        let units = |s: &str| s.encode_utf16().collect::<Vec<_>>();
        assert_eq!(
            english.compare(&units("I"), &units("i")).unwrap(),
            Ordering::Equal
        );
        assert_ne!(
            turkish.compare(&units("I"), &units("i")).unwrap(),
            Ordering::Equal
        );
        drop(english);
        assert!(old.upgrade().is_none(), "evicted profile must be released");

        let current = Rc::downgrade(&turkish.handle);
        drop(turkish);
        assert!(current.upgrade().is_some());
        assert!(matches!(
            SearchLabelCollator::new("en\0US"),
            Err(CollationError::LocaleUnavailable)
        ));
        assert!(current.upgrade().is_some(), "failed open preserves cache");
        LAST_COMMAND_PROFILE.with(|last| last.replace(None));
        assert!(current.upgrade().is_none());
    }
    #[test]
    fn native_comparison_matches_all_original_reference_pairs() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/search-collation.json"))
                .unwrap();
        let mut count = 0;
        for culture in fixture["corpus"]["cultures"].as_array().unwrap() {
            let collator = SearchLabelCollator::new(culture["name"].as_str().unwrap()).unwrap();
            for case in culture["pairs"].as_array().unwrap() {
                let units = |key: &str| {
                    case[key]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|n| n.as_u64().unwrap() as u16)
                        .collect::<Vec<_>>()
                };
                let actual = match collator.compare(&units("left"), &units("right")).unwrap() {
                    Ordering::Less => -1,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
                assert_eq!(actual, case["order"].as_i64().unwrap(), "{culture} {case}");
                count += 1;
            }
        }
        assert_eq!(count, 144);
    }
}
