//! Native locale-aware display-list sorting and language filtering.
//!
//! These are the platform's standard linguistic rules, without .NET PAL tailoring.
//! Text remains UTF-16 at the existing keyboard boundary; buffers are borrowed.
use super::Utf16Text;
mod error;
pub use error::CollationError;
mod current;
pub use current::{
    current_culture_name as current_locale_name,
    install_current_culture_provider as install_locale_provider,
};
use std::cmp::Ordering;

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod icu;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use icu as backend;
#[cfg(target_os = "emscripten")]
mod browser;
#[cfg(target_os = "emscripten")]
use browser as backend;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as backend;
#[cfg(any(target_os = "ios", all(test, target_os = "macos")))]
mod core_foundation;
#[cfg(target_os = "ios")]
use core_foundation as backend;
#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
use android as backend;
#[cfg(target_os = "android")]
pub use android::install_java_vm as install_android_java_vm;

/// The OS-preferred locale. POSIX C/POSIX locales explicitly select root collation.
/// Absence of a locale is reported, rather than guessed from a keyboard subtype.
pub fn system_locale_name() -> Result<String, CollationError> {
    sys_locale::get_locale()
        .map(|name| match name.as_str() {
            "C" | "POSIX" => String::new(),
            _ => name,
        })
        .ok_or(CollationError::SystemLocaleUnavailable)
}

/// Synchronous owner-thread locale service for keyboard resource labels and UI search.
/// Uses system ICU on macOS/Linux, the browser's `Intl.Collator` on Emscripten, public CoreFoundation on iOS, Android ICU through JNI and Windows NLS on Windows. Results can vary with
/// platform data versions. Empty locale means root/invariant; embedded NUL is invalid.
pub struct LocaleText {
    #[cfg(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows",
        target_os = "ios",
        target_os = "android",
        target_os = "emscripten"
    ))]
    inner: backend::Collator,
}
impl LocaleText {
    pub fn new(locale: &str) -> Result<Self, CollationError> {
        if locale.contains('\0') {
            return Err(CollationError::InvalidLocale);
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "windows",
            target_os = "ios",
            target_os = "android",
            target_os = "emscripten"
        ))]
        {
            Ok(Self {
                inner: backend::Collator::new(locale)?,
            })
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "windows",
            target_os = "ios",
            target_os = "android",
            target_os = "emscripten"
        )))]
        {
            Err(CollationError::UnsupportedPlatform)
        }
    }
    pub fn compare(
        &self,
        left: &Utf16Text,
        right: &Utf16Text,
        ignore_case: bool,
    ) -> Result<Ordering, CollationError> {
        #[cfg(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "windows",
            target_os = "ios",
            target_os = "android",
            target_os = "emscripten"
        ))]
        {
            self.inner.compare(left.units(), right.units(), ignore_case)
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "windows",
            target_os = "ios",
            target_os = "android",
            target_os = "emscripten"
        )))]
        {
            let _ = (left, right, ignore_case);
            Err(CollationError::UnsupportedPlatform)
        }
    }
    pub fn contains(
        &self,
        text: &Utf16Text,
        pattern: &Utf16Text,
        ignore_case: bool,
    ) -> Result<bool, CollationError> {
        #[cfg(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "windows",
            target_os = "ios",
            target_os = "android",
            target_os = "emscripten"
        ))]
        {
            self.inner
                .contains(text.units(), pattern.units(), ignore_case)
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "windows",
            target_os = "ios",
            target_os = "android",
            target_os = "emscripten"
        )))]
        {
            let _ = (text, pattern, ignore_case);
            Err(CollationError::UnsupportedPlatform)
        }
    }
}
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "ios",
    target_os = "android"
))]
fn length(text: &[u16]) -> Result<i32, CollationError> {
    i32::try_from(text.len()).map_err(|_| CollationError::InputTooLong)
}
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "ios",
    target_os = "android"
))]
fn pointer(text: &[u16]) -> *const u16 {
    static EMPTY: u16 = 0;
    if text.is_empty() {
        &EMPTY
    } else {
        text.as_ptr()
    }
}
#[cfg(test)]
mod tests;
