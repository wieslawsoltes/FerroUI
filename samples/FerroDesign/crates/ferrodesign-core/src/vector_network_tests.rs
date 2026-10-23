use super::*;
use crate::{Document, PathPointId};

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn close(actual: Point, expected: Point) {
    assert!(actual.distance(expected) < 1e-7, "{actual:?} differs from {expected:?}");
}

fn sample(network: &VectorNetwork, id: VectorEdgeId, t: f64) -> Point {
    let edge = &network.edges[&id];
    let a = network.vertices[&edge.start].position;
    let b = network.vertices[&edge.end].position;
    let u = 1.0 - t;
    match edge.curve {
        NetworkCurve::Line => a * u + b * t,
        NetworkCurve::Cubic { control1, control2 } => {
            a * (u * u * u) + control1 * (3.0 * u * u * t)
                + control2 * (3.0 * u * t * t) + b * (t * t * t)
        }
    }
}

fn ring(network: &mut VectorNetwork, points: &[Point]) -> (Vec<VectorVertexId>, Vec<NetworkEdgeRef>) {
    let vertices: Vec<_> = points.iter().map(|point| network.add_vertex(*point).unwrap()).collect();
    let edges = vertices.iter().zip(vertices.iter().cycle().skip(1))
        .map(|(&a, &b)| NetworkEdgeRef::forward(network.connect(a, b).unwrap()))
        .collect();
    (vertices, edges)
}

fn arch() -> VectorPath {
    VectorPath {
        segments: vec![
            PathSegment::MoveTo { point: p(0.0, 0.0) },
            PathSegment::CubicTo {
                control1: p(0.0, 100.0), control2: p(100.0, 100.0), point: p(100.0, 0.0),
            },
        ],
        ..VectorPath::default()
    }
}

#[test]
fn network_import_elevates_quadratics_and_keeps_implicit_fill_edges_unstroked() {
    let path = VectorPath {
        segments: vec![
            PathSegment::MoveTo { point: p(0.0, 0.0) },
            PathSegment::QuadraticTo { control: p(50.0, 100.0), point: p(100.0, 0.0) },
        ],
        ..VectorPath::default()
    };
    let network = VectorNetwork::from_path(&path).unwrap();
    assert_eq!(network.vertices.len(), 2);
    assert_eq!(network.edges.len(), 2);
    assert_eq!(network.regions.len(), 1);
    let visible = *network.edges.iter().find(|(_, edge)| edge.stroke).unwrap().0;
    let hidden = *network.edges.iter().find(|(_, edge)| !edge.stroke).unwrap().0;
    assert!(matches!(network.edges[&visible].curve, NetworkCurve::Cubic { .. }));
    for step in 0..=100 {
        let t = step as f64 / 100.0;
        close(sample(&network, visible, t), p(100.0 * t, 200.0 * t * (1.0 - t)));
    }
    assert_eq!(network.stroke_path().segments.len(), 2);
    assert!(network.contains_fill(p(50.0, 20.0)));
    assert!(!network.contains_fill(p(50.0, 70.0)));
    assert_eq!(network.nearest_segment(p(50.0, -1.0)).unwrap().edge, hidden);
    assert_eq!(network.nearest_stroke(p(50.0, -1.0)).unwrap().edge, visible);
    for query in [p(50.0, 20.0), p(5.0, 1.0), p(90.0, 45.0), p(-1.0, 0.0)] {
        assert_eq!(network.contains_fill(query), path.contains(query));
    }
    network.validate().unwrap();
}

#[test]
fn network_import_welds_only_closed_aliases_and_retains_tangent_constraints() {
    let mut path = VectorPath {
        segments: vec![
            PathSegment::MoveTo { point: p(0.0, 0.0) },
            PathSegment::CubicTo {
                control1: p(20.0, 0.0), control2: p(100.0, 20.0), point: p(100.0, 0.0),
            },
            PathSegment::CubicTo {
                control1: p(100.0, -20.0), control2: p(-20.0, 0.0), point: p(0.0, 0.0),
            },
            PathSegment::Close,
            PathSegment::MoveTo { point: p(0.0, 0.0) },
            PathSegment::LineTo { point: p(30.0, 30.0) },
        ],
        ..VectorPath::default()
    };
    path.anchor_modes.insert(0, AnchorMode::Symmetric);
    path.anchor_modes.insert(1, AnchorMode::Symmetric);
    let mut network = VectorNetwork::from_path(&path).unwrap();
    assert_eq!(network.vertices.len(), 4);
    assert_eq!(network.edges.values().filter(|edge| edge.stroke).count(), 3);
    let origins: Vec<_> = network.vertices.iter()
        .filter(|(_, vertex)| vertex.position == p(0.0, 0.0)).map(|(&id, _)| id).collect();
    assert_eq!(origins.len(), 2);
    assert_eq!(network.vertices[&origins[0]].mode, AnchorMode::Symmetric);
    assert_eq!(network.vertices[&origins[1]].mode, AnchorMode::Corner);
    assert!(network.move_edit_point(NetworkPointId::Vertex(origins[0]), p(-10.0, -20.0)));
    assert_eq!(network.vertices[&origins[1]].position, p(0.0, 0.0));
    assert_eq!(network.regions.len(), 2);
    network.validate().unwrap();
}

#[test]
fn network_stroke_trails_paint_every_edge_once_with_correct_reverse_controls() {
    let mut network = VectorNetwork::default();
    let center = network.add_vertex(p(0.0, 0.0)).unwrap();
    let left = network.add_vertex(p(-100.0, 0.0)).unwrap();
    let right = network.add_vertex(p(100.0, 0.0)).unwrap();
    let bottom = network.add_vertex(p(0.0, 100.0)).unwrap();
    let tail = network.add_vertex(p(100.0, 100.0)).unwrap();
    network.connect_curve(left, center, NetworkCurve::Cubic {
        control1: p(-80.0, -20.0), control2: p(-20.0, -30.0),
    }).unwrap();
    network.connect(center, right).unwrap();
    network.connect(center, bottom).unwrap();
    network.connect(bottom, tail).unwrap();
    let path = network.stroke_path();
    assert_eq!(path.segments.iter().filter(|s| matches!(s, PathSegment::MoveTo { .. })).count(), 3);
    assert_eq!(path.segments.len(), 7);
    assert_eq!(path.segments[1], PathSegment::CubicTo {
        control1: p(-20.0, -30.0), control2: p(-80.0, -20.0), point: p(-100.0, 0.0),
    });
    ring(&mut network, &[p(200.0, 0.0), p(300.0, 0.0), p(300.0, 100.0), p(200.0, 100.0)]);
    let loop_vertex = network.add_vertex(p(400.0, 0.0)).unwrap();
    network.connect_curve(loop_vertex, loop_vertex, NetworkCurve::Cubic {
        control1: p(430.0, -100.0), control2: p(470.0, 100.0),
    }).unwrap();
    let path = network.stroke_path();
    assert_eq!(path.segments.iter().filter(|s| matches!(s, PathSegment::Close)).count(), 2);
    assert_eq!(path.segments.iter().filter(|s| matches!(s, PathSegment::LineTo { .. } | PathSegment::CubicTo { .. })).count(), network.edges.len());
    assert!(network.fill_path().segments.is_empty());
}

#[test]
fn network_explicit_region_boundaries_support_holes_and_ignore_unfilled_branches() {
    let mut network = VectorNetwork { fill_rule: FillRule::EvenOdd, ..VectorNetwork::default() };
    let (_, outer) = ring(&mut network, &[p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)]);
    let (_, inner) = ring(&mut network, &[p(20.0, 20.0), p(80.0, 20.0), p(80.0, 80.0), p(20.0, 80.0)]);
    let (outside, _) = ring(&mut network, &[p(200.0, 0.0), p(300.0, 0.0), p(250.0, 100.0)]);
    let region = network.add_region(vec![outer.clone(), inner.clone()]).unwrap();
    assert!(network.contains_fill(p(10.0, 10.0)));
    assert!(!network.contains_fill(p(50.0, 50.0)));
    assert!(!network.contains_fill(p(250.0, 20.0)));
    network.fill_rule = FillRule::NonZero;
    assert!(network.contains_fill(p(50.0, 50.0)));
    network.regions.get_mut(&region).unwrap().contours[1] = inner.iter().rev()
        .map(|edge| NetworkEdgeRef { edge: edge.edge, reversed: !edge.reversed }).collect();
    assert!(!network.contains_fill(p(50.0, 50.0)));
    assert!(network.region_from_vertices(&outside).is_some());
    assert!(network.contains_fill(p(250.0, 20.0)));
    network.validate().unwrap();
}

#[test]
fn network_region_creation_rejects_discontinuous_reused_and_ambiguous_edges_atomically() {
    let mut network = VectorNetwork::default();
    let (vertices, edges) = ring(&mut network, &[p(0.0, 0.0), p(100.0, 0.0), p(50.0, 100.0)]);
    let before = network.clone();
    for contour in [vec![edges[0]], vec![edges[0], edges[2], edges[1]], vec![edges[0], NetworkEdgeRef::reverse(edges[0].edge)]] {
        assert!(network.add_region(vec![contour]).is_none());
        assert_eq!(network, before);
    }
    let region = network.region_from_vertices(&vertices).unwrap();
    let next = network.next_id;
    assert!(network.region_from_vertices(&[vertices[1], vertices[2], vertices[0]]).is_none());
    assert_eq!(network.next_id, next);
    network.connect_curve(vertices[0], vertices[1], NetworkCurve::Cubic {
        control1: p(30.0, 30.0), control2: p(70.0, 30.0),
    }).unwrap();
    let before = network.clone();
    assert!(network.region_from_vertices(&vertices).is_none());
    assert_eq!(network, before);
    assert!(network.regions.contains_key(&region));
    network.validate().unwrap();
}

fn shared_regions() -> (VectorNetwork, [VectorVertexId; 4], VectorEdgeId, [VectorRegionId; 2]) {
    let mut network = VectorNetwork::default();
    let (vertices, edges) = ring(&mut network, &[p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)]);
    let diagonal = network.connect_curve(vertices[0], vertices[2], NetworkCurve::Cubic {
        control1: p(20.0, 60.0), control2: p(80.0, 40.0),
    }).unwrap();
    let first = network.add_region(vec![vec![edges[0], edges[1], NetworkEdgeRef::reverse(diagonal)]]).unwrap();
    let second = network.add_region(vec![vec![NetworkEdgeRef::forward(diagonal), edges[2], edges[3]]]).unwrap();
    (network, vertices.try_into().unwrap(), diagonal, [first, second])
}

#[test]
fn network_split_is_exact_and_preserves_shared_region_orientation_and_ids() {
    let (mut network, vertices, diagonal, regions) = shared_regions();
    let before = network.clone();
    let inserted = network.split_edge(diagonal, 0.3).unwrap();
    let second = inserted + 1;
    assert_eq!(inserted, before.next_id);
    assert_eq!(network.next_id, before.next_id + 2);
    assert_eq!(network.vertices[&inserted].mode, AnchorMode::Smooth);
    assert_eq!(network.edges[&diagonal].start, vertices[0]);
    assert_eq!(network.edges[&diagonal].end, inserted);
    assert_eq!(network.edges[&second].start, inserted);
    assert_eq!(network.edges[&second].end, vertices[2]);
    for step in 0..=100 {
        let t = step as f64 / 100.0;
        close(sample(&network, diagonal, t), sample(&before, diagonal, t * 0.3));
        close(sample(&network, second, t), sample(&before, diagonal, 0.3 + t * 0.7));
    }
    let reverse = &network.regions[&regions[0]].contours[0];
    assert_eq!(&reverse[2..], &[NetworkEdgeRef::reverse(second), NetworkEdgeRef::reverse(diagonal)]);
    let forward = &network.regions[&regions[1]].contours[0];
    assert_eq!(&forward[..2], &[NetworkEdgeRef::forward(diagonal), NetworkEdgeRef::forward(second)]);
    for (&id, edge) in &before.edges {
        if id != diagonal { assert_eq!(&network.edges[&id], edge); }
    }
    for (&id, vertex) in &before.vertices { assert_eq!(&network.vertices[&id], vertex); }
    network.validate().unwrap();
}

#[test]
fn network_splitting_relaxes_only_affected_symmetric_handle_lengths() {
    let mut network = VectorNetwork::default();
    let a = network.add_vertex(p(-100.0, 0.0)).unwrap();
    let center = network.add_vertex(p(0.0, 0.0)).unwrap();
    let b = network.add_vertex(p(100.0, 0.0)).unwrap();
    let left = network.connect_curve(a, center, NetworkCurve::Cubic {
        control1: p(-80.0, 30.0), control2: p(-20.0, 0.0),
    }).unwrap();
    let right = network.connect_curve(center, b, NetworkCurve::Cubic {
        control1: p(20.0, 0.0), control2: p(80.0, -30.0),
    }).unwrap();
    assert!(network.set_vertex_mode(center, AnchorMode::Symmetric));
    let before = network.clone();
    let inserted = network.split_edge(right, 0.25).unwrap();
    assert_eq!(network.vertices[&center].mode, AnchorMode::Smooth);
    assert_eq!(network.vertices[&inserted].mode, AnchorMode::Smooth);
    assert_eq!(network.edges[&left], before.edges[&left]);
    close(network.point_position(NetworkPointId::Control1(right)).unwrap(), p(5.0, 0.0));
    close(network.point_position(NetworkPointId::Control2(left)).unwrap(), p(-20.0, 0.0));
    for step in 0..=40 {
        let t = step as f64 / 40.0;
        close(sample(&network, right, t), sample(&before, right, t * 0.25));
        close(sample(&network, inserted + 1, t), sample(&before, right, 0.25 + t * 0.75));
    }
    network.validate().unwrap();
}

#[test]
fn network_shared_vertex_drag_translates_each_attached_handle_once_including_self_loops() {
    let mut network = VectorNetwork::default();
    let center = network.add_vertex(p(10.0, 20.0)).unwrap();
    let other = network.add_vertex(p(200.0, 100.0)).unwrap();
    let loop_edge = network.connect_curve(center, center, NetworkCurve::Cubic {
        control1: p(80.0, -40.0), control2: p(-30.0, 60.0),
    }).unwrap();
    network.connect_curve(center, other, NetworkCurve::Cubic {
        control1: p(50.0, 70.0), control2: p(160.0, 140.0),
    }).unwrap();
    network.add_region(vec![vec![NetworkEdgeRef::forward(loop_edge)]]).unwrap();
    let points = network.edit_points();
    let delta = p(-150.0, 230.0);
    assert!(network.move_edit_point(NetworkPointId::Vertex(center), p(10.0, 20.0) + delta));
    for point in points {
        close(network.point_position(point.id).unwrap(), point.position + if point.vertex == center { delta } else { Point::default() });
    }
    assert_eq!(network.regions.len(), 1);
    network.validate().unwrap();
}

#[test]
fn network_tangent_constraints_follow_vertex_ownership_independent_of_edge_direction() {
    let mut network = VectorNetwork::default();
    let center = network.add_vertex(p(0.0, 0.0)).unwrap();
    let a = network.add_vertex(p(100.0, 0.0)).unwrap();
    let b = network.add_vertex(p(-100.0, 0.0)).unwrap();
    let first = network.connect_curve(center, a, NetworkCurve::Cubic {
        control1: p(20.0, 0.0), control2: p(80.0, 0.0),
    }).unwrap();
    let second = network.connect_curve(center, b, NetworkCurve::Cubic {
        control1: p(-20.0, 0.0), control2: p(-80.0, 0.0),
    }).unwrap();
    assert!(network.set_vertex_mode(center, AnchorMode::Symmetric));
    assert!(network.move_edit_point(NetworkPointId::Control1(first), p(0.0, 30.0)));
    close(network.point_position(NetworkPointId::Control1(second)).unwrap(), p(0.0, -30.0));
    assert!(network.set_vertex_mode(center, AnchorMode::Smooth));
    assert!(network.move_edit_point(NetworkPointId::Control1(first), p(40.0, 0.0)));
    close(network.point_position(NetworkPointId::Control1(second)).unwrap(), p(-30.0, 0.0));
    let branch = network.add_vertex(p(0.0, 100.0)).unwrap();
    network.connect(center, branch).unwrap();
    assert_eq!(network.vertices[&center].mode, AnchorMode::Corner);
    assert!(!network.set_vertex_mode(center, AnchorMode::Smooth));
    assert!(network.move_edit_point(NetworkPointId::Control1(first), p(50.0, 60.0)));
    close(network.point_position(NetworkPointId::Control1(second)).unwrap(), p(-30.0, 0.0));
    network.validate().unwrap();
}

#[test]
fn network_welding_preserves_edge_region_ids_and_translates_source_handles() {
    let (mut network, vertices, _, regions) = shared_regions();
    let source = network.add_vertex(p(-100.0, -100.0)).unwrap();
    let tip = network.add_vertex(p(-200.0, 100.0)).unwrap();
    let edge = network.connect_curve(source, tip, NetworkCurve::Cubic {
        control1: p(-120.0, -80.0), control2: p(-180.0, 80.0),
    }).unwrap();
    let before = network.clone();
    assert!(network.merge_vertices(source, vertices[0]));
    assert!(!network.vertices.contains_key(&source));
    assert_eq!(network.edges[&edge].start, vertices[0]);
    assert_eq!(network.edges[&edge].end, tip);
    close(network.point_position(NetworkPointId::Control1(edge)).unwrap(), p(-20.0, 20.0));
    close(network.point_position(NetworkPointId::Control2(edge)).unwrap(), p(-180.0, 80.0));
    assert_eq!(network.next_id, before.next_id);
    for region in regions { assert_eq!(network.regions[&region], before.regions[&region]); }
    assert!(!network.merge_vertices(vertices[0], vertices[0]));
    network.validate().unwrap();
    let source = network.add_vertex(p(-1e308, 0.0)).unwrap();
    let target = network.add_vertex(p(1e308, 0.0)).unwrap();
    let before = network.clone();
    assert!(!network.merge_vertices(source, target));
    assert_eq!(network, before);
}

#[test]
fn network_edge_and_vertex_erase_remove_only_regions_with_missing_boundaries() {
    let (mut network, vertices, diagonal, regions) = shared_regions();
    let erased = network.regions[&regions[0]].contours[0][0].edge;
    let before = network.clone();
    assert!(network.remove_edge(erased));
    assert!(!network.regions.contains_key(&regions[0]));
    assert_eq!(network.regions[&regions[1]], before.regions[&regions[1]]);
    assert_eq!(network.vertices, before.vertices);
    assert!(network.edges.contains_key(&diagonal));
    assert!(!network.remove_edge(erased));
    assert!(network.remove_vertex(vertices[0]));
    assert!(network.regions.is_empty());
    assert!(!network.edges.contains_key(&diagonal));
    assert!(network.edges.values().all(|edge| edge.start != vertices[0] && edge.end != vertices[0]));
    assert_eq!(network.next_id, before.next_id);
    network.validate().unwrap();
}

#[test]
fn network_nearest_queries_and_bounds_distinguish_strokes_from_fill_closures() {
    let mut network = VectorNetwork::from_path(&arch()).unwrap();
    let visible = *network.edges.iter().find(|(_, edge)| edge.stroke).unwrap().0;
    let hidden = *network.edges.iter().find(|(_, edge)| !edge.stroke).unwrap().0;
    let hit = network.nearest_stroke(p(50.0, 90.0)).unwrap();
    assert_eq!(hit.edge, visible);
    assert!((hit.t - 0.5).abs() < 1e-8);
    close(hit.position, p(50.0, 75.0));
    assert!((hit.distance - 15.0).abs() < 1e-8);
    assert_eq!(network.tight_bounds(), Some(Rect::new(0.0, 0.0, 100.0, 75.0)));
    assert_eq!(network.nearest_segment(p(50.0, 1.0)).unwrap().edge, hidden);
    assert!(!network.hit_test(p(50.0, 1.0), 2.0, false));
    assert!(network.hit_test(p(50.0, 1.0), 2.0, true));
    assert!(network.nearest_segment(p(f64::NAN, 0.0)).is_none());
    let transform = Transform {
        a: 4.0, b: 0.2, c: 1.3, d: 0.5, e: -800.0, f: 350.0,
    };
    let query = transform.apply(sample(&network, visible, 0.37));
    let transformed_hit = network.nearest_segment_transformed(query, transform).unwrap();
    assert_eq!(transformed_hit.edge, visible);
    assert!((transformed_hit.t - 0.37).abs() < 1e-8);
    close(transformed_hit.position, query);
    assert!(transformed_hit.distance < 1e-7);
    network.add_vertex(p(-20.0, -30.0)).unwrap();
    assert_eq!(network.tight_bounds(), Some(Rect::new(-20.0, -30.0, 120.0, 105.0)));
}

#[test]
fn network_bend_keeps_anchors_regions_and_ids_while_matching_the_pointer() {
    let (mut network, _, diagonal, _) = shared_regions();
    let before = network.clone();
    let target = sample(&before, diagonal, 0.3) + p(-30.0, 45.0);
    assert!(network.bend_edge(diagonal, 0.3, target));
    close(sample(&network, diagonal, 0.3), target);
    assert_eq!(network.vertices, before.vertices);
    assert_eq!(network.regions, before.regions);
    assert_eq!(network.next_id, before.next_id);
    for (&id, edge) in &before.edges {
        if id != diagonal { assert_eq!(&network.edges[&id], edge); }
    }
    let first = network.point_position(NetworkPointId::Control1(diagonal)).unwrap()
        - before.point_position(NetworkPointId::Control1(diagonal)).unwrap();
    let second = network.point_position(NetworkPointId::Control2(diagonal)).unwrap()
        - before.point_position(NetworkPointId::Control2(diagonal)).unwrap();
    close(first * (0.3 / 0.7), second);
    let line = *network.edges.iter().find(|(_, edge)| matches!(edge.curve, NetworkCurve::Line)).unwrap().0;
    let target = sample(&network, line, 0.5) + p(10.0, -20.0);
    assert!(network.bend_edge(line, 0.5, target));
    assert!(matches!(network.edges[&line].curve, NetworkCurve::Cubic { .. }));
    close(sample(&network, line, 0.5), target);
    network.validate().unwrap();
}

#[test]
fn network_bend_propagates_symmetric_partners_and_declines_constrained_loops_atomically() {
    let mut network = VectorNetwork::default();
    let center = network.add_vertex(p(0.0, 0.0)).unwrap();
    let left = network.add_vertex(p(-100.0, 0.0)).unwrap();
    let right = network.add_vertex(p(100.0, 0.0)).unwrap();
    let a = network.connect(center, left).unwrap();
    let b = network.connect(center, right).unwrap();
    assert!(network.set_vertex_mode(center, AnchorMode::Symmetric));
    let target = sample(&network, a, 0.4) + p(-10.0, 40.0);
    assert!(network.bend_edge(a, 0.4, target));
    close(sample(&network, a, 0.4), target);
    close(
        network.point_position(NetworkPointId::Control1(a)).unwrap()
            + network.point_position(NetworkPointId::Control1(b)).unwrap(),
        Point::default(),
    );
    let vertex = network.add_vertex(p(300.0, 0.0)).unwrap();
    let edge = network.connect_curve(vertex, vertex, NetworkCurve::Cubic {
        control1: p(340.0, 80.0), control2: p(260.0, -80.0),
    }).unwrap();
    assert!(network.set_vertex_mode(vertex, AnchorMode::Symmetric));
    let before = network.clone();
    assert!(!network.bend_edge(edge, 0.5, p(300.0, 50.0)));
    assert!(!network.bend_edge(a, 0.0, p(10.0, 50.0)));
    assert!(!network.bend_edge(a, 0.5, p(f64::INFINITY, 0.0)));
    assert_eq!(network, before);
    network.validate().unwrap();
}

#[test]
fn network_invalid_edits_and_exhausted_ids_do_not_partially_change_graphs() {
    let mut network = VectorNetwork::from_path(&arch()).unwrap();
    let edge = *network.edges.keys().next().unwrap();
    let before = network.clone();
    assert!(network.add_vertex(p(f64::INFINITY, 1.0)).is_none());
    assert!(network.connect(999, 1000).is_none());
    assert!(!network.move_edit_point(NetworkPointId::Control1(edge), p(f64::NAN, 0.0)));
    for t in [0.0, 1.0, -0.1, f64::NAN] { assert!(network.split_edge(edge, t).is_none()); }
    assert_eq!(network, before);
    network.next_id = u64::MAX;
    let before = network.clone();
    assert!(network.add_vertex(p(10.0, 10.0)).is_none());
    assert!(network.split_edge(edge, 0.5).is_none());
    assert_eq!(network, before);
    network.validate().unwrap();
    let mut invalid = network.clone();
    invalid.edges.get_mut(&edge).unwrap().start = 999;
    assert!(invalid.validate().is_err());
    let mut invalid = network.clone();
    invalid.next_id = edge;
    assert!(invalid.validate().is_err());
}

fn network_editor() -> (Editor, NodeId) {
    let mut document = Document::new("Network invariants");
    let page = document.pages[0].id;
    let mut parent = Node::new(NodeKind::Frame, "Affine frame", 150.0, -40.0, 600.0, 400.0);
    parent.rotation = -18.0;
    parent.flip_y = true;
    parent.affine_transform = Some(Transform { a: 1.7, c: 0.35, d: 0.7, ..Transform::IDENTITY });
    let parent = document.insert_node(page, parent).unwrap();
    let mut node = Node::new(NodeKind::Vector, "Network", 30.0, 45.0, 100.0, 100.0);
    node.rotation = 27.0;
    node.flip_x = true;
    node.affine_transform = Some(Transform { a: 0.85, b: 0.15, c: -0.22, d: 1.3, e: 3.0, f: -5.0 });
    node.path = arch();
    let id = document.insert_node(parent, node).unwrap();
    (Editor::new(document), id)
}

#[test]
fn network_editor_conversion_and_scoped_drag_preserve_world_geometry_and_one_undo() {
    let (mut editor, id) = network_editor();
    editor.selection = vec![id];
    let original = editor.document.to_json().unwrap();
    let original_world = editor.document.world_transform(id).unwrap();
    let original_points: Vec<_> = editor.document.nodes[&id].path.edit_points()
        .into_iter().map(|point| original_world.apply(point.position)).collect();
    assert!(editor.convert_vector_to_network(id));
    assert_eq!(editor.history_len(), 1);
    assert!(editor.document.nodes[&id].path.segments.is_empty());
    let converted = editor.document.to_json().unwrap();
    let network = editor.document.nodes[&id].vector_network.as_ref().unwrap();
    let world = editor.document.world_transform(id).unwrap();
    let points = network.edit_points();
    for position in original_points {
        assert!(points.iter().any(|point| world.apply(point.position).distance(position) < 1e-7));
    }
    let vertex = *network.vertices.keys().next().unwrap();
    let start = world.apply(network.vertices[&vertex].position);
    let delta = p(-420.0, 210.0);
    let world_points: Vec<_> = points.into_iter().map(|point| (point, world.apply(point.position))).collect();
    editor.begin_node_transaction(&[id], "Drag shared network vertex");
    for step in 1..=24 {
        assert!(editor.move_network_point(id, NetworkPointId::Vertex(vertex), start + delta * (step as f64 / 24.0)));
    }
    assert_eq!(editor.history_len(), 1);
    assert!(editor.commit_transaction());
    assert_eq!(editor.history_len(), 2);
    assert_eq!(editor.undo_label(), Some("Drag shared network vertex"));
    let network = editor.document.nodes[&id].vector_network.as_ref().unwrap();
    let world = editor.document.world_transform(id).unwrap();
    for (point, original) in world_points {
        close(world.apply(network.point_position(point.id).unwrap()), original + if point.vertex == vertex { delta } else { Point::default() });
    }
    let committed = editor.document.to_json().unwrap();
    assert_eq!(Document::from_json(&committed).unwrap().to_json().unwrap(), committed);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), converted);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), committed);
    assert!(editor.undo());
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), original);
    assert_eq!(editor.selection, vec![id]);
}

#[test]
fn network_editor_branch_is_atomic_and_legacy_or_locked_edits_are_declined() {
    let (mut editor, id) = network_editor();
    assert!(editor.convert_vector_to_network(id));
    let source = *editor.document.nodes[&id].vector_network.as_ref().unwrap().vertices.keys().next().unwrap();
    let before = editor.document.to_json().unwrap();
    let (vertex, edge) = editor.branch_network_vertex(id, source, p(500.0, -300.0)).unwrap();
    assert_eq!(editor.history_len(), 2);
    let network = editor.document.nodes[&id].vector_network.as_ref().unwrap();
    assert_eq!(network.edges[&edge].start, source);
    assert_eq!(network.edges[&edge].end, vertex);
    close(editor.document.world_transform(id).unwrap().apply(network.vertices[&vertex].position), p(500.0, -300.0));
    let branched = editor.document.to_json().unwrap();
    assert!(editor.branch_network_vertex(id, u64::MAX, p(10.0, 10.0)).is_none());
    assert!(!editor.move_vector_point(id, PathPointId::anchor(0), p(10.0, 10.0)));
    assert_eq!(editor.document.to_json().unwrap(), branched);
    assert_eq!(editor.history_len(), 2);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    editor.document.nodes.get_mut(&id).unwrap().locked = true;
    let locked = editor.document.to_json().unwrap();
    assert!(!editor.move_network_point(id, NetworkPointId::Vertex(source), p(20.0, 40.0)));
    assert!(editor.add_network_vertex(id, p(20.0, 40.0)).is_none());
    assert_eq!(editor.document.to_json().unwrap(), locked);
}

#[test]
fn network_resize_scales_authoritative_graph_and_retains_topology_on_undo() {
    let (mut editor, id) = network_editor();
    assert!(editor.convert_vector_to_network(id));
    let before = editor.document.to_json().unwrap();
    let node = editor.document.nodes[&id].clone();
    let network = node.vector_network.as_ref().unwrap();
    editor.resize_node(id, node.width * 2.0, node.height * 0.5);
    let resized = &editor.document.nodes[&id];
    let expected = network.transformed(Transform::scale(2.0, 0.5));
    assert_eq!(resized.vector_network.as_ref().unwrap(), &expected);
    assert!(resized.path.segments.is_empty());
    editor.document.validate().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
}

#[test]
fn network_document_roundtrip_rejects_mixed_geometry_and_invalid_region_references() {
    let (mut editor, id) = network_editor();
    assert!(editor.convert_vector_to_network(id));
    let node = editor.document.nodes.get_mut(&id).unwrap();
    node.path = arch();
    assert!(editor.document.validate().is_err());
    editor.document.nodes.get_mut(&id).unwrap().path = VectorPath::default();
    let network = editor.document.nodes.get_mut(&id).unwrap().vector_network.as_mut().unwrap();
    let region = network.regions.values_mut().next().unwrap();
    region.contours[0][0].edge = u64::MAX;
    assert!(editor.document.validate().is_err());
    assert!(Document::from_json(&editor.document.to_json().unwrap()).is_err());
}
