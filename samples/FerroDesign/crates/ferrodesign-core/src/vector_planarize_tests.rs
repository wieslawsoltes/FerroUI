use super::*;
use crate::{Color, Document, Fill};

fn p(x: f64, y: f64) -> Point { Point::new(x, y) }

fn square(network: &mut VectorNetwork) -> ([VectorVertexId; 4], [VectorEdgeId; 4]) {
    let vertices = [p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)]
        .map(|p| network.add_vertex(p).unwrap());
    let edges = std::array::from_fn(|i| network.connect(vertices[i], vertices[(i + 1) % 4]).unwrap());
    (vertices, edges)
}

fn crossing(cubics: usize) -> VectorNetwork {
    let mut network = VectorNetwork::default();
    let (vertices, _) = square(&mut network);
    for (index, (a, b, c1, c2)) in [(0, 2, p(20.0, 60.0), p(80.0, 40.0)),
        (1, 3, p(80.0, 60.0), p(20.0, 40.0))].into_iter().enumerate()
    {
        let curve = if index < cubics { NetworkCurve::Cubic { control1: c1, control2: c2 } } else { NetworkCurve::Line };
        network.connect_curve(vertices[a], vertices[b], curve).unwrap();
    }
    network
}

fn editor(network: VectorNetwork) -> (Editor, NodeId) {
    let mut document = Document::new("Checked intersection authoring");
    let mut node = Node::new(NodeKind::Vector, "Crossings", 40.0, 50.0, 100.0, 100.0);
    node.fills = vec![Fill::solid(Color::rgb(33, 99, 220))];
    node.vector_network = Some(network);
    let id = document.insert_node(document.pages[0].id, node).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    (editor, id)
}

#[test]
fn planarize_preparation_is_read_only_and_line_cubic_crossings_keep_first_edge_ids() {
    for cubics in 0..=2 {
        let mut network = crossing(cubics);
        let before = network.clone();
        assert!(matches!(network.detect_faces(), Err(NetworkFaceError::UnsplitIntersection(..))));
        let map = NetworkFaceMap::prepare(&network).unwrap();
        assert_eq!(network, before, "preparing hover cannot allocate authored IDs");
        assert_eq!(map.len(), 4);
        assert!(map.matches_source(&network));
        assert!(!map.matches_geometry(&network));
        assert!(network.planarize_intersections().unwrap());
        assert_eq!(network.vertices.len(), 5);
        assert_eq!(network.edges.len(), 8);
        assert_eq!(network.next_id, before.next_id + 3);
        assert_eq!(network.detect_faces().unwrap().len(), 4);
        for (&id, vertex) in &before.vertices { assert_eq!(&network.vertices[&id], vertex); }
        for (&id, edge) in &before.edges {
            assert_eq!(network.edges[&id].start, edge.start);
            assert_eq!(network.edges[&id].stroke, edge.stroke);
        }
        let prepared = network.clone();
        assert!(!network.planarize_intersections().unwrap());
        assert_eq!(network, prepared, "idempotent preparation cannot consume IDs");
        assert!(map.matches_geometry(&network));
    }
}

#[test]
fn planarize_cubic_subdivision_preserves_bezier_commands_and_exact_midpoint_handles() {
    let mut network = crossing(1);
    let original_edge = 9;
    let before = network.edge_curve(original_edge).unwrap();
    let (first, second) = split(before, 0.5);
    network.planarize_intersections().unwrap();
    let edge = &network.edges[&original_edge];
    assert_eq!(edge.curve, NetworkCurve::Cubic { control1: first.points[1], control2: first.points[2] });
    assert_eq!(network.vertices[&edge.end].position, p(50.0, 50.0));
    let continuation = network.edges.iter().find(|(&id, e)| id != original_edge && e.start == edge.end && e.end == 3).unwrap().0;
    assert_eq!(network.edges[continuation].curve, NetworkCurve::Cubic { control1: second.points[1], control2: second.points[2] });
    let a = network.edge_curve(original_edge).unwrap();
    let b = network.edge_curve(*continuation).unwrap();
    for i in 0..=100 {
        let t = i as f64 / 100.0;
        let expected = before.evaluate(t);
        let actual = if t <= 0.5 { a.evaluate(t * 2.0) } else { b.evaluate((t - 0.5) * 2.0) };
        assert!((actual - expected).x.hypot((actual - expected).y) < 1e-11);
    }
}

#[test]
fn planarize_preserves_explicit_paints_reversed_contours_and_unstroked_edges() {
    let mut network = crossing(1);
    let region = network.add_region(vec![vec![NetworkEdgeRef::reverse(9), NetworkEdgeRef::reverse(8), NetworkEdgeRef::reverse(7)]]).unwrap();
    network.regions.get_mut(&region).unwrap().fills = Some(vec![Fill::solid(Color::rgba(220, 30, 90, 90))]);
    network.edges.get_mut(&9).unwrap().stroke = false;
    let before = network.clone();
    network.planarize_intersections().unwrap();
    let after = &network.regions[&region];
    assert_eq!(after.fills, before.regions[&region].fills);
    assert_eq!(after.contours[0].len(), 4);
    assert_eq!(after.contours[0][1], NetworkEdgeRef::reverse(9));
    assert!(!network.edges[&after.contours[0][0].edge].stroke);
    assert!(!network.edges[&9].stroke);
    network.validate().unwrap();
    for point in [p(75.0, 10.0), p(25.0, 90.0), p(95.0, 70.0)] {
        assert_eq!(network.region_contains(region, point), before.region_contains(region, point));
    }
    let saved = serde_json::to_string(&network).unwrap();
    assert_eq!(serde_json::from_str::<VectorNetwork>(&saved).unwrap(), network);
}

#[test]
fn planarize_three_line_crossing_has_one_shared_vertex_and_six_faces() {
    let mut network = crossing(0);
    let top = network.add_vertex(p(50.0, -20.0)).unwrap();
    let bottom = network.add_vertex(p(50.0, 120.0)).unwrap();
    network.connect(top, bottom).unwrap();
    network.planarize_intersections().unwrap();
    let center: Vec<_> = network.vertices.iter().filter(|(_, v)| v.position == p(50.0, 50.0)).collect();
    assert_eq!(center.len(), 1);
    assert_eq!(network.edges.values().filter(|e| e.start == *center[0].0 || e.end == *center[0].0).count(), 6);
    assert_eq!(network.detect_faces().unwrap().len(), 6);
}

#[test]
fn planarize_nearby_distinct_crossings_cannot_collapse_a_face_into_one_junction() {
    let make = |offset| {
        let mut network = VectorNetwork::default();
        for (start, end) in [
            (p(0.0, 0.5), p(1.0, 0.5)),
            (p(0.5, 0.0), p(0.5, 1.0)),
            (p(0.0, offset), p(1.0, 1.0 + offset)),
        ] {
            let start = network.add_vertex(start).unwrap();
            let end = network.add_vertex(end).unwrap();
            network.connect(start, end).unwrap();
        }
        network
    };
    let mut exact = make(0.0);
    assert!(exact.planarize_intersections().unwrap());
    assert_eq!(exact.vertices.len(), 7);
    assert_eq!(exact.edges.len(), 6);
    assert!(exact.detect_faces().unwrap().is_empty(), "An exact common junction has no bounded face");
    for offset in [-1e-14_f64, 1e-14_f64] {
        // These three transverse roots are distinct representable values,
        // separated by less than the old parameter/position weld tolerances.
        // They enclose a triangle. If its subdivision cannot be certified,
        // rejecting it is valid; silently erasing it is not.
        assert_ne!(0.5 + offset, 0.5);
        assert!(offset.abs() < PARAMETER_ROUND_OFF);
        for scale in [1.0, 1e6] {
            let mut network = make(offset).transformed(Transform::scale(scale, scale));
            let before = network.clone();
            assert!(matches!(network.planarize_intersections(), Err(NetworkFaceError::UnsplitIntersection(..))));
            assert_eq!(network, before, "An ambiguous split must preserve graph, paints and allocator");
            assert!(network.fill_prepared_faces().is_err());
            assert_eq!(network, before);
            assert!(network.paint_prepared_face_at(p(0.5 * scale, 0.5 * scale), Some(Vec::new())).is_err());
            assert_eq!(network, before);
        }
    }
}

#[test]
fn planarize_exact_nondyadic_line_concurrence_survives_translation() {
    // The common point is (1/3, 1/3), not an exactly representable f64.
    // A rounded point-equality test cannot establish this concurrence. The
    // exact determinant on the authored endpoints can, even after translation.
    for translation in [p(0.0, 0.0), p(128.0, -256.0)] {
        let mut network = VectorNetwork::default();
        for (start, end) in [(p(0.0, 0.0), p(1.0, 1.0)),
            (p(1.0, 0.0), p(-1.0, 1.0)), (p(0.0, 1.0), p(1.0, -1.0))]
        {
            let start = network.add_vertex(start + translation).unwrap();
            let end = network.add_vertex(end + translation).unwrap();
            network.connect(start, end).unwrap();
        }
        assert!(network.planarize_intersections().unwrap());
        assert_eq!(network.vertices.len(), 7);
        assert_eq!(network.edges.len(), 6);
        assert!(network.detect_faces().unwrap().is_empty());
        let junctions: Vec<_> = network.vertices.keys().filter(|&&id|
            network.edges.values().filter(|edge| edge.start == id || edge.end == id).count() == 6).collect();
        assert_eq!(junctions.len(), 1);
        let point = network.vertices[junctions[0]].position - translation;
        assert!(point.distance(p(1.0 / 3.0, 1.0 / 3.0)) < 1e-9);
        let prepared = network.clone();
        assert!(!network.planarize_intersections().unwrap());
        assert_eq!(network, prepared);
    }
}

#[test]
fn planarize_concurrence_predicate_is_exact_even_when_rounded_coordinates_cancel() {
    let source = [Curve::line(p(0.0, 0.0), p(1.0, 1.0)),
        Curve::line(p(1.0, 0.0), p(-1.0, 1.0)), Curve::line(p(0.0, 1.0), p(1.0, -1.0))];
    for translation in [p(0.0, 0.0), p(1_048_576.0, -2_097_152.0)] {
        let mut curves = source;
        for curve in &mut curves { curve.points = curve.points.map(|point| point + translation); }
        assert!(exact_line_concurrence(curves));
        curves[2].points[1].y += 1e-8;
        assert!(!exact_line_concurrence(curves));
    }
    for offset in [-1e-14, 1e-14] {
        let curves = [Curve::line(p(0.0, 0.5), p(1.0, 0.5)), Curve::line(p(0.5, 0.0), p(0.5, 1.0)),
            Curve::line(p(0.0, offset), p(1.0, 1.0 + offset))];
        assert!(!exact_line_concurrence(curves));
    }
}

#[test]
fn planarize_welds_exact_endpoint_and_t_junction_contacts_without_renumbering() {
    let mut network = VectorNetwork::default();
    let segments = [(p(0.0, 0.0), p(100.0, 0.0)), (p(100.0, 0.0), p(100.0, 100.0)),
        (p(100.0, 100.0), p(0.0, 100.0)), (p(0.0, 100.0), p(0.0, 0.0)),
        (p(50.0, 0.0), p(50.0, 100.0))];
    for (a, b) in segments {
        let a = network.add_vertex(a).unwrap(); let b = network.add_vertex(b).unwrap();
        network.connect(a, b).unwrap();
    }
    let ids: Vec<_> = network.edges.keys().copied().collect();
    network.planarize_intersections().unwrap();
    assert!(ids.iter().all(|id| network.edges.contains_key(id)));
    assert_eq!(network.vertices.len(), 6);
    assert_eq!(network.edges.len(), 7);
    assert_eq!(network.detect_faces().unwrap().len(), 2);
    assert!(network.vertices.contains_key(&1), "lowest coincident original vertex ID survives");
    assert!(!network.vertices.contains_key(&11));
    let mut continuation = VectorNetwork::default();
    for (a, b) in [(p(0.0, 0.0), p(50.0, 0.0)), (p(50.0, 0.0), p(100.0, 0.0))] {
        let a = continuation.add_vertex(a).unwrap(); let b = continuation.add_vertex(b).unwrap();
        continuation.connect(a, b).unwrap();
    }
    let next = continuation.next_id;
    continuation.planarize_intersections().unwrap();
    assert_eq!(continuation.vertices.len(), 3);
    assert_eq!(continuation.edges.len(), 2);
    assert_eq!(continuation.next_id, next, "welding exact endpoints allocates no IDs");
}

#[test]
fn planarize_isolates_three_line_cubic_roots_and_cubic_self_intersection() {
    let mut network = VectorNetwork::default();
    let a = network.add_vertex(p(0.0, -80.0)).unwrap();
    let b = network.add_vertex(p(300.0, 80.0)).unwrap();
    network.connect_curve(a, b, NetworkCurve::Cubic { control1: p(100.0, 200.0), control2: p(200.0, -200.0) }).unwrap();
    let c = network.add_vertex(p(-10.0, 0.0)).unwrap();
    let d = network.add_vertex(p(310.0, 0.0)).unwrap();
    network.connect(c, d).unwrap();
    network.planarize_intersections().unwrap();
    assert_eq!(network.vertices.len(), 7);
    assert_eq!(network.edges.len(), 8);
    assert_eq!(network.detect_faces().unwrap().len(), 2);
    let mut looped = VectorNetwork::default();
    let a = looped.add_vertex(p(0.0, 0.0)).unwrap();
    let b = looped.add_vertex(p(10.0, 0.0)).unwrap();
    looped.connect_curve(a, b, NetworkCurve::Cubic { control1: p(100.0, 100.0), control2: p(-100.0, 100.0) }).unwrap();
    looped.planarize_intersections().unwrap();
    assert_eq!(looped.vertices.len(), 3);
    assert_eq!(looped.edges.len(), 3);
    assert_eq!(looped.detect_faces().unwrap().len(), 1);
}

#[test]
fn planarize_ambiguous_overlaps_tangencies_and_id_exhaustion_are_atomic() {
    for tangent in [false, true] {
        let mut network = VectorNetwork::default();
        let a = network.add_vertex(p(0.0, 0.0)).unwrap(); let b = network.add_vertex(p(100.0, 0.0)).unwrap();
        network.connect(a, b).unwrap();
        let c = network.add_vertex(if tangent { p(0.0, 75.0) } else { p(20.0, 0.0) }).unwrap();
        let d = network.add_vertex(if tangent { p(100.0, 75.0) } else { p(80.0, 0.0) }).unwrap();
        network.connect_curve(c, d, if tangent {
            NetworkCurve::Cubic { control1: p(25.0, -25.0), control2: p(75.0, -25.0) }
        } else { NetworkCurve::Line }).unwrap();
        let before = network.clone();
        assert!(network.planarize_intersections().is_err());
        assert_eq!(network, before);
        assert!(network.fill_prepared_faces().is_err());
        assert_eq!(network, before);
    }
    let mut exhausted = crossing(2);
    exhausted.next_id = u64::MAX - 1;
    let before = exhausted.clone();
    assert!(matches!(exhausted.planarize_intersections(), Err(NetworkFaceError::IdExhausted)));
    assert_eq!(exhausted, before);
    // Enough IDs for the shared vertex and split edges, but none for paint.
    // Failure after preparation still cannot publish a half-authored graph.
    exhausted.next_id = u64::MAX - 3;
    let before = exhausted.clone();
    assert_eq!(exhausted.paint_prepared_face_at(p(50.0, 10.0), Some(Vec::new())), Err(NetworkFaceError::IdExhausted));
    assert_eq!(exhausted, before);
    assert_eq!(exhausted.fill_prepared_faces(), Err(NetworkFaceError::IdExhausted));
    assert_eq!(exhausted, before);
}

#[test]
fn planarize_prepared_map_handles_paint_changes_but_rejects_allocator_or_geometry_changes() {
    let mut network = crossing(0);
    let region = network.add_region(vec![vec![NetworkEdgeRef::forward(5), NetworkEdgeRef::forward(6), NetworkEdgeRef::forward(7), NetworkEdgeRef::forward(8)]]).unwrap();
    let map = NetworkFaceMap::prepare(&network).unwrap();
    let paint = vec![Fill::solid(Color::rgba(230, 50, 20, 128))];
    network.regions.get_mut(&region).unwrap().fills = Some(paint.clone());
    assert!(map.matches_source(&network));
    network.paint_mapped_face_at(&map, p(50.0, 10.0), Vec::new()).unwrap();
    assert_eq!(network.regions[&region].fills, Some(paint));
    let mut stale = crossing(0);
    let map = NetworkFaceMap::prepare(&stale).unwrap();
    stale.add_region(vec![vec![NetworkEdgeRef::forward(5), NetworkEdgeRef::forward(6), NetworkEdgeRef::forward(7), NetworkEdgeRef::forward(8)]]).unwrap();
    let before = stale.clone();
    assert!(!map.matches_source(&stale));
    assert_eq!(stale.paint_mapped_face_at(&map, p(50.0, 10.0), Vec::new()), Err(NetworkFaceError::StaleFaceMap));
    assert_eq!(stale, before);
}

#[test]
fn planarize_native_fill_and_bucket_preserve_affine_bounds_and_one_history_step() {
    for fill_all in [false, true] {
        let (mut editor, id) = editor(crossing(2));
        editor.document.nodes.get_mut(&id).unwrap().set_local_transform(Transform { a: -1.1, b: 0.3, c: 0.4, d: 0.8, e: 150.0, f: 20.0 });
        editor.refresh();
        let transform = editor.document.world_transform(id);
        let before = editor.document.to_json().unwrap();
        let fills = editor.document.nodes[&id].fills.clone();
        assert!(if fill_all { editor.fill_network_faces(id) } else { editor.paint_network_face(id, p(50.0, 10.0), Some(fills)) }.unwrap());
        let after = editor.document.to_json().unwrap();
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.selection, vec![id]);
        assert_eq!(editor.document.world_transform(id), transform);
        let network = editor.document.nodes[&id].vector_network.as_ref().unwrap();
        assert_eq!(network.edges.len(), 8);
        assert_eq!(network.regions.len(), if fill_all { 4 } else { 1 });
        assert!(editor.undo()); assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(editor.redo()); assert_eq!(editor.document.to_json().unwrap(), after);
    }
}

#[test]
fn planarize_outside_bucket_and_open_fill_do_not_change_document_or_redo() {
    let (mut editor, id) = editor(crossing(0));
    editor.fill_network_faces(id).unwrap();
    editor.undo();
    let before = editor.document.to_json().unwrap();
    let history = editor.history_len();
    assert!(!editor.paint_network_face(id, p(-10.0, -10.0), Some(Vec::new())).unwrap());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.history_len(), history);
    assert!(editor.redo());
    let mut open = crossing(0);
    for id in [5, 6, 7, 8] { open.remove_edge(id); }
    let before = open.clone();
    assert!(!open.fill_prepared_faces().unwrap());
    assert_eq!(open, before);
    assert!(open.planarize_intersections().unwrap(), "explicit topology preparation is independently available");
}

#[test]
fn planarize_transform_covariance_keeps_crossing_topology_for_small_large_and_reflected_geometry() {
    for transform in [Transform::scale(1e-6, 1e-6), Transform::scale(1e6, 1e6),
        Transform { a: -1.0, b: 0.2, c: 0.3, d: 1.0, e: 1000.0, f: -2000.0 }]
    {
        let mut network = crossing(2).transformed(transform);
        network.planarize_intersections().unwrap();
        assert_eq!(network.detect_faces().unwrap().len(), 4);
        assert_eq!(network.vertices.len(), 5);
        assert_eq!(network.edges.len(), 8);
    }
}

#[test]
fn planarize_fill_component_override_keeps_subdivision_and_default_paint_after_reconciliation() {
    let mut document = Document::new("Prepared component faces");
    let page = document.pages[0].id;
    let master = document.insert_node(page, Node::new(NodeKind::Component, "Master", 0.0, 0.0, 200.0, 200.0)).unwrap();
    let mut source = Node::new(NodeKind::Vector, "Crossings", 10.0, 10.0, 100.0, 100.0);
    source.fills.clear();
    source.vector_network = Some(crossing(2));
    document.insert_node(master, source).unwrap();
    let mut editor = Editor::new(document);
    let instance = editor.create_instance(master, 300.0, 0.0).unwrap();
    let child = editor.document.nodes[&instance].children[0];
    editor.clear_history();
    let before = editor.document.to_json().unwrap();
    assert!(editor.fill_network_faces(child).unwrap());
    let node = &editor.document.nodes[&child];
    assert!(node.overridden_properties.contains("path"));
    assert!(node.overridden_properties.contains("fills"));
    let prepared = node.vector_network.clone();
    let fills = node.fills.clone();
    assert_eq!(fills, vec![Fill::solid(Color::rgb(217, 217, 217))]);
    assert_eq!(editor.history_len(), 1);
    editor.document.refresh_component_instances(master).unwrap();
    assert_eq!(editor.document.nodes[&child].vector_network, prepared);
    assert_eq!(editor.document.nodes[&child].fills, fills);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
}
