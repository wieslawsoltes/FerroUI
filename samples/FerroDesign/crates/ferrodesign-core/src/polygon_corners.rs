//! Circular fillets for the editable Polygon and Star primitives.
//!
//! The authored radius and node box are retained. Adjacent edges bound the
//! effective local radius; a common reduction makes their tangent distances
//! fit without reversing an edge. Convex and concave turns use the same signed
//! construction. Circular arcs are approximated by cubics of at most 90 degrees.
//! This defines our geometry, not an undocumented exact Figma numerical match.
use crate::{DocumentError, Editor, Node, NodeId, NodeKind, PathSegment, Point, VectorPath};

/// Kinds whose authored uniform radius currently changes their native outline.
pub fn supports_corner_radius(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Rectangle
            | NodeKind::Frame
            | NodeKind::Component
            | NodeKind::Instance
            | NodeKind::Section
            | NodeKind::Polygon
            | NodeKind::Star
    )
}

impl Editor {
    /// Change uniform box/polygon/star radii in one validated history step.
    /// Missing IDs or invalid numbers fail before any target changes. Locked
    /// and unsupported layers are skipped; direct edits detach the radius
    /// variable even when its currently resolved value matches the input.
    pub fn set_corner_radius(
        &mut self,
        ids: &[NodeId],
        radius: f64,
    ) -> Result<usize, DocumentError> {
        if !radius.is_finite() || radius < 0.0 {
            return Err(DocumentError::Invalid(
                "Corner radius must be finite and nonnegative.".into(),
            ));
        }
        let mut changed = std::collections::BTreeSet::new();
        for &id in ids {
            let node = self
                .document
                .nodes
                .get(&id)
                .ok_or(DocumentError::MissingNode(id))?;
            if !supports_corner_radius(node.kind) || self.document.effective_locked(id) {
                continue;
            }
            if node.corner_radius != radius
                || node.corner_radii.is_some()
                || node.variable_bindings.contains_key("corner_radius")
            {
                changed.insert(id);
            }
        }
        if changed.is_empty() {
            return Ok(0);
        }
        let ids: Vec<_> = changed.into_iter().collect();
        self.update_nodes(&ids, "Corner radius", |node| {
            node.corner_radius = radius;
            node.corner_radii = None;
            node.variable_bindings.remove("corner_radius");
            node.overridden_properties.insert("corner_radius".into());
        });
        Ok(ids.len())
    }
}

/// Return a closed local outline for a Polygon or Star, including its uniform
/// `corner_radius`. Neither independent rectangle radii nor corner smoothing
/// apply to this primitive. A zero radius retains the exact sharp path.
///
/// Work is linear in the existing bounded point count (at most 256 vertices,
/// two arc cubics per vertex). The authored dimensions and transform never
/// change when a rounded outline becomes smaller than the layer's box.
pub fn polygon_corner_path(node: &Node) -> VectorPath {
    if !matches!(node.kind, NodeKind::Polygon | NodeKind::Star)
        || !node.width.is_finite()
        || !node.height.is_finite()
        || node.width < 0.0
        || node.height < 0.0
        || (node.kind == NodeKind::Star && !node.star_inner_radius.is_finite())
    {
        return VectorPath::default();
    }
    let points = node.polygon_points();
    if node.width == 0.0
        || node.height == 0.0
        || (node.kind == NodeKind::Star && node.star_inner_radius <= 0.0)
    {
        return VectorPath::polygon(&points, true);
    }
    fillet_path(&points, node.corner_radius)
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Corner {
    pub(crate) incoming: Point,
    pub(crate) outgoing: Point,
    pub(crate) turn: f64,
    pub(crate) radius: f64,
    pub(crate) tangent_factor: f64,
}

fn dot(a: Point, b: Point) -> f64 {
    a.x * b.x + a.y * b.y
}
fn cross(a: Point, b: Point) -> f64 {
    a.x * b.y - a.y * b.x
}
fn rotate(point: Point, angle: f64) -> Point {
    let (sin, cos) = angle.sin_cos();
    Point::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
}

/// The same bounded corner plan drives artwork and native radius handles.
/// Callers validate finite vertices/radius; a zero radius still has directions.
pub(crate) fn fillet_corners(points: &[Point], radius: f64) -> (Vec<Corner>, f64) {
    let count = points.len();
    let lengths: Vec<_> = (0..count)
        .map(|index| points[index].distance(points[(index + 1) % count]))
        .collect();
    let corners: Vec<_> = (0..count)
        .map(|index| {
            let previous = (index + count - 1) % count;
            let next = (index + 1) % count;
            let incoming_length = lengths[previous];
            let outgoing_length = lengths[index];
            if incoming_length == 0.0
                || outgoing_length == 0.0
                || !incoming_length.is_finite()
                || !outgoing_length.is_finite()
            {
                return Corner::default();
            }
            // Component division remains finite for positive subnormal edge
            // lengths; multiplying by their reciprocal can produce infinity
            // before the vector components have a chance to cancel it.
            let before = points[index] - points[previous];
            let after = points[next] - points[index];
            let incoming = Point::new(before.x / incoming_length, before.y / incoming_length);
            let outgoing = Point::new(after.x / outgoing_length, after.y / outgoing_length);
            if ![incoming.x, incoming.y, outgoing.x, outgoing.y]
                .iter()
                .all(|value| value.is_finite())
            {
                return Corner::default();
            }
            let turn = cross(incoming, outgoing).atan2(dot(incoming, outgoing).clamp(-1.0, 1.0));
            // Straight and reversing edges have no unique finite fillet. Keeping
            // the original vertex also preserves zero-area/duplicate-point input.
            let sum = incoming + outgoing;
            let denominator = sum.x.hypot(sum.y);
            let difference = incoming - outgoing;
            let numerator = difference.x.hypot(difference.y);
            if denominator <= 1e-12 || numerator <= 1e-12 || turn == 0.0 {
                return Corner::default();
            }
            Corner {
                incoming,
                outgoing,
                turn,
                radius: radius.min(incoming_length * 0.5).min(outgoing_length * 0.5),
                tangent_factor: numerator / denominator,
            }
        })
        .collect();
    // Divide before multiplying so a large requested radius or a very acute
    // corner cannot overflow an intermediate tangent-distance sum.
    let mut scale = 1.0_f64;
    for index in 0..count {
        let length = lengths[index];
        if length == 0.0 || !length.is_finite() {
            continue;
        }
        let next = (index + 1) % count;
        let used = (corners[index].radius / length) * corners[index].tangent_factor
            + (corners[next].radius / length) * corners[next].tangent_factor;
        if used > 1.0 {
            scale = scale.min(used.recip());
        }
    }
    (corners, scale)
}

fn fillet_path(points: &[Point], radius: f64) -> VectorPath {
    let sharp = || VectorPath::polygon(points, true);
    if points.len() < 3 || !radius.is_finite() || radius <= 0.0 {
        return sharp();
    }
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return VectorPath::default();
    }
    let count = points.len();
    let (corners, scale) = fillet_corners(points, radius);
    if corners.iter().all(|corner| corner.radius * scale == 0.0) {
        return sharp();
    }
    let mut path = VectorPath::default();
    path.segments.reserve(count * 3 + 1);
    for index in 0..count {
        let corner = corners[index];
        let radius = corner.radius * scale;
        let trim = radius * corner.tangent_factor;
        let start = points[index] - corner.incoming * trim;
        let end = points[index] + corner.outgoing * trim;
        path.segments.push(if index == 0 {
            PathSegment::MoveTo { point: start }
        } else {
            PathSegment::LineTo { point: start }
        });
        if radius == 0.0 {
            continue;
        }
        append_arc(&mut path, start, end, corner, radius);
    }
    path.segments.push(PathSegment::Close);
    path
}

fn append_arc(path: &mut VectorPath, start: Point, end: Point, corner: Corner, radius: f64) {
    let pieces = (corner.turn.abs() / std::f64::consts::FRAC_PI_2)
        .ceil()
        .clamp(1.0, 2.0) as usize;
    let step = corner.turn / pieces as f64;
    let handle = (4.0 / 3.0) * (step.abs() * 0.25).tan() * radius;
    let normal = Point::new(-corner.incoming.y, corner.incoming.x);
    let center = start + normal * (radius * corner.turn.signum());
    let radial = start - center;
    let mut previous = start;
    let mut tangent = corner.incoming;
    for piece in 1..=pieces {
        let last = piece == pieces;
        let angle = step * piece as f64;
        let point = if last {
            end
        } else {
            center + rotate(radial, angle)
        };
        let next_tangent = if last {
            corner.outgoing
        } else {
            rotate(corner.incoming, angle)
        };
        path.segments.push(PathSegment::CubicTo {
            control1: previous + tangent * handle,
            control2: point - next_tangent * handle,
            point,
        });
        previous = point;
        tangent = next_tangent;
    }
}

#[cfg(test)]
#[path = "polygon_corners_tests.rs"]
mod tests;
