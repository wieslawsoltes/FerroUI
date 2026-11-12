use std::rc::Rc;

use crate::media::text_formatting::GenericTextRunProperties;
use crate::media::{FontFeature, GlyphTypeface};
use crate::utilities::Locale;

/// Options to customize text shaping.
#[derive(Clone)]
pub struct TextShaperOptions {
    glyph_typeface: Rc<GlyphTypeface>,
    font_rendering_em_size: f64,
    bidi_level: i8,
    culture: Option<Locale>,
    incremental_tab_width: f64,
    letter_spacing: f64,
    font_features: Option<Rc<Vec<FontFeature>>>,
    /// Whether the shaper records, for every glyph, whether the text may be cut and the pieces
    /// shaped apart at its cluster ([`ShapedBuffer::concat_flags`]), for a later edit of the
    /// text (step 7 of docs/porting/text-layout-performance.md). Off by default: it costs the
    /// shaper time.
    ///
    /// [`ShapedBuffer::concat_flags`]: crate::media::text_formatting::ShapedBuffer::concat_flags
    produce_concat_flags: bool,
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
            produce_concat_flags: false,
        }
    }

    /// Creates options with every value specified.
    pub fn with_all(
        typeface: Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        bidi_level: i8,
        culture: Option<Locale>,
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
            produce_concat_flags: false,
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
    pub fn culture(&self) -> Option<&Locale> {
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

    /// Whether the shaper records the concatenation flags of the glyphs it makes.
    pub fn produce_concat_flags(&self) -> bool {
        self.produce_concat_flags
    }

    /// These options, with the shaper recording the concatenation flags of the glyphs it makes or
    /// not.
    pub fn with_concat_flags(mut self, produce: bool) -> Self {
        self.produce_concat_flags = produce;
        self
    }

    /// Whether `other` shapes a text as these options do: the same typeface, size, bidi level,
    /// culture, tab width, letter spacing and features (whether the flags are recorded aside).
    pub(crate) fn shapes_as(&self, other: &TextShaperOptions) -> bool {
        Rc::ptr_eq(&self.glyph_typeface, &other.glyph_typeface)
            && self.font_rendering_em_size == other.font_rendering_em_size
            && self.bidi_level == other.bidi_level
            && self.culture == other.culture
            && self.incremental_tab_width == other.incremental_tab_width
            && self.letter_spacing == other.letter_spacing
            && match (&self.font_features, &other.font_features) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b) || a == b,
                _ => false,
            }
    }
}
