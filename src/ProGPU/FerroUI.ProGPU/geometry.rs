//! Geometries (docs/porting/progpu-backend.md, section 2.5): a geometry is its figures and canonical
//! path segments (`progpu_native_path_segment`, the engine's own records, arcs kept as arcs) with
//! its fill rule; every query is answered by the engine's device-independent geometry ABI:
//! bounds, stroke bounds and hits, fill hits, relations, Boolean combination and widening, and
//! the path measure (lengths, points, tangents and pieces of the first contour, with the contour
//! semantics of the other backends).

use crate::runtime::runtime;
use ferroui_base::media::{
    FillRule, GeometryCombineMode, GeometryPath, GeometryPathData, GeometryPathSegment, IPen, IntersectionResult,
    PenLineCap, PenLineJoin, SweepDirection,
};
use ferroui_base::platform::{IGeometryContext, IGeometryImpl, IStreamGeometryBuilder, ITransformedGeometryImpl};
use ferroui_base::{Matrix, Point, Rect, Size};
use progpu_native_sys::progpu::*;
use std::any::Any;
use std::ptr;
use std::sync::{Arc, OnceLock};

/// The tolerance of flattening in the queries that flatten, in local units.
const TOLERANCE: f32 = 0.25;

/// Figure flags of the query transport.
const FIGURE_CLOSED: u32 = 1;
const FIGURE_FILLED: u32 = 2;
/// Segment flags of the query transport.
const SEGMENT_STROKED: u8 = 1;

fn point(value: Point) -> progpu_native_point {
    progpu_native_point { x: value.x as f32, y: value.y as f32 }
}

fn segment(kind: u32, p0: progpu_native_point, p1: progpu_native_point) -> progpu_native_path_segment {
    let zero = progpu_native_point { x: 0.0, y: 0.0 };
    progpu_native_path_segment { p0, p1, p2: zero, p3: zero, kind, pad0: 0, pad1: 0, pad2: 0 }
}

/// The end point of a canonical segment.
fn end_point(value: &progpu_native_path_segment) -> progpu_native_point {
    match value.kind as i32 {
        k if k == PROGPU_NATIVE_PATH_SEGMENT_QUADRATIC as i32 || k == PROGPU_NATIVE_PATH_SEGMENT_RATIONAL_QUADRATIC as i32 => value.p2,
        k if k == PROGPU_NATIVE_PATH_SEGMENT_CUBIC as i32 || k == PROGPU_NATIVE_PATH_SEGMENT_RATIONAL_CUBIC as i32 => value.p3,
        _ => value.p1,
    }
}

/// The center parameterization of an endpoint arc, as the engine resolves it (its
/// `geometry::resolve_arc`, in single precision): the center, the radii (scaled up when too small),
/// the start angle and the signed sweep.
fn resolve_arc(
    start: progpu_native_point,
    end: progpu_native_point,
    radii: Size,
    rotation_degrees: f32,
    large_arc: bool,
    clockwise: bool,
) -> Option<(progpu_native_point, f32, f32, f32, f32)> {
    use std::f32::consts::PI;
    const EPSILON: f32 = 0.00001;
    let mut radius_x = (radii.width as f32).abs();
    let mut radius_y = (radii.height as f32).abs();
    let (chord_x, chord_y) = (start.x - end.x, start.y - end.y);
    if chord_x * chord_x + chord_y * chord_y <= EPSILON * EPSILON || radius_x <= EPSILON || radius_y <= EPSILON {
        return None;
    }
    let phi = rotation_degrees * PI / 180.0;
    let (sine_phi, cosine_phi) = phi.sin_cos();
    let (dx, dy) = (chord_x * 0.5, chord_y * 0.5);
    let x1p = cosine_phi * dx + sine_phi * dy;
    let y1p = -sine_phi * dx + cosine_phi * dy;
    let mut rx2 = radius_x * radius_x;
    let mut ry2 = radius_y * radius_y;
    let (x1p2, y1p2) = (x1p * x1p, y1p * y1p);
    let check = x1p2 / rx2 + y1p2 / ry2;
    if !check.is_finite() {
        return None;
    }
    if check > 1.0 {
        let scale = check.sqrt();
        radius_x *= scale;
        radius_y *= scale;
        rx2 = radius_x * radius_x;
        ry2 = radius_y * radius_y;
    }
    let denominator = rx2 * y1p2 + ry2 * x1p2;
    if denominator <= 0.0 || !denominator.is_finite() {
        return None;
    }
    let sign = if large_arc == clockwise { -1.0 } else { 1.0 };
    let square = ((rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2) / denominator).max(0.0);
    if !square.is_finite() {
        return None;
    }
    let coefficient = sign * square.sqrt();
    let cxp = coefficient * (radius_x * y1p / radius_y);
    let cyp = coefficient * -(radius_y * x1p / radius_x);
    let center = progpu_native_point {
        x: cosine_phi * cxp - sine_phi * cyp + (start.x + end.x) * 0.5,
        y: sine_phi * cxp + cosine_phi * cyp + (start.y + end.y) * 0.5,
    };
    let theta1 = ((y1p - cyp) / radius_y).atan2((x1p - cxp) / radius_x);
    let theta2 = ((-y1p - cyp) / radius_y).atan2((-x1p - cxp) / radius_x);
    let mut delta = if square == 0.0 {
        if clockwise {
            PI
        } else {
            -PI
        }
    } else {
        theta2 - theta1
    };
    if clockwise {
        if delta < 0.0 {
            delta += 2.0 * PI;
        }
    } else if delta > 0.0 {
        delta -= 2.0 * PI;
    }
    (center.x.is_finite() && center.y.is_finite() && theta1.is_finite() && delta.is_finite())
        .then_some((center, radius_x, radius_y, theta1, delta))
}

/// The cubic pieces of an arc record (at most a quarter turn each).
fn arc_to_cubics(arc: &progpu_native_path_segment, out: &mut Vec<progpu_native_path_segment>) {
    let center = arc.p2;
    let (rx, ry) = (arc.p3.x as f64, arc.p3.y as f64);
    let theta = f32::from_bits(arc.pad0) as f64;
    let sweep = f32::from_bits(arc.pad1) as f64;
    let (sine_r, cosine_r) = (f32::from_bits(arc.pad2) as f64).sin_cos();
    let pieces = ((sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
    let step = sweep / pieces as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let at = |angle: f64, scale: f64, tangent: bool| {
        let (s, c) = angle.sin_cos();
        let (ex, ey) = if tangent { (-rx * s * scale, ry * c * scale) } else { (rx * c, ry * s) };
        (ex * cosine_r - ey * sine_r, ex * sine_r + ey * cosine_r)
    };
    let mut start = arc.p0;
    for index in 0..pieces {
        let a0 = theta + step * index as f64;
        let a1 = a0 + step;
        let (x0, y0) = at(a0, 1.0, false);
        let (x1, y1) = at(a1, 1.0, false);
        let (t0x, t0y) = at(a0, k, true);
        let (t1x, t1y) = at(a1, k, true);
        let end = if index + 1 == pieces {
            arc.p1
        } else {
            progpu_native_point { x: (center.x as f64 + x1) as f32, y: (center.y as f64 + y1) as f32 }
        };
        let control1 = progpu_native_point { x: (center.x as f64 + x0 + t0x) as f32, y: (center.y as f64 + y0 + t0y) as f32 };
        let mut cubic = segment(PROGPU_NATIVE_PATH_SEGMENT_CUBIC as u32, start, control1);
        cubic.p2 = progpu_native_point { x: (center.x as f64 + x1 - t1x) as f32, y: (center.y as f64 + y1 - t1y) as f32 };
        cubic.p3 = end;
        out.push(cubic);
        start = end;
    }
}

/// The figures and segments of a geometry, in the query transport of the engine.
#[derive(Clone, Default)]
pub struct PathData {
    figures: Vec<progpu_native_geometry_query_figure>,
    segments: Vec<progpu_native_path_segment>,
    flags: Vec<u8>,
    fill_rule: FillRule,
}

impl PathData {
    /// The figures (query-figure layout).
    pub fn figures(&self) -> &[progpu_native_geometry_query_figure] {
        &self.figures
    }

    /// The canonical segments of every figure, in order.
    pub fn segments(&self) -> &[progpu_native_path_segment] {
        &self.segments
    }

    /// The fill rule.
    pub fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }

    fn native_fill_rule(&self) -> u32 {
        match self.fill_rule {
            FillRule::EvenOdd => PROGPU_NATIVE_FILL_RULE_EVEN_ODD as u32,
            FillRule::NonZero => PROGPU_NATIVE_FILL_RULE_NON_ZERO as u32,
        }
    }

    /// The segments of the filled figures (implicitly closed contours of the fill transport).
    pub fn filled_segments(&self) -> Vec<progpu_native_path_segment> {
        let mut result = Vec::with_capacity(self.segments.len());
        for figure in self.figures.iter().filter(|figure| figure.flags & FIGURE_FILLED != 0) {
            let first = figure.first_segment as usize;
            result.extend_from_slice(&self.segments[first..first + figure.segment_count as usize]);
        }
        result
    }

    /// The data of a rectangle (one closed, filled figure).
    pub fn rectangle(rect: Rect) -> Self {
        let mut builder = PathBuilder::default();
        builder.begin_figure(Point::new(rect.x, rect.y), true);
        builder.line_to(Point::new(rect.x + rect.width, rect.y), true);
        builder.line_to(Point::new(rect.x + rect.width, rect.y + rect.height), true);
        builder.line_to(Point::new(rect.x, rect.y + rect.height), true);
        builder.end_figure(true);
        builder.finish()
    }

    /// The data of the ellipse inscribed in a rectangle (two half-turn arcs, closed and filled).
    pub fn ellipse(rect: Rect) -> Self {
        let (rx, ry) = (rect.width / 2.0, rect.height / 2.0);
        let (cx, cy) = (rect.x + rx, rect.y + ry);
        let mut builder = PathBuilder::default();
        builder.begin_figure(Point::new(cx + rx, cy), true);
        builder.arc_to(Point::new(cx - rx, cy), Size::new(rx, ry), 0.0, false, SweepDirection::Clockwise, true);
        builder.arc_to(Point::new(cx + rx, cy), Size::new(rx, ry), 0.0, false, SweepDirection::Clockwise, true);
        builder.end_figure(true);
        builder.finish()
    }

    /// The data of a line (one open, unfilled figure).
    pub fn line(from: Point, to: Point) -> Self {
        let mut builder = PathBuilder::default();
        builder.begin_figure(from, false);
        builder.line_to(to, true);
        builder.end_figure(false);
        builder.finish()
    }

    /// The figures of several geometries under one fill rule.
    pub fn group(fill_rule: FillRule, children: &[&PathData]) -> Self {
        let mut result = PathData { fill_rule, ..Default::default() };
        for child in children {
            let base = result.segments.len() as u32;
            result.figures.extend(child.figures.iter().map(|figure| progpu_native_geometry_query_figure {
                first_segment: figure.first_segment + base,
                ..*figure
            }));
            result.segments.extend_from_slice(&child.segments);
            result.flags.extend_from_slice(&child.flags);
        }
        result
    }

    /// Closed, filled polygon contours (the output of the engine's Boolean combination).
    fn polygons(points: &[progpu_native_point], offsets: &[u32]) -> Self {
        let mut result = PathData { fill_rule: FillRule::EvenOdd, ..Default::default() };
        for window in offsets.windows(2) {
            let contour = &points[window[0] as usize..window[1] as usize];
            if contour.len() < 3 {
                continue;
            }
            let first = result.segments.len() as u32;
            for (index, start) in contour.iter().enumerate() {
                let end = contour[(index + 1) % contour.len()];
                result.segments.push(segment(PROGPU_NATIVE_PATH_SEGMENT_LINE as u32, *start, end));
                result.flags.push(SEGMENT_STROKED);
            }
            result.figures.push(progpu_native_geometry_query_figure {
                start: contour[0],
                first_segment: first,
                segment_count: contour.len() as u32,
                flags: FIGURE_CLOSED | FIGURE_FILLED,
            });
        }
        result
    }

    /// Filled contours of contiguous segments (a discontinuity begins the next contour), the
    /// output of the engine's widening.
    fn contours(segments: &[progpu_native_path_segment], fill_rule: u32) -> Self {
        let mut result = PathData {
            fill_rule: if fill_rule == PROGPU_NATIVE_FILL_RULE_EVEN_ODD as u32 { FillRule::EvenOdd } else { FillRule::NonZero },
            ..Default::default()
        };
        let mut current: Option<progpu_native_point> = None;
        for value in segments {
            let continues = current.is_some_and(|point| point.x == value.p0.x && point.y == value.p0.y);
            if !continues {
                result.figures.push(progpu_native_geometry_query_figure {
                    start: value.p0,
                    first_segment: result.segments.len() as u32,
                    segment_count: 0,
                    flags: FIGURE_CLOSED | FIGURE_FILLED,
                });
            }
            result.segments.push(*value);
            result.flags.push(SEGMENT_STROKED);
            result.figures.last_mut().unwrap().segment_count += 1;
            current = Some(end_point(value));
        }
        result
    }

    /// The data with every point transformed (arcs become cubics: a transformed arc is not in
    /// general one).
    pub fn transformed(&self, matrix: &Matrix) -> Self {
        let map = |value: progpu_native_point| point(matrix.transform(Point::new(value.x as f64, value.y as f64)));
        let mut result = PathData { fill_rule: self.fill_rule, ..Default::default() };
        let mut pieces = Vec::new();
        for figure in &self.figures {
            let first = result.segments.len() as u32;
            let source = &self.segments[figure.first_segment as usize..(figure.first_segment + figure.segment_count) as usize];
            let flags = &self.flags[figure.first_segment as usize..(figure.first_segment + figure.segment_count) as usize];
            for (value, flag) in source.iter().zip(flags) {
                pieces.clear();
                if value.kind == PROGPU_NATIVE_PATH_SEGMENT_ARC as u32 {
                    arc_to_cubics(value, &mut pieces);
                } else {
                    pieces.push(*value);
                }
                for piece in &pieces {
                    let mut mapped = *piece;
                    mapped.p0 = map(piece.p0);
                    mapped.p1 = map(piece.p1);
                    if piece.kind != PROGPU_NATIVE_PATH_SEGMENT_LINE as u32 {
                        mapped.p2 = map(piece.p2);
                    }
                    if piece.kind == PROGPU_NATIVE_PATH_SEGMENT_CUBIC as u32 || piece.kind == PROGPU_NATIVE_PATH_SEGMENT_RATIONAL_CUBIC as u32 {
                        mapped.p3 = map(piece.p3);
                    }
                    result.segments.push(mapped);
                    result.flags.push(*flag);
                }
            }
            result.figures.push(progpu_native_geometry_query_figure {
                start: map(figure.start),
                first_segment: first,
                segment_count: result.segments.len() as u32 - first,
                flags: figure.flags,
            });
        }
        result
    }

    /// The tight bounds of the geometry (every figure's start point included).
    pub fn bounds(&self) -> Rect {
        let Ok(runtime) = runtime() else { return Rect::default() };
        let mut bounds = progpu_native_image_rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
        let mut has = 0u32;
        // SAFETY: the spans are borrowed for the call; the outputs are written by it.
        let status = unsafe {
            runtime.progpu.progpu_native_geometry_bounds(
                self.figures.as_ptr(),
                self.figures.len() as u32,
                self.segments.as_ptr(),
                self.flags.as_ptr(),
                self.segments.len() as u32,
                ptr::null(),
                &mut bounds,
                &mut has,
            )
        };
        if status != PROGPU_NATIVE_STATUS_SUCCESS || has == 0 {
            return Rect::default();
        }
        Rect::new(bounds.x as f64, bounds.y as f64, bounds.width as f64, bounds.height as f64)
    }

    /// The stroke query of the engine: the bounds of the stroke (`point` none) or whether the
    /// stroke contains a point.
    fn stroke_query(&self, pen: &dyn IPen, point_query: Option<Point>) -> (Option<Rect>, bool) {
        let Ok(runtime) = runtime() else { return (None, false) };
        let (native_pen, dashes) = native_pen(pen);
        let mut bounds = progpu_native_image_rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
        let (mut has, mut contains) = (0u32, 0u32);
        let query = point_query.map(point);
        // SAFETY: the spans are borrowed for the call; the outputs are written by it.
        let status = unsafe {
            runtime.progpu.progpu_native_geometry_stroke_query(
                self.figures.as_ptr(),
                self.figures.len() as u32,
                self.segments.as_ptr(),
                self.flags.as_ptr(),
                self.segments.len() as u32,
                &native_pen,
                if dashes.is_empty() { ptr::null() } else { dashes.as_ptr() },
                dashes.len() as u32,
                ptr::null(),
                query.as_ref().map_or(ptr::null(), |value| value as *const _),
                TOLERANCE,
                &mut bounds,
                &mut has,
                &mut contains,
            )
        };
        if status != PROGPU_NATIVE_STATUS_SUCCESS {
            return (None, false);
        }
        let rect = (has != 0).then(|| Rect::new(bounds.x as f64, bounds.y as f64, bounds.width as f64, bounds.height as f64));
        (rect, contains != 0)
    }

    /// Whether the fill contains a point.
    pub fn fill_contains(&self, value: Point) -> bool {
        let Ok(runtime) = runtime() else { return false };
        let filled = self.filled_segments();
        let query = point(value);
        let mut contains = 0u32;
        // SAFETY: the spans are borrowed for the call; the output is written by it.
        let status = unsafe {
            runtime.progpu.progpu_native_geometry_fill_contains(
                filled.as_ptr(),
                filled.len() as u32,
                self.native_fill_rule(),
                &query,
                TOLERANCE,
                &mut contains,
            )
        };
        status == PROGPU_NATIVE_STATUS_SUCCESS && contains != 0
    }

    /// The relation of this fill to another.
    pub fn relation(&self, other: &PathData) -> IntersectionResult {
        let Ok(runtime) = runtime() else { return IntersectionResult::NotCalculated };
        let (first, second) = (self.filled_segments(), other.filled_segments());
        let mut relation = 0u32;
        // SAFETY: the spans are borrowed for the call; the output is written by it.
        let status = unsafe {
            runtime.progpu.progpu_native_geometry_compare_fill(
                first.as_ptr(),
                first.len() as u32,
                self.native_fill_rule(),
                second.as_ptr(),
                second.len() as u32,
                other.native_fill_rule(),
                TOLERANCE,
                &mut relation,
            )
        };
        if status != PROGPU_NATIVE_STATUS_SUCCESS {
            return IntersectionResult::NotCalculated;
        }
        match relation as i32 {
            r if r == PROGPU_NATIVE_GEOMETRY_RELATION_DISJOINT as i32 => IntersectionResult::Empty,
            r if r == PROGPU_NATIVE_GEOMETRY_RELATION_IS_CONTAINED as i32 => IntersectionResult::FullyInside,
            r if r == PROGPU_NATIVE_GEOMETRY_RELATION_CONTAINS as i32 => IntersectionResult::FullyContains,
            r if r == PROGPU_NATIVE_GEOMETRY_RELATION_OVERLAP as i32 => IntersectionResult::Intersects,
            _ => IntersectionResult::NotCalculated,
        }
    }

    /// The Boolean combination of two fills.
    pub fn combine(&self, other: &PathData, mode: GeometryCombineMode) -> Option<PathData> {
        let runtime = runtime().ok()?;
        let (first, second) = (self.filled_segments(), other.filled_segments());
        let mut outline: *mut progpu_native_geometry_outline = ptr::null_mut();
        let mut points: *const progpu_native_point = ptr::null();
        let mut offsets: *const u32 = ptr::null();
        let (mut point_count, mut contour_count) = (0u32, 0u32);
        // SAFETY: the spans are borrowed for the call; the views live until the outline is destroyed.
        unsafe {
            let status = runtime.progpu.progpu_native_geometry_combine(
                first.as_ptr(),
                first.len() as u32,
                self.native_fill_rule(),
                second.as_ptr(),
                second.len() as u32,
                other.native_fill_rule(),
                mode as u32,
                TOLERANCE,
                &mut outline,
                &mut points,
                &mut point_count,
                &mut offsets,
                &mut contour_count,
            );
            if status != PROGPU_NATIVE_STATUS_SUCCESS {
                return None;
            }
            let result = PathData::polygons(
                std::slice::from_raw_parts(points, point_count as usize),
                std::slice::from_raw_parts(offsets, contour_count as usize + 1),
            );
            runtime.progpu.progpu_native_geometry_outline_destroy(outline);
            Some(result)
        }
    }

    /// The area a pen covers, as filled contours.
    pub fn widen(&self, pen: &dyn IPen) -> PathData {
        let Ok(runtime) = runtime() else { return PathData::default() };
        let (native_pen, dashes) = native_pen(pen);
        let mut path: *mut progpu_native_geometry_path = ptr::null_mut();
        let mut outline: *const progpu_native_path_segment = ptr::null();
        let (mut count, mut fill_rule) = (0u32, 0u32);
        // SAFETY: the spans are borrowed for the call; the view lives until the path is destroyed.
        unsafe {
            let status = runtime.progpu.progpu_native_geometry_widen(
                self.figures.as_ptr(),
                self.figures.len() as u32,
                self.segments.as_ptr(),
                self.flags.as_ptr(),
                self.segments.len() as u32,
                &native_pen,
                if dashes.is_empty() { ptr::null() } else { dashes.as_ptr() },
                dashes.len() as u32,
                ptr::null(),
                TOLERANCE,
                &mut path,
                &mut outline,
                &mut count,
                &mut fill_rule,
            );
            if status != PROGPU_NATIVE_STATUS_SUCCESS {
                return PathData::default();
            }
            let result = PathData::contours(std::slice::from_raw_parts(outline, count as usize), fill_rule);
            runtime.progpu.progpu_native_geometry_path_destroy(path);
            result
        }
    }

    /// The fill and stroke paths as Bezier commands (arcs as cubics).
    pub fn to_path_data(&self) -> GeometryPathData {
        let to_point = |value: progpu_native_point| Point::new(value.x as f64, value.y as f64);
        let mut fill = GeometryPath { fill_rule: self.fill_rule, segments: Vec::new() };
        let mut stroke = GeometryPath { fill_rule: self.fill_rule, segments: Vec::new() };
        let mut pieces = Vec::new();
        for figure in &self.figures {
            let range = figure.first_segment as usize..(figure.first_segment + figure.segment_count) as usize;
            let filled = figure.flags & FIGURE_FILLED != 0;
            let closed = figure.flags & FIGURE_CLOSED != 0;
            let mut commands = vec![GeometryPathSegment::MoveTo(to_point(figure.start))];
            for value in &self.segments[range] {
                pieces.clear();
                if value.kind == PROGPU_NATIVE_PATH_SEGMENT_ARC as u32 {
                    arc_to_cubics(value, &mut pieces);
                } else {
                    pieces.push(*value);
                }
                for piece in &pieces {
                    commands.push(match piece.kind as i32 {
                        k if k == PROGPU_NATIVE_PATH_SEGMENT_QUADRATIC as i32 => {
                            GeometryPathSegment::QuadraticTo { control: to_point(piece.p1), point: to_point(piece.p2) }
                        }
                        k if k == PROGPU_NATIVE_PATH_SEGMENT_CUBIC as i32 => GeometryPathSegment::CubicTo {
                            control1: to_point(piece.p1),
                            control2: to_point(piece.p2),
                            point: to_point(piece.p3),
                        },
                        _ => GeometryPathSegment::LineTo(to_point(end_point(piece))),
                    });
                }
            }
            if closed {
                commands.push(GeometryPathSegment::Close);
            }
            if filled {
                fill.segments.extend_from_slice(&commands);
            }
            stroke.segments.extend(commands);
        }
        GeometryPathData {
            fill: (!fill.segments.is_empty()).then_some(fill),
            stroke: (!stroke.segments.is_empty()).then_some(stroke),
        }
    }
}

/// The pen of the engine's stroke queries, with its dashes (in thickness units, as the pen has
/// them).
fn native_pen(pen: &dyn IPen) -> (progpu_native_geometry_query_pen, Vec<f32>) {
    let cap = match pen.line_cap() {
        PenLineCap::Flat => PROGPU_NATIVE_STROKE_CAP_FLAT,
        PenLineCap::Round => PROGPU_NATIVE_STROKE_CAP_ROUND,
        PenLineCap::Square => PROGPU_NATIVE_STROKE_CAP_SQUARE,
    } as u32;
    // A miter beyond its limit becomes a bevel, as in the other backends.
    let join = match pen.line_join() {
        PenLineJoin::Bevel => PROGPU_NATIVE_STROKE_JOIN_BEVEL,
        PenLineJoin::Miter => PROGPU_NATIVE_STROKE_JOIN_MITER_OR_BEVEL,
        PenLineJoin::Round => PROGPU_NATIVE_STROKE_JOIN_ROUND,
    } as u32;
    let (dashes, offset) = match pen.dash_style() {
        Some(style) => (
            style.dashes().unwrap_or_default().iter().map(|value| *value as f32).collect::<Vec<_>>(),
            style.offset() as f32,
        ),
        None => (Vec::new(), 0.0),
    };
    let dashes = if dashes.iter().sum::<f32>() > 0.0 { dashes } else { Vec::new() };
    (
        progpu_native_geometry_query_pen {
            thickness: pen.thickness().max(0.0) as f32,
            miter_limit: pen.miter_limit().max(1.0) as f32,
            dash_offset: offset,
            start_cap: cap,
            end_cap: cap,
            dash_cap: cap,
            line_join: join,
        },
        dashes,
    )
}

/// Records the figures of a path in the query transport of the engine.
pub struct PathBuilder {
    data: PathData,
    figure_open: bool,
    current: progpu_native_point,
}

impl Default for PathBuilder {
    fn default() -> Self {
        PathBuilder { data: PathData::default(), figure_open: false, current: progpu_native_point { x: 0.0, y: 0.0 } }
    }
}

impl PathBuilder {
    fn close_current(&mut self, closed: bool) {
        if !self.figure_open {
            return;
        }
        let figure = self.data.figures.last_mut().unwrap();
        if closed {
            figure.flags |= FIGURE_CLOSED;
            // The transport's closed figures return to their start explicitly.
            if figure.segment_count != 0 && (self.current.x != figure.start.x || self.current.y != figure.start.y) {
                let start = figure.start;
                self.data.segments.push(segment(PROGPU_NATIVE_PATH_SEGMENT_LINE as u32, self.current, start));
                self.data.flags.push(SEGMENT_STROKED);
                self.data.figures.last_mut().unwrap().segment_count += 1;
            }
        }
        self.figure_open = false;
    }

    fn push(&mut self, value: progpu_native_path_segment, stroked: bool) {
        if !self.figure_open {
            // A segment without a figure begins one at the current point.
            self.begin(self.current, true);
        }
        self.data.segments.push(value);
        self.data.flags.push(if stroked { SEGMENT_STROKED } else { 0 });
        self.data.figures.last_mut().unwrap().segment_count += 1;
        self.current = end_point(&value);
    }

    fn begin(&mut self, start: progpu_native_point, filled: bool) {
        self.close_current(false);
        self.data.figures.push(progpu_native_geometry_query_figure {
            start,
            first_segment: self.data.segments.len() as u32,
            segment_count: 0,
            flags: if filled { FIGURE_FILLED } else { 0 },
        });
        self.figure_open = true;
        self.current = start;
    }

    /// The recorded data (an open figure stays open).
    pub fn finish(&mut self) -> PathData {
        self.close_current(false);
        std::mem::take(&mut self.data)
    }
}

impl IGeometryContext for PathBuilder {
    fn arc_to(&mut self, point_to: Point, size: Size, rotation_angle: f64, is_large_arc: bool, sweep_direction: SweepDirection, is_stroked: bool) {
        let end = point(point_to);
        let clockwise = sweep_direction == SweepDirection::Clockwise;
        match resolve_arc(self.current, end, size, rotation_angle as f32, is_large_arc, clockwise) {
            Some((center, rx, ry, theta, delta)) => {
                let mut arc = segment(PROGPU_NATIVE_PATH_SEGMENT_ARC as u32, self.current, end);
                arc.p2 = center;
                arc.p3 = progpu_native_point { x: rx, y: ry };
                arc.pad0 = theta.to_bits();
                arc.pad1 = delta.to_bits();
                arc.pad2 = (rotation_angle as f32 * std::f32::consts::PI / 180.0).to_bits();
                self.push(arc, is_stroked);
            }
            // A degenerate arc (no radius, or coincident ends) is a line.
            None => self.push(segment(PROGPU_NATIVE_PATH_SEGMENT_LINE as u32, self.current, end), is_stroked),
        }
    }

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        self.begin(point(start_point), is_filled);
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        let mut value = segment(PROGPU_NATIVE_PATH_SEGMENT_CUBIC as u32, self.current, point(control_point1));
        value.p2 = point(control_point2);
        value.p3 = point(end_point);
        self.push(value, is_stroked);
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
        let mut value = segment(PROGPU_NATIVE_PATH_SEGMENT_QUADRATIC as u32, self.current, point(control_point));
        value.p2 = point(end_point);
        self.push(value, is_stroked);
    }

    fn line_to(&mut self, point_to: Point, is_stroked: bool) {
        self.push(segment(PROGPU_NATIVE_PATH_SEGMENT_LINE as u32, self.current, point(point_to)), is_stroked);
    }

    fn end_figure(&mut self, is_closed: bool) {
        self.close_current(is_closed);
    }

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        self.data.fill_rule = fill_rule;
    }

    fn dispose(&mut self) {}
}

/// The path measure of the engine over the first contour of a geometry (owned; immutable).
struct Measure(*mut progpu_native_path_measure);

// SAFETY: a measure is immutable; the engine allows concurrent queries of one measure, and it is
// destroyed once, by its owner.
unsafe impl Send for Measure {}
unsafe impl Sync for Measure {}

impl Measure {
    fn new(data: &PathData) -> Option<Measure> {
        let runtime = runtime().ok()?;
        let mut raw = ptr::null_mut();
        // SAFETY: the spans are borrowed for the call.
        let status = unsafe {
            runtime.progpu.progpu_native_path_measure_create(
                data.figures.as_ptr(),
                data.figures.len() as u32,
                data.segments.as_ptr(),
                data.segments.len() as u32,
                0,
                1.0,
                &mut raw,
            )
        };
        (status == PROGPU_NATIVE_STATUS_SUCCESS && !raw.is_null()).then_some(Measure(raw))
    }

    fn length(&self) -> f64 {
        let Ok(runtime) = runtime() else { return 0.0 };
        let mut length = 0.0f32;
        // SAFETY: the measure is alive.
        let status = unsafe { runtime.progpu.progpu_native_path_measure_contour(self.0, 0, &mut length, ptr::null_mut()) };
        if status == PROGPU_NATIVE_STATUS_SUCCESS {
            length as f64
        } else {
            0.0
        }
    }

    fn position_tangent(&self, distance: f64) -> Option<(Point, Point)> {
        let runtime = runtime().ok()?;
        let (mut position, mut tangent) = (progpu_native_point { x: 0.0, y: 0.0 }, progpu_native_point { x: 0.0, y: 0.0 });
        // SAFETY: the measure is alive.
        let status = unsafe {
            runtime.progpu.progpu_native_path_measure_position_tangent(self.0, 0, distance as f32, &mut position, &mut tangent)
        };
        (status == PROGPU_NATIVE_STATUS_SUCCESS).then(|| {
            (Point::new(position.x as f64, position.y as f64), Point::new(tangent.x as f64, tangent.y as f64))
        })
    }

    fn segment(&self, start: f64, stop: f64) -> Option<PathData> {
        let runtime = runtime().ok()?;
        let mut count = 0u32;
        // SAFETY: the measure is alive; the buffer has the capacity the first call reported.
        unsafe {
            if runtime.progpu.progpu_native_path_measure_segment(self.0, 0, start as f32, stop as f32, ptr::null_mut(), 0, &mut count)
                != PROGPU_NATIVE_STATUS_SUCCESS
                || count == 0
            {
                return None;
            }
            let zero = progpu_native_point { x: 0.0, y: 0.0 };
            let mut pieces = vec![segment(PROGPU_NATIVE_PATH_SEGMENT_LINE as u32, zero, zero); count as usize];
            if runtime.progpu.progpu_native_path_measure_segment(self.0, 0, start as f32, stop as f32, pieces.as_mut_ptr(), count, &mut count)
                != PROGPU_NATIVE_STATUS_SUCCESS
            {
                return None;
            }
            let mut data = PathData::default();
            data.figures.push(progpu_native_geometry_query_figure { start: pieces[0].p0, first_segment: 0, segment_count: count, flags: 0 });
            data.flags = vec![SEGMENT_STROKED; count as usize];
            data.segments = pieces;
            Some(data)
        }
    }
}

impl Drop for Measure {
    fn drop(&mut self) {
        if let Ok(runtime) = runtime() {
            // SAFETY: the measure is owned here.
            unsafe { runtime.progpu.progpu_native_path_measure_destroy(self.0) };
        }
    }
}

/// A geometry of the backend.
pub struct ProGpuGeometry {
    data: Arc<PathData>,
    measure: OnceLock<Option<Measure>>,
    bounds: OnceLock<Rect>,
}

impl ProGpuGeometry {
    /// A geometry of the data.
    pub fn new(data: PathData) -> Arc<Self> {
        Arc::new(ProGpuGeometry { data: Arc::new(data), measure: OnceLock::new(), bounds: OnceLock::new() })
    }

    /// The figures and segments.
    pub fn data(&self) -> &Arc<PathData> {
        &self.data
    }

    fn measure(&self) -> Option<&Measure> {
        self.measure.get_or_init(|| Measure::new(&self.data)).as_ref()
    }
}

/// The data of a geometry of this backend.
pub fn path_data_of(geometry: &dyn IGeometryImpl) -> Option<Arc<PathData>> {
    if let Some(value) = geometry.as_any().downcast_ref::<ProGpuGeometry>() {
        return Some(value.data.clone());
    }
    geometry.as_any().downcast_ref::<ProGpuTransformedGeometry>().map(|value| value.inner.data.clone())
}

fn render_bounds(data: &PathData, bounds: Rect, pen: Option<&dyn IPen>) -> Rect {
    match pen.filter(|pen| pen.thickness() > 0.0) {
        Some(pen) => data.stroke_query(pen, None).0.unwrap_or(bounds),
        None => bounds,
    }
}

impl IGeometryImpl for ProGpuGeometry {
    fn bounds(&self) -> Rect {
        *self.bounds.get_or_init(|| self.data.bounds())
    }

    fn contour_length(&self) -> f64 {
        self.measure().map_or(0.0, Measure::length)
    }

    fn get_render_bounds(&self, pen: Option<&dyn IPen>) -> Rect {
        render_bounds(&self.data, self.bounds(), pen)
    }

    fn get_widened_geometry(&self, pen: &dyn IPen) -> Arc<dyn IGeometryImpl> {
        ProGpuGeometry::new(self.data.widen(pen))
    }

    fn fill_contains(&self, value: Point) -> bool {
        self.data.fill_contains(value)
    }

    fn get_fill_intersection_result(&self, geometry: &dyn IGeometryImpl) -> IntersectionResult {
        path_data_of(geometry).map_or(IntersectionResult::NotCalculated, |other| self.data.relation(&other))
    }

    fn intersect(&self, geometry: &dyn IGeometryImpl) -> Option<Arc<dyn IGeometryImpl>> {
        let other = path_data_of(geometry)?;
        Some(ProGpuGeometry::new(self.data.combine(&other, GeometryCombineMode::Intersect)?))
    }

    fn stroke_contains(&self, pen: Option<&dyn IPen>, value: Point) -> bool {
        pen.filter(|pen| pen.thickness() > 0.0).is_some_and(|pen| self.data.stroke_query(pen, Some(value)).1)
    }

    fn with_transform(&self, transform: Matrix) -> Arc<dyn ITransformedGeometryImpl> {
        ProGpuTransformedGeometry::new(ProGpuGeometry::new((*self.data).clone()), transform)
    }

    fn try_get_point_at_distance(&self, distance: f64) -> Option<Point> {
        self.measure()?.position_tangent(distance).map(|(position, _)| position)
    }

    fn try_get_point_and_tangent_at_distance(&self, distance: f64) -> Option<(Point, Point)> {
        self.measure()?.position_tangent(distance)
    }

    fn try_get_segment(&self, start_distance: f64, stop_distance: f64, _start_on_begin_figure: bool) -> Option<Arc<dyn IGeometryImpl>> {
        Some(ProGpuGeometry::new(self.measure()?.segment(start_distance, stop_distance)?))
    }

    fn try_get_path_data(&self) -> Option<GeometryPathData> {
        Some(self.data.to_path_data())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A geometry with a transform: the source and the transformed data.
pub struct ProGpuTransformedGeometry {
    source: Arc<dyn IGeometryImpl>,
    transform: Matrix,
    inner: Arc<ProGpuGeometry>,
}

impl ProGpuTransformedGeometry {
    /// The source geometry under a transform.
    pub fn new(source: Arc<dyn IGeometryImpl>, transform: Matrix) -> Arc<Self> {
        let data = path_data_of(source.as_ref()).map(|data| data.transformed(&transform)).unwrap_or_default();
        Arc::new(ProGpuTransformedGeometry { source, transform, inner: ProGpuGeometry::new(data) })
    }
}

impl IGeometryImpl for ProGpuTransformedGeometry {
    fn bounds(&self) -> Rect {
        self.inner.bounds()
    }

    fn contour_length(&self) -> f64 {
        self.inner.contour_length()
    }

    fn get_render_bounds(&self, pen: Option<&dyn IPen>) -> Rect {
        self.inner.get_render_bounds(pen)
    }

    fn get_widened_geometry(&self, pen: &dyn IPen) -> Arc<dyn IGeometryImpl> {
        self.inner.get_widened_geometry(pen)
    }

    fn fill_contains(&self, value: Point) -> bool {
        self.inner.fill_contains(value)
    }

    fn get_fill_intersection_result(&self, geometry: &dyn IGeometryImpl) -> IntersectionResult {
        self.inner.get_fill_intersection_result(geometry)
    }

    fn intersect(&self, geometry: &dyn IGeometryImpl) -> Option<Arc<dyn IGeometryImpl>> {
        self.inner.intersect(geometry)
    }

    fn stroke_contains(&self, pen: Option<&dyn IPen>, value: Point) -> bool {
        self.inner.stroke_contains(pen, value)
    }

    fn with_transform(&self, transform: Matrix) -> Arc<dyn ITransformedGeometryImpl> {
        ProGpuTransformedGeometry::new(self.source.clone(), self.transform * transform)
    }

    fn try_get_point_at_distance(&self, distance: f64) -> Option<Point> {
        self.inner.try_get_point_at_distance(distance)
    }

    fn try_get_point_and_tangent_at_distance(&self, distance: f64) -> Option<(Point, Point)> {
        self.inner.try_get_point_and_tangent_at_distance(distance)
    }

    fn try_get_segment(&self, start_distance: f64, stop_distance: f64, start_on_begin_figure: bool) -> Option<Arc<dyn IGeometryImpl>> {
        self.inner.try_get_segment(start_distance, stop_distance, start_on_begin_figure)
    }

    fn try_get_path_data(&self) -> Option<GeometryPathData> {
        self.inner.try_get_path_data()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_transformed_geometry(&self) -> Option<&dyn ITransformedGeometryImpl> {
        Some(self)
    }
}

impl ITransformedGeometryImpl for ProGpuTransformedGeometry {
    fn source_geometry(&self) -> Arc<dyn IGeometryImpl> {
        self.source.clone()
    }

    fn transform(&self) -> Matrix {
        self.transform
    }
}

/// The stream geometry builder of the backend.
#[derive(Default)]
pub struct ProGpuStreamGeometryBuilder {
    builder: PathBuilder,
}

impl ProGpuStreamGeometryBuilder {
    /// A builder that continues the figures of a geometry, if one is given.
    pub fn new(source: Option<&dyn IGeometryImpl>) -> Self {
        let mut builder = PathBuilder::default();
        if let Some(data) = source.and_then(path_data_of) {
            builder.data = (*data).clone();
        }
        ProGpuStreamGeometryBuilder { builder }
    }
}

impl IGeometryContext for ProGpuStreamGeometryBuilder {
    fn arc_to(&mut self, point_to: Point, size: Size, rotation_angle: f64, is_large_arc: bool, sweep_direction: SweepDirection, is_stroked: bool) {
        self.builder.arc_to(point_to, size, rotation_angle, is_large_arc, sweep_direction, is_stroked);
    }

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        self.builder.begin_figure(start_point, is_filled);
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        self.builder.cubic_bezier_to(control_point1, control_point2, end_point, is_stroked);
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
        self.builder.quadratic_bezier_to(control_point, end_point, is_stroked);
    }

    fn line_to(&mut self, point_to: Point, is_stroked: bool) {
        self.builder.line_to(point_to, is_stroked);
    }

    fn end_figure(&mut self, is_closed: bool) {
        self.builder.end_figure(is_closed);
    }

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        self.builder.set_fill_rule(fill_rule);
    }

    fn dispose(&mut self) {}
}

impl IStreamGeometryBuilder for ProGpuStreamGeometryBuilder {
    fn to_geometry(&mut self) -> Arc<dyn IGeometryImpl> {
        ProGpuGeometry::new(self.builder.finish())
    }
}
