use super::*;
use crate::{Document, Fill};

fn p(x: f64, y: f64) -> Point { Point::new(x, y) }

fn ring(network: &mut VectorNetwork, points: &[Point]) -> (Vec<VectorVertexId>, Vec<VectorEdgeId>) {
    let vertices: Vec<_> = points.iter().map(|&point| network.add_vertex(point).unwrap()).collect();
    let edges = vertices.iter().zip(vertices.iter().cycle().skip(1))
        .map(|(&a, &b)| network.connect(a, b).unwrap()).collect();
    (vertices, edges)
}

fn square(network: &mut VectorNetwork, x: f64, y: f64, size: f64) -> (Vec<VectorVertexId>, Vec<VectorEdgeId>) {
    ring(network, &[p(x, y), p(x + size, y), p(x + size, y + size), p(x, y + size)])
}

fn with_region(network: &VectorNetwork, region: NetworkRegion) -> VectorNetwork {
    let mut result = network.clone();
    result.regions.clear();
    result.add_region(region.contours).unwrap();
    result
}

#[test]
fn cached_network_faces_are_read_only_keep_holes_and_reject_changed_geometry() {
    let mut network = VectorNetwork::default();
    square(&mut network, 0.0, 0.0, 200.0);
    square(&mut network, 25.0, 25.0, 150.0);
    let before = network.clone();
    let map = NetworkFaceMap::new(&network).unwrap();
    assert_eq!(network, before);
    assert_eq!(map.len(), 2);
    let outer = map.face_at(p(10.0, 10.0)).unwrap();
    let inner = map.face_at(p(100.0, 100.0)).unwrap();
    assert_ne!(outer, inner);
    assert!(!map.path(outer).unwrap().contains(p(100.0, 100.0)));
    assert_eq!(map.face_at(p(220.0, 100.0)), None);
    assert!(network.paint_mapped_face_at(&map, p(10.0, 10.0), vec![Fill::solid(crate::Color::WHITE)]).unwrap());
    assert!(map.matches_geometry(&network));
    let region = network.explicit_region_at(p(10.0, 10.0)).unwrap();
    let next_id = network.next_id;
    assert!(!network.paint_mapped_face_at(&map, p(10.0, 10.0), vec![Fill::solid(crate::Color::WHITE)]).unwrap());
    assert!(network.paint_mapped_face_at(&map, p(10.0, 10.0), vec![]).unwrap());
    assert_eq!(network.explicit_region_at(p(10.0, 10.0)), Some(region));
    assert_eq!(network.next_id, next_id);
    network.vertices.values_mut().next().unwrap().position.x -= 1.0;
    let modified = network.clone();
    assert_eq!(network.paint_mapped_face_at(&map, p(10.0, 10.0), vec![]), Err(NetworkFaceError::StaleFaceMap));
    assert_eq!(network, modified);
}

#[test]
fn cached_network_segments_visit_narrow_intermediate_faces_in_order() {
    let mut network = VectorNetwork::default();
    // Three adjacent cells, with a middle cell far narrower than a screen pixel.
    let xs = [0.0, 40.0, 40.000_01, 100.0];
    let top: Vec<_> = xs.iter().map(|&x| network.add_vertex(p(x, 0.0)).unwrap()).collect();
    let bottom: Vec<_> = xs.iter().map(|&x| network.add_vertex(p(x, 100.0)).unwrap()).collect();
    for index in 0..xs.len() {
        network.connect(top[index], bottom[index]).unwrap();
        if index > 0 {
            network.connect(top[index - 1], top[index]).unwrap();
            network.connect(bottom[index - 1], bottom[index]).unwrap();
        }
    }
    let map = NetworkFaceMap::new(&network).unwrap();
    let hits = map.faces_along_segment(p(-10.0, 50.0), p(110.0, 50.0)).unwrap();
    assert_eq!(hits.len(), 3);
    assert_eq!(hits.iter().map(|&(face, _)| face).collect::<Vec<_>>(),
        [p(20.0, 50.0), p(40.000_005, 50.0), p(70.0, 50.0)].map(|point| map.face_at(point).unwrap()));
    assert!(hits.windows(2).all(|pair| pair[0].1.x < pair[1].1.x));
    let reverse = map.faces_along_segment(p(110.0, 50.0), p(-10.0, 50.0)).unwrap();
    assert_eq!(reverse.iter().map(|&(face, _)| face).collect::<Vec<_>>(), hits.iter().rev().map(|&(face, _)| face).collect::<Vec<_>>());
    assert_eq!(map.faces_along_segment(p(-10.0, -5.0), p(110.0, -5.0)).unwrap(), []);
}

#[test]
fn cached_network_faces_keep_curved_micro_annuli_and_segment_crossings() {
    for scale in [0.000_001, 1.0, 1000.0] {
        let mut network = VectorNetwork::default();
        for factor in [1.0, 0.999] {
            let transform = |point: Point| (p(50.0, 0.0) + (point - p(50.0, 0.0)) * factor) * scale;
            let a = network.add_vertex(transform(p(0.0, 0.0))).unwrap();
            let b = network.add_vertex(transform(p(100.0, 0.0))).unwrap();
            network.connect_curve(a, b, NetworkCurve::Cubic { control1: transform(p(15.0, -100.0)), control2: transform(p(85.0, -100.0)) }).unwrap();
            network.connect_curve(b, a, NetworkCurve::Cubic { control1: transform(p(85.0, 100.0)), control2: transform(p(15.0, 100.0)) }).unwrap();
        }
        let map = NetworkFaceMap::new(&network).unwrap();
        let annulus = map.face_at(p(50.0, -74.97) * scale).unwrap();
        let inner = map.face_at(p(50.0, 0.0) * scale).unwrap();
        assert_ne!(annulus, inner);
        let hits = map.faces_along_segment(p(50.0, -100.0) * scale, p(50.0, 100.0) * scale).unwrap();
        assert_eq!(hits.iter().map(|&(face, _)| face).collect::<Vec<_>>(), [annulus, inner]);
        assert!(map.face_at(p(50.0, -75.01) * scale).is_none());
    }
}

#[test]
fn cached_network_batch_updates_once_and_rolls_back_allocator_exhaustion() {
    let mut network = VectorNetwork::default();
    let (vertices, _) = square(&mut network, 0.0, 0.0, 100.0);
    network.connect(vertices[0], vertices[2]).unwrap();
    let map = NetworkFaceMap::new(&network).unwrap();
    let mut document = Document::new("Mapped batch");
    let mut node = Node::new(NodeKind::Vector, "Faces", 0.0, 0.0, 100.0, 100.0);
    node.vector_network = Some(network);
    let id = document.insert_node(1, node).unwrap();
    let mut editor = Editor::new(document);
    let before = editor.document.to_json().unwrap();
    let revision = editor.revision;
    let fills = vec![Fill::solid(crate::Color::WHITE)];
    assert!(editor.paint_mapped_network_faces(id, &map, &[p(75.0, 25.0), p(25.0, 75.0)], &fills).unwrap());
    assert_eq!(editor.revision, revision + 1, "one node refresh per pointer segment");
    assert_eq!(editor.history_len(), 1);
    assert_eq!(editor.document.nodes[&id].vector_network.as_ref().unwrap().regions.len(), 2);
    editor.undo();
    assert_eq!(editor.document.to_json().unwrap(), before);
    editor.document.nodes.get_mut(&id).unwrap().vector_network.as_mut().unwrap().next_id = u64::MAX - 1;
    editor.refresh();
    let before = editor.document.to_json().unwrap();
    let revision = editor.revision;
    assert_eq!(editor.paint_mapped_network_faces(id, &map, &[p(75.0, 25.0), p(25.0, 75.0)], &fills), Err(NetworkFaceError::IdExhausted));
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.revision, revision);
    assert!(editor.can_redo());
}

#[test]
fn network_faces_extract_adjacent_cells_with_exact_opposite_shared_boundaries() {
    let mut network = VectorNetwork::default();
    let (vertices, boundary) = square(&mut network, 0.0, 0.0, 100.0);
    let diagonal = network.connect(vertices[0], vertices[2]).unwrap();
    let before = network.clone();
    let faces = network.detect_faces().unwrap();
    assert_eq!(faces.len(), 2);
    assert_eq!(network, before, "Discovery must never mutate allocator or regions");
    let uses: Vec<_> = faces.iter().flat_map(|face| &face.contours).flatten()
        .filter(|reference| reference.edge == diagonal).map(|reference| reference.reversed).collect();
    assert_eq!(uses.len(), 2);
    assert_ne!(uses[0], uses[1]);
    for edge in boundary {
        assert_eq!(faces.iter().flat_map(|face| &face.contours).flatten().filter(|reference| reference.edge == edge).count(), 1);
    }
    assert!(network.fill_all_faces().unwrap());
    for point in [p(25.0, 70.0), p(70.0, 25.0), p(50.0, 50.0)] { assert!(network.contains_fill(point)); }
    assert!(!network.contains_fill(p(-1.0, 20.0)));
    assert_eq!(network.vertices, before.vertices);
    assert_eq!(network.edges, before.edges);
    let filled = network.clone();
    assert!(!network.fill_all_faces().unwrap());
    assert_eq!(network, filled);
}

#[test]
fn network_faces_keep_nested_holes_and_inner_faces_for_both_fill_rules() {
    for fill_rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let mut network = VectorNetwork { fill_rule, ..VectorNetwork::default() };
        square(&mut network, 0.0, 0.0, 200.0);
        // Opposite source orientations have no effect on extracted faces.
        ring(&mut network, &[p(30.0, 30.0), p(30.0, 170.0), p(170.0, 170.0), p(170.0, 30.0)]);
        square(&mut network, 70.0, 70.0, 60.0);
        square(&mut network, 250.0, 0.0, 100.0);
        let faces = network.detect_faces().unwrap();
        assert_eq!(faces.len(), 4);
        assert_eq!(faces.iter().filter(|face| face.contours.len() == 2).count(), 2);
        let outer = with_region(&network, network.face_at(p(15.0, 15.0)).unwrap().unwrap());
        assert!(outer.contains_fill(p(15.0, 15.0)));
        assert!(!outer.contains_fill(p(50.0, 50.0)));
        assert!(!outer.contains_fill(p(100.0, 100.0)));
        let middle = with_region(&network, network.face_at(p(50.0, 50.0)).unwrap().unwrap());
        assert!(middle.contains_fill(p(50.0, 50.0)));
        assert!(!middle.contains_fill(p(100.0, 100.0)));
        assert!(!middle.contains_fill(p(15.0, 15.0)));
        let inner = with_region(&network, network.face_at(p(100.0, 100.0)).unwrap().unwrap());
        assert!(inner.contains_fill(p(100.0, 100.0)));
        assert!(network.face_at(p(225.0, 20.0)).unwrap().is_none());
        assert!(network.face_at(p(f64::NAN, 20.0)).unwrap().is_none());
        assert!(network.fill_all_faces().unwrap());
        for point in [p(15.0, 15.0), p(50.0, 50.0), p(100.0, 100.0), p(280.0, 30.0)] {
            assert!(network.contains_fill(point), "{fill_rule:?}: {point:?}");
        }
    }
}

#[test]
fn network_faces_curved_micro_annuli_use_polynomial_containment_without_flattening() {
    for scale in [1.0, 0.000_001, 1000.0] {
        let mut network = VectorNetwork::default();
        let mut top_edges = Vec::new();
        for factor in [1.0, 0.999] {
            let transform = |point: Point| (p(50.0, 0.0) + (point - p(50.0, 0.0)) * factor) * scale;
            let a = network.add_vertex(transform(p(0.0, 0.0))).unwrap();
            let b = network.add_vertex(transform(p(100.0, 0.0))).unwrap();
            top_edges.push(network.connect_curve(a, b, NetworkCurve::Cubic {
                control1: transform(p(15.0, -100.0)), control2: transform(p(85.0, -100.0)),
            }).unwrap());
            network.connect_curve(b, a, NetworkCurve::Cubic {
                control1: transform(p(85.0, 100.0)), control2: transform(p(15.0, 100.0)),
            }).unwrap();
        }
        let faces = network.detect_faces().unwrap();
        assert_eq!(faces.len(), 2);
        assert_eq!(faces.iter().filter(|face| face.contours.len() == 2).count(), 1);
        for t in [0.13, 0.37, 0.5, 0.81] {
            let outside = network.edge_curve(top_edges[0]).unwrap().evaluate(t);
            let inside = network.edge_curve(top_edges[1]).unwrap().evaluate(t);
            let between = outside + (inside - outside) * 0.5;
            let region = network.face_at(between).unwrap().expect("The narrow annulus is a real face at every scale");
            assert_eq!(region.contours.len(), 2, "scale={scale}, t={t}");
        }
        let inner = network.face_at(p(50.0, 0.0) * scale).unwrap().unwrap();
        assert_eq!(inner.contours.len(), 1);
        assert!(network.face_at(p(50.0, -75.01) * scale).unwrap().is_none());
        // Rays through extrema and shared endpoints use half-open y intervals.
        for point in [p(-1.0, 0.0), p(101.0, 0.0), p(-1.0, 75.0), p(101.0, -75.0)] {
            assert!(network.face_at(point * scale).unwrap().is_none());
        }
    }
}

#[test]
fn network_faces_prune_bridges_and_dangling_trees_without_deleting_strokes() {
    let mut network = VectorNetwork::default();
    let (outer, _) = square(&mut network, 0.0, 0.0, 200.0);
    let (inner, _) = square(&mut network, 40.0, 40.0, 100.0);
    let bridge = network.connect(outer[0], inner[0]).unwrap();
    let a = network.add_vertex(p(-20.0, -20.0)).unwrap();
    let b = network.add_vertex(p(-50.0, -20.0)).unwrap();
    let c = network.add_vertex(p(-20.0, -50.0)).unwrap();
    let dangling = [network.connect(outer[0], a).unwrap(), network.connect(a, b).unwrap(), network.connect(a, c).unwrap()];
    let before = network.clone();
    let faces = network.detect_faces().unwrap();
    assert_eq!(faces.len(), 2);
    assert_eq!(faces.iter().map(|face| face.contours.len()).sum::<usize>(), 3);
    assert!(!faces.iter().flat_map(|face| &face.contours).flatten()
        .any(|reference| reference.edge == bridge || dangling.contains(&reference.edge)));
    assert!(network.fill_all_faces().unwrap());
    assert_eq!(network.edges, before.edges);
    assert_eq!(network.vertices, before.vertices);
    assert_eq!(network.stroke_path(), before.stroke_path());
    network.validate().unwrap();
}

#[test]
fn network_faces_articulation_boundaries_and_disconnected_regions_are_deterministic() {
    let mut network = VectorNetwork::default();
    let (left, _) = square(&mut network, 0.0, 0.0, 100.0);
    let a = network.add_vertex(p(150.0, 150.0)).unwrap();
    let b = network.add_vertex(p(150.0, 50.0)).unwrap();
    network.connect(left[2], a).unwrap();
    network.connect(a, b).unwrap();
    network.connect(b, left[2]).unwrap();
    let faces = network.detect_faces().unwrap();
    assert_eq!(faces.len(), 2);
    assert_eq!(network.detect_faces().unwrap(), faces);
    let json = serde_json::to_string(&network).unwrap();
    let imported: VectorNetwork = serde_json::from_str(&json).unwrap();
    assert_eq!(imported.detect_faces().unwrap(), faces);
    assert_eq!(with_region(&network, network.face_at(p(25.0, 25.0)).unwrap().unwrap()).regions.len(), 1);
}

#[test]
fn network_faces_preserve_cubic_lenses_hidden_closures_and_curved_shared_edges() {
    let mut network = VectorNetwork::default();
    let a = network.add_vertex(p(0.0, 0.0)).unwrap();
    let b = network.add_vertex(p(100.0, 0.0)).unwrap();
    let upper = network.connect_curve(a, b, NetworkCurve::Cubic { control1: p(15.0, -90.0), control2: p(85.0, -90.0) }).unwrap();
    let lower = network.connect_curve(b, a, NetworkCurve::Cubic { control1: p(85.0, 60.0), control2: p(15.0, 60.0) }).unwrap();
    let divider = network.connect(a, b).unwrap();
    network.edges.get_mut(&divider).unwrap().stroke = false;
    let faces = network.detect_faces().unwrap();
    assert_eq!(faces.len(), 2);
    assert!(network.fill_all_faces().unwrap());
    assert!(network.contains_fill(p(50.0, -40.0)));
    assert!(network.contains_fill(p(50.0, 20.0)));
    assert!(!network.contains_fill(p(50.0, 60.0)));
    let path = network.fill_path();
    assert_eq!(path.segments.iter().filter(|segment| matches!(segment, PathSegment::CubicTo { .. })).count(), 2);
    assert!(network.edges.contains_key(&upper) && network.edges.contains_key(&lower));
    assert!(!network.edges[&divider].stroke);
    assert_eq!(network.stroke_path().segments.iter().filter(|segment| matches!(segment, PathSegment::LineTo { .. })).count(), 0);
}

#[test]
fn network_faces_exact_curved_area_distinguishes_self_loop_orientation() {
    let mut network = VectorNetwork::default();
    let vertex = network.add_vertex(p(40.0, 50.0)).unwrap();
    let edge = network.connect_curve(vertex, vertex, NetworkCurve::Cubic {
        control1: p(140.0, 150.0), control2: p(-60.0, 150.0),
    }).unwrap();
    let faces = network.detect_faces().unwrap();
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].contours[0].len(), 1);
    assert_eq!(faces[0].contours[0][0].edge, edge);
    assert!(network.fill_all_faces().unwrap());
    assert!(network.contains_fill(p(40.0, 100.0)));
    assert!(!network.contains_fill(p(100.0, 100.0)));
    let reflected = network.transformed(Transform::scale(-1.0, 1.0));
    let faces = reflected.detect_faces().unwrap();
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].contours[0][0].reversed, !network.regions.values().next().unwrap().contours[0][0].reversed);
}

#[test]
fn network_faces_reject_unsplit_crossings_overlaps_and_unwelded_contacts_atomically() {
    let make = |segments: &[(Point, Point)]| {
        let mut network = VectorNetwork::default();
        for &(a, b) in segments {
            let a = network.add_vertex(a).unwrap();
            let b = network.add_vertex(b).unwrap();
            network.connect(a, b).unwrap();
        }
        network
    };
    for mut network in [
        make(&[(p(0.0, 0.0), p(100.0, 100.0)), (p(0.0, 100.0), p(100.0, 0.0))]),
        make(&[(p(0.0, 0.0), p(100.0, 0.0)), (p(20.0, 0.0), p(80.0, 0.0))]),
        make(&[(p(0.0, 0.0), p(100.0, 0.0)), (p(100.0, 0.0), p(100.0, 100.0))]),
        make(&[(p(0.0, 0.0), p(100.0, 0.0)), (p(50.0, 0.0), p(50.0, 100.0))]),
    ] {
        let before = network.clone();
        assert!(matches!(network.fill_all_faces(), Err(NetworkFaceError::UnsplitIntersection(..))));
        assert_eq!(network, before);
    }
}

#[test]
fn network_faces_crossing_cubics_and_duplicate_curve_contacts_are_declined() {
    let mut network = VectorNetwork::default();
    let (vertices, _) = square(&mut network, 0.0, 0.0, 100.0);
    network.connect_curve(vertices[0], vertices[2], NetworkCurve::Cubic {
        control1: p(20.0, 60.0), control2: p(80.0, 40.0),
    }).unwrap();
    network.connect_curve(vertices[1], vertices[3], NetworkCurve::Cubic {
        control1: p(80.0, 60.0), control2: p(20.0, 40.0),
    }).unwrap();
    let before = network.clone();
    assert!(matches!(network.fill_all_faces(), Err(NetworkFaceError::UnsplitIntersection(..))));
    assert_eq!(network, before);
    let mut singular = VectorNetwork::default();
    let a = singular.add_vertex(p(0.0, 0.0)).unwrap();
    let b = singular.add_vertex(p(100.0, 0.0)).unwrap();
    singular.connect_curve(a, b, NetworkCurve::Cubic { control1: p(30.0, 0.0), control2: p(70.0, 0.0) }).unwrap();
    singular.connect(a, b).unwrap();
    assert!(singular.detect_faces().is_err());
    let mut self_crossing = VectorNetwork::default();
    let a = self_crossing.add_vertex(p(0.0, 0.0)).unwrap();
    let b = self_crossing.add_vertex(p(10.0, 0.0)).unwrap();
    let edge = self_crossing.connect_curve(a, b, NetworkCurve::Cubic {
        control1: p(100.0, 100.0), control2: p(-100.0, 100.0),
    }).unwrap();
    assert_eq!(self_crossing.detect_faces(), Err(NetworkFaceError::UnsplitIntersection(edge, edge)));
}

#[test]
fn network_faces_degenerate_invalid_and_exhausted_networks_never_allocate_partially() {
    let mut network = VectorNetwork::default();
    let a = network.add_vertex(p(0.0, 0.0)).unwrap();
    let edge = network.connect(a, a).unwrap();
    let before = network.clone();
    assert_eq!(network.fill_all_faces(), Err(NetworkFaceError::DegenerateEdge(edge)));
    assert_eq!(network, before);
    let mut network = VectorNetwork::default();
    square(&mut network, 0.0, 0.0, 100.0);
    network.next_id = u64::MAX;
    let before = network.clone();
    assert_eq!(network.fill_all_faces(), Err(NetworkFaceError::IdExhausted));
    assert_eq!(network, before);
    network.edges.values_mut().next().unwrap().end = 9999;
    assert!(matches!(network.detect_faces(), Err(NetworkFaceError::InvalidNetwork(_))));
}

#[test]
fn network_faces_keep_existing_ids_reversed_contours_and_new_allocator_identity() {
    let mut network = VectorNetwork::default();
    let (vertices, edges) = square(&mut network, 0.0, 0.0, 100.0);
    let reversed: Vec<_> = edges.iter().rev().copied().map(NetworkEdgeRef::reverse).collect();
    let retained = network.add_region(vec![reversed]).unwrap();
    let before = network.clone();
    assert!(!network.fill_all_faces().unwrap());
    assert_eq!(network, before);
    square(&mut network, 200.0, 0.0, 100.0);
    let next = network.next_id;
    assert!(network.fill_all_faces().unwrap());
    assert_eq!(network.regions[&retained], before.regions[&retained]);
    assert!(network.regions.contains_key(&next));
    assert_eq!(network.next_id, next + 1);
    network.connect(vertices[0], vertices[2]).unwrap();
    assert!(network.fill_all_faces().unwrap());
    assert!(!network.regions.contains_key(&retained), "A split face cannot retain a whole-boundary region ID");
    assert!(network.regions.contains_key(&next));
    network.validate().unwrap();
}

#[test]
fn network_faces_affine_reflection_shear_and_large_translation_preserve_exact_commands() {
    let mut network = VectorNetwork::default();
    let (vertices, _) = square(&mut network, 0.0, 0.0, 100.0);
    network.connect_curve(vertices[0], vertices[2], NetworkCurve::Cubic {
        control1: p(15.0, 60.0), control2: p(80.0, 45.0),
    }).unwrap();
    let transform = Transform { a: -1.4, b: 0.3, c: 0.6, d: 1.8, e: 1e9, f: -2e9 };
    let mut moved = network.transformed(transform);
    let before = moved.clone();
    assert!(moved.fill_all_faces().unwrap());
    assert_eq!(moved.regions.len(), 2);
    assert_eq!(moved.vertices, before.vertices);
    assert_eq!(moved.edges, before.edges);
    for point in [p(25.0, 70.0), p(70.0, 25.0)] { assert!(moved.contains_fill(transform.apply(point))); }
    assert!(!moved.contains_fill(transform.apply(p(-20.0, 50.0))));
}

#[test]
fn network_faces_shared_junction_grid_is_topological_and_idempotent() {
    let count = 16;
    let mut network = VectorNetwork::default();
    let vertices: Vec<_> = (0..=count).map(|y| (0..=count).map(|x| network.add_vertex(p(x as f64 * 10.0, y as f64 * 10.0)).unwrap()).collect::<Vec<_>>()).collect();
    for y in 0..=count { for x in 0..=count {
        if x < count { network.connect(vertices[y][x], vertices[y][x + 1]).unwrap(); }
        if y < count { network.connect(vertices[y][x], vertices[y + 1][x]).unwrap(); }
    }}
    assert!(network.fill_all_faces().unwrap());
    assert_eq!(network.regions.len(), count * count);
    assert!(network.regions.values().all(|region| region.contours.len() == 1 && region.contours[0].len() == 4));
    let before = network.clone();
    assert!(!network.fill_all_faces().unwrap());
    assert_eq!(network, before);
}

#[test]
fn network_faces_editor_history_locks_noops_and_errors_preserve_redo() {
    let mut document = Document::new("Automatic vector faces");
    let mut node = Node::new(NodeKind::Vector, "Planar network", 25.0, 30.0, 200.0, 200.0);
    node.fills = vec![Fill::solid(crate::Color::from_hex("FF0000").unwrap())];
    let mut network = VectorNetwork::default();
    let (vertices, _) = square(&mut network, 0.0, 0.0, 100.0);
    network.connect(vertices[0], vertices[2]).unwrap();
    node.vector_network = Some(network);
    let id = document.insert_node(document.pages[0].id, node).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let before = editor.document.to_json().unwrap();
    let world = editor.document.world_transform(id);
    assert!(editor.fill_network_faces(id).unwrap());
    assert_eq!(editor.history_len(), 1);
    assert_eq!(editor.document.world_transform(id), world);
    assert_eq!(editor.selection, vec![id]);
    let after = editor.document.to_json().unwrap();
    let filled_revision = editor.revision;
    assert!(!editor.fill_network_faces(id).unwrap());
    assert_eq!(editor.revision, filled_revision);
    assert_eq!(editor.history_len(), 1);
    assert_eq!(editor.document.to_json().unwrap(), after);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.can_redo());
    let undo_revision = editor.revision;
    assert_eq!(editor.fill_network_faces(99999), Err(NetworkFaceError::NotEditable));
    assert_eq!(editor.revision, undo_revision);
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.can_redo());
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
    editor.document.nodes.get_mut(&id).unwrap().locked = true;
    let locked = editor.document.to_json().unwrap();
    assert_eq!(editor.fill_network_faces(id), Err(NetworkFaceError::NotEditable));
    assert_eq!(editor.document.to_json().unwrap(), locked);
    assert_eq!(editor.history_len(), 1);
}

#[test]
fn network_faces_default_fill_detaches_empty_style_in_the_same_transaction() {
    let mut document = Document::new("Empty paint ownership");
    let mut node = Node::new(NodeKind::Vector, "Closed graph", 0.0, 0.0, 100.0, 100.0);
    node.fills.clear();
    let mut network = VectorNetwork::default();
    square(&mut network, 0.0, 0.0, 100.0);
    // Existing regions without paint must become visible without new region IDs.
    network.fill_all_faces().unwrap();
    let regions = network.regions.clone();
    let allocator = network.next_id;
    node.vector_network = Some(network);
    let id = document.insert_node(document.pages[0].id, node).unwrap();
    let mut editor = Editor::new(document);
    let style = editor.create_local_style_for_nodes(&[id], crate::LocalStyleKind::Fill, "Empty", "").unwrap();
    editor.clear_history();
    let before = editor.document.to_json().unwrap();
    assert!(editor.fill_network_faces(id).unwrap());
    let node = &editor.document.nodes[&id];
    assert_eq!(node.fill_style, None);
    assert_eq!(node.fills, vec![Fill::solid(crate::Color::rgb(217, 217, 217))]);
    assert_eq!(node.vector_network.as_ref().unwrap().regions, regions);
    assert_eq!(node.vector_network.as_ref().unwrap().next_id, allocator);
    assert!(editor.document.styles.contains_key(&style));
    assert_eq!(editor.history_len(), 1);
    let after = editor.document.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
}
