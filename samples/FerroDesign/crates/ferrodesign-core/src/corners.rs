//! Shared rounded/smoothed corner geometry for artwork, clips, picking and export.
//!
//! The construction follows Figma's published cubic/arc scheme: a corner of
//! radius R consumes (1 + smoothing) R along each adjacent rectangle edge;
//! the two collinear handles split the remaining approach in a 2:1 ratio.
//! https://www.figma.com/blog/desperately-seeking-squircles/
//! Curves are retained as cubic paths, never rasterized by the document engine.
use crate::{DocumentError, Editor, Node, NodeId, NodeKind, PathSegment, Point, VectorPath};

pub(crate) fn is_zero(value: &f64) -> bool { *value == 0.0 }

/// Scale adjacent radii together instead of independently clipping each one.
/// This preserves asymmetric corner proportions when a shape becomes small.
pub fn normalized_corner_radii(width: f64, height: f64, radii: [f64; 4]) -> [f64; 4] {
    let width = finite_positive(width);
    let height = finite_positive(height);
    let radii = radii.map(finite_positive);
    let ratio = |length: f64, sum: f64| if sum > 0.0 { (length / sum).min(1.0) } else { 1.0 };
    let scale = [ratio(width, radii[0] + radii[1]), ratio(width, radii[3] + radii[2]),
        ratio(height, radii[0] + radii[3]), ratio(height, radii[1] + radii[2])]
        .into_iter().fold(1.0_f64, f64::min);
    radii.map(|radius| radius * scale)
}

fn finite_positive(value: f64) -> f64 {
    if value.is_finite() { value.max(0.0) } else { 0.0 }
}

/// The outline of a frame's clipping box, including independent corner radii.
/// Smoothing retreats when adjacent corners consume an edge; radii retreat
/// only when their unsmoothed arcs no longer fit. At zero smoothing the usual
/// four quarter-circle cubics are produced, retaining legacy artwork.
pub fn corner_path(node: &Node) -> VectorPath {
    let (w, h) = (finite_positive(node.width), finite_positive(node.height));
    let r = normalized_corner_radii(w, h, node.corner_radii.unwrap_or([node.corner_radius; 4]));
    let requested = if node.corner_smoothing.is_finite() { node.corner_smoothing.clamp(0.0, 1.0) } else { 0.0 };
    let extra = |length: f64, sum: f64| {
        if sum > 0.0 { (length / sum - 1.0).clamp(0.0, requested) } else { requested }
    };
    let top = extra(w, r[0] + r[1]);
    let right = extra(h, r[1] + r[2]);
    let bottom = extra(w, r[2] + r[3]);
    let left = extra(h, r[3] + r[0]);
    let smooth = [top.min(left), top.min(right), bottom.min(right), bottom.min(left)];
    let p = std::array::from_fn::<_, 4, _>(|i| r[i] * (1.0 + smooth[i]));
    let mut path = VectorPath::default();
    path.segments.push(PathSegment::MoveTo { point: Point::new(p[0], 0.0) });
    // Each canonical corner starts at (-p, 0) and ends at (0, p).
    for (index, origin, x, y) in [
        (1, Point::new(w, 0.0), Point::new(1.0, 0.0), Point::new(0.0, 1.0)),
        (2, Point::new(w, h), Point::new(0.0, 1.0), Point::new(-1.0, 0.0)),
        (3, Point::new(0.0, h), Point::new(-1.0, 0.0), Point::new(0.0, -1.0)),
        (0, Point::new(0.0, 0.0), Point::new(0.0, -1.0), Point::new(1.0, 0.0)),
    ] {
        let map = |a: f64, b: f64| Point::new(origin.x + x.x * a + y.x * b, origin.y + x.y * a + y.y * b);
        path.segments.push(PathSegment::LineTo { point: map(-p[index], 0.0) });
        let radius = r[index];
        if radius == 0.0 {
            path.segments.push(PathSegment::LineTo { point: origin });
            continue;
        }
        let alpha = smooth[index] * std::f64::consts::FRAC_PI_4;
        let beta = std::f64::consts::FRAC_PI_2 - 2.0 * alpha;
        let (sin, cos) = alpha.sin_cos();
        let start = (-radius + radius * sin, radius * (1.0 - cos));
        let end = (radius * (cos - 1.0), radius * (1.0 - sin));
        // Intersection between the circle tangent and the straight incoming
        // edge. tan(alpha/2) is stable at zero, unlike (1-cos)/sin.
        let tangent = radius * (alpha * 0.5).tan();
        let c2x = start.0 - tangent * cos;
        let a = (c2x + p[index]) * (2.0 / 3.0);
        if smooth[index] > 0.0 {
            path.segments.push(PathSegment::CubicTo {
                control1: map(-p[index] + a, 0.0), control2: map(c2x, 0.0), point: map(start.0, start.1),
            });
        }
        if beta > 0.0 {
            let k = (4.0 / 3.0) * (beta * 0.25).tan() * radius;
            path.segments.push(PathSegment::CubicTo {
                control1: map(start.0 + k * cos, start.1 + k * sin),
                control2: map(end.0 - k * sin, end.1 - k * cos), point: map(end.0, end.1),
            });
        }
        if smooth[index] > 0.0 {
            path.segments.push(PathSegment::CubicTo {
                control1: map(0.0, -c2x), control2: map(0.0, p[index] - a), point: map(0.0, p[index]),
            });
        }
    }
    path.segments.push(PathSegment::Close);
    path
}

impl Editor {
    /// Commit a validated, bounded appearance change once for all editable box
    /// layers. Missing targets and invalid values fail before opening history.
    pub fn set_corner_smoothing(&mut self, ids: &[NodeId], value: f64) -> Result<usize, DocumentError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(DocumentError::Invalid("Corner smoothing must be between 0 and 1.".into()));
        }
        let mut changed = std::collections::BTreeSet::new();
        for &id in ids {
            let node = self.document.nodes.get(&id).ok_or(DocumentError::MissingNode(id))?;
            if !self.document.effective_locked(id) && node.corner_smoothing != value && matches!(node.kind,
                NodeKind::Rectangle | NodeKind::Frame | NodeKind::Component | NodeKind::Instance | NodeKind::Section)
            {
                changed.insert(id);
            }
        }
        if changed.is_empty() { return Ok(0); }
        let ids: Vec<_> = changed.into_iter().collect();
        self.update_nodes(&ids, "Corner smoothing", |node| {
            node.corner_smoothing = value;
            node.overridden_properties.insert("corner_smoothing".into());
        });
        Ok(ids.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Document, Transform};

    fn shape(radius: f64, smoothing: f64) -> Node {
        let mut node = Node::new(NodeKind::Rectangle, "Smooth", 0.0, 0.0, 200.0, 120.0).with_radius(radius);
        node.corner_smoothing = smoothing;
        node
    }

    #[test]
    fn smoothing_retains_cubics_symmetry_and_zero_curvature_at_edges() {
        let node = shape(20.0, 0.6);
        let path = corner_path(&node);
        assert_eq!(path.segments.iter().filter(|s| matches!(s, PathSegment::CubicTo { .. })).count(), 12);
        let PathSegment::LineTo { point: start } = path.segments[1] else { panic!() };
        let PathSegment::CubicTo { control1, control2, point } = path.segments[2] else { panic!() };
        assert_eq!(start, Point::new(168.0, 0.0));
        assert_eq!(control1.y, start.y);
        assert_eq!(control2.y, start.y);
        assert!(point.y > 0.0 && point.x > control2.x);
        for contour in path.flattened(0.01) {
            for point in &contour {
                assert!(node.local_bounds().inflate(1e-9).contains(*point));
                let mirrored = Point::new(node.width - point.x, node.height - point.y);
                assert!(contour.iter().any(|p| p.distance(mirrored) < 1e-7));
            }
        }
    }

    #[test]
    fn smoothing_recedes_before_radius_and_asymmetric_radii_stay_proportional() {
        let mut node = shape(60.0, 1.0);
        assert_eq!(corner_path(&node), corner_path(&shape(60.0, 0.0)));
        node.width = 60.0;
        node.height = 40.0;
        node.corner_radii = Some([60.0, 20.0, 40.0, 10.0]);
        let radii = normalized_corner_radii(node.width, node.height, node.corner_radii.unwrap());
        assert!(radii.into_iter().zip([240.0 / 7.0, 80.0 / 7.0, 160.0 / 7.0, 40.0 / 7.0])
            .all(|(a, b)| (a - b).abs() < 1e-12));
        assert!(corner_path(&node).flattened(0.02).iter().flatten()
            .all(|point| node.local_bounds().inflate(1e-8).contains(*point)));
    }

    #[test]
    fn independent_corners_and_degenerate_shapes_have_finite_closed_geometry() {
        for (width, height) in [(0.0, 0.0), (0.0, 100.0), (1e-9, 100.0), (100.0, 1e-9)] {
            let mut node = shape(200.0, 1.0);
            node.width = width; node.height = height;
            node.corner_radii = Some([200.0, 10.0, 40.0, 0.0]);
            let path = corner_path(&node);
            assert!(matches!(path.segments.last(), Some(PathSegment::Close)));
            assert!(path.flattened(0.1).iter().flatten().all(|p| p.x.is_finite() && p.y.is_finite()));
        }
    }

    #[test]
    fn shared_smoothing_geometry_survives_document_roundtrip_and_svg() {
        let mut doc = Document::new("Corners");
        let mut node = shape(20.0, 0.6).with_fill(Color::BLACK);
        node.set_local_transform(Transform { a: 1.0, b: 0.2, c: -0.3, d: 1.0, e: 35.0, f: 70.0 });
        let id = doc.insert_node(doc.pages[0].id, node).unwrap();
        let restored = Document::from_json(&doc.to_json().unwrap()).unwrap();
        assert_eq!(doc.nodes[&id].corner_smoothing, restored.nodes[&id].corner_smoothing);
        assert_eq!(crate::outline_path(&doc.nodes[&id]), corner_path(&doc.nodes[&id]));
        for bad in [-0.01, 1.01, f64::NAN, f64::INFINITY] {
            let mut invalid = doc.clone(); invalid.nodes.get_mut(&id).unwrap().corner_smoothing = bad;
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn smoothing_command_is_atomic_locked_aware_and_preserves_redo_on_noop() {
        let mut doc = Document::new("Corners");
        let page = doc.pages[0].id;
        let a = doc.insert_node(page, shape(20.0, 0.0)).unwrap();
        let mut locked = shape(20.0, 0.0); locked.locked = true;
        let b = doc.insert_node(page, locked).unwrap();
        let mut editor = Editor::new(doc);
        let original = editor.document.to_json().unwrap();
        assert!(editor.set_corner_smoothing(&[a, u64::MAX], 0.6).is_err());
        assert!(editor.set_corner_smoothing(&[a], f64::NAN).is_err());
        assert_eq!(editor.document.to_json().unwrap(), original);
        assert_eq!(editor.set_corner_smoothing(&[a, a, b], 0.6).unwrap(), 1);
        assert_eq!(editor.document.nodes[&b].corner_smoothing, 0.0);
        let after = editor.document.to_json().unwrap();
        assert!(editor.undo());
        assert_eq!(editor.document.to_json().unwrap(), original);
        assert_eq!(editor.set_corner_smoothing(&[a], 0.0).unwrap(), 0);
        assert!(editor.redo());
        assert_eq!(editor.document.to_json().unwrap(), after);
    }
}
