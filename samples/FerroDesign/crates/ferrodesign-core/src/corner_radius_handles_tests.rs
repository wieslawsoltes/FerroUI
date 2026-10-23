use super::*;
use crate::{Document, PathSegment, Transform, Variable, VariableValue};

fn shape(kind: NodeKind, radius: f64) -> Node {
    let mut node = Node::new(kind, "Radius handles", 0.0, 0.0, 200.0, 160.0);
    node.corner_radius = radius;
    node.polygon_sides = 5;
    node.star_inner_radius = 0.45;
    node
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn box_handles_share_normalized_independent_radii_and_preserve_authored_values() {
    let mut node = shape(NodeKind::Rectangle, 8.0);
    node.corner_radii = Some([160.0, 80.0, 40.0, 120.0]);
    let before = node.clone();
    let handles = corner_radius_handles(&node);
    assert_eq!(handles.len(), 4);
    let normalized = crate::normalized_corner_radii(200.0, 160.0, node.corner_radii.unwrap());
    for (index, handle) in handles.iter().enumerate() {
        assert_eq!(handle.corner, index);
        assert_eq!(handle.authored_radius, node.corner_radii.unwrap()[index]);
        assert_eq!(handle.effective_radius, normalized[index]);
        assert_eq!(handle.vertex, node.local_bounds().corners()[index]);
        assert_eq!(
            handle.position,
            handle.vertex + handle.local_per_radius * normalized[index]
        );
    }
    node.set_local_transform(Transform {
        a: -1.1,
        b: 0.2,
        c: 0.4,
        d: 0.8,
        e: 230.0,
        f: -20.0,
    });
    assert_eq!(
        corner_radius_handles(&node),
        handles,
        "Camera/layer transforms do not rewrite local radius units"
    );
    node.set_local_transform(before.local_transform());
    assert_eq!(node.corner_radii, before.corner_radii);
    assert_eq!(node.corner_radius, before.corner_radius);
}

#[test]
fn polygon_and_concave_star_handle_centers_match_actual_arc_endpoints_at_every_clamp() {
    for kind in [NodeKind::Polygon, NodeKind::Star] {
        for radius in [0.0, 8.75, 1e300] {
            let node = shape(kind, radius);
            let handles = corner_radius_handles(&node);
            let vertices = node.polygon_points();
            assert_eq!(handles.len(), vertices.len());
            if radius == 0.0 {
                assert_eq!(
                    crate::polygon_corner_path(&node),
                    crate::VectorPath::polygon(&vertices, true)
                );
                assert!(handles.iter().all(
                    |handle| handle.position == handle.vertex && handle.effective_radius == 0.0
                ));
                continue;
            }
            let path = crate::polygon_corner_path(&node);
            let mut corner = 0;
            for segment in &path.segments {
                match *segment {
                    PathSegment::MoveTo { point } | PathSegment::LineTo { point } => {
                        let handle = &handles[corner];
                        near(point.distance(handle.position), handle.effective_radius);
                        corner += 1;
                    }
                    PathSegment::CubicTo { point, .. } => {
                        let handle = &handles[corner - 1];
                        near(point.distance(handle.position), handle.effective_radius);
                    }
                    PathSegment::Close => {}
                    _ => panic!(
                        "Procedural circular corners must retain their cubic path representation"
                    ),
                }
            }
            assert_eq!(corner, vertices.len());
            assert!(handles.iter().all(|handle| handle.authored_radius == radius
                && handle.effective_radius > 0.0
                && handle.effective_radius <= radius));
            assert_eq!(crate::outline_path(&node), path);
        }
    }
}

#[test]
fn corner_plan_work_is_bounded_and_degenerate_geometry_never_produces_nonfinite_handles() {
    for kind in [NodeKind::Rectangle, NodeKind::Polygon, NodeKind::Star] {
        let mut node = shape(kind, 10.0);
        node.polygon_sides = 10_000;
        let handles = corner_radius_handles(&node);
        assert!(handles.len() <= 256);
        assert!(handles.iter().all(|handle| finite(handle.vertex)
            && finite(handle.position)
            && finite(handle.local_per_radius)
            && handle.effective_radius.is_finite()));
        for size in [0.0, f64::NAN, f64::INFINITY] {
            node.width = size;
            assert!(corner_radius_handles(&node).is_empty());
        }
        node.width = 1e-310;
        node.height = 1e-310;
        assert!(corner_radius_handles(&node)
            .iter()
            .all(|handle| finite(handle.position) && finite(handle.local_per_radius)));
    }
    assert!(corner_radius_handles(&shape(NodeKind::Ellipse, 10.0)).is_empty());
    let mut star = shape(NodeKind::Star, 10.0);
    star.star_inner_radius = 0.0;
    assert!(corner_radius_handles(&star).is_empty());
}

fn bound_editor() -> (Editor, NodeId) {
    let mut document = Document::new("Independent corner metadata");
    document.variables.insert(
        900,
        Variable {
            id: 900,
            name: "Radius".into(),
            collection: "Shapes".into(),
            values: std::collections::BTreeMap::from([(
                "Default".into(),
                VariableValue::Number(8.0),
            )]),
        },
    );
    let mut node = shape(NodeKind::Rectangle, 8.0);
    node.variable_bindings.insert("corner_radius".into(), 900);
    let id = document.insert_node(1, node).unwrap();
    (Editor::new(document), id)
}

#[test]
fn independent_rectangle_radius_detaches_one_property_with_exact_atomic_history() {
    let (mut editor, id) = bound_editor();
    let before = editor.document.to_json().unwrap();
    assert_eq!(
        editor
            .set_individual_corner_radius(&[id, id], 1, 24.75)
            .unwrap(),
        1
    );
    let node = &editor.document.nodes[&id];
    assert_eq!(node.corner_radius, 8.0);
    assert_eq!(node.corner_radii, Some([8.0, 24.75, 8.0, 8.0]));
    assert!(!node.variable_bindings.contains_key("corner_radius"));
    assert_eq!(
        node.overridden_properties,
        std::collections::BTreeSet::from(["corner_radius".into()])
    );
    assert_eq!(editor.history_len(), 1);
    editor.document.validate().unwrap();
    let after = editor.document.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
    assert_eq!(
        editor
            .set_individual_corner_radius(&[id], 1, 24.75)
            .unwrap(),
        0
    );
    assert_eq!(editor.history_len(), 1);
    assert_eq!(editor.set_corner_radius(&[id], 12.5).unwrap(), 1);
    assert_eq!(editor.document.nodes[&id].corner_radii, None);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), after);
}

#[test]
fn independent_radius_validates_all_ids_and_skips_unsupported_or_inherited_locked_nodes() {
    let (mut editor, id) = bound_editor();
    editor.set_individual_corner_radius(&[id], 2, 9.0).unwrap();
    editor.undo();
    let before = editor.document.to_json().unwrap();
    for (ids, corner, value) in [
        (vec![id], 4, 8.0),
        (vec![id], 0, -1.0),
        (vec![id], 0, f64::NAN),
        (vec![id], 0, f64::INFINITY),
        (vec![id, u64::MAX], 0, 9.0),
    ] {
        assert!(editor
            .set_individual_corner_radius(&ids, corner, value)
            .is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(editor.can_redo());
    }
    let mut document = editor.document.clone();
    let mut parent = shape(NodeKind::Frame, 3.0);
    parent.locked = true;
    let parent = document.insert_node(1, parent).unwrap();
    let child = document
        .insert_node(parent, shape(NodeKind::Rectangle, 4.0))
        .unwrap();
    let star = document.insert_node(1, shape(NodeKind::Star, 5.0)).unwrap();
    let mut editor = Editor::new(document);
    let before = editor.document.to_json().unwrap();
    assert_eq!(
        editor
            .set_individual_corner_radius(&[parent, child, star], 0, 10.0)
            .unwrap(),
        0
    );
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.history_len(), 0);
}
