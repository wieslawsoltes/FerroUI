use std::collections::HashMap;

use crate::media::{FontFallback, FontFamily};

/// Options to customize the behavior of the font manager.
#[derive(Clone, Debug, Default)]
pub struct FontManagerOptions {
    /// Gets or sets the default font family's name.
    pub default_family_name: Option<String>,
    /// Gets or sets the font fallbacks.
    ///
    /// A fallback contains a family name and an optional `UnicodeRange`.
    pub font_fallbacks: Option<Vec<FontFallback>>,
    /// Gets or sets the font family mappings.
    ///
    /// A font family mapping is a dictionary that defines family names to be
    /// replaced by a different font family. Names are matched ignoring case.
    pub font_family_mappings: Option<HashMap<String, FontFamily>>,
}
