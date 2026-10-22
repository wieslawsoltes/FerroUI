//! Complete text artwork recorded from the same public layout that paints the
//! editor. Each glyph stays a separate source-over draw, including overlapping
//! glyphs in one translucent run. No backend types or font outline guesses.
use super::*;
use ferroui_base::media::text_formatting::ITextDrawingSink;
use ferroui_base::media::{GeometryPath, GeometryPathSegment, GlyphRun};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SolidPaint {
    pub color: ferroui_base::media::Color,
    pub opacity: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DecorationPen {
    pub paint: SolidPaint,
    pub thickness: f64,
    pub cap: PenLineCap,
    pub join: PenLineJoin,
    pub miter_limit: f64,
    pub dashes: Option<Vec<f64>>,
    pub dash_offset: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct TextPaintOperation {
    pub path: GeometryPath,
    pub transform: Matrix,
    pub fill: Option<SolidPaint>,
    pub stroke: Option<DecorationPen>,
    /// The native brush was resolved against this box, not the outline bounds.
    /// Solid paints do not depend on it. Future gradient support must retain
    /// this source coordinate system for every separately drawn glyph.
    pub brush_bounds: UiRect,
}

#[derive(Default, Debug)]
pub(crate) struct TextArtwork {
    pub operations: Vec<TextPaintOperation>,
    pub glyph_runs: usize,
    pub glyphs: usize,
}

#[derive(Clone, Copy)]
struct Limits {
    glyphs: usize,
    operations: usize,
    segments: usize,
}

impl Default for Limits {
    fn default() -> Self {
        // Text outlining is explicit export work, not a canvas hot path. Keep
        // a pathological font/document from allocating unbounded path output.
        Self { glyphs: 50_000, operations: 100_000, segments: 500_000 }
    }
}

fn unsupported(message: &str) -> String {
    format!("Exact text XAML export {message} Use native PNG for this artwork or keep the editable design.")
}

fn solid(brush: &dyn IBrush) -> Result<SolidPaint, String> {
    let paint = brush.as_solid_color_brush()
        .ok_or_else(|| unsupported("currently supports solid foregrounds and rich-text colors only."))?;
    if !brush.opacity().is_finite() {
        return Err(unsupported("requires a finite paint opacity."));
    }
    Ok(SolidPaint { color: paint.color(), opacity: brush.opacity() })
}

fn decoration(pen: &dyn IPen) -> Result<Option<DecorationPen>, String> {
    let Some(brush) = pen.brush() else { return Ok(None); };
    let paint = solid(&*brush)?;
    let dash = pen.dash_style();
    let dashes = dash.as_ref().and_then(|dash| dash.dashes());
    let dash_offset = dash.as_ref().map_or(0.0, |dash| dash.offset());
    if !pen.thickness().is_finite() || pen.thickness() < 0.0
        || !pen.miter_limit().is_finite() || !dash_offset.is_finite()
        || dashes.as_ref().is_some_and(|dashes| dashes.iter().any(|v| !v.is_finite() || *v < 0.0)) {
        return Err(unsupported("cannot encode an invalid decoration pen."));
    }
    Ok(Some(DecorationPen { paint, thickness: pen.thickness(), cap: pen.line_cap(),
        join: pen.line_join(), miter_limit: pen.miter_limit(), dashes, dash_offset }))
}

fn finite_point(point: UiPoint) -> bool { point.x.is_finite() && point.y.is_finite() }

fn finite_path(path: &GeometryPath) -> bool {
    path.segments.iter().all(|segment| match segment {
        GeometryPathSegment::MoveTo(point) | GeometryPathSegment::LineTo(point) => finite_point(*point),
        GeometryPathSegment::QuadraticTo { control, point } => finite_point(*control) && finite_point(*point),
        GeometryPathSegment::CubicTo { control1, control2, point } =>
            finite_point(*control1) && finite_point(*control2) && finite_point(*point),
        GeometryPathSegment::Close => true,
    })
}

struct Recorder {
    artwork: TextArtwork,
    transform: Matrix,
    transforms: Vec<Matrix>,
    segments: usize,
    limits: Limits,
    error: Option<String>,
}

impl Recorder {
    fn new(limits: Limits) -> Self {
        Self { artwork: TextArtwork::default(), transform: Matrix::IDENTITY,
            transforms: Vec::new(), segments: 0, limits, error: None }
    }

    fn record(&mut self, operation: impl FnOnce(&mut Self) -> Result<(), String>) {
        if self.error.is_some() { return; }
        if let Err(error) = operation(self) { self.error = Some(error); }
    }

    fn append(&mut self, path: GeometryPath, fill: Option<SolidPaint>, stroke: Option<DecorationPen>,
        brush_bounds: UiRect) -> Result<(), String> {
        if path.segments.is_empty() || (fill.is_none() && stroke.is_none()) { return Ok(()); }
        if self.artwork.operations.len() >= self.limits.operations
            || path.segments.len() > self.limits.segments.saturating_sub(self.segments) {
            return Err(unsupported("exceeds its bounded outline operation/curve budget."));
        }
        let matrix = self.transform;
        if !finite_path(&path) || ![matrix.m11, matrix.m12, matrix.m21, matrix.m22, matrix.m31, matrix.m32,
            brush_bounds.x, brush_bounds.y, brush_bounds.width, brush_bounds.height].into_iter().all(f64::is_finite)
            || matrix.m13 != 0.0 || matrix.m23 != 0.0 || matrix.m33 != 1.0 {
            return Err(unsupported("requires finite affine text geometry."));
        }
        self.segments += path.segments.len();
        self.artwork.operations.push(TextPaintOperation { path, transform: matrix, fill, stroke, brush_bounds });
        Ok(())
    }

    fn glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, run: &Rc<GlyphRun>) -> Result<(), String> {
        let Some(foreground) = foreground else { return Ok(()); };
        let paint = solid(&**foreground)?;
        let infos = run.glyph_infos();
        let infos = infos.borrow();
        if infos.len() > self.limits.glyphs.saturating_sub(self.artwork.glyphs) {
            return Err(unsupported("exceeds its 50000-glyph outline budget."));
        }
        let bounds = run.bounds();
        let baseline = run.baseline_origin();
        let mut advance = 0.0;
        self.artwork.glyph_runs += 1;
        for source in infos.iter() {
            // GlyphInfo is already shaped and visually ordered. Preserve the
            // exact original advance accumulation, offsets, font and baseline.
            // Rebuilding one glyph at a time avoids combining overlapping
            // glyph contours into one alpha coverage operation.
            let mut info = *source;
            info.glyph_offset.x += advance;
            let glyph = GlyphRun::new(run.glyph_typeface().clone(), run.font_rendering_em_size(),
                run.characters(), vec![info].into(), Some(baseline), run.bidi_level());
            let geometry = glyph.try_build_geometry().ok_or_else(|| unsupported(
                "cannot preserve a bitmap, color-font or unavailable glyph outline."))?;
            let data = geometry.try_get_path_data().ok_or_else(|| unsupported(
                "requires complete public glyph path extraction from the active renderer."))?;
            if let Some(path) = data.fill { self.append(path, Some(paint), None, bounds)?; }
            self.artwork.glyphs += 1;
            advance += source.glyph_advance;
        }
        Ok(())
    }

    fn finish(self) -> Result<TextArtwork, String> {
        if let Some(error) = self.error { return Err(error); }
        if !self.transforms.is_empty() { return Err(unsupported("received an unbalanced native text transform.")); }
        Ok(self.artwork)
    }
}

impl ITextDrawingSink for Recorder {
    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        self.record(|this| this.glyph_run(foreground, glyph_run));
    }

    fn draw_rectangle(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: UiRect) {
        self.record(|this| {
            let fill = brush.map(|brush| solid(&**brush)).transpose()?;
            let stroke = pen.map(|pen| decoration(&**pen)).transpose()?.flatten();
            let path = GeometryPath { fill_rule: ferroui_base::media::FillRule::NonZero, segments: vec![
                GeometryPathSegment::MoveTo(UiPoint::new(rect.x, rect.y)),
                GeometryPathSegment::LineTo(UiPoint::new(rect.right(), rect.y)),
                GeometryPathSegment::LineTo(UiPoint::new(rect.right(), rect.bottom())),
                GeometryPathSegment::LineTo(UiPoint::new(rect.x, rect.bottom())), GeometryPathSegment::Close,
            ] };
            this.append(path, fill, stroke, rect)
        });
    }

    fn draw_line(&mut self, pen: &Rc<dyn IPen>, p1: UiPoint, p2: UiPoint) {
        self.record(|this| {
            let stroke = decoration(&**pen)?;
            let bounds = UiRect::new(p1.x.min(p2.x), p1.y.min(p2.y), (p2.x - p1.x).abs(), (p2.y - p1.y).abs());
            this.append(GeometryPath { fill_rule: ferroui_base::media::FillRule::NonZero,
                segments: vec![GeometryPathSegment::MoveTo(p1), GeometryPathSegment::LineTo(p2)] }, None, stroke, bounds)
        });
    }

    fn push_transform(&mut self, matrix: Matrix) {
        self.transforms.push(self.transform);
        self.transform = matrix * self.transform;
    }

    fn pop_transform(&mut self) {
        if let Some(matrix) = self.transforms.pop() { self.transform = matrix; }
        else if self.error.is_none() { self.error = Some(unsupported("received an unbalanced native text transform.")); }
    }
}

/// Prepare a complete, immutable export result before writing any output.
/// The normal layout constructor supplies the same auto-size, wrapping,
/// OpenType, fallback fonts, bidi, paragraph and vertical origin as the canvas.
pub(crate) fn prepare(node: &Node) -> Result<TextArtwork, String> {
    if node.kind != NodeKind::Text { return Err("Text outline export requires a text layer.".into()); }
    let fail = |message: &str| format!("Text layer ‘{}’: {}", node.name, unsupported(message));
    if node.is_mask || !matches!(node.blend_mode, BlendMode::Normal | BlendMode::PassThrough)
        || node.effects.iter().any(|effect| effect.visible) {
        return Err(fail("does not yet encode text masks, effects or non-normal blends."));
    }
    if node.strokes.iter().any(|stroke| stroke.visible && stroke.opacity > 0.0 && stroke.width > 0.0) {
        return Err(fail("does not yet encode layer strokes around the complete text outline."));
    }
    if node.fills.iter().any(|fill| fill.visible && fill.opacity > 0.0 && !matches!(fill.paint, Paint::Solid(_))) {
        return Err(fail("currently supports solid foregrounds and rich-text colors only."));
    }
    let mut recorder = Recorder::new(Limits::default());
    for fill in node.fills.iter().rev().filter(|fill| fill.visible && fill.opacity > 0.0) {
        let Paint::Solid(color) = &fill.paint else { unreachable!("All visible text paints were validated"); };
        let text = make_text_layout(node, solid_brush(*color, fill.opacity));
        text.layout.draw(&mut recorder, text.origin);
        if recorder.error.is_some() { break; }
    }
    recorder.finish().map_err(|error| format!("Text layer ‘{}’: {error}", node.name))
}

#[cfg(all(test, not(target_os = "emscripten")))]
#[path = "text_export_tests.rs"]
mod tests;
