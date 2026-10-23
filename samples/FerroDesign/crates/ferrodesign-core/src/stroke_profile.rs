//! Renderer-independent variable-width stroke profiles and bounded outlines.
//!
//! The preset anchors follow the published VariableWidthStrokeProperties API.
//! Their interpolation is explicitly piecewise linear, as is custom-point
//! interpolation; it does not claim Figma's undocumented interpolation math.
//! Each independent contour repeats the profile over its own approximate arc
//! length. Output is one nonzero-fill path, so overlapping stroke pieces share
//! one paint operation rather than accumulating alpha.
//!
//! https://developers.figma.com/docs/plugins/api/VariableWidthStrokeProperties/
use crate::{
    DocumentError, FillRule, Node, NodeKind, PathSegment, Point, Stroke, StrokeAlign, StrokeCap,
    StrokeJoin, Transform, VectorPath,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_STROKE_WIDTH_POINTS: usize = 256;
const MAX_INPUT_COMMANDS: usize = 32_768;
const MAX_FLAT_POINTS: usize = 65_536;
const MAX_OUTPUT_COMMANDS: usize = 262_144;
const MAX_SUBDIVISION_WORK: usize = 1_048_576;
const MAX_SUBDIVISION_DEPTH: u8 = 32;
const MITER_LIMIT: f64 = 10.0; // The editor's existing native ImmutablePen limit.

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StrokeWidthPoint {
    /// Fraction of the contour's length, in [0, 1].
    pub position: f64,
    /// Nonnegative multiplier of Stroke::width. Values above one are allowed.
    pub width: f64,
}

impl StrokeWidthPoint {
    pub const fn new(position: f64, width: f64) -> Self {
        Self { position, width }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StrokeWidthProfile {
    #[default]
    Uniform,
    Wedge,
    Taper,
    QuarterTaper,
    Eye,
    MirroredTaper,
    Custom {
        points: Vec<StrokeWidthPoint>,
    },
}

impl StrokeWidthProfile {
    /// Custom points are ordered, finite, and strictly distinct. A single
    /// point is a constant multiplier; missing endpoints extend the nearest
    /// supplied width. No sorting or silent repair changes authored data.
    pub fn validate(&self) -> Result<(), DocumentError> {
        if let Self::Custom { points } = self {
            if points.is_empty() || points.len() > MAX_STROKE_WIDTH_POINTS {
                return Err(invalid("Custom stroke width requires 1 to 256 points."));
            }
            let mut previous = -1.0;
            for point in points {
                if !point.position.is_finite()
                    || !(0.0..=1.0).contains(&point.position)
                    || point.position <= previous
                    || !point.width.is_finite()
                    || point.width < 0.0
                {
                    return Err(invalid("Stroke width points need increasing positions in [0, 1] and finite nonnegative widths."));
                }
                previous = point.position;
            }
        }
        Ok(())
    }

    /// True only for the default variant; custom constant profiles retain
    /// their authored points when serialized.
    pub fn is_uniform(&self) -> bool {
        matches!(self, Self::Uniform)
    }

    /// Authored custom points or the preset's exact interpolation anchors.
    /// Copy these when converting a preset to an editable custom profile.
    pub fn control_points(&self) -> &[StrokeWidthPoint] {
        self.points()
    }

    pub fn maximum_multiplier(&self) -> f64 {
        if self.validate().is_err() {
            return f64::NAN;
        }
        self.points()
            .iter()
            .map(|point| point.width)
            .fold(0.0, f64::max)
    }

    /// Sample the piecewise-linear profile, extending endpoint widths.
    /// Finite positions outside [0, 1] clamp to the endpoint. Invalid profiles
    /// or nonfinite positions return NaN instead of substituting Uniform.
    pub fn sample(&self, position: f64) -> f64 {
        if !position.is_finite() || self.validate().is_err() {
            return f64::NAN;
        }
        sample_points(self.points(), position.clamp(0.0, 1.0))
    }

    fn points(&self) -> &[StrokeWidthPoint] {
        use StrokeWidthPoint as W;
        const UNIFORM: &[W] = &[W::new(0.0, 1.0), W::new(1.0, 1.0)];
        const WEDGE: &[W] = &[W::new(0.0, 1.0), W::new(1.0, 0.0)];
        const TAPER: &[W] = &[W::new(0.0, 1.0), W::new(1.0, 0.25)];
        const QUARTER: &[W] = &[W::new(0.0, 0.25), W::new(0.25, 1.0), W::new(1.0, 0.25)];
        const EYE: &[W] = &[W::new(0.0, 0.0), W::new(0.5, 1.0), W::new(1.0, 0.0)];
        const MIRRORED: &[W] = &[W::new(0.0, 0.25), W::new(0.5, 1.0), W::new(1.0, 0.25)];
        match self {
            Self::Uniform => UNIFORM,
            Self::Wedge => WEDGE,
            Self::Taper => TAPER,
            Self::QuarterTaper => QUARTER,
            Self::Eye => EYE,
            Self::MirroredTaper => MIRRORED,
            Self::Custom { points } => points,
        }
    }
}

fn sample_points(points: &[StrokeWidthPoint], position: f64) -> f64 {
    let index = points.partition_point(|point| point.position < position);
    if index == 0 {
        points[0].width
    } else if index == points.len() {
        points[index - 1].width
    } else {
        let (a, b) = (points[index - 1], points[index]);
        let t = (position - a.position) / (b.position - a.position);
        // This convex form also avoids overflow in (b.width - a.width).
        a.width * (1.0 - t) + b.width * t
    }
}

/// Cheap document-validation preflight. Uniform strokes keep their existing
/// supported node/cap/dash combinations. Nonuniform support checks do not
/// tessellate curves or allocate an outline.
pub fn validate_variable_width_support(node: &Node, stroke: &Stroke) -> Result<(), DocumentError> {
    validate_width(stroke)?;
    if stroke.width_profile.is_uniform() {
        return Ok(());
    }
    if !stroke.dash.is_empty() {
        return Err(invalid(
            "Variable-width strokes do not support dashed strokes.",
        ));
    }
    if stroke.cap == StrokeCap::Arrow {
        return Err(invalid(
            "Variable-width strokes require a butt, round, or square cap.",
        ));
    }
    if matches!(
        node.kind,
        NodeKind::Text | NodeKind::Boolean | NodeKind::Group
    ) {
        return Err(invalid("Convert text, Boolean, or group artwork to an outline before applying a variable-width stroke."));
    }
    if let Some(network) = &node.vector_network {
        if network.edges.len() > MAX_INPUT_COMMANDS {
            return Err(invalid(
                "Variable-width network exceeds the 32768-edge work limit.",
            ));
        }
        let mut degree = BTreeMap::<_, usize>::new();
        for edge in network.edges.values().filter(|edge| edge.stroke) {
            for vertex in [edge.start, edge.end] {
                if !network.vertices.contains_key(&vertex) {
                    return Err(invalid("Variable-width network has a missing vertex."));
                }
                let count = degree.entry(vertex).or_default();
                *count += 1;
                if *count > 2 {
                    return Err(invalid(
                        "Variable-width strokes do not support branching networks.",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_width(stroke: &Stroke) -> Result<(), DocumentError> {
    stroke.width_profile.validate()?;
    if !stroke.width.is_finite()
        || stroke.width < 0.0
        || !(stroke.width * stroke.width_profile.maximum_multiplier()).is_finite()
    {
        return Err(invalid(
            "Stroke width and its profile product must be finite and nonnegative.",
        ));
    }
    Ok(())
}

/// Build local filled stroke geometry for supported primitive/vector nodes.
/// Paint, opacity, visibility, transforms, and source geometry are unchanged.
/// For closed inside/outside strokes this is a DOUBLE-WIDTH CENTERED BAND:
/// the caller must clip once with the source fill (Intersect/Exclude).
pub fn variable_width_outline(node: &Node, stroke: &Stroke) -> Result<VectorPath, DocumentError> {
    validate_variable_width_support(node, stroke)?;
    let path = node_stroke_path(node)?;
    variable_width_path_outline(
        &path,
        stroke,
        stroke.align != StrokeAlign::Center && node.has_closed_stroke_contours(),
    )
}

fn node_stroke_path(node: &Node) -> Result<VectorPath, DocumentError> {
    if !node.width.is_finite() || !node.height.is_finite() || node.width < 0.0 || node.height < 0.0
    {
        return Err(invalid(
            "Variable-width node dimensions must be finite and nonnegative.",
        ));
    }
    Ok(match node.kind {
        NodeKind::Line => VectorPath::polygon(
            &[Point::new(0.0, 0.0), Point::new(node.width, node.height)],
            false,
        ),
        NodeKind::Vector => {
            if let Some(network) = &node.vector_network {
                network.validate().map_err(DocumentError::Invalid)?;
                network.stroke_path()
            } else {
                node.path.clone()
            }
        }
        NodeKind::Ellipse => ellipse_path(node.width, node.height),
        NodeKind::Polygon | NodeKind::Star => crate::polygon_corner_path(node),
        NodeKind::Rectangle
        | NodeKind::Frame
        | NodeKind::Section
        | NodeKind::Component
        | NodeKind::Instance => crate::corner_path(node),
        NodeKind::Text | NodeKind::Boolean | NodeKind::Group => {
            return Err(invalid(
                "This node needs a prepared vector outline for a variable-width stroke.",
            ))
        }
    })
}

/// A local position on the same bounded centerline used to construct ink.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeWidthLocation {
    pub position: f64,
    pub point: Point,
    pub tangent: Point,
    pub normal: Point,
}

/// Cached, immutable contour sampling for native width handles. Points and
/// normalized positions are shared with outline construction, including every
/// width knot. No document geometry, IDs, or width points are changed.
#[derive(Clone, Debug)]
pub struct StrokeWidthContour {
    samples: Vec<WidthSample>,
    closed: bool,
    length: f64,
}

impl StrokeWidthContour {
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    pub fn length(&self) -> f64 {
        self.length
    }
    pub fn point_count(&self) -> usize {
        self.samples.len()
    }

    /// Clamp finite positions to [0, 1]. At a join the unit tangent bisects
    /// adjacent chords; an exact reversal uses the outgoing chord.
    pub fn sample(&self, position: f64) -> Option<StrokeWidthLocation> {
        if !position.is_finite() || self.samples.len() < 2 {
            return None;
        }
        let position = position.clamp(0.0, 1.0);
        let index = self
            .samples
            .partition_point(|sample| sample.position < position);
        if index < self.samples.len() && self.samples[index].position == position {
            return self.vertex_location(index);
        }
        let index = index.clamp(1, self.samples.len() - 1);
        let (a, b) = (self.samples[index - 1], self.samples[index]);
        let t = (position - a.position) / (b.position - a.position);
        let tangent = b.incoming;
        Some(StrokeWidthLocation {
            position,
            point: a.point.lerp(b.point, t),
            tangent,
            normal: normal(tangent),
        })
    }

    pub fn nearest(&self, point: Point) -> Option<(StrokeWidthLocation, f64)> {
        self.nearest_transformed(point, Transform::IDENTITY)
    }

    /// Input point and returned distance use transformed coordinates; the
    /// location remains local. Projecting transformed segments first preserves
    /// nearest-point choice under nonuniform scale, reflection, and shear.
    pub fn nearest_transformed(
        &self,
        point: Point,
        transform: Transform,
    ) -> Option<(StrokeWidthLocation, f64)> {
        if !finite(point)
            || ![
                transform.a,
                transform.b,
                transform.c,
                transform.d,
                transform.e,
                transform.f,
            ]
            .iter()
            .all(|value| value.is_finite())
        {
            return None;
        }
        let mut best: Option<(StrokeWidthLocation, f64)> = None;
        for pair in self.samples.windows(2) {
            let (a, b) = (
                transform.apply(pair[0].point),
                transform.apply(pair[1].point),
            );
            let length = a.distance(b);
            if !finite(a) || !finite(b) || !length.is_finite() {
                return None;
            }
            if length == 0.0 {
                continue;
            }
            let t = (dot(point - a, (b - a) * (1.0 / length)) / length).clamp(0.0, 1.0);
            let distance = point.distance(a.lerp(b, t));
            if !distance.is_finite() {
                return None;
            }
            if best
                .as_ref()
                .is_none_or(|(_, previous)| distance < *previous)
            {
                let position = pair[0].position * (1.0 - t) + pair[1].position * t;
                best = Some((self.sample(position)?, distance));
            }
        }
        best
    }

    fn vertex_location(&self, index: usize) -> Option<StrokeWidthLocation> {
        let last = self.samples.len() - 1;
        let outgoing = if index < last {
            Some(self.samples[index + 1].incoming)
        } else if self.closed {
            Some(self.samples[1].incoming)
        } else {
            None
        };
        let incoming = if index > 0 {
            Some(self.samples[index].incoming)
        } else if self.closed {
            Some(self.samples[last].incoming)
        } else {
            None
        };
        let tangent = match (incoming, outgoing) {
            (Some(a), Some(b)) => direction(Point::new(0.0, 0.0), a + b).unwrap_or(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            _ => return None,
        };
        let sample = self.samples[index];
        Some(StrokeWidthLocation {
            position: sample.position,
            point: sample.point,
            tangent,
            normal: normal(tangent),
        })
    }
}

pub fn stroke_width_contours(
    node: &Node,
    stroke: &Stroke,
) -> Result<Vec<StrokeWidthContour>, DocumentError> {
    validate_variable_width_support(node, stroke)?;
    let path = node_stroke_path(node)?;
    prepare_width_contours(
        &path,
        stroke,
        stroke.align != StrokeAlign::Center && node.has_closed_stroke_contours(),
    )
    .map(|(contours, _)| contours)
}

/// Build one NonZero filled outline from a prepared centerline. `aligned`
/// requests the doubled centered band described by `variable_width_outline`;
/// every drawable contour must then be closed. The caller performs clipping.
///
/// Curve subdivision checks the control hull, derivative cone, and a summed
/// chord/control-polygon arc-length gap in f64. The local tolerance is at most
/// 0.05 document units and 0.1% of the geometry/width extent. Width knots are
/// always inserted; their positions use this bounded polyline arc length.
/// Resource/precision limits return an error, never a uniform-stroke fallback.
pub fn variable_width_path_outline(
    path: &VectorPath,
    stroke: &Stroke,
    aligned: bool,
) -> Result<VectorPath, DocumentError> {
    let (contours, tolerance) = prepare_width_contours(path, stroke, aligned)?;
    let mut outline = Outline {
        path: VectorPath {
            fill_rule: FillRule::NonZero,
            ..VectorPath::default()
        },
        tolerance,
    };
    if stroke.width == 0.0 || stroke.width_profile.maximum_multiplier() == 0.0 {
        return Ok(outline.path);
    }
    for contour in contours {
        let samples = contour.samples;
        if samples.len() < 2 {
            continue;
        }
        let directions: Vec<_> = samples
            .iter()
            .skip(1)
            .map(|sample| sample.incoming)
            .collect();
        for (pair, &u) in samples.windows(2).zip(&directions) {
            let (a, b) = (pair[0], pair[1]);
            let n = normal(u);
            outline.polygon(&[
                a.point + n * a.radius,
                a.point - n * a.radius,
                b.point - n * b.radius,
                b.point + n * b.radius,
            ])?;
        }
        for index in 1..samples.len() - 1 {
            let sample = samples[index];
            outline.join(
                sample.point,
                directions[index - 1],
                directions[index],
                sample.radius,
                sample.radius,
                if sample.corner {
                    stroke.join
                } else {
                    StrokeJoin::Round
                },
            )?;
        }
        if contour.closed {
            outline.join(
                samples[0].point,
                *directions.last().unwrap(),
                directions[0],
                samples.last().unwrap().radius,
                samples[0].radius,
                stroke.join,
            )?;
        } else {
            outline.cap(samples[0], directions[0], stroke.cap, true)?;
            outline.cap(
                *samples.last().unwrap(),
                *directions.last().unwrap(),
                stroke.cap,
                false,
            )?;
        }
    }
    Ok(outline.path)
}

fn prepare_width_contours(
    path: &VectorPath,
    stroke: &Stroke,
    aligned: bool,
) -> Result<(Vec<StrokeWidthContour>, f64), DocumentError> {
    validate_width(stroke)?;
    if !stroke.dash.is_empty() || stroke.cap == StrokeCap::Arrow {
        return Err(invalid(
            "A variable-width outline requires an undashed butt, round, or square stroke.",
        ));
    }
    if path.segments.len() > MAX_INPUT_COMMANDS {
        return Err(invalid(
            "Variable-width path exceeds the 32768-command work limit.",
        ));
    }
    let radius = stroke.width * if aligned { 1.0 } else { 0.5 };
    let max_radius = radius * stroke.width_profile.maximum_multiplier();
    let tolerance = path_tolerance(path, max_radius)?;
    let Some(tolerance) = tolerance else {
        return Ok((Vec::new(), 0.05));
    };
    let mut budget = Budget::default();
    let contours = flatten_path(path, tolerance, max_radius, &mut budget)?;
    if aligned
        && contours
            .iter()
            .any(|contour| !contour.closed && contour.points.len() > 1)
    {
        return Err(invalid(
            "Inside/outside variable-width bands require closed contours.",
        ));
    }
    let mut result = Vec::new();
    for contour in contours {
        let samples = width_samples(&contour, stroke.width_profile.points(), radius, &mut budget)?;
        if samples.len() < 2 {
            continue;
        }
        let length = contour
            .points
            .windows(2)
            .map(|pair| pair[0].point.distance(pair[1].point))
            .sum();
        result.push(StrokeWidthContour {
            samples,
            closed: contour.closed,
            length,
        });
    }
    Ok((result, tolerance))
}

fn ellipse_path(width: f64, height: f64) -> VectorPath {
    let (rx, ry) = (width * 0.5, height * 0.5);
    let k = 4.0 * (std::f64::consts::PI / 8.0).tan() / 3.0;
    let p = |x, y| Point::new(rx + x * rx, ry + y * ry);
    VectorPath {
        segments: vec![
            PathSegment::MoveTo { point: p(1.0, 0.0) },
            PathSegment::CubicTo {
                control1: p(1.0, k),
                control2: p(k, 1.0),
                point: p(0.0, 1.0),
            },
            PathSegment::CubicTo {
                control1: p(-k, 1.0),
                control2: p(-1.0, k),
                point: p(-1.0, 0.0),
            },
            PathSegment::CubicTo {
                control1: p(-1.0, -k),
                control2: p(-k, -1.0),
                point: p(0.0, -1.0),
            },
            PathSegment::CubicTo {
                control1: p(k, -1.0),
                control2: p(1.0, -k),
                point: p(1.0, 0.0),
            },
            PathSegment::Close,
        ],
        ..VectorPath::default()
    }
}

fn path_tolerance(path: &VectorPath, max_radius: f64) -> Result<Option<f64>, DocumentError> {
    let mut min = Point::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut count = 0;
    let mut open = false;
    let mut visit = |point: Point| -> Result<(), DocumentError> {
        if !finite(point) {
            return Err(invalid(
                "Variable-width path contains a nonfinite coordinate.",
            ));
        }
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
        count += 1;
        Ok(())
    };
    for command in &path.segments {
        if matches!(command, PathSegment::MoveTo { .. }) {
            open = true;
        } else if !open {
            return Err(invalid(
                "Variable-width path commands must begin with MoveTo.",
            ));
        }
        match *command {
            PathSegment::MoveTo { point } | PathSegment::LineTo { point } => visit(point)?,
            PathSegment::QuadraticTo { control, point } => {
                visit(control)?;
                visit(point)?;
            }
            PathSegment::CubicTo {
                control1,
                control2,
                point,
            } => {
                visit(control1)?;
                visit(control2)?;
                visit(point)?;
            }
            PathSegment::Close => open = false,
        }
    }
    if count == 0 {
        return Ok(None);
    }
    let extent = (max.x - min.x).max(max.y - min.y).max(max_radius);
    if !extent.is_finite() {
        return Err(invalid(
            "Variable-width geometry extent is not representable.",
        ));
    }
    if extent == 0.0 {
        return Ok(None);
    }
    let tolerance = 0.05_f64.min(extent * 0.001);
    let magnitude = min
        .x
        .abs()
        .max(min.y.abs())
        .max(max.x.abs())
        .max(max.y.abs())
        .max(max_radius);
    if tolerance == 0.0 || magnitude * (32.0 * f64::EPSILON) > tolerance {
        return Err(invalid(
            "Variable-width geometry exceeds the supported coordinate precision.",
        ));
    }
    Ok(Some(tolerance))
}

#[derive(Clone, Copy)]
struct FlatPoint {
    point: Point,
    corner: bool,
}
#[derive(Default)]
struct Contour {
    points: Vec<FlatPoint>,
    closed: bool,
}
#[derive(Default)]
struct Budget {
    points: usize,
    work: usize,
}
impl Budget {
    fn point(&mut self) -> Result<(), DocumentError> {
        self.points += 1;
        if self.points > MAX_FLAT_POINTS {
            Err(invalid(
                "Variable-width outline exceeds the 65536-point work limit.",
            ))
        } else {
            Ok(())
        }
    }
    fn work(&mut self) -> Result<(), DocumentError> {
        self.work += 1;
        if self.work > MAX_SUBDIVISION_WORK {
            Err(invalid(
                "Variable-width curve subdivision work limit exceeded.",
            ))
        } else {
            Ok(())
        }
    }
}

impl Contour {
    fn push(
        &mut self,
        point: Point,
        corner: bool,
        budget: &mut Budget,
    ) -> Result<(), DocumentError> {
        if let Some(last) = self.points.last_mut() {
            if last.point == point {
                last.corner |= corner;
                return Ok(());
            }
        }
        budget.point()?;
        self.points.push(FlatPoint { point, corner });
        Ok(())
    }
}

fn flatten_path(
    path: &VectorPath,
    tolerance: f64,
    max_radius: f64,
    budget: &mut Budget,
) -> Result<Vec<Contour>, DocumentError> {
    let mut contours = Vec::new();
    let mut current: Option<Contour> = None;
    let arc_tolerance = tolerance / path.segments.len().max(1) as f64;
    for command in &path.segments {
        if let PathSegment::MoveTo { point } = *command {
            if let Some(contour) = current.take() {
                contours.push(contour);
            }
            let mut contour = Contour::default();
            contour.push(point, true, budget)?;
            current = Some(contour);
            continue;
        }
        let contour = current
            .as_mut()
            .ok_or_else(|| invalid("Variable-width path commands must begin with MoveTo."))?;
        if contour.closed {
            return Err(invalid(
                "A closed variable-width contour needs MoveTo before more commands.",
            ));
        }
        let start = contour.points.last().unwrap().point;
        match *command {
            PathSegment::LineTo { point } => contour.push(point, true, budget)?,
            PathSegment::QuadraticTo { control, point } => {
                flatten_curve(
                    [
                        start,
                        start.lerp(control, 2.0 / 3.0),
                        point.lerp(control, 2.0 / 3.0),
                        point,
                    ],
                    contour,
                    tolerance,
                    arc_tolerance,
                    max_radius,
                    budget,
                )?;
            }
            PathSegment::CubicTo {
                control1,
                control2,
                point,
            } => {
                flatten_curve(
                    [start, control1, control2, point],
                    contour,
                    tolerance,
                    arc_tolerance,
                    max_radius,
                    budget,
                )?;
            }
            PathSegment::Close => {
                contour.push(contour.points[0].point, true, budget)?;
                contour.closed = true;
            }
            PathSegment::MoveTo { .. } => unreachable!(),
        }
    }
    if let Some(contour) = current {
        contours.push(contour);
    }
    Ok(contours)
}

fn flatten_curve(
    curve: [Point; 4],
    contour: &mut Contour,
    tolerance: f64,
    arc_tolerance: f64,
    max_radius: f64,
    budget: &mut Budget,
) -> Result<(), DocumentError> {
    let mut stack = vec![(curve, 0_u8, 1.0, true)];
    while let Some((points, depth, span, authored_end)) = stack.pop() {
        budget.work()?;
        if curve_flat(points, tolerance, arc_tolerance * span, max_radius)? {
            contour.push(points[3], authored_end, budget)?;
            continue;
        }
        if depth == MAX_SUBDIVISION_DEPTH {
            return Err(invalid(
                "Variable-width curve cannot meet its precision bound within 32 subdivisions.",
            ));
        }
        let a = points[0].lerp(points[1], 0.5);
        let b = points[1].lerp(points[2], 0.5);
        let c = points[2].lerp(points[3], 0.5);
        let d = a.lerp(b, 0.5);
        let e = b.lerp(c, 0.5);
        let middle = d.lerp(e, 0.5);
        if [a, b, c, d, e, middle].iter().any(|point| !finite(*point)) {
            return Err(invalid(
                "Variable-width curve subdivision produced nonfinite geometry.",
            ));
        }
        stack.push((
            [middle, e, c, points[3]],
            depth + 1,
            span * 0.5,
            authored_end,
        ));
        stack.push(([points[0], a, d, middle], depth + 1, span * 0.5, false));
    }
    Ok(())
}

fn curve_flat(
    points: [Point; 4],
    tolerance: f64,
    arc_tolerance: f64,
    max_radius: f64,
) -> Result<bool, DocumentError> {
    let chord = points[0].distance(points[3]);
    let lengths = [
        points[0].distance(points[1]),
        points[1].distance(points[2]),
        points[2].distance(points[3]),
    ];
    let polygon = lengths.iter().sum::<f64>();
    if !chord.is_finite() || !polygon.is_finite() {
        return Err(invalid("Variable-width curve length is not representable."));
    }
    if chord == 0.0 {
        return Ok(polygon == 0.0);
    }
    if (polygon - chord).max(0.0) > arc_tolerance {
        return Ok(false);
    }
    let u = (points[3] - points[0]) * (1.0 / chord);
    for point in [points[1], points[2]] {
        let delta = point - points[0];
        let t = dot(delta, u).clamp(0.0, chord);
        if (delta - u * t).distance(Point::new(0.0, 0.0)) > tolerance {
            return Ok(false);
        }
    }
    for (pair, length) in points.windows(2).zip(lengths) {
        if length == 0.0 {
            continue;
        }
        let derivative = (pair[1] - pair[0]) * (1.0 / length);
        if dot(derivative, u) <= 0.0 || derivative.distance(u) * max_radius > tolerance {
            return Ok(false);
        }
    }
    Ok(true)
}

#[derive(Clone, Copy, Debug)]
struct WidthSample {
    position: f64,
    point: Point,
    radius: f64,
    corner: bool,
    incoming: Point,
}

fn width_samples(
    contour: &Contour,
    profile: &[StrokeWidthPoint],
    radius: f64,
    budget: &mut Budget,
) -> Result<Vec<WidthSample>, DocumentError> {
    if contour.points.len() < 2 {
        return Ok(Vec::new());
    }
    let lengths: Vec<_> = contour
        .points
        .windows(2)
        .map(|pair| pair[0].point.distance(pair[1].point))
        .collect();
    let total = lengths.iter().sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return Err(invalid(
            "Variable-width contour length is not representable.",
        ));
    }
    let mut result = vec![WidthSample {
        position: 0.0,
        point: contour.points[0].point,
        radius: radius * sample_points(profile, 0.0),
        corner: true,
        incoming: direction(contour.points[0].point, contour.points[1].point)?,
    }];
    let mut distance = 0.0;
    let mut knot = profile.partition_point(|point| point.position <= 0.0);
    for (index, pair) in contour.points.windows(2).enumerate() {
        // Keep the certified chord's direction when a width knot is extremely
        // close to a geometric sample. Re-normalizing that rounded tiny
        // interval could create a spurious perpendicular band or join.
        let incoming = direction(pair[0].point, pair[1].point)?;
        let end = if index + 1 == lengths.len() {
            total
        } else {
            distance + lengths[index]
        };
        if end <= distance {
            return Err(invalid(
                "Variable-width arc-length intervals exceed the supported coordinate precision.",
            ));
        }
        while knot < profile.len() && profile[knot].position * total < end {
            let target = profile[knot].position * total;
            if target > distance {
                budget.point()?;
                let point = pair[0]
                    .point
                    .lerp(pair[1].point, (target - distance) / lengths[index]);
                if point == result.last().unwrap().point || point == pair[1].point {
                    return Err(invalid(
                        "Stroke width knots are too close for the geometry's coordinate precision.",
                    ));
                }
                result.push(WidthSample {
                    position: profile[knot].position,
                    point,
                    radius: radius * profile[knot].width,
                    corner: false,
                    incoming,
                });
            }
            knot += 1;
        }
        let position = if index + 1 == lengths.len() {
            1.0
        } else {
            end / total
        };
        if position <= result.last().unwrap().position {
            return Err(invalid(
                "Stroke width knots exceed the supported arc-length precision.",
            ));
        }
        result.push(WidthSample {
            position,
            point: pair[1].point,
            radius: radius * sample_points(profile, position),
            corner: pair[1].corner,
            incoming,
        });
        distance = end;
    }
    Ok(result)
}

struct Outline {
    path: VectorPath,
    tolerance: f64,
}
impl Outline {
    fn polygon(&mut self, points: &[Point]) -> Result<(), DocumentError> {
        if points.len() < 3 {
            return Ok(());
        }
        if points.iter().any(|point| !finite(*point)) {
            return Err(invalid(
                "Variable-width outline contains a nonfinite coordinate.",
            ));
        }
        let origin = points[0];
        let extent = points
            .iter()
            .map(|&point| point.distance(origin))
            .fold(0.0, f64::max);
        if !extent.is_finite() {
            return Err(invalid(
                "Variable-width outline extent is not representable.",
            ));
        }
        if extent == 0.0 {
            return Ok(());
        }
        let mut area = 0.0;
        for index in 0..points.len() {
            area += cross(
                (points[index] - origin) * (1.0 / extent),
                (points[(index + 1) % points.len()] - origin) * (1.0 / extent),
            );
        }
        if area == 0.0 {
            return Ok(());
        }
        if self.path.segments.len() + points.len() + 1 > MAX_OUTPUT_COMMANDS {
            return Err(invalid(
                "Variable-width outline exceeds the 262144-command output limit.",
            ));
        }
        self.path
            .segments
            .push(PathSegment::MoveTo { point: points[0] });
        if area > 0.0 {
            self.path.segments.extend(
                points[1..]
                    .iter()
                    .map(|&point| PathSegment::LineTo { point }),
            );
        } else {
            self.path.segments.extend(
                points[1..]
                    .iter()
                    .rev()
                    .map(|&point| PathSegment::LineTo { point }),
            );
        }
        self.path.segments.push(PathSegment::Close);
        Ok(())
    }

    fn arc_sector(
        &mut self,
        center: Point,
        start: Point,
        angle: f64,
        first_radius: f64,
        last_radius: f64,
    ) -> Result<(), DocumentError> {
        let bound = first_radius.max(last_radius) * angle * angle
            + 2.0 * (last_radius - first_radius).abs() * angle.abs();
        let count = (bound / (8.0 * self.tolerance)).sqrt().ceil().max(1.0);
        if !count.is_finite() || count > MAX_FLAT_POINTS as f64 {
            return Err(invalid(
                "Variable-width round cap/join exceeds its subdivision limit.",
            ));
        }
        let count = count as usize;
        if self.path.segments.len() + count + 4 > MAX_OUTPUT_COMMANDS {
            return Err(invalid(
                "Variable-width outline exceeds the 262144-command output limit.",
            ));
        }
        let start_angle = start.y.atan2(start.x);
        let mut points = Vec::with_capacity(count + 2);
        points.push(center);
        for index in 0..=count {
            let t = index as f64 / count as f64;
            let (sin, cos) = (start_angle + angle * t).sin_cos();
            let radius = first_radius * (1.0 - t) + last_radius * t;
            points.push(center + Point::new(cos, sin) * radius);
        }
        self.polygon(&points)
    }

    fn join(
        &mut self,
        point: Point,
        incoming: Point,
        outgoing: Point,
        first_radius: f64,
        last_radius: f64,
        join: StrokeJoin,
    ) -> Result<(), DocumentError> {
        if first_radius == 0.0 && last_radius == 0.0 {
            return Ok(());
        }
        let turn = cross(incoming, outgoing);
        let cosine = dot(incoming, outgoing).clamp(-1.0, 1.0);
        if turn.abs() <= 32.0 * f64::EPSILON && cosine > 0.0 {
            return Ok(());
        }
        let side = if turn < 0.0 { 1.0 } else { -1.0 };
        let a_normal = normal(incoming) * side;
        let b_normal = normal(outgoing) * side;
        let (a, b) = (
            point + a_normal * first_radius,
            point + b_normal * last_radius,
        );
        if join == StrokeJoin::Round {
            let angle = if turn.abs() <= 32.0 * f64::EPSILON {
                std::f64::consts::PI
            } else {
                turn.atan2(cosine)
            };
            return self.arc_sector(point, a_normal, angle, first_radius, last_radius);
        }
        if join == StrokeJoin::Miter && turn.abs() > 32.0 * f64::EPSILON {
            let t = cross(b - a, outgoing) / turn;
            let s = cross(b - a, incoming) / turn;
            let miter = a + incoming * t;
            if t >= 0.0
                && s <= 0.0
                && finite(miter)
                && miter.distance(point) <= MITER_LIMIT * first_radius.max(last_radius)
            {
                return self.polygon(&[point, a, miter, b]);
            }
        }
        self.polygon(&[point, a, b])
    }

    fn cap(
        &mut self,
        sample: WidthSample,
        direction: Point,
        cap: StrokeCap,
        start: bool,
    ) -> Result<(), DocumentError> {
        if sample.radius == 0.0 || cap == StrokeCap::Butt {
            return Ok(());
        }
        let outward = direction * if start { -1.0 } else { 1.0 };
        let n = normal(outward);
        if cap == StrokeCap::Round {
            return self.arc_sector(
                sample.point,
                n * -1.0,
                std::f64::consts::PI,
                sample.radius,
                sample.radius,
            );
        }
        let (a, b) = (
            sample.point + n * sample.radius,
            sample.point - n * sample.radius,
        );
        self.polygon(&[
            a,
            b,
            b + outward * sample.radius,
            a + outward * sample.radius,
        ])
    }
}

fn direction(a: Point, b: Point) -> Result<Point, DocumentError> {
    let length = a.distance(b);
    if !length.is_finite() || length == 0.0 {
        return Err(invalid(
            "Variable-width path contains an unrepresentable interval.",
        ));
    }
    let result = (b - a) * (1.0 / length);
    if finite(result) {
        Ok(result)
    } else {
        Err(invalid(
            "Variable-width path direction is not representable.",
        ))
    }
}
fn normal(point: Point) -> Point {
    Point::new(-point.y, point.x)
}
fn dot(a: Point, b: Point) -> f64 {
    a.x * b.x + a.y * b.y
}
fn cross(a: Point, b: Point) -> f64 {
    a.x * b.y - a.y * b.x
}
fn finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}
fn invalid(message: &str) -> DocumentError {
    DocumentError::Invalid(message.into())
}

#[cfg(test)]
#[path = "stroke_profile_tests.rs"]
mod tests;
