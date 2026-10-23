use super::*;
use crate::{Color, Document, Fill, GradientStop, Paint};

fn p(x: f64, y: f64) -> Point { Point::new(x, y) }

fn square(network: &mut VectorNetwork, x: f64, y: f64, size: f64) -> Vec<VectorVertexId> {
    let vertices: Vec<_> = [p(x, y), p(x + size, y), p(x + size, y + size), p(x, y + size)]
        .into_iter().map(|point| network.add_vertex(point).unwrap()).collect();
    for (&a, &b) in vertices.iter().zip(vertices.iter().cycle().skip(1)) { network.connect(a, b).unwrap(); }
    vertices
}

fn red() -> Vec<Fill> { vec![Fill::solid(Color::rgba(240, 20, 30, 128))] }
fn blue() -> Vec<Fill> { vec![Fill::solid(Color::rgba(20, 70, 230, 128))] }

fn fixture() -> (Editor, NodeId) {
    let mut document = Document::new("Region paints");
    let mut network = VectorNetwork::default();
    let vertices = square(&mut network, 0.0, 0.0, 100.0);
    network.connect(vertices[0], vertices[2]).unwrap();
    let mut node = Node::new(NodeKind::Vector, "Faces", 20.0, 30.0, 100.0, 100.0);
    node.fills = red();
    node.vector_network = Some(network);
    let id = document.insert_node(document.pages[0].id, node).unwrap();
    (Editor::new(document), id)
}

#[test]
fn region_paints_legacy_json_and_explicit_empty_round_trip_without_schema_change() {
    let mut network = VectorNetwork::default();
    let vertices = square(&mut network, 0.0, 0.0, 100.0);
    let region = network.region_from_vertices(&vertices).unwrap();
    let json = serde_json::to_string(&network).unwrap();
    assert!(!json.contains("\"fills\""));
    let legacy: VectorNetwork = serde_json::from_str(&json).unwrap();
    assert_eq!(legacy, network);
    assert_eq!(legacy.regions[&region].fills, None);
    network.set_region_fills(region, Some(Vec::new())).unwrap();
    let json = serde_json::to_string(&network).unwrap();
    assert!(json.contains("\"fills\":[]"));
    let restored: VectorNetwork = serde_json::from_str(&json).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, network);
    assert!(!restored.contains_fill_with_paints(p(25.0, 50.0), &red()));
    assert_eq!(restored.region_at(p(25.0, 50.0)), Some(region));
}

#[test]
fn region_paints_bucket_replaces_inherited_square_without_changing_geometry_or_ids() {
    for fill_rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let mut network = VectorNetwork { fill_rule, ..VectorNetwork::default() };
        let vertices = square(&mut network, 0.0, 0.0, 100.0);
        let original_region = network.region_from_vertices(&vertices).unwrap();
        network.connect(vertices[0], vertices[2]).unwrap();
        let before = network.clone();
        let upper = p(75.0, 25.0);
        let lower = p(25.0, 75.0);
        assert!(network.paint_face_at(upper, Some(blue())).unwrap());
        let painted = network.explicit_region_at(upper).unwrap();
        assert_ne!(painted, original_region);
        assert_eq!(network.vertices, before.vertices);
        assert_eq!(network.edges, before.edges);
        assert_eq!(network.regions[&original_region], before.regions[&original_region]);
        assert_eq!(network.inherited_fill_path(), before.fill_path());
        assert_eq!(network.regions[&painted].fills, Some(blue()));
        assert!(network.region_fill_path(painted).unwrap().contains(upper));
        assert!(!network.region_fill_path(painted).unwrap().contains(lower));
        assert!(network.contains_fill_with_paints(lower, &red()));
        assert!(network.contains_fill_with_paints(upper, &[]));
        let next = network.next_id;
        assert!(!network.paint_face_at(upper, Some(blue())).unwrap());
        assert_eq!(network.next_id, next);
        assert!(network.paint_face_at(upper, Some(Vec::new())).unwrap());
        assert_eq!(network.next_id, next);
        assert!(!network.contains_fill_with_paints(upper, &red()));
        assert!(network.contains_fill_with_paints(lower, &red()));
        assert!(network.paint_face_at(upper, None).unwrap());
        assert_eq!(network.regions, before.regions, "Reset reveals the original compound coverage");
        assert_eq!(network.next_id, next, "Removed stable IDs remain retired");
        assert!(network.contains_fill_with_paints(upper, &red()));
        assert!(!network.paint_face_at(upper, None).unwrap());
    }
}

#[test]
fn region_paints_reset_respects_compound_evenodd_holes_and_nested_faces() {
    let mut network = VectorNetwork { fill_rule: FillRule::EvenOdd, ..VectorNetwork::default() };
    let outer = square(&mut network, 0.0, 0.0, 200.0);
    network.region_from_vertices(&outer).unwrap();
    let inner = square(&mut network, 30.0, 30.0, 140.0);
    network.region_from_vertices(&inner).unwrap();
    let before = network.clone();
    assert!(network.contains_fill_with_paints(p(10.0, 10.0), &red()));
    assert!(!network.contains_fill_with_paints(p(100.0, 100.0), &red()));
    assert_eq!(network.region_at(p(100.0, 100.0)), None);
    assert!(network.paint_face_at(p(10.0, 10.0), Some(blue())).unwrap());
    let painted = network.explicit_region_at(p(10.0, 10.0)).unwrap();
    assert_eq!(network.regions[&painted].contours.len(), 2);
    assert!(!network.region_contains(painted, p(100.0, 100.0)));
    assert!(!network.contains_fill_with_paints(p(100.0, 100.0), &red()));
    assert!(network.set_region_fills(painted, None).unwrap());
    assert_eq!(network.regions, before.regions);
    assert_eq!(network.inherited_fill_path(), before.fill_path());
}

#[test]
fn region_paints_later_explicit_footprint_replaces_even_when_transparent() {
    let mut network = VectorNetwork::default();
    let vertices = square(&mut network, 0.0, 0.0, 100.0);
    let background = network.region_from_vertices(&vertices).unwrap();
    network.set_region_fills(background, Some(red())).unwrap();
    network.connect(vertices[0], vertices[2]).unwrap();
    let upper = p(75.0, 25.0);
    let lower = p(25.0, 75.0);
    assert!(network.paint_face_at(upper, Some(blue())).unwrap());
    let top = network.explicit_region_at(upper).unwrap();
    assert!(top > background);
    assert_eq!(network.explicit_region_at(lower), Some(background));
    assert!(network.paint_face_at(upper, Some(Vec::new())).unwrap());
    assert_eq!(network.explicit_region_at(upper), Some(top));
    assert!(!network.contains_fill_with_paints(upper, &red()));
    assert!(network.contains_fill_with_paints(lower, &[]));
    assert!(network.paint_face_at(upper, None).unwrap());
    assert_eq!(network.explicit_region_at(upper), Some(background));
    assert!(network.contains_fill_with_paints(upper, &[]));
    assert!(network.region_fill_path(top).is_none());
}

#[test]
fn region_paints_new_face_reset_restores_live_inheritance_and_full_stack() {
    let (mut editor, id) = fixture();
    let point = p(25.0, 75.0);
    let fills = vec![Fill {
        paint: Paint::AngularGradient {
            center: p(0.25, 0.75), angle: 37.0,
            stops: vec![GradientStop { offset: 0.0, color: Color::WHITE }, GradientStop { offset: 1.0, color: Color::BLACK }],
        }, opacity: 0.3, visible: true,
    }, Fill { paint: Paint::Solid(Color::rgba(1, 2, 3, 40)), opacity: 0.7, visible: false }];
    assert!(editor.paint_network_face(id, point, Some(fills.clone())).unwrap());
    let network = editor.document.nodes[&id].vector_network.as_ref().unwrap();
    let region = network.explicit_region_at(point).unwrap();
    let next_id = network.next_id;
    assert_eq!(network.regions[&region].effective_fills(&red()), fills.as_slice());
    assert!(editor.set_network_region_fills(id, region, None).unwrap());
    let network = editor.document.nodes[&id].vector_network.as_ref().unwrap();
    assert_eq!(network.regions[&region].fills, None);
    assert_eq!(network.next_id, next_id);
    assert!(network.contains_fill_with_paints(point, &red()));
    assert!(!network.contains_fill_with_paints(point, &[]));
    let revision = editor.revision;
    assert!(!editor.set_network_region_fills(id, region, None).unwrap());
    assert_eq!(editor.revision, revision);
}

#[test]
fn region_paints_atomic_history_affine_ownership_styles_and_locks() {
    let (mut editor, id) = fixture();
    editor.document.nodes.get_mut(&id).unwrap().set_local_transform(
        Transform::translation(60.0, 90.0) * Transform { a: -1.2, b: 0.3, c: 0.4, d: 0.7, e: 0.0, f: 0.0 });
    editor.refresh();
    let style = editor.create_local_style_for_nodes(&[id], crate::LocalStyleKind::Fill, "Shared layer paint", "").unwrap();
    editor.selection = vec![id];
    editor.clear_history();
    let before = editor.document.to_json().unwrap();
    let transform = editor.document.world_transform(id);
    assert!(editor.paint_network_face(id, p(75.0, 25.0), Some(blue())).unwrap());
    assert_eq!(editor.document.world_transform(id), transform);
    assert_eq!(editor.document.nodes[&id].fill_style, Some(style));
    assert_eq!(editor.document.nodes[&id].fills, red());
    assert_eq!(editor.history_len(), 1);
    assert_eq!(editor.selection, vec![id]);
    let after = editor.document.to_json().unwrap();
    let revision = editor.revision;
    assert!(!editor.paint_network_face(id, p(75.0, 25.0), Some(blue())).unwrap());
    assert!(!editor.paint_network_face(id, p(-100.0, -100.0), Some(blue())).unwrap());
    assert_eq!(editor.revision, revision);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    let revision = editor.revision;
    assert!(!editor.paint_network_face(id, p(-100.0, -100.0), Some(blue())).unwrap());
    assert!(editor.paint_network_face(999999, p(75.0, 25.0), Some(blue())).is_err());
    assert!(editor.set_network_region_fills(id, 999999, Some(blue())).is_err());
    assert_eq!(editor.revision, revision);
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.can_redo());
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
    editor.document.nodes.get_mut(&id).unwrap().locked = true;
    let before = editor.document.to_json().unwrap();
    assert_eq!(editor.paint_network_face(id, p(75.0, 25.0), Some(red())), Err(NetworkFaceError::NotEditable));
    assert_eq!(editor.document.to_json().unwrap(), before);
}

#[test]
fn region_paints_invalid_inputs_and_exhausted_ids_never_mutate() {
    let (editor, id) = fixture();
    let mut network = editor.document.nodes[&id].vector_network.clone().unwrap();
    let invalid = [
        Fill { paint: Paint::Solid(Color::WHITE), opacity: f64::NAN, visible: true },
        Fill { paint: Paint::Solid(Color::WHITE), opacity: -0.1, visible: true },
        Fill { paint: Paint::LinearGradient { start: p(0.0, 0.0), end: p(f64::INFINITY, 1.0), stops: vec![] }, opacity: 1.0, visible: true },
        Fill { paint: Paint::RadialGradient { center: p(0.5, 0.5), radius: 0.0, stops: vec![] }, opacity: 1.0, visible: true },
        Fill { paint: Paint::AngularGradient { center: p(0.5, 0.5), angle: 0.0, stops: vec![GradientStop { offset: 0.8, color: Color::WHITE }, GradientStop { offset: 0.2, color: Color::BLACK }] }, opacity: 1.0, visible: true },
        Fill { paint: Paint::Image { href: " ".into(), scale: crate::ImageScale::Fill, crop: None }, opacity: 1.0, visible: true },
        Fill { paint: Paint::Image { href: "data:image/png;base64,AA==".into(), scale: crate::ImageScale::Fill, crop: Some(Rect::new(0.0, 0.0, 0.0, 1.0)) }, opacity: 1.0, visible: true },
    ];
    for fill in invalid {
        let before = network.clone();
        assert!(matches!(network.paint_face_at(p(75.0, 25.0), Some(vec![fill])), Err(NetworkFaceError::InvalidRegionFills(_))));
        assert_eq!(network, before);
    }
    let before = network.clone();
    assert!(!network.paint_face_at(p(f64::NAN, 0.0), Some(red())).unwrap());
    assert_eq!(network, before);
    network.next_id = u64::MAX;
    let before = network.clone();
    assert_eq!(network.paint_face_at(p(75.0, 25.0), Some(red())), Err(NetworkFaceError::IdExhausted));
    assert_eq!(network, before);
}

#[test]
fn region_paints_import_validation_rejects_invalid_explicit_stacks() {
    let (mut editor, id) = fixture();
    editor.paint_network_face(id, p(75.0, 25.0), Some(red())).unwrap();
    let region = editor.document.nodes[&id].vector_network.as_ref().unwrap().explicit_region_at(p(75.0, 25.0)).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&editor.document.to_json().unwrap()).unwrap();
    json["nodes"][id.to_string()]["vector_network"]["regions"][region.to_string()]["fills"][0]["opacity"] = serde_json::json!(2.0);
    assert!(Document::from_json(&serde_json::to_string(&json).unwrap()).is_err());
}

#[test]
fn region_paints_fill_all_preserves_stacks_and_never_adds_unused_default_fill() {
    let (mut editor, id) = fixture();
    editor.document.nodes.get_mut(&id).unwrap().fills.clear();
    editor.refresh();
    assert!(editor.paint_network_face(id, p(75.0, 25.0), Some(blue())).unwrap());
    assert!(editor.paint_network_face(id, p(25.0, 75.0), Some(Vec::new())).unwrap());
    let before = editor.document.to_json().unwrap();
    let revision = editor.revision;
    assert!(!editor.fill_network_faces(id).unwrap());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.revision, revision);
    let network = editor.document.nodes[&id].vector_network.as_ref().unwrap();
    assert!(network.contains_fill_with_paints(p(75.0, 25.0), &[]));
    assert!(!network.contains_fill_with_paints(p(25.0, 75.0), &[]));
}

#[test]
fn region_paints_fill_all_preserves_composite_overrides_and_priority() {
    let mut network = VectorNetwork::default();
    let vertices = square(&mut network, 0.0, 0.0, 100.0);
    let composite = network.region_from_vertices(&vertices).unwrap();
    network.set_region_fills(composite, Some(blue())).unwrap();
    network.connect(vertices[0], vertices[2]).unwrap();
    network.paint_face_at(p(75.0, 25.0), Some(Vec::new())).unwrap();
    let empty = network.explicit_region_at(p(75.0, 25.0)).unwrap();
    let composite_before = network.regions[&composite].clone();
    assert!(network.fill_all_faces().unwrap());
    assert_eq!(network.regions[&composite], composite_before);
    assert_eq!(network.regions[&empty].fills, Some(Vec::new()));
    assert_eq!(network.explicit_region_at(p(25.0, 75.0)), Some(composite));
    assert_eq!(network.explicit_region_at(p(75.0, 25.0)), Some(empty));
    assert!(!network.contains_fill_with_paints(p(75.0, 25.0), &red()));
    assert!(network.contains_fill_with_paints(p(25.0, 75.0), &[]));
    let before = network.clone();
    assert!(!network.fill_all_faces().unwrap());
    assert_eq!(network, before);
}

#[test]
fn region_paints_split_curve_and_transform_keep_owned_stacks_and_region_ids() {
    let mut network = VectorNetwork::default();
    let a = network.add_vertex(p(0.0, 50.0)).unwrap();
    let b = network.add_vertex(p(100.0, 50.0)).unwrap();
    let top = network.connect_curve(a, b, NetworkCurve::Cubic { control1: p(0.0, 0.0), control2: p(100.0, 0.0) }).unwrap();
    let bottom = network.connect_curve(b, a, NetworkCurve::Cubic { control1: p(100.0, 100.0), control2: p(0.0, 100.0) }).unwrap();
    let region = network.add_region(vec![vec![NetworkEdgeRef::forward(top), NetworkEdgeRef::forward(bottom)]]).unwrap();
    network.set_region_fills(region, Some(blue())).unwrap();
    let mut reversed = network.regions[&region].contours[0].clone();
    reversed.reverse();
    for reference in &mut reversed { reference.reversed = !reference.reversed; }
    network.regions.get_mut(&region).unwrap().contours[0] = reversed;
    assert!(!network.fill_all_faces().unwrap(), "Canonical reversal must retain the existing region and paint");
    let next = network.next_id;
    network.split_edge(top, 0.37).unwrap();
    assert_eq!(network.next_id, next + 2);
    assert_eq!(network.regions[&region].fills, Some(blue()));
    assert_eq!(network.regions[&region].contours[0].len(), 3);
    let transformed = network.transformed(Transform::translation(250.0, 100.0) * Transform::scale(-2.0, 0.5));
    assert_eq!(transformed.regions, network.regions);
    assert!(transformed.contains_fill_with_paints(p(150.0, 125.0), &[]));
}

#[test]
fn region_paints_mixed_composite_reset_is_rejected_atomically() {
    let mut network = VectorNetwork::default();
    let vertices = square(&mut network, 0.0, 0.0, 100.0);
    network.connect(vertices[0], vertices[2]).unwrap();
    let inherited = network.face_at(p(75.0, 25.0)).unwrap().unwrap();
    network.add_region(inherited.contours).unwrap();
    let composite = network.region_from_vertices(&vertices).unwrap();
    network.set_region_fills(composite, Some(blue())).unwrap();
    let before = network.clone();
    assert_eq!(network.set_region_fills(composite, None), Err(NetworkFaceError::AmbiguousRegionReset(composite)));
    assert_eq!(network, before);
}

#[test]
fn region_paints_component_override_survives_reconciliation_with_local_style() {
    let mut document = Document::new("Region component ownership");
    let page = document.pages[0].id;
    let master = document.insert_node(page, Node::new(NodeKind::Component, "Master", 0.0, 0.0, 200.0, 200.0)).unwrap();
    let mut network = VectorNetwork::default();
    square(&mut network, 0.0, 0.0, 100.0);
    network.fill_all_faces().unwrap();
    let mut source = Node::new(NodeKind::Vector, "Artwork", 10.0, 10.0, 100.0, 100.0);
    source.fills = red();
    source.vector_network = Some(network);
    let source_id = document.insert_node(master, source).unwrap();
    let mut editor = Editor::new(document);
    let style = editor.create_local_style_for_nodes(&[source_id], crate::LocalStyleKind::Fill, "Region inherited", "").unwrap();
    let instance = editor.create_instance(master, 300.0, 0.0).unwrap();
    let child = editor.document.nodes[&instance].children[0];
    assert!(editor.paint_network_face(child, p(25.0, 25.0), Some(blue())).unwrap());
    assert!(editor.document.nodes[&child].overridden_properties.contains("path"));
    assert_eq!(editor.document.nodes[&child].fill_style, Some(style));
    let painted = editor.document.nodes[&child].vector_network.clone();
    editor.document.refresh_component_instances(master).unwrap();
    assert_eq!(editor.document.nodes[&child].vector_network, painted);
    assert_eq!(editor.document.nodes[&child].fill_style, Some(style));
    assert_eq!(editor.document.nodes[&child].fills, red());
}
