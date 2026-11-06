//! Native keyboard platform integration.
mod emoji_font;
pub use emoji_font::{EmojiFont, EmojiFontError};
mod platform_services;
pub use platform_services::*;
mod emoji_font_fallback;
#[doc(hidden)]
pub use emoji_font_fallback::EmojiFontFallback;
mod emoji_font_metadata;
pub(crate) use emoji_font_metadata::register as register_emoji_font_metadata;

mod clipboard_monitor;
pub use clipboard_monitor::ClipboardMonitor;

mod system_input_method_guard;
pub use system_input_method_guard::SystemInputMethodGuard;

#[cfg(any(target_os = "ios", all(test, target_os = "macos")))]
mod ios_haptics;
#[cfg(target_os = "ios")]
pub use ios_haptics::IosHaptics;

#[cfg(target_os = "android")]
mod android_haptics;
#[cfg(target_os = "android")]
pub use android_haptics::AndroidHaptics;
