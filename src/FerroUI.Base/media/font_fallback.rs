use crate::media::{FontFamily, UnicodeRange};

/// Font fallback definition that is used to override the default fallback
/// lookup of the current font manager implementation.
#[derive(Clone, Debug, PartialEq)]
pub struct FontFallback {
    /// Get or set the fallback `FontFamily`.
    pub font_family: FontFamily,
    /// Get or set the `UnicodeRange` that is covered by the fallback.
    pub unicode_range: UnicodeRange,
}

impl Default for FontFallback {
    fn default() -> Self {
        Self { font_family: FontFamily::default_family(), unicode_range: UnicodeRange::default_range() }
    }
}
