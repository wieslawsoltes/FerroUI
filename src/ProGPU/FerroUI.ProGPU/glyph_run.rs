//! The glyph run of the backend until its text is the text stack of ProGPU (step P4 of
//! docs/porting/progpu-backend.md): the measures the layout needs, and nothing to draw.

use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::GlyphTypeface;
use ferroui_base::platform::IGlyphRunImpl;
use ferroui_base::{Point, Rect};
use std::any::Any;

/// A glyph run: its size, its origin and the box of its advances and of the line of its font.
pub struct ProGpuGlyphRun {
    font_rendering_em_size: f64,
    baseline_origin: Point,
    bounds: Rect,
}

impl ProGpuGlyphRun {
    /// The run of `glyph_infos` of `glyph_typeface` at `font_rendering_em_size`.
    pub fn new(glyph_typeface: &GlyphTypeface, font_rendering_em_size: f64, glyph_infos: &[GlyphInfo], baseline_origin: Point) -> Self {
        let metrics = glyph_typeface.metrics();
        let scale = if metrics.design_em_height == 0 { 0.0 } else { font_rendering_em_size / metrics.design_em_height as f64 };
        let width: f64 = glyph_infos.iter().map(|glyph| glyph.glyph_advance).sum();
        // The ascent of the font is above the baseline (negative).
        let top = metrics.ascent as f64 * scale;
        let height = (metrics.descent - metrics.ascent) as f64 * scale;
        ProGpuGlyphRun {
            font_rendering_em_size,
            baseline_origin,
            bounds: Rect::new(baseline_origin.x, baseline_origin.y + top, width, height.max(0.0)),
        }
    }
}

impl IGlyphRunImpl for ProGpuGlyphRun {
    fn font_rendering_em_size(&self) -> f64 {
        self.font_rendering_em_size
    }

    fn baseline_origin(&self) -> Point {
        self.baseline_origin
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn get_intersections(&self, _lower_limit: f32, _upper_limit: f32) -> Vec<f32> {
        Vec::new()
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}
