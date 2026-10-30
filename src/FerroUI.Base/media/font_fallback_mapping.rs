use std::rc::Rc;

use crate::media::{FontFamily, GlyphTypeface, Typeface};

/// The font and scale selected for a nonempty prefix of logical UTF-16 text.
/// The scale is relative to the source font size; it is never cumulative.
#[derive(Clone)]
pub struct FontFallbackMapping {
    glyph_typeface: Rc<GlyphTypeface>,
    typeface: Typeface,
    mapped_length: usize,
    scale: f64,
}

impl FontFallbackMapping {
    /// Creates a mapping with a finite positive scale and a nonzero length.
    /// Returns `None` for invalid values or an unnamed glyph face. The
    /// formatter additionally checks the length against the offered text,
    /// Unicode boundaries, glyph coverage and the effective em size.
    pub fn new(
        glyph_typeface: Rc<GlyphTypeface>,
        mapped_length: usize,
        scale: f64,
    ) -> Option<Self> {
        if mapped_length == 0
            || !scale.is_finite()
            || scale <= 0.0
            || glyph_typeface.family_name().is_empty()
        {
            return None;
        }
        let typeface = Typeface::with_style(
            FontFamily::new(glyph_typeface.family_name()),
            glyph_typeface.style(),
            glyph_typeface.weight(),
            glyph_typeface.stretch(),
        );
        Some(Self {
            glyph_typeface,
            typeface,
            mapped_length,
            scale,
        })
    }

    /// The actual selected face, retained for shaping and rendering.
    pub fn glyph_typeface(&self) -> &Rc<GlyphTypeface> {
        &self.glyph_typeface
    }

    /// Descriptive face metadata. Shaping uses the retained actual glyph face.
    pub fn typeface(&self) -> &Typeface {
        &self.typeface
    }

    /// The nonempty prefix length in logical UTF-16 code units.
    pub fn mapped_length(&self) -> usize {
        self.mapped_length
    }

    /// A finite positive multiplier of the source's requested font size.
    pub fn scale(&self) -> f64 {
        self.scale
    }
}

pub(crate) enum FontFallbackMappingResult {
    Unavailable,
    Fallback,
    Mapped(FontFallbackMapping),
}
