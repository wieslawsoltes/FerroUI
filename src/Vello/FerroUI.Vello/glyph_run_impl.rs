use crate::vello_typeface::{VelloFontFace, VelloTypeface};
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::GlyphTypeface;
use ferroui_base::platform::IGlyphRunImpl;
use ferroui_base::{Point, Rect, Vector};
use kurbo::{BezPath, Line, ParamCurve, ParamCurveExtrema, PathSeg, Shape};
use skrifa::instance::{LocationRef, Size};
use skrifa::raw::types::Tag;
use skrifa::MetadataProvider;
use std::any::Any;
use std::sync::{Arc, OnceLock};

/// The Vello implementation of a glyph run: glyph indices and positions
/// that a scene draws with the font of the run.
///
/// The Skia backend builds a text blob for each set of text options the run
/// is drawn with; here the run is plain values and the renderer has the
/// glyph cache (design document, section 5).
pub struct GlyphRunImpl {
    face: Arc<VelloFontFace>,
    glyph_indices: Vec<u16>,
    /// The origin of each glyph relative to the baseline origin of the run.
    glyph_positions: Vec<(f32, f32)>,
    /// The outlines of the glyphs at the em size of the run, each at its
    /// origin: what the intersections are computed from.
    glyph_paths: OnceLock<Vec<Option<BezPath>>>,
    font_rendering_em_size: f64,
    baseline_origin: Point,
    bounds: Rect,
}

impl GlyphRunImpl {
    /// Creates the glyph run of the given glyphs of a glyph typeface.
    ///
    /// # Panics
    /// Panics when the platform typeface of the glyph typeface was not
    /// created by this backend.
    pub fn new(
        glyph_typeface: &GlyphTypeface,
        font_rendering_em_size: f64,
        glyph_infos: &[GlyphInfo],
        baseline_origin: Point,
    ) -> Self {
        let glyph_typeface_impl = VelloTypeface::try_get(&**glyph_typeface.platform_typeface())
            .unwrap_or_else(|| panic!("The platform typeface was not created by the Vello backend"));

        Self::from_face(glyph_typeface_impl.face().clone(), font_rendering_em_size, glyph_infos, baseline_origin)
    }

    /// Creates a glyph run from the font of a typeface, which carries
    /// whatever the typeface needs applied to render as intended (the
    /// position in the variation space, synthetic bold or italic).
    pub fn from_face(
        face: Arc<VelloFontFace>,
        font_rendering_em_size: f64,
        glyphs: &[GlyphInfo],
        baseline_origin: Point,
    ) -> Self {
        let count = glyphs.len();

        let glyph_indices: Vec<u16> = glyphs.iter().map(|glyph| glyph.glyph_index).collect();
        let mut glyph_positions = Vec::with_capacity(count);

        // What a glyph without an outline covers (a bitmap glyph of a
        // colour font): its advance, from the ascent to the descent.
        let line_extent = OnceLock::new();

        // Build the glyph positions and union the run bounds in a single
        // pass: one accumulator covers both outputs.
        let mut current_x = 0.0;
        let mut run_bounds = Rect::default();

        for glyph in glyphs {
            let offset = glyph.glyph_offset;

            glyph_positions.push(((current_x + offset.x) as f32, offset.y as f32));

            let glyph_bounds = match face.glyph_path(glyph.glyph_index, font_rendering_em_size) {
                Some(path) if !path.elements().is_empty() => Some(pixel_bounds(path.bounding_box())),
                Some(_) => None,
                None => {
                    let (top, bottom) =
                        *line_extent.get_or_init(|| bitmap_glyph_extent(&face, font_rendering_em_size));

                    (glyph.glyph_advance > 0.0 && bottom > top)
                        .then(|| Rect::new(0.0, top, glyph.glyph_advance, bottom - top))
                }
            };

            // As in the Skia backend, the bounds of a glyph are placed at
            // its pen position: the offset of the glyph is not added.
            if let Some(bounds) = glyph_bounds {
                run_bounds = run_bounds.union(Rect::new(
                    current_x + bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                ));
            }

            current_x += glyph.glyph_advance;
        }

        Self {
            face,
            glyph_indices,
            glyph_positions,
            glyph_paths: OnceLock::new(),
            font_rendering_em_size,
            baseline_origin,
            bounds: run_bounds.translate(Vector::new(baseline_origin.x, baseline_origin.y)),
        }
    }

    /// The font of the run.
    pub fn face(&self) -> &Arc<VelloFontFace> {
        &self.face
    }

    /// The indices of the glyphs in the font.
    pub fn glyph_indices(&self) -> &[u16] {
        &self.glyph_indices
    }

    /// The origin of each glyph relative to the baseline origin of the run.
    pub fn glyph_positions(&self) -> &[(f32, f32)] {
        &self.glyph_positions
    }

    fn glyph_paths(&self) -> &[Option<BezPath>] {
        self.glyph_paths.get_or_init(|| {
            self.glyph_indices
                .iter()
                .zip(&self.glyph_positions)
                .map(|(glyph_index, (x, y))| {
                    let path = self.face.glyph_path(*glyph_index, self.font_rendering_em_size)?;

                    Some(kurbo::Affine::translate((*x as f64, *y as f64)) * path)
                })
                .collect()
        })
    }
}

/// The pixels the outline of a glyph touches when its origin is on a pixel,
/// and one more on every side: the bounds the Skia backend reports for a
/// glyph are those of its mask, which Skia rounds out and outsets for the
/// anti-aliasing of an edge. The bounds of a run are conservative by the
/// contract, and what uses them (the dirty rectangle of a text) must cover
/// a glyph drawn between two pixels.
fn pixel_bounds(bounds: kurbo::Rect) -> Rect {
    let (left, top) = (bounds.x0.floor() - 1.0, bounds.y0.floor() - 1.0);
    let (right, bottom) = (bounds.x1.ceil() + 1.0, bounds.y1.ceil() + 1.0);

    Rect::new(left, top, right - left, bottom - top)
}

/// From where to where a glyph without an outline extends above and below
/// the baseline, in a space of which y points down: the ascent and the
/// descent of the font, or of the em square for a font that states none.
fn bitmap_glyph_extent(face: &VelloFontFace, em_size: f64) -> (f64, f64) {
    let font = face.font_ref();

    let has_color_glyphs = [b"sbix", b"CBDT", b"COLR", b"SVG "]
        .iter()
        .any(|tag| font.table_data(Tag::new(tag)).is_some());

    if !has_color_glyphs {
        return (0.0, 0.0);
    }

    let metrics = font.metrics(Size::new(em_size as f32), LocationRef::new(face.normalized_coords()));

    if metrics.ascent > 0.0 || metrics.descent < 0.0 {
        (-(metrics.ascent as f64), -(metrics.descent as f64))
    } else {
        (-em_size, 0.0)
    }
}

/// Widens `interval` by the part of a segment that lies between two
/// horizontal lines.
fn intersect_segment(segment: PathSeg, lower: f64, upper: f64, interval: &mut Option<(f64, f64)>) {
    let mut include = |x: f64| {
        *interval = Some(match *interval {
            Some((left, right)) => (left.min(x), right.max(x)),
            None => (x, x),
        });
    };

    let bounds = Shape::bounding_box(&segment);
    if bounds.y1 < lower || bounds.y0 > upper {
        return;
    }

    // Where the segment crosses the two lines.
    for y in [lower, upper] {
        let line = Line::new((bounds.x0 - 1.0, y), (bounds.x1 + 1.0, y));

        for intersection in segment.intersect_line(line) {
            include(segment.eval(intersection.segment_t).x);
        }
    }

    // Its ends and, for a curve, where it turns back, when they lie between
    // the lines.
    let mut parameters = vec![0.0, 1.0];
    parameters.extend(segment.extrema());

    for t in parameters {
        let point = segment.eval(t);

        if point.y >= lower && point.y <= upper {
            include(point.x);
        }
    }
}

impl IGlyphRunImpl for GlyphRunImpl {
    fn font_rendering_em_size(&self) -> f64 {
        self.font_rendering_em_size
    }

    fn baseline_origin(&self) -> Point {
        self.baseline_origin
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    /// For each glyph of which the outline lies partly between the two
    /// limits (distances below the baseline), from where to where it does:
    /// two values a glyph, relative to the baseline origin of the run, in
    /// the order of the glyphs. This is what a text blob of Skia answers
    /// (`SkTextBlob::getIntercepts`), which the Skia backend returns.
    fn get_intersections(&self, lower_limit: f32, upper_limit: f32) -> Vec<f32> {
        let (lower, upper) = (lower_limit.min(upper_limit) as f64, lower_limit.max(upper_limit) as f64);
        let mut intersections = Vec::new();

        for path in self.glyph_paths().iter().flatten() {
            let mut interval = None;

            for segment in path.segments() {
                intersect_segment(segment, lower, upper, &mut interval);
            }

            if let Some((left, right)) = interval {
                intersections.push(left as f32);
                intersections.push(right as f32);
            }
        }

        intersections
    }

    fn dispose(&self) {
        // The run holds values and a shared font: nothing to release.
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
