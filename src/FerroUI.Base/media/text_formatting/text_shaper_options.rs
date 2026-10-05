use std::rc::Rc;

use crate::media::text_formatting::GenericTextRunProperties;
use crate::media::{FontFeature, GlyphTypeface};
use crate::utilities::CultureInfo;

/// Options to customize text shaping.
#[derive(Clone)]
pub struct TextShaperOptions {
    glyph_typeface: Rc<GlyphTypeface>,
    font_rendering_em_size: f64,
    bidi_level: i8,
    culture: Option<CultureInfo>,
    incremental_tab_width: f64,
    letter_spacing: f64,
    font_features: Option<Rc<Vec<FontFeature>>>,
}

impl TextShaperOptions {
    /// Creates options with the default em size, bidi level 0 and no culture,
    /// tab width, letter spacing or features.
    pub fn new(typeface: Rc<GlyphTypeface>) -> Self {
        Self {
            glyph_typeface: typeface,
            font_rendering_em_size: GenericTextRunProperties::DEFAULT_FONT_RENDERING_EM_SIZE,
            bidi_level: 0,
            culture: None,
            incremental_tab_width: 0.0,
            letter_spacing: 0.0,
            font_features: None,
        }
    }

    /// Creates options with every value specified.
    pub fn with_all(
        typeface: Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        bidi_level: i8,
        culture: Option<CultureInfo>,
        incremental_tab_width: f64,
        letter_spacing: f64,
        font_features: Option<Rc<Vec<FontFeature>>>,
    ) -> Self {
        Self {
            glyph_typeface: typeface,
            font_rendering_em_size,
            bidi_level,
            culture,
            incremental_tab_width,
            letter_spacing,
            font_features,
        }
    }

    /// Get the typeface.
    pub fn glyph_typeface(&self) -> &Rc<GlyphTypeface> {
        &self.glyph_typeface
    }

    /// Get the font rendering em size.
    pub fn font_rendering_em_size(&self) -> f64 {
        self.font_rendering_em_size
    }

    /// Get the bidi level of the text.
    pub fn bidi_level(&self) -> i8 {
        self.bidi_level
    }

    /// Get the culture.
    pub fn culture(&self) -> Option<&CultureInfo> {
        self.culture.as_ref()
    }

    /// Get the incremental tab width.
    pub fn incremental_tab_width(&self) -> f64 {
        self.incremental_tab_width
    }

    /// Get the letter spacing.
    pub fn letter_spacing(&self) -> f64 {
        self.letter_spacing
    }

    /// Get features.
    pub fn font_features(&self) -> Option<&Rc<Vec<FontFeature>>> {
        self.font_features.as_ref()
    }
}
