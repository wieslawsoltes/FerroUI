use crate::media::GlyphTypeface;

/// A metric that holds information about text specific measurements.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMetrics {
    /// Em size of font used to format and display text.
    pub font_rendering_em_size: f64,
    /// Gets the recommended distance from the top of the text to the baseline.
    pub baseline: f64,
    /// Gets the recommended distance above the baseline.
    pub ascent: f64,
    /// Gets the recommended distance under the baseline.
    pub descent: f64,
    /// Gets the recommended additional space between two lines of text.
    pub line_gap: f64,
    /// Gets the recommended line height.
    pub line_height: f64,
    /// Gets a value that indicates the thickness of the underline.
    pub underline_thickness: f64,
    /// Gets a value that indicates the distance of the underline from the baseline.
    pub underline_position: f64,
    /// Gets a value that indicates the thickness of the strikethrough.
    pub strikethrough_thickness: f64,
    /// Gets a value that indicates the distance of the strikethrough from the baseline.
    pub strikethrough_position: f64,
}

impl TextMetrics {
    pub fn new(glyph_typeface: &GlyphTypeface, font_rendering_em_size: f64) -> Self {
        let font_metrics = glyph_typeface.metrics();

        let scale = font_rendering_em_size / font_metrics.design_em_height as f64;

        let ascent = font_metrics.ascent as f64 * scale;
        let descent = font_metrics.descent as f64 * scale;
        let line_gap = font_metrics.line_gap as f64 * scale;

        Self {
            font_rendering_em_size,
            ascent,
            descent,
            line_gap,
            baseline: -ascent + line_gap * 0.5,
            line_height: descent - ascent + line_gap,
            underline_thickness: font_metrics.underline_thickness as f64 * scale,
            underline_position: font_metrics.underline_position as f64 * scale,
            strikethrough_thickness: font_metrics.strikethrough_thickness as f64 * scale,
            strikethrough_position: font_metrics.strikethrough_position as f64 * scale,
        }
    }
}
