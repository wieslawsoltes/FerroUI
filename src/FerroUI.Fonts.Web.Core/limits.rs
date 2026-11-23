//! The hard limits on untrusted fonts.

/// Limits applied before any allocation that depends on the content of a font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebFontLimits {
    /// The largest encoded file, in bytes.
    pub max_encoded_size: usize,
    /// The largest decoded font, in bytes.
    pub max_decoded_size: usize,
    /// The most tables a font may have.
    pub max_tables: usize,
    /// The deepest nesting of composite glyphs.
    pub max_component_depth: usize,
}

impl WebFontLimits {
    /// The default limits (`docs/porting/web-fonts.md`, section 7).
    pub const DEFAULT: Self = Self {
        max_encoded_size: 32 * 1024 * 1024,
        max_decoded_size: 64 * 1024 * 1024,
        max_tables: 128,
        max_component_depth: 16,
    };
}

impl Default for WebFontLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}
