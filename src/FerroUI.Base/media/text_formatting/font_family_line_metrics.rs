use crate::media::{FontFamily, FontMetrics, TextLineBounds, Typeface};

/// The line box of the regular face of a requested font family.
///
/// These are layout metrics. The actual glyph typeface still supplies glyphs,
/// ink bounds and decoration metrics. The family baseline excludes the line
/// gap; the gap belongs below the baseline until the bottom is trimmed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FontFamilyLineMetrics {
    /// Distance from the top of the bounded line box to the baseline.
    pub baseline: f64,
    /// Height of the bounded line box.
    pub height: f64,
}

impl FontFamilyLineMetrics {
    /// Resolves the regular face of `font_family` through the font manager and
    /// bounds its metrics at `font_rendering_em_size`.
    ///
    /// Family lookup and missing cap-height behavior follow the font manager
    /// and [`crate::media::GlyphTypeface`], including their font fallback.
    pub fn new(
        font_family: &FontFamily,
        font_rendering_em_size: f64,
        text_line_bounds: TextLineBounds,
    ) -> Self {
        let typeface = Self::line_typeface(Typeface::new(font_family.clone()).glyph_typeface(), font_family, font_rendering_em_size);
        Self::from_font_metrics(typeface.metrics(), font_rendering_em_size, text_line_bounds)
    }

    /// The face whose metrics bound the line: the regular face of the family, or, for a family
    /// that asks for automatic optical sizing (`FontFamily::with_automatic_optical_sizing`), its
    /// instance at the optical size of the text. The line height of every other family, the
    /// variable system fonts included, stays that of the regular face.
    pub(crate) fn line_typeface(
        regular: std::rc::Rc<crate::media::GlyphTypeface>,
        font_family: &FontFamily,
        font_rendering_em_size: f64,
    ) -> std::rc::Rc<crate::media::GlyphTypeface> {
        if !font_family.automatic_optical_sizing() {
            return regular;
        }
        crate::media::glyph_typeface_optical_size::glyph_typeface_for_text(
            regular,
            font_family,
            font_family,
            font_rendering_em_size,
        )
    }

    pub(crate) fn from_font_metrics(
        metrics: FontMetrics,
        font_rendering_em_size: f64,
        text_line_bounds: TextLineBounds,
    ) -> Self {
        let scale = font_rendering_em_size / metrics.design_em_height as f64;
        let baseline = -(metrics.ascent as f64) * scale;
        let height = metrics.line_spacing() as f64 * scale;
        let cap_height = metrics.cap_height as f64 * scale;
        match text_line_bounds {
            TextLineBounds::Full => Self { baseline, height },
            TextLineBounds::TrimToCapHeight => Self {
                baseline: cap_height,
                height: height - baseline + cap_height,
            },
            TextLineBounds::TrimToBaseline => Self {
                baseline,
                height: baseline,
            },
            TextLineBounds::Tight => Self {
                baseline: cap_height,
                height: cap_height,
            },
        }
    }
}
