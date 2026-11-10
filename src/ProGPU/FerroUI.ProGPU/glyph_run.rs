//! The glyph run of the backend: the glyphs of one face at one size, placed from a baseline
//! origin, as the text stack of the engine draws them.

use crate::font::FontFace;
use crate::geometry::{PathBuilder, PathData};
use crate::typeface::ProGpuTypeface;
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{FontSimulations, GlyphTypeface};
use ferroui_base::platform::{IGeometryContext, IGlyphRunImpl};
use ferroui_base::{Point, Rect};
use progpu_native_sys::progpu::{
    progpu_native_path_segment, PROGPU_NATIVE_PATH_SEGMENT_CUBIC, PROGPU_NATIVE_PATH_SEGMENT_LINE, PROGPU_NATIVE_PATH_SEGMENT_QUADRATIC,
};
use std::any::Any;
use std::sync::Arc;

/// The shear of a simulated oblique, as the other backends shear it.
pub(crate) const OBLIQUE_SKEW: f32 = 0.3;
/// The widening of a simulated bold, in ems.
pub(crate) const BOLD_EM_FRACTION: f32 = 1.0 / 24.0;

/// A glyph of a run: its index in the face and its place from the baseline origin of the run,
/// in the units of the run (Y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunGlyph {
    /// The glyph of the face.
    pub glyph: u16,
    /// The horizontal place of the origin of the glyph.
    pub x: f32,
    /// The vertical place of the origin of the glyph.
    pub y: f32,
}

/// The font of a run: the face of the engine, its variation instance and its simulations.
#[derive(Clone)]
pub(crate) struct RunFont {
    pub face: Arc<FontFace>,
    pub coordinates: Arc<[i16]>,
    pub simulations: FontSimulations,
}

/// A glyph run: its font, its size, its origin, its glyphs and a box that holds them.
pub struct ProGpuGlyphRun {
    font: Option<RunFont>,
    font_rendering_em_size: f64,
    baseline_origin: Point,
    glyphs: Vec<RunGlyph>,
    bounds: Rect,
}

impl ProGpuGlyphRun {
    /// The run of `glyph_infos` of `glyph_typeface` at `font_rendering_em_size`.
    pub fn new(glyph_typeface: &GlyphTypeface, font_rendering_em_size: f64, glyph_infos: &[GlyphInfo], baseline_origin: Point) -> Self {
        let font = ProGpuTypeface::try_get(&**glyph_typeface.platform_typeface()).map(|typeface| RunFont {
            face: typeface.face().clone(),
            coordinates: typeface.coordinates().clone(),
            simulations: glyph_typeface.font_simulations(),
        });

        let mut glyphs = Vec::with_capacity(glyph_infos.len());
        let mut advance = 0.0f64;
        for info in glyph_infos {
            glyphs.push(RunGlyph { glyph: info.glyph_index, x: (advance + info.glyph_offset.x) as f32, y: info.glyph_offset.y as f32 });
            advance += info.glyph_advance;
        }

        let bounds = match &font {
            Some(font) => Self::font_bounds(font, font_rendering_em_size, &glyphs, baseline_origin),
            None => {
                // A typeface of another font manager has no face of the engine: the run keeps
                // the measures of the line and draws nothing.
                let metrics = glyph_typeface.metrics();
                let scale = if metrics.design_em_height == 0 { 0.0 } else { font_rendering_em_size / metrics.design_em_height as f64 };
                let top = metrics.ascent as f64 * scale;
                let height = (metrics.descent - metrics.ascent) as f64 * scale;
                Rect::new(baseline_origin.x, baseline_origin.y + top, advance, height.max(0.0))
            }
        };

        ProGpuGlyphRun { font, font_rendering_em_size, baseline_origin, glyphs, bounds }
    }

    /// The union of the box of the face at the place of every glyph: a box no glyph leaves.
    fn font_bounds(font: &RunFont, em_size: f64, glyphs: &[RunGlyph], origin: Point) -> Rect {
        if glyphs.is_empty() {
            return Rect::new(origin.x, origin.y, 0.0, 0.0);
        }
        let units = f64::from(font.face.units_per_em().max(1));
        let scale = em_size / units;
        let [min_x, min_y, max_x, max_y] = font.face.design_box().map(f64::from);
        // Y up in the face, Y down in the run.
        let (mut left, top, mut right, bottom) = (min_x * scale, -max_y * scale, max_x * scale, -min_y * scale);
        if font.simulations.contains(FontSimulations::Oblique) {
            // The shear moves what is above the baseline right and what is below it left.
            right += f64::from(OBLIQUE_SKEW) * (-top).max(0.0);
            left -= f64::from(OBLIQUE_SKEW) * bottom.max(0.0);
        }
        if font.simulations.contains(FontSimulations::Bold) {
            let widening = f64::from(BOLD_EM_FRACTION) * em_size;
            left -= widening;
            right += widening;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for glyph in glyphs {
            x0 = x0.min(f64::from(glyph.x) + left);
            x1 = x1.max(f64::from(glyph.x) + right);
            y0 = y0.min(f64::from(glyph.y) + top);
            y1 = y1.max(f64::from(glyph.y) + bottom);
        }
        Rect::new(origin.x + x0, origin.y + y0, x1 - x0, y1 - y0)
    }

    /// The font of the run, when its typeface is one of this backend.
    pub(crate) fn font(&self) -> Option<&RunFont> {
        self.font.as_ref()
    }

    /// The glyphs of the run, placed from its baseline origin.
    pub fn glyphs(&self) -> &[RunGlyph] {
        &self.glyphs
    }
}

/// Adds the outline of a glyph (design units, Y up) to a path: scaled, sheared for a simulated
/// oblique and moved to the place of the glyph.
pub(crate) fn add_outline(builder: &mut PathBuilder, segments: &[progpu_native_path_segment], scale: f64, skew: f64, x: f64, y: f64) {
    let place = |point: progpu_native_sys::progpu::progpu_native_point| {
        let (px, py) = (f64::from(point.x) * scale, -f64::from(point.y) * scale);
        Point::new(x + px - py * skew, y + py)
    };
    let mut end: Option<(f32, f32)> = None;
    for segment in segments {
        if end != Some((segment.p0.x, segment.p0.y)) {
            if end.is_some() {
                builder.end_figure(true);
            }
            builder.begin_figure(place(segment.p0), true);
        }
        let last = if segment.kind == PROGPU_NATIVE_PATH_SEGMENT_LINE as u32 {
            builder.line_to(place(segment.p1), true);
            segment.p1
        } else if segment.kind == PROGPU_NATIVE_PATH_SEGMENT_QUADRATIC as u32 {
            builder.quadratic_bezier_to(place(segment.p1), place(segment.p2), true);
            segment.p2
        } else if segment.kind == PROGPU_NATIVE_PATH_SEGMENT_CUBIC as u32 {
            builder.cubic_bezier_to(place(segment.p1), place(segment.p2), place(segment.p3), true);
            segment.p3
        } else {
            continue;
        };
        end = Some((last.x, last.y));
    }
    if end.is_some() {
        builder.end_figure(true);
    }
}

/// The outlines of glyphs of a typeface, placed along a baseline: the geometry of a run.
pub(crate) fn outline_geometry(glyph_typeface: &GlyphTypeface, em_size: f64, glyph_infos: &[GlyphInfo], baseline_origin: Point) -> Option<PathData> {
    let typeface = ProGpuTypeface::try_get(&**glyph_typeface.platform_typeface())?;
    let face = typeface.face();
    let scale = em_size / f64::from(face.units_per_em().max(1));
    let skew = if glyph_typeface.font_simulations().contains(FontSimulations::Oblique) { f64::from(OBLIQUE_SKEW) } else { 0.0 };
    let mut builder = PathBuilder::default();
    let mut advance = 0.0;
    for info in glyph_infos {
        if let Some(outline) = face.outline(info.glyph_index, typeface.coordinates()) {
            add_outline(
                &mut builder,
                &outline.segments,
                scale,
                skew,
                baseline_origin.x + advance + info.glyph_offset.x,
                baseline_origin.y + info.glyph_offset.y,
            );
        }
        advance += info.glyph_advance;
    }
    Some(builder.finish())
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
