//! Narrow native services used by FerroRibbon product behavior.
//! ICU supplies command collation and number data for the supported targets.
#[cfg(target_os = "emscripten")]
mod browser_icu;
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "emscripten",
    target_os = "linux"
))]
mod search_collation;
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "emscripten",
    target_os = "linux"
))]
pub use search_collation::{CollationError, SearchLabelCollator};
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "emscripten",
    target_os = "linux"
))]
mod number_data;
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "emscripten",
    target_os = "linux"
))]
pub use number_data::{NativeNumberDataError, native_number_format};

// Track package content changes even when restored files have old timestamps.
const _: Option<&str> = option_env!("FERRORIBBON_BUILD_FINGERPRINT");
