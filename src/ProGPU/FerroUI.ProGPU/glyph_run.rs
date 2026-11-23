//! The glyph run of the backend: the glyphs of one face at one size, placed from a baseline
//! origin, as the text stack of the engine draws them.

use crate::font::{FontFace, OBLIQUE_SKEW, WEIGHT_STEPS_PER_EM};
use crate::geometry::{PathBuilder, PathData};
use crate::outline_log;
use crate::typeface::ProGpuTypeface;
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{FontSimulations, GlyphTypeface};
use ferroui_base::platform::{IGeometryContext, IGlyphRunImpl};
use ferroui_base::{Point, Rect};
use progpu_native_sys::progpu::{
    progpu_native_glyph_outline_key, progpu_native_path_segment, progpu_native_scene_glyph_key,
    progpu_native_scene_glyph_outline, PROGPU_NATIVE_PATH_SEGMENT_CUBIC,
    PROGPU_NATIVE_PATH_SEGMENT_LINE, PROGPU_NATIVE_PATH_SEGMENT_QUADRATIC,
};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The widening of a simulated bold, in ems.
pub(crate) const BOLD_EM_FRACTION: f32 = 1.0 / 24.0;

/// The weight of antialiased text over the outlines of its font. The text the Skia backend
/// draws on macOS is heavier than the outlines: measured on stems, bars, diagonals and rounds
/// from 8 to 96 pixels an em, every edge lies this part of the em further out sideways, an
/// edge that faces up lies twice as far up, and an edge that faces down is where the outline
/// has it (docs/porting/progpu-backend.md, the text quality check).
const TEXT_WEIGHT_EM: f32 = 0.0054;
/// The most an edge moves sideways, in device pixels: what the measured weight levels off at
/// from about fifty pixels an em.
const TEXT_WEIGHT_PIXELS: f32 = 0.29;

/// The weight of antialiased text at `pixels_per_em`, in the steps of the outlines of a face.
/// One weight for every size below the size the weight levels off at, so a glyph has one
/// weighted outline there.
fn text_weight(pixels_per_em: f32) -> u8 {
    if !(pixels_per_em.is_finite() && pixels_per_em > 0.0) {
        return 0;
    }
    let weight = TEXT_WEIGHT_EM.min(TEXT_WEIGHT_PIXELS / pixels_per_em);
    (weight * WEIGHT_STEPS_PER_EM).round().clamp(0.0, 255.0) as u8
}

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

/// How a run lies on the pixels of a target: what its rasters are made for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Placement {
    /// Upright text at one scale. Every glyph is drawn at a whole device pixel; the quarter
    /// pixel its origin falls on is the phase of its raster, so the atlas is sampled texel for
    /// texel. The baseline is on a whole pixel.
    Snapped {
        /// Device pixels per unit of the run.
        scale: f32,
        /// The part of a pixel the origin of the run is right of a whole pixel.
        fraction_x: f32,
        /// The part of a pixel the origin of the run is below a whole pixel.
        fraction_y: f32,
        /// Aliased text: whole pixels only, no phases.
        aliased: bool,
    },
    /// Text under any other transform: rasters at the scale of the transform, drawn through it.
    Free {
        /// Device pixels per unit of the run, the root of the area the transform gives a unit.
        scale: f32,
    },
}

/// A glyph of a run as it is drawn: its raster and its place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlacedGlyph {
    /// The index of the raster in the keys of the run.
    pub outline: u32,
    /// Snapped: whole device pixels from the pixel of the origin of the run. Free: units of the
    /// run from its origin.
    pub x: f32,
    /// As `x`, downwards.
    pub y: f32,
}

/// What the engine draws a run from at one placement: the keys of the outlines of its glyphs
/// with the scale and the phase each is rasterized at, and the place of every glyph. The
/// outlines themselves are with the engines (the outline log), not in the frame.
pub(crate) struct RunRaster {
    placement: Placement,
    /// One key for every glyph and phase of the run.
    pub keys: Vec<progpu_native_scene_glyph_key>,
    /// The glyphs of the run that have an outline, in the order of the run.
    pub glyphs: Vec<PlacedGlyph>,
}

/// A glyph of an LCD run: which outline, the frame of its raster and its place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LcdTile {
    pub outline: u32,
    pub width: u32,
    pub height: u32,
    pub x_start: f32,
    pub y_start: f32,
    pub scale: f32,
    pub subpixel_x: f32,
    /// Whole device pixels from the pixel of the origin of the run to the top left of the raster.
    pub x: i32,
    pub y: i32,
}

/// What a run needs to be drawn with a coverage for each colour channel: its outlines with their
/// segments, and a tile for every glyph.
pub(crate) struct LcdRun {
    placement: Placement,
    pub outlines: Vec<progpu_native_scene_glyph_outline>,
    pub segments: Vec<progpu_native_path_segment>,
    pub tiles: Vec<LcdTile>,
}

/// The pixels a raster of an LCD run has beyond its outline: the quarter-pixel phase, and the two
/// subpixels the filter reads to either side.
const LCD_PADDING_X: f32 = 2.0;
const LCD_PADDING_Y: f32 = 1.0;
/// The largest side of a tile the engine takes.
const LCD_MAX_SIDE: f32 = 4096.0;

/// A glyph run: its font, its size, its origin, its glyphs and a box that holds them.
pub struct ProGpuGlyphRun {
    font: Option<RunFont>,
    font_rendering_em_size: f64,
    baseline_origin: Point,
    glyphs: Vec<RunGlyph>,
    bounds: Rect,
    /// What the run was drawn from last: a run is drawn at the same placement frame after
    /// frame.
    raster: Mutex<Option<Arc<RunRaster>>>,
    /// The LCD form of the run at the placement it was last drawn at.
    lcd: Mutex<Option<Arc<LcdRun>>>,
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

        ProGpuGlyphRun {
            font,
            font_rendering_em_size,
            baseline_origin,
            glyphs,
            bounds,
            raster: Mutex::new(None),
            lcd: Mutex::new(None),
        }
    }

    /// What the engine draws the run from at `placement`: made once and kept while the
    /// placement stays. None for a run of a typeface of another font manager.
    pub(crate) fn raster(&self, placement: Placement) -> Option<Arc<RunRaster>> {
        let font = self.font.as_ref()?;
        let mut kept = self.raster.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(raster) = kept.as_ref().filter(|raster| raster.placement == placement) {
            return Some(raster.clone());
        }
        let raster = Arc::new(self.build_raster(font, placement));
        *kept = Some(raster.clone());
        Some(raster)
    }

    /// The run for a coverage per colour channel at `placement` (upright text): the glyphs and
    /// quarter-pixel phases of [`raster`](Self::raster), with the same weighted outlines. None
    /// for a run of another font manager or with a glyph too large for a tile.
    pub(crate) fn lcd(&self, placement: Placement) -> Option<Arc<LcdRun>> {
        let mut kept = self.lcd.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(run) = kept.as_ref().filter(|run| run.placement == placement) {
            return Some(run.clone());
        }
        let font = self.font.as_ref()?;
        let raster = self.raster(placement)?;
        let mut run = LcdRun { placement, outlines: Vec::new(), segments: Vec::new(), tiles: Vec::new() };
        let mut frames = Vec::with_capacity(raster.keys.len());
        for key in &raster.keys {
            let variant = key.outline.variant;
            let (weight, oblique) = ((variant & 0xFF) as u8, variant & 0x100 != 0);
            let outline = font.face.cached_outline(key.outline.glyph as u16, &font.coordinates, weight, oblique)?;
            let scale = key.raster_scale;
            let x_start = (outline.min_x * scale).floor() - LCD_PADDING_X;
            let y_start = (-outline.max_y * scale).floor() - LCD_PADDING_Y;
            let width = (outline.max_x * scale).ceil() + LCD_PADDING_X + 1.0 - x_start;
            let height = (-outline.min_y * scale).ceil() + LCD_PADDING_Y - y_start;
            if !(width > 0.0 && height > 0.0 && width <= LCD_MAX_SIDE && height <= LCD_MAX_SIDE) {
                return None;
            }
            frames.push((x_start, y_start, width as u32, height as u32, scale, key.subpixel_x));
            run.outlines.push(progpu_native_scene_glyph_outline {
                segment_offset: run.segments.len() as u64,
                segment_count: outline.segments.len() as u64,
                min_x: outline.min_x,
                min_y: outline.min_y,
                max_x: outline.max_x,
                max_y: outline.max_y,
                raster_scale: scale,
                subpixel_x: key.subpixel_x,
            });
            run.segments.extend_from_slice(&outline.segments);
        }
        for glyph in &raster.glyphs {
            let (x_start, y_start, width, height, scale, subpixel_x) = *frames.get(glyph.outline as usize)?;
            run.tiles.push(LcdTile {
                outline: glyph.outline,
                width,
                height,
                x_start,
                y_start,
                scale,
                subpixel_x,
                x: (glyph.x + x_start) as i32,
                y: (glyph.y + y_start) as i32,
            });
        }
        let run = Arc::new(run);
        *kept = Some(run.clone());
        Some(run)
    }

    fn build_raster(&self, font: &RunFont, placement: Placement) -> RunRaster {
        let em_size = self.font_rendering_em_size;
        let units = f64::from(font.face.units_per_em().max(1));
        let device_scale = match placement {
            Placement::Snapped { scale, .. } | Placement::Free { scale } => f64::from(scale),
        };
        let raster_scale = (em_size * device_scale / units) as f32;
        let mut raster = RunRaster { placement, keys: Vec::new(), glyphs: Vec::with_capacity(self.glyphs.len()) };
        if !(raster_scale.is_finite() && raster_scale > 0.0) {
            return raster;
        }
        let oblique = font.simulations.contains(FontSimulations::Oblique);
        // Aliased text is the outline of the font; antialiased text has the weight of the text
        // of the platform.
        let weight = match placement {
            Placement::Snapped { aliased: true, .. } => 0,
            _ => text_weight(raster_scale * units as f32),
        };
        // A simulated bold is two strikes of the glyph, half the widening left and right of
        // its place.
        let widening = f64::from(BOLD_EM_FRACTION) * em_size;
        let strikes: &[f64] = if font.simulations.contains(FontSimulations::Bold) { &[-0.5, 0.5] } else { &[0.0] };

        // The key of the outline of a glyph (None for a glyph with nothing to draw), and the
        // key of a glyph at a phase. An outline is in the outline log before a run names it.
        let mut outlines: HashMap<u16, Option<progpu_native_glyph_outline_key>> = HashMap::new();
        let mut indices: HashMap<(u16, u8), u32> = HashMap::new();
        for glyph in &self.glyphs {
            let outline = *outlines.entry(glyph.glyph).or_insert_with(|| {
                let outline = font.face.cached_outline(glyph.glyph, &font.coordinates, weight, oblique)?;
                if !outline.drawable() {
                    return None;
                }
                outline_log::register(&outline);
                Some(outline.key())
            });
            let Some(outline_key) = outline else { continue };
            for strike in strikes {
                let local_x = f64::from(glyph.x) + strike * widening;
                let (x, y, phase) = match placement {
                    Placement::Snapped { scale, fraction_x, fraction_y, aliased } => {
                        let x = f64::from(fraction_x) + f64::from(scale) * local_x;
                        let y = (f64::from(fraction_y) + f64::from(scale) * f64::from(glyph.y) + 0.5).floor();
                        if aliased {
                            ((x + 0.5).floor(), y, 0u8)
                        } else {
                            // The nearest quarter of a pixel: whole pixels and a phase.
                            let quarters = (x * 4.0 + 0.5).floor() as i64;
                            (quarters.div_euclid(4) as f64, y, quarters.rem_euclid(4) as u8)
                        }
                    }
                    Placement::Free { .. } => (local_x, f64::from(glyph.y), 0u8),
                };
                let keys = &mut raster.keys;
                let outline = *indices.entry((glyph.glyph, phase)).or_insert_with(|| {
                    keys.push(progpu_native_scene_glyph_key { outline: outline_key, raster_scale, subpixel_x: f32::from(phase) / 4.0 });
                    (keys.len() - 1) as u32
                });
                raster.glyphs.push(PlacedGlyph { outline, x: x as f32, y: y as f32 });
            }
        }
        raster
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
        // The weight of antialiased text: sideways, and twice that upwards.
        let weight = f64::from(TEXT_WEIGHT_EM) * em_size;
        let (left, top, right) = (left - weight, top - 2.0 * weight, right + weight);
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for glyph in glyphs {
            x0 = x0.min(f64::from(glyph.x) + left);
            x1 = x1.max(f64::from(glyph.x) + right);
            y0 = y0.min(f64::from(glyph.y) + top);
            y1 = y1.max(f64::from(glyph.y) + bottom);
        }
        Rect::new(origin.x + x0, origin.y + y0, x1 - x0, y1 - y0)
    }

    /// The outlines of the glyphs of the run as one path, in the units of the run.
    pub(crate) fn outline_path(&self) -> Option<PathData> {
        let font = self.font.as_ref()?;
        let scale = self.font_rendering_em_size / f64::from(font.face.units_per_em().max(1));
        let skew = if font.simulations.contains(FontSimulations::Oblique) { f64::from(OBLIQUE_SKEW) } else { 0.0 };
        let mut builder = PathBuilder::default();
        for glyph in &self.glyphs {
            if let Some(outline) = font.face.cached_outline(glyph.glyph, &font.coordinates, 0, false) {
                let (x, y) = (self.baseline_origin.x + f64::from(glyph.x), self.baseline_origin.y + f64::from(glyph.y));
                add_outline(&mut builder, &outline.segments, scale, skew, x, y);
            }
        }
        Some(builder.finish())
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

/// The line pieces a curve of an outline is measured on: at the size of text the error is far
/// below what an underline can show.
const CURVE_PIECES: u32 = 16;

/// How far the outline of a glyph reaches across a horizontal band.
#[derive(Clone, Copy)]
struct BandExtent {
    lower: f64,
    upper: f64,
    left: f64,
    right: f64,
}

impl BandExtent {
    fn new(lower: f64, upper: f64) -> Self {
        BandExtent { lower, upper, left: f64::INFINITY, right: f64::NEG_INFINITY }
    }

    /// Adds what the line from `a` to `b` has inside the band: the line is cut to the range of
    /// its parameter between the two limits.
    fn add_line(&mut self, a: (f64, f64), b: (f64, f64)) {
        let rise = b.1 - a.1;
        let (mut enter, mut leave) = (0.0f64, 1.0f64);
        if rise == 0.0 {
            if a.1 < self.lower || a.1 > self.upper {
                return;
            }
        } else {
            let (at_lower, at_upper) = ((self.lower - a.1) / rise, (self.upper - a.1) / rise);
            enter = enter.max(at_lower.min(at_upper));
            leave = leave.min(at_lower.max(at_upper));
            if enter > leave {
                return;
            }
        }
        for t in [enter, leave] {
            let x = a.0 + (b.0 - a.0) * t;
            self.left = self.left.min(x);
            self.right = self.right.max(x);
        }
    }

    /// Adds a segment of an outline, a curve as `CURVE_PIECES` lines.
    fn add_segment(
        &mut self,
        segment: &progpu_native_path_segment,
        place: &impl Fn(progpu_native_sys::progpu::progpu_native_point) -> (f64, f64),
    ) {
        let (p0, p1, p2, p3) = (place(segment.p0), place(segment.p1), place(segment.p2), place(segment.p3));
        let lerp = |a: (f64, f64), b: (f64, f64), t: f64| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        let curve: fn(_, _, _, _, _, f64) -> (f64, f64) = if segment.kind == PROGPU_NATIVE_PATH_SEGMENT_QUADRATIC as u32
        {
            |lerp: &dyn Fn((f64, f64), (f64, f64), f64) -> (f64, f64), a, b, c, _, t| {
                lerp(lerp(a, b, t), lerp(b, c, t), t)
            }
        } else if segment.kind == PROGPU_NATIVE_PATH_SEGMENT_CUBIC as u32 {
            |lerp: &dyn Fn((f64, f64), (f64, f64), f64) -> (f64, f64), a, b, c, d, t| {
                let (ab, bc, cd) = (lerp(a, b, t), lerp(b, c, t), lerp(c, d, t));
                lerp(lerp(ab, bc, t), lerp(bc, cd, t), t)
            }
        } else {
            if segment.kind == PROGPU_NATIVE_PATH_SEGMENT_LINE as u32 {
                self.add_line(p0, p1);
            }
            return;
        };
        let mut from = p0;
        for piece in 1..=CURVE_PIECES {
            let to = curve(&lerp, p0, p1, p2, p3, f64::from(piece) / f64::from(CURVE_PIECES));
            self.add_line(from, to);
            from = to;
        }
    }

    /// From where to where the outline is in the band, when it is.
    fn range(&self) -> Option<(f64, f64)> {
        (self.left <= self.right).then_some((self.left, self.right))
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

    /// For each glyph whose outline lies partly between the two limits (distances below the
    /// baseline), from where to where it does: two values a glyph, relative to the baseline origin
    /// of the run, in the order of the glyphs. An underline uses them to skip descenders.
    fn get_intersections(&self, lower_limit: f32, upper_limit: f32) -> Vec<f32> {
        let Some(font) = &self.font else { return Vec::new() };
        let (lower, upper) = (lower_limit.min(upper_limit) as f64, lower_limit.max(upper_limit) as f64);
        let scale = self.font_rendering_em_size / f64::from(font.face.units_per_em().max(1));
        let skew = if font.simulations.contains(FontSimulations::Oblique) { f64::from(OBLIQUE_SKEW) } else { 0.0 };
        // A simulated bold is two strikes, half the widening to each side.
        let widening = if font.simulations.contains(FontSimulations::Bold) {
            f64::from(BOLD_EM_FRACTION) * self.font_rendering_em_size / 2.0
        } else {
            0.0
        };
        let mut spans = Vec::new();
        for glyph in &self.glyphs {
            let Some(outline) = font.face.cached_outline(glyph.glyph, &font.coordinates, 0, false) else { continue };
            let (x, y) = (f64::from(glyph.x), f64::from(glyph.y));
            let place = |point: progpu_native_sys::progpu::progpu_native_point| {
                let (px, py) = (f64::from(point.x) * scale, -f64::from(point.y) * scale);
                (x + px - py * skew, y + py)
            };
            let mut extent = BandExtent::new(lower, upper);
            for segment in &outline.segments {
                extent.add_segment(segment, &place);
            }
            if let Some((left, right)) = extent.range() {
                spans.extend([(left - widening) as f32, (right + widening) as f32]);
            }
        }
        spans
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}
