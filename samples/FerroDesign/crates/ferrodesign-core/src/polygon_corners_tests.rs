use super::*;
use crate::{Color, Document, Editor, Stroke, StrokeWidthProfile, Transform};

fn shape(kind: NodeKind, radius: f64) -> Node {
    let mut node = Node::new(kind, "Rounded primitive", 0.0, 0.0, 120.0, 120.0);
    node.polygon_sides = if kind == NodeKind::Star { 5 } else { 3 };
    node.star_inner_radius = 0.45;
    node.corner_radius = radius;
    node
}

fn cubic_count(path: &VectorPath) -> usize {
    path.segments
        .iter()
        .filter(|segment| matches!(segment, PathSegment::CubicTo { .. }))
        .count()
}

fn area(path: &VectorPath) -> f64 {
    path.flattened(0.001)
        .iter()
        .map(|contour| {
            contour
                .windows(2)
                .map(|pair| cross(pair[0], pair[1]) * 0.5)
                .sum::<f64>()
        })
        .sum()
}

fn all_points(path: &VectorPath) -> Vec<Point> {
    path.segments
        .iter()
        .flat_map(|segment| match *segment {
            PathSegment::MoveTo { point } | PathSegment::LineTo { point } => vec![point],
            PathSegment::CubicTo {
                control1,
                control2,
                point,
            } => vec![control1, control2, point],
            PathSegment::QuadraticTo { control, point } => vec![control, point],
            PathSegment::Close => Vec::new(),
        })
        .collect()
}

#[test]
fn zero_radius_retains_the_exact_legacy_polygon_and_star_commands() {
    for kind in [NodeKind::Polygon, NodeKind::Star] {
        for sides in [3, 5, 128] {
            let mut node = shape(kind, 0.0);
            node.polygon_sides = sides;
            assert_eq!(
                polygon_corner_path(&node),
                VectorPath::polygon(&node.polygon_points(), true)
            );
        }
    }
}

#[test]
fn quarter_circle_fillets_have_tangent_controls_and_expected_area() {
    let path = fillet_path(
        &[
            Point::new(0.0, 0.0),
            Point::new(120.0, 0.0),
            Point::new(120.0, 80.0),
            Point::new(0.0, 80.0),
        ],
        10.0,
    );
    assert_eq!(cubic_count(&path), 4);
    let PathSegment::MoveTo { point: start } = path.segments[0] else {
        panic!()
    };
    let PathSegment::CubicTo {
        control1,
        control2,
        point: end,
    } = path.segments[1]
    else {
        panic!()
    };
    assert_eq!(start, Point::new(0.0, 10.0));
    assert_eq!(end, Point::new(10.0, 0.0));
    assert_eq!(control1.x, start.x);
    assert_eq!(control2.y, end.y);
    let center = Point::new(10.0, 10.0);
    for sample in 0..=64 {
        let t = sample as f64 / 64.0;
        let point = start * (1.0 - t).powi(3)
            + control1 * (3.0 * (1.0 - t).powi(2) * t)
            + control2 * (3.0 * (1.0 - t) * t * t)
            + end * t.powi(3);
        assert!(
            (point.distance(center) - 10.0).abs() < 0.003,
            "Standard quarter-arc cubic error must stay bounded"
        );
    }
    let circular_area = 120.0 * 80.0 - (4.0 - std::f64::consts::PI) * 100.0;
    assert!((area(&path) - circular_area).abs() < 0.2);
}

#[test]
fn adjacent_tangent_distances_clamp_large_triangle_radii_without_reversing_edges() {
    let node = shape(NodeKind::Polygon, 1e300);
    let path = polygon_corner_path(&node);
    assert_eq!(
        cubic_count(&path),
        6,
        "Three 120-degree turns split into two cubics each"
    );
    let points = path.flattened(0.001);
    for point in &points[0] {
        assert!((point.distance(Point::new(60.0, 60.0)) - 30.0).abs() < 0.002);
    }
    assert!((area(&path) - std::f64::consts::PI * 900.0).abs() < 0.2);
    assert_eq!(
        node.corner_radius, 1e300,
        "The effective geometry does not rewrite the authored radius"
    );
    let mut previous = Point::default();
    let original = node.polygon_points();
    for segment in &path.segments {
        match *segment {
            PathSegment::MoveTo { point } | PathSegment::CubicTo { point, .. } => previous = point,
            PathSegment::LineTo { point } => {
                let direction = point - previous;
                assert!(
                    direction.x.hypot(direction.y) < 1e-10,
                    "Maximum fillets meet at the edge midpoint, without reversed line segments"
                );
                assert!(
                    original
                        .windows(2)
                        .any(|pair| crate::distance_to_segment(point, pair[0], pair[1]) < 1e-7)
                        || crate::distance_to_segment(point, original[2], original[0]) < 1e-7
                );
                previous = point;
            }
            _ => {}
        }
    }
}

#[test]
fn concave_star_rounds_both_tips_and_notches_and_preserves_winding() {
    let node = shape(NodeKind::Star, 12.0);
    let sharp = VectorPath::polygon(&node.polygon_points(), true);
    let rounded = polygon_corner_path(&node);
    let tip = Point::new(60.0, 1.0);
    assert!(sharp.contains(tip));
    assert!(!rounded.contains(tip));
    let notch = node.polygon_points()[1];
    let outward = (notch - Point::new(60.0, 60.0)) * (0.5 / 27.0);
    assert!(!sharp.contains(notch + outward));
    assert!(
        rounded.contains(notch + outward),
        "The concave fillet fills the sharp notch, rather than cutting another spike"
    );
    assert!(rounded.contains(Point::new(60.0, 60.0)));
    assert!(area(&rounded) > 0.0);
    let mut reversed = node.polygon_points();
    reversed.reverse();
    let reversed = fillet_path(&reversed, node.corner_radius);
    assert!((area(&rounded) + area(&reversed)).abs() < 1e-7);
    for y in (4..120).step_by(7) {
        for x in (4..120).step_by(7) {
            let point = Point::new(x as f64, y as f64);
            assert_eq!(rounded.contains(point), reversed.contains(point));
        }
    }
}

#[test]
fn fillets_remain_finite_for_zero_duplicate_straight_and_reversing_edges() {
    let degenerate = [
        vec![Point::new(0.0, 0.0); 4],
        vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(20.0, 0.0),
        ],
        vec![
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            Point::new(20.0, 0.0),
            Point::new(20.0, 20.0),
            Point::new(0.0, 20.0),
        ],
    ];
    for points in degenerate {
        let path = fillet_path(&points, 1e300);
        assert!(matches!(path.segments.last(), Some(PathSegment::Close)));
        assert!(all_points(&path)
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite()));
        assert!(path.segments.len() <= points.len() * 3 + 1);
    }
    for kind in [NodeKind::Polygon, NodeKind::Star] {
        for (width, height) in [
            (0.0, 0.0),
            (0.0, 100.0),
            (100.0, 0.0),
            (1e-9, 100.0),
            (100.0, 1e-9),
        ] {
            let mut node = shape(kind, 1e300);
            node.width = width;
            node.height = height;
            let path = polygon_corner_path(&node);
            assert!(all_points(&path)
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite()));
            assert!(matches!(path.segments.last(), Some(PathSegment::Close)));
        }
    }
    let mut zero_star = shape(NodeKind::Star, 20.0);
    zero_star.star_inner_radius = 0.0;
    assert_eq!(
        polygon_corner_path(&zero_star),
        VectorPath::polygon(&zero_star.polygon_points(), true)
    );
}

#[test]
fn rounded_paths_stay_inside_authored_boxes_with_bounded_command_counts() {
    for kind in [NodeKind::Polygon, NodeKind::Star] {
        for sides in [3, 4, 5, 17, 128] {
            for ratio in [0.001, 0.45, 1.0] {
                let mut node = shape(kind, 1e300);
                node.polygon_sides = sides;
                node.width = 200.0;
                node.height = 37.0;
                node.star_inner_radius = ratio;
                let path = polygon_corner_path(&node);
                assert!(all_points(&path)
                    .iter()
                    .all(|point| node.local_bounds().inflate(1e-8).contains(*point)));
                assert!(path.segments.len() <= node.polygon_points().len() * 3 + 1);
                assert!(cubic_count(&path) <= node.polygon_points().len() * 2);
            }
        }
    }
}

#[test]
fn polygon_radius_is_independent_of_rectangle_only_corner_settings() {
    let mut node = shape(NodeKind::Star, 12.0);
    let expected = polygon_corner_path(&node);
    node.corner_radii = Some([0.0, 100.0, 7.0, 50.0]);
    node.corner_smoothing = 1.0;
    assert_eq!(polygon_corner_path(&node), expected);
}

#[test]
fn affine_rounding_keeps_local_radius_and_preserves_path_area_by_determinant() {
    let mut node = shape(NodeKind::Star, 12.5);
    let local = polygon_corner_path(&node);
    let transform = Transform {
        a: -1.1,
        b: 0.3,
        c: 0.45,
        d: 0.8,
        e: 190.0,
        f: -37.0,
    };
    node.set_local_transform(transform);
    assert_eq!(polygon_corner_path(&node), local);
    let world = local.transformed(node.local_transform());
    assert!(
        (area(&world) - area(&local) * (transform.a * transform.d - transform.b * transform.c))
            .abs()
            < 0.2
    );
    assert_eq!(node.width, 120.0);
    assert_eq!(node.height, 120.0);
    assert!(world.contains(transform.apply(Point::new(60.0, 60.0))));
    assert!(!world.contains(transform.apply(Point::new(60.0, 1.0))));
}

#[test]
fn native_primitive_picking_outline_and_profiled_strokes_share_rounded_geometry() {
    for kind in [NodeKind::Polygon, NodeKind::Star] {
        let node = shape(kind, 12.0);
        assert!(!node.hit_test_local(Point::new(60.0, 1.0), 0.0));
        assert!(node.hit_test_local(Point::new(60.0, 60.0), 0.0));
        assert_eq!(crate::outline_path(&node), polygon_corner_path(&node));
        let stroke = Stroke {
            width_profile: StrokeWidthProfile::Eye,
            ..Stroke::new(Color::BLACK, 4.0)
        };
        let outline = crate::variable_width_outline(&node, &stroke).unwrap();
        let mut vector = node.clone();
        vector.kind = NodeKind::Vector;
        vector.path = polygon_corner_path(&node);
        vector.corner_radius = 0.0;
        assert_eq!(
            outline,
            crate::variable_width_outline(&vector, &stroke).unwrap()
        );
    }
}

#[test]
fn radius_roundtrip_validation_and_one_step_history_preserve_authored_geometry() {
    let mut document = Document::new("Polygon radius history");
    let page = document.pages[0].id;
    let mut node = shape(NodeKind::Star, 0.0);
    node.set_local_transform(Transform {
        a: -1.0,
        b: 0.2,
        c: 0.3,
        d: 1.0,
        e: 140.0,
        f: 5.0,
    });
    let id = document.insert_node(page, node).unwrap();
    let mut locked = shape(NodeKind::Polygon, 0.0);
    locked.locked = true;
    let locked = document.insert_node(page, locked).unwrap();
    let mut editor = Editor::new(document);
    let before = editor.document.to_json().unwrap();
    let original_transform = editor.document.nodes[&id].local_transform();
    assert_eq!(
        editor.set_corner_radius(&[id, id, locked], 12.5).unwrap(),
        1
    );
    assert_eq!(editor.document.nodes[&locked].corner_radius, 0.0);
    assert_eq!(
        editor.document.nodes[&id].local_transform(),
        original_transform
    );
    let after = editor.document.to_json().unwrap();
    assert_eq!(editor.history_len(), 1);
    let restored = Document::from_json(&after).unwrap();
    assert_eq!(
        polygon_corner_path(&restored.nodes[&id]),
        polygon_corner_path(&editor.document.nodes[&id])
    );
    assert!(
        restored.export_svg(page, &[id]).contains(" C"),
        "SVG must retain the curved primitive outline"
    );
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.set_corner_radius(&[id], 0.0).unwrap(), 0);
    for bad in [-0.1, f64::NAN, f64::INFINITY] {
        assert!(editor.set_corner_radius(&[id], bad).is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(editor.can_redo());
    }
    assert!(editor.set_corner_radius(&[id, u64::MAX], 12.5).is_err());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.can_redo());
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
    for bad in [-0.1, f64::NAN, f64::INFINITY] {
        let mut invalid = restored.clone();
        invalid.nodes.get_mut(&id).unwrap().corner_radius = bad;
        assert!(invalid.validate().is_err());
    }
}

#[test]
fn radius_command_preserves_inherited_locks_and_skips_unsupported_mixed_targets() {
    let mut document = Document::new("Corner eligibility");
    let page = document.pages[0].id;
    let mut parent = Node::new(NodeKind::Frame, "Locked frame", 0.0, 0.0, 200.0, 200.0);
    parent.locked = true;
    let parent = document.insert_node(page, parent).unwrap();
    let child = document
        .insert_node(parent, shape(NodeKind::Star, 0.0))
        .unwrap();
    let ellipse = document
        .insert_node(
            page,
            Node::new(NodeKind::Ellipse, "Ellipse", 0.0, 0.0, 120.0, 120.0),
        )
        .unwrap();
    let mut rectangle = Node::new(
        NodeKind::Rectangle,
        "Independent corners",
        0.0,
        0.0,
        100.0,
        100.0,
    );
    rectangle.corner_radii = Some([1.0, 2.0, 3.0, 4.0]);
    let rectangle = document.insert_node(page, rectangle).unwrap();
    let polygon = document
        .insert_node(page, shape(NodeKind::Polygon, 0.0))
        .unwrap();
    let mut editor = Editor::new(document);
    assert_eq!(
        editor
            .set_corner_radius(&[child, ellipse, rectangle, polygon], 8.25)
            .unwrap(),
        2
    );
    assert_eq!(editor.document.nodes[&child].corner_radius, 0.0);
    assert_eq!(editor.document.nodes[&ellipse].corner_radius, 0.0);
    assert_eq!(editor.document.nodes[&rectangle].corner_radii, None);
    assert_eq!(editor.document.nodes[&rectangle].corner_radius, 8.25);
    assert_eq!(editor.document.nodes[&polygon].corner_radius, 8.25);
    assert!(editor.document.nodes[&polygon]
        .overridden_properties
        .contains("corner_radius"));
    assert_eq!(editor.history_len(), 1);
}

#[test]
fn same_value_direct_radius_edit_detaches_variable_with_exact_undo_redo() {
    use crate::{Variable, VariableValue};
    let mut document = Document::new("Bound polygon radius");
    let page = document.pages[0].id;
    document.variables.insert(
        900,
        Variable {
            id: 900,
            name: "Radius".into(),
            collection: "Shape".into(),
            values: std::collections::BTreeMap::from([(
                "Default".into(),
                VariableValue::Number(12.5),
            )]),
        },
    );
    let mut node = shape(NodeKind::Star, 12.5);
    node.variable_bindings.insert("corner_radius".into(), 900);
    let id = document.insert_node(page, node).unwrap();
    let mut editor = Editor::new(document);
    let before = editor.document.to_json().unwrap();
    assert_eq!(editor.set_corner_radius(&[id], 12.5).unwrap(), 1);
    assert!(!editor.document.nodes[&id]
        .variable_bindings
        .contains_key("corner_radius"));
    let after = editor.document.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
    assert_eq!(editor.set_corner_radius(&[id], 12.5).unwrap(), 0);
    assert_eq!(editor.history_len(), 1);
}
