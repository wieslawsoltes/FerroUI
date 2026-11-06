//! Port of Uno/Services/EmojiFontFallback.cs (Apache-2.0).
//!
//! Source constants/classifier retained for existing markup consumers.
//! Native font installation uses `EmojiFont` and FerroUI font collections/options.
use ferroboard_core::text::Utf16Text;

#[doc(hidden)]
pub struct EmojiFontFallback;
impl EmojiFontFallback {
    pub const FAMILY_NAME: &'static str = "Noto Color Emoji";
    pub const DEFAULT_NOTO_COLOR_EMOJI_URL: &'static str =
        "https://cdn.jsdelivr.net/gh/googlefonts/noto-emoji@v2.051/fonts/NotoColorEmoji.ttf";

    /// Shared source literal storage; callers and native field getters retain this identity.
    pub fn family_name() -> Utf16Text {
        static VALUE: std::sync::LazyLock<Utf16Text> =
            std::sync::LazyLock::new(|| EmojiFontFallback::FAMILY_NAME.into());
        VALUE.clone()
    }
    pub fn default_noto_color_emoji_url() -> Utf16Text {
        static VALUE: std::sync::LazyLock<Utf16Text> =
            std::sync::LazyLock::new(|| EmojiFontFallback::DEFAULT_NOTO_COLOR_EMOJI_URL.into());
        VALUE.clone()
    }

    pub fn is_emoji_code_point(cp: i32) -> bool {
        super::emoji_font::EMOJI_RANGES
            .iter()
            .any(|&(start, end)| (start..=end).contains(&cp))
    }
}
