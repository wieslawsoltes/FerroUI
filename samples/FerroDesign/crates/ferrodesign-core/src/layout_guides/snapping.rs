//! Constant-work guide-line projection in transformed frame coordinates.
use crate::{LayoutGridPattern, Node, NodeId, Point, Transform, GEOMETRY_EPSILON};

pub const MAX_LAYOUT_GUIDE_SNAP_POINTS: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutGuideSnapLine {
    pub owner: NodeId,
    pub guide: usize,
    pub vertical: bool,
    /// Guide coordinate in its owner's local space.
    pub coordinate: f64,
    /// Original candidate point in world space; also permits final clip checks.
    pub source: Point,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutGuideSnap {
    pub delta: Point,
    pub lines: [Option<LayoutGuideSnapLine>; 2],
}

#[derive(Clone, Copy)]
struct Candidate { line: LayoutGuideSnapLine, local_delta: f64, delta: Point, distance: f64 }

/// Snap at most nine candidate points to square lines or row/column band edges.
///
/// `threshold` is a world-space distance (the canvas supplies five screen pixels
/// divided by zoom). `direction` restricts the correction to a world-space drag
/// axis or aspect-ratio ray. Hidden guides remain active; hidden owners are
/// filtered by the caller. The callback rejects points outside ancestor clips.
/// Dense guide axes with a period below four screen pixels are suppressed.
pub fn snap_to_layout_guides(node: &Node, world: Transform, points: &[Point], threshold: f64,
    direction: Option<Point>, mut visible: impl FnMut(Point) -> bool) -> Option<LayoutGuideSnap> {
    if !threshold.is_finite() || threshold <= 0.0 || !crate::LayoutGrid::supports(node.kind) { return None; }
    let inverse = world.inverse()?;
    let direction = match direction {
        Some(value) => {
            let length = value.x.hypot(value.y);
            if !length.is_finite() || length <= GEOMETRY_EPSILON { return None; }
            Some(value * (1.0 / length))
        }
        None => None,
    };
    let mut candidates: [Option<Candidate>; 2] = [None, None];
    for (index, guide) in node.layout_guides().take(super::MAX_LAYOUT_GUIDES).enumerate() {
        if guide.validate().is_err() { continue; }
        for vertical in [true, false] {
            if matches!(guide.pattern, LayoutGridPattern::Columns { .. }) && !vertical
                || matches!(guide.pattern, LayoutGridPattern::Rows { .. }) && vertical { continue; }
            let extent = if vertical { node.width } else { node.height };
            if !extent.is_finite() || extent <= 0.0 { continue; }
            let row = if vertical { Point::new(inverse.a, inverse.c) } else { Point::new(inverse.b, inverse.d) };
            let length = row.x.hypot(row.y);
            if !length.is_finite() || length <= GEOMETRY_EPSILON { continue; }
            let normal = row * (1.0 / length);
            let bands = if guide.pattern.is_square() { None } else {
                let Some(bands) = guide.bands(extent) else { continue; }; Some(bands)
            };
            let period = bands.map_or(guide.size, |bands| bands.step);
            if period / length < threshold * 0.8 { continue; }
            for &source in points.iter().take(MAX_LAYOUT_GUIDE_SNAP_POINTS) {
                if !source.x.is_finite() || !source.y.is_finite() { continue; }
                let local = inverse.apply(source);
                let coordinate = if vertical { local.x } else { local.y };
                let edges = if let Some(bands) = bands {
                    let first = ((coordinate - bands.start) / bands.step).round().clamp(0.0, (bands.count - 1) as f64);
                    let last = ((coordinate - bands.start - bands.size) / bands.step).round().clamp(0.0, (bands.count - 1) as f64);
                    [Some(bands.start + first * bands.step), Some(bands.start + last * bands.step + bands.size)]
                } else { [Some((coordinate / guide.size).round() * guide.size), None] };
                for target in edges.into_iter().flatten() {
                    if !target.is_finite() || target < 0.0 || target > extent { continue; }
                    let local_delta = target - coordinate;
                    let perpendicular = local_delta / length;
                    let delta = if let Some(direction) = direction {
                        let projection = normal.x * direction.x + normal.y * direction.y;
                        if projection.abs() <= GEOMETRY_EPSILON { continue; }
                        direction * (perpendicular / projection)
                    } else { normal * perpendicular };
                    let distance = delta.x.hypot(delta.y);
                    if !distance.is_finite() || distance > threshold { continue; }
                    let axis = usize::from(!vertical);
                    if candidates[axis].is_some_and(|previous| previous.distance <= distance) { continue; }
                    if !visible(source + delta) { continue; }
                    candidates[axis] = Some(Candidate { line: LayoutGuideSnapLine {
                        owner: node.id, guide: index, vertical, coordinate: target, source,
                    }, local_delta, delta, distance });
                }
            }
        }
    }
    if direction.is_none() {
        if let (Some(x), Some(y)) = (candidates[0], candidates[1]) {
            let delta = world.apply_vector(Point::new(x.local_delta, y.local_delta));
            // Highly sheared axes can intersect far outside either snap radius.
            // Prefer one honest nearby alignment over a large unexpected jump.
            if delta.x.hypot(delta.y) <= threshold * std::f64::consts::SQRT_2
                && visible(x.line.source + delta) && visible(y.line.source + delta) {
                return Some(LayoutGuideSnap { delta, lines: [Some(x.line), Some(y.line)] });
            }
        }
    }
    let closest = candidates.into_iter().flatten().min_by(|a, b| a.distance.total_cmp(&b.distance))?;
    let mut lines = [None, None]; lines[usize::from(!closest.line.vertical)] = Some(closest.line);
    Some(LayoutGuideSnap { delta: closest.delta, lines })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutGrid, LayoutGridAlignment, LayoutGridAxis, NodeKind};

    fn frame() -> Node {
        let mut node = Node::new(NodeKind::Frame, "Guide frame", 0.0, 0.0, 300.0, 200.0);
        node.id = 12; node.set_layout_guides(&[LayoutGrid { size: 20.0, ..LayoutGrid::default() }]); node
    }
    #[test]
    fn square_snaps_two_axes_exactly_and_restricts_shift_motion() {
        let node = frame(); let source = Point::new(38.0, 63.0);
        let snap = snap_to_layout_guides(&node, Transform::IDENTITY, &[source], 5.0, None, |_| true).unwrap();
        assert_eq!(snap.delta, Point::new(2.0, -3.0)); assert!(snap.lines.iter().all(Option::is_some));
        let locked = snap_to_layout_guides(&node, Transform::IDENTITY, &[source], 5.0, Some(Point::new(1.0, 0.0)), |_| true).unwrap();
        assert_eq!(locked.delta, Point::new(2.0, 0.0)); assert!(locked.lines[1].is_none());
        assert!(snap_to_layout_guides(&node, Transform::IDENTITY, &[Point::new(31.0, 71.0)], 5.0, None, |_| true).is_none());
    }
    #[test]
    fn hidden_bands_snap_both_edges_but_not_gutter_centers() {
        let mut node = frame();
        node.set_layout_guides(&[LayoutGrid { size: 40.0, visible: false, pattern: LayoutGridPattern::Columns {
            axis: LayoutGridAxis { alignment: LayoutGridAlignment::Start, count: Some(3), gutter: 20.0, offset: 10.0 }
        }, ..LayoutGrid::default() }]);
        for (source, target) in [(9.0, 10.0), (52.0, 50.0), (68.0, 70.0)] {
            let result = snap_to_layout_guides(&node, Transform::IDENTITY, &[Point::new(source, 35.0)], 5.0, None, |_| true).unwrap();
            assert_eq!(source + result.delta.x, target); assert_eq!(result.delta.y, 0.0);
        }
        assert!(snap_to_layout_guides(&node, Transform::IDENTITY, &[Point::new(60.0, 35.0)], 5.0, None, |_| true).is_none());
    }
    #[test]
    fn rotated_sheared_and_flipped_guides_use_screen_distance_and_parent_clips() {
        let node = frame();
        let transform = Transform::translation(80.0, 40.0) * Transform::rotation(31.0)
            * Transform { a: -1.2, b: 0.0, c: 0.3, d: 0.8, e: 0.0, f: 0.0 };
        let source = transform.apply(Point::new(38.0, 63.0));
        let snap = snap_to_layout_guides(&node, transform, &[source], 5.0, None, |_| true).unwrap();
        let actual = transform.inverse().unwrap().apply(source + snap.delta);
        assert!((actual.x - 40.0).abs() < 1e-8); assert!((actual.y - 60.0).abs() < 1e-8);
        assert!(snap_to_layout_guides(&node, transform, &[source], 5.0, None, |_| false).is_none());
        let horizontal = snap_to_layout_guides(&node, transform, &[source], 5.0, Some(Point::new(1.0, 0.0)), |_| true).unwrap();
        assert_eq!(horizontal.delta.y, 0.0);
    }
    #[test]
    fn aspect_ray_remains_collinear_and_near_singular_pairs_do_not_jump() {
        let node = frame(); let source = Point::new(38.0, 57.0);
        let snap = snap_to_layout_guides(&node, Transform::IDENTITY, &[source], 5.0, Some(Point::new(2.0, 1.0)), |_| true).unwrap();
        assert!((snap.delta.x - 2.0 * snap.delta.y).abs() < 1e-8);
        let transform = Transform { a: 1.0, b: 0.0, c: 10.0, d: 1.0, e: 0.0, f: 0.0 };
        let result = snap_to_layout_guides(&node, transform, &[transform.apply(Point::new(38.0, 63.0))], 5.0, None, |_| true).unwrap();
        assert!(result.delta.x.hypot(result.delta.y) <= 5.0 * std::f64::consts::SQRT_2);
    }
    #[test]
    fn dense_million_sections_and_excess_candidate_points_have_constant_work() {
        let mut node = frame(); node.width = 1_000_000_000.0;
        node.set_layout_guides(&[LayoutGrid { size: 10.0, pattern: LayoutGridPattern::Columns {
            axis: LayoutGridAxis { count: Some(1_000_000), alignment: LayoutGridAlignment::Start, gutter: 10.0, offset: 0.0 }
        }, ..LayoutGrid::default() }]);
        let mut tested = 0;
        let points = vec![Point::new(12_345_678.0, 55.0); 10_000];
        assert!(snap_to_layout_guides(&node, Transform::IDENTITY, &points, 5.0, None, |_| { tested += 1; true }).is_some());
        assert!(tested <= MAX_LAYOUT_GUIDE_SNAP_POINTS * 2);
        assert!(snap_to_layout_guides(&node, Transform::scale(0.001, 0.001), &points, 5.0, None, |_| true).is_none());
    }
}
