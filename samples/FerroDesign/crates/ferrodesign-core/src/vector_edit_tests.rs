use crate::*;

fn point(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn close(actual: Point, expected: Point) {
    assert!(
        actual.distance(expected) < 1e-7,
        "{actual:?} differs from {expected:?}"
    );
}

fn path(segments: Vec<PathSegment>) -> VectorPath {
    VectorPath {
        segments,
        fill_rule: FillRule::EvenOdd,
        ..VectorPath::default()
    }
}

fn cubic() -> VectorPath {
    path(vec![
        PathSegment::MoveTo {
            point: point(0.0, 0.0),
        },
        PathSegment::CubicTo {
            control1: point(25.0, 70.0),
            control2: point(70.0, -40.0),
            point: point(100.0, 50.0),
        },
        PathSegment::CubicTo {
            control1: point(130.0, 140.0),
            control2: point(180.0, 20.0),
            point: point(200.0, 100.0),
        },
    ])
}

fn evaluate(start: Point, segment: &PathSegment, t: f64) -> Point {
    let interpolate = |a: Point, b: Point| a * (1.0 - t) + b * t;
    match *segment {
        PathSegment::LineTo { point } => interpolate(start, point),
        PathSegment::QuadraticTo { control, point } => {
            interpolate(interpolate(start, control), interpolate(control, point))
        }
        PathSegment::CubicTo {
            control1,
            control2,
            point,
        } => {
            let a = interpolate(start, control1);
            let b = interpolate(control1, control2);
            let c = interpolate(control2, point);
            interpolate(interpolate(a, b), interpolate(b, c))
        }
        _ => panic!("expected a drawable command"),
    }
}

fn edit_point(path: &VectorPath, segment: usize, kind: PathPointKind) -> Point {
    path.point_position(PathPointId { segment, kind }).unwrap()
}

#[test]
fn vector_topology_welds_closed_endpoint_aliases_and_keeps_subpaths_separate() {
    let p = path(vec![
        PathSegment::MoveTo {
            point: point(0.0, 0.0),
        },
        PathSegment::CubicTo {
            control1: point(25.0, 0.0),
            control2: point(75.0, 50.0),
            point: point(100.0, 50.0),
        },
        PathSegment::CubicTo {
            control1: point(100.0, 75.0),
            control2: point(0.0, 25.0),
            point: point(0.0, 0.0),
        },
        PathSegment::Close,
        PathSegment::MoveTo {
            point: point(200.0, 200.0),
        },
        PathSegment::QuadraticTo {
            control: point(220.0, 260.0),
            point: point(240.0, 200.0),
        },
    ]);
    let points = p.edit_points();
    let anchors: Vec<_> = points
        .iter()
        .filter(|p| p.id.kind == PathPointKind::Anchor)
        .map(|p| p.id.segment)
        .collect();
    assert_eq!(anchors, vec![0, 1, 4, 5]);
    let closing_handle = points
        .iter()
        .find(|p| p.id.segment == 2 && p.id.kind == PathPointKind::Control2)
        .unwrap();
    assert_eq!(closing_handle.anchor_id, PathPointId::anchor(0));
    close(closing_handle.anchor, point(0.0, 0.0));
    close(
        p.point_position(PathPointId::anchor(2)).unwrap(),
        point(0.0, 0.0),
    );
    let quadratic = points
        .iter()
        .find(|p| p.id.kind == PathPointKind::QuadraticControl)
        .unwrap();
    assert_eq!(quadratic.anchor_id, PathPointId::anchor(5));
    assert_eq!(quadratic.secondary_anchor, Some(point(200.0, 200.0)));
}

#[test]
fn vector_anchor_move_translates_adjacent_handles_and_preserves_unrelated_points() {
    let mut p = cubic();
    let before = p.clone();
    let delta = point(-30.0, 80.0);
    assert!(p.move_edit_point(PathPointId::anchor(1), point(100.0, 50.0) + delta));
    for (segment, kind, shifted) in [
        (0, PathPointKind::Anchor, false),
        (1, PathPointKind::Anchor, true),
        (2, PathPointKind::Anchor, false),
        (1, PathPointKind::Control1, false),
        (1, PathPointKind::Control2, true),
        (2, PathPointKind::Control1, true),
        (2, PathPointKind::Control2, false),
    ] {
        let expected =
            edit_point(&before, segment, kind) + if shifted { delta } else { Point::default() };
        close(edit_point(&p, segment, kind), expected);
    }
    assert_eq!(p.fill_rule, FillRule::EvenOdd);
}

#[test]
fn vector_closed_anchor_move_welds_both_endpoints_and_moves_each_handle_once() {
    let mut p = path(vec![
        PathSegment::MoveTo {
            point: point(10.0, 20.0),
        },
        PathSegment::CubicTo {
            control1: point(40.0, 0.0),
            control2: point(-30.0, 50.0),
            point: point(10.0, 20.0),
        },
        PathSegment::Close,
    ]);
    assert!(p.move_edit_point(PathPointId::anchor(1), point(25.0, 12.0)));
    assert_eq!(
        p.segments[0],
        PathSegment::MoveTo {
            point: point(25.0, 12.0)
        }
    );
    assert_eq!(
        p.segments[1],
        PathSegment::CubicTo {
            control1: point(55.0, -8.0),
            control2: point(-15.0, 42.0),
            point: point(25.0, 12.0)
        }
    );
    assert_eq!(
        p.edit_points()
            .iter()
            .filter(|p| p.id.kind == PathPointKind::Anchor)
            .count(),
        1
    );
    let mut quadratic = path(vec![
        PathSegment::MoveTo {
            point: Point::default(),
        },
        PathSegment::QuadraticTo {
            control: point(40.0, 50.0),
            point: Point::default(),
        },
        PathSegment::Close,
    ]);
    assert!(quadratic.move_edit_point(PathPointId::anchor(0), point(3.0, 7.0)));
    close(
        edit_point(&quadratic, 1, PathPointKind::QuadraticControl),
        point(43.0, 57.0),
    );
}

#[test]
fn vector_control_move_changes_only_the_requested_control() {
    let mut p = cubic();
    let mut expected = p.clone();
    let PathSegment::CubicTo {
        ref mut control1, ..
    } = expected.segments[2]
    else {
        unreachable!()
    };
    *control1 = point(170.0, -200.0);
    assert!(p.move_edit_point(
        PathPointId {
            segment: 2,
            kind: PathPointKind::Control1
        },
        point(170.0, -200.0)
    ));
    assert_eq!(p, expected);
    let before = p.clone();
    assert!(!p.move_edit_point(
        PathPointId {
            segment: 0,
            kind: PathPointKind::Control1
        },
        point(1.0, 1.0)
    ));
    assert!(!p.move_edit_point(PathPointId::anchor(1000), point(1.0, 1.0)));
    assert!(!p.move_edit_point(PathPointId::anchor(0), point(f64::NAN, 3.0)));
    assert_eq!(p, before);
}

#[test]
fn vector_subdivision_preserves_cubic_and_quadratic_curves_for_every_parameter() {
    for original in [
        cubic().segments[1].clone(),
        PathSegment::QuadraticTo {
            control: point(-170.0, 380.0),
            point: point(100.0, 50.0),
        },
        PathSegment::LineTo {
            point: point(100.0, 50.0),
        },
    ] {
        for split in [0.001, 0.17, 0.5, 0.823, 0.999] {
            let start = point(12.0, -16.0);
            let tail = PathSegment::MoveTo {
                point: point(1000.0, 1000.0),
            };
            let mut p = path(vec![
                PathSegment::MoveTo { point: start },
                original.clone(),
                tail.clone(),
            ]);
            assert_eq!(p.split_segment(1, split), Some(PathPointId::anchor(1)));
            let middle = p.point_position(PathPointId::anchor(1)).unwrap();
            close(middle, evaluate(start, &original, split));
            for i in 0..=100 {
                let t = i as f64 / 100.0;
                close(
                    evaluate(start, &p.segments[1], t),
                    evaluate(start, &original, t * split),
                );
                close(
                    evaluate(middle, &p.segments[2], t),
                    evaluate(start, &original, split + t * (1.0 - split)),
                );
            }
            assert_eq!(p.segments[3], tail);
            assert_eq!(p.fill_rule, FillRule::EvenOdd);
        }
    }
}

#[test]
fn vector_subdivision_of_close_keeps_closure_and_never_bridges_move_commands() {
    let mut p = path(vec![
        PathSegment::MoveTo {
            point: point(0.0, 0.0),
        },
        PathSegment::LineTo {
            point: point(100.0, 0.0),
        },
        PathSegment::LineTo {
            point: point(100.0, 100.0),
        },
        PathSegment::Close,
        PathSegment::MoveTo {
            point: point(1000.0, 0.0),
        },
        PathSegment::LineTo {
            point: point(1000.0, 100.0),
        },
    ]);
    let hit = p.nearest_segment(point(25.0, 24.0)).unwrap();
    assert_eq!(hit.segment, 3);
    assert!((hit.t - 0.755).abs() < 1e-10);
    assert_eq!(p.split_segment(3, 0.5), Some(PathPointId::anchor(3)));
    assert_eq!(
        p.segments[3],
        PathSegment::LineTo {
            point: point(50.0, 50.0)
        }
    );
    assert_eq!(p.segments[4], PathSegment::Close);
    let before = p.clone();
    assert_eq!(p.split_segment(5, 0.5), None);
    assert_eq!(p.split_segment(1, 0.0), None);
    assert_eq!(p.split_segment(1, 1.0), None);
    assert_eq!(p.split_segment(1, f64::NAN), None);
    assert_eq!(p, before);
    let isolated = path(vec![
        PathSegment::MoveTo {
            point: point(0.0, 0.0),
        },
        PathSegment::MoveTo {
            point: point(100.0, 0.0),
        },
    ]);
    assert!(isolated.nearest_segment(point(50.0, 0.0)).is_none());
}

#[test]
fn vector_nearest_curve_retains_global_minimum_on_loops_cusps_and_large_translations() {
    let commands = [
        PathSegment::CubicTo {
            control1: point(300.0, 500.0),
            control2: point(-300.0, 500.0),
            point: point(20.0, 0.0),
        },
        PathSegment::CubicTo {
            control1: point(300.0, 400.0),
            control2: point(-300.0, -400.0),
            point: point(0.0, 0.0),
        },
        PathSegment::QuadraticTo {
            control: point(150.0, 180.0),
            point: point(0.0, 0.0),
        },
        PathSegment::CubicTo {
            control1: point(0.0, 0.0),
            control2: point(0.0, 0.0),
            point: point(0.0, 0.0),
        },
    ];
    for command in commands {
        for offset in [point(0.0, 0.0), point(1e8, -1e8)] {
            let p = path(vec![
                PathSegment::MoveTo {
                    point: Point::default(),
                },
                command.clone(),
            ])
            .transformed(Transform::translation(offset.x, offset.y));
            for query in [
                point(0.0, 250.0),
                point(45.0, 100.0),
                point(-80.0, 175.0),
                point(100.0, -50.0),
                point(0.0, 0.0),
            ] {
                let query = query + offset;
                let hit = p.nearest_segment(query).unwrap();
                assert_eq!(hit.segment, 1);
                assert!((0.0..=1.0).contains(&hit.t));
                close(hit.position, evaluate(offset, &p.segments[1], hit.t));
                let sampled = (0..=10000)
                    .map(|i| evaluate(offset, &p.segments[1], i as f64 / 10000.0).distance(query))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    hit.distance <= sampled + 1e-6,
                    "{} should be <= {} at {:?}",
                    hit.distance,
                    sampled,
                    query
                );
            }
        }
    }
}

#[test]
fn vector_tight_bounds_use_curve_extrema_instead_of_the_control_hull() {
    let quadratic = path(vec![
        PathSegment::MoveTo {
            point: Point::default(),
        },
        PathSegment::QuadraticTo {
            control: point(50.0, 100.0),
            point: point(100.0, 0.0),
        },
    ]);
    assert_eq!(
        quadratic.tight_bounds(),
        Some(Rect::new(0.0, 0.0, 100.0, 50.0))
    );
    let cubic = path(vec![
        PathSegment::MoveTo {
            point: Point::default(),
        },
        PathSegment::CubicTo {
            control1: point(0.0, 100.0),
            control2: point(100.0, 100.0),
            point: point(100.0, 0.0),
        },
    ]);
    let bounds = cubic.tight_bounds().unwrap();
    close(point(bounds.width, bounds.height), point(100.0, 75.0));
    for i in 0..=1000 {
        assert!(bounds.inflate(1e-10).contains(evaluate(
            Point::default(),
            &cubic.segments[1],
            i as f64 / 1000.0
        )));
    }
}

#[test]
fn vector_anchor_delete_preserves_tangents_at_the_retained_endpoints() {
    let mut p = cubic();
    assert!(p.delete_anchor(1));
    assert_eq!(p.segments.len(), 2);
    assert_eq!(
        p.segments[1],
        PathSegment::CubicTo {
            control1: point(25.0, 70.0),
            control2: point(180.0, 20.0),
            point: point(200.0, 100.0),
        }
    );
    assert!(p.delete_anchor(0));
    assert_eq!(
        p.segments,
        vec![PathSegment::MoveTo {
            point: point(200.0, 100.0)
        }]
    );
    assert!(p.delete_anchor(0));
    assert!(p.segments.is_empty());
    assert_eq!(p.fill_rule, FillRule::EvenOdd);
}

#[test]
fn vector_closed_anchor_delete_rotates_the_seam_and_preserves_other_contours() {
    let tail = [
        PathSegment::MoveTo {
            point: point(500.0, 500.0),
        },
        PathSegment::LineTo {
            point: point(600.0, 600.0),
        },
    ];
    let mut p = VectorPath::polygon(
        &[
            point(0.0, 0.0),
            point(100.0, 0.0),
            point(100.0, 100.0),
            point(0.0, 100.0),
        ],
        true,
    );
    p.segments.extend(tail.clone());
    assert!(p.delete_anchor(0));
    assert_eq!(
        p.segments,
        vec![
            PathSegment::MoveTo {
                point: point(100.0, 0.0)
            },
            PathSegment::LineTo {
                point: point(100.0, 100.0)
            },
            PathSegment::LineTo {
                point: point(0.0, 100.0)
            },
            PathSegment::Close,
            tail[0].clone(),
            tail[1].clone(),
        ]
    );
    let mut curved = cubic();
    curved.segments.push(PathSegment::CubicTo {
        control1: point(200.0, 200.0),
        control2: point(-25.0, -70.0),
        point: Point::default(),
    });
    curved.segments.push(PathSegment::Close);
    assert!(curved.delete_anchor(3));
    assert_eq!(
        curved.segments[0],
        PathSegment::MoveTo {
            point: point(100.0, 50.0)
        }
    );
    assert_eq!(
        curved.segments[2],
        PathSegment::CubicTo {
            control1: point(200.0, 200.0),
            control2: point(70.0, -40.0),
            point: point(100.0, 50.0)
        }
    );
    assert_eq!(curved.segments[3], PathSegment::Close);
}

#[test]
fn vector_anchor_conversion_establishes_smooth_and_symmetric_tangents() {
    let mut p = cubic();
    if let PathSegment::CubicTo { control1, .. } = &mut p.segments[2] {
        *control1 = point(140.0, 180.0);
    }
    let anchor = edit_point(&p, 1, PathPointKind::Anchor);
    let original_in = anchor.distance(edit_point(&p, 1, PathPointKind::Control2));
    let original_out = anchor.distance(edit_point(&p, 2, PathPointKind::Control1));
    assert!(p.convert_anchor(1, AnchorMode::Smooth));
    let incoming = anchor - edit_point(&p, 1, PathPointKind::Control2);
    let outgoing = edit_point(&p, 2, PathPointKind::Control1) - anchor;
    assert!((incoming.x * outgoing.y - incoming.y * outgoing.x).abs() < 1e-9);
    assert!((incoming.distance(Point::default()) - original_in).abs() < 1e-9);
    assert!((outgoing.distance(Point::default()) - original_out).abs() < 1e-9);
    assert!(p.convert_anchor(1, AnchorMode::Symmetric));
    close(
        anchor - edit_point(&p, 1, PathPointKind::Control2),
        edit_point(&p, 2, PathPointKind::Control1) - anchor,
    );
    assert!(p.convert_anchor(1, AnchorMode::Corner));
    close(edit_point(&p, 1, PathPointKind::Control2), anchor);
    close(edit_point(&p, 2, PathPointKind::Control1), anchor);
    assert!(!p.convert_anchor(1, AnchorMode::Corner));
}

#[test]
fn vector_quadratic_conversion_degree_elevates_untouched_endpoint_tangents() {
    let mut p = path(vec![
        PathSegment::MoveTo {
            point: Point::default(),
        },
        PathSegment::QuadraticTo {
            control: point(60.0, 90.0),
            point: point(120.0, 0.0),
        },
    ]);
    assert!(p.convert_anchor(1, AnchorMode::Corner));
    assert_eq!(
        p.segments[1],
        PathSegment::CubicTo {
            control1: point(40.0, 60.0),
            control2: point(120.0, 0.0),
            point: point(120.0, 0.0)
        }
    );
}

#[test]
fn vector_tangent_modes_round_trip_and_legacy_paths_keep_independent_handles() {
    let original = cubic();
    let legacy = serde_json::json!({
        "segments": original.segments,
        "fill_rule": "even_odd"
    });
    let mut p: VectorPath = serde_json::from_value(legacy).unwrap();
    assert!(p.anchor_modes.is_empty());
    assert_eq!(p.anchor_mode(1), Some(AnchorMode::Corner));
    assert_eq!(p.anchor_mode(99), None);
    assert!(p.validate_anchor_modes());
    assert!(p.convert_anchor(1, AnchorMode::Smooth));
    let json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["anchor_modes"]["1"], "smooth");
    let restored: VectorPath = serde_json::from_value(json).unwrap();
    assert_eq!(restored, p);
    assert!(restored.validate_anchor_modes());
    let transformed = restored.transformed(Transform {
        a: 0.9,
        b: 0.3,
        c: -0.5,
        d: 1.7,
        e: -81.0,
        f: 240.0,
    });
    assert_eq!(transformed.anchor_modes, p.anchor_modes);
    assert!(transformed.validate_anchor_modes());
}

#[test]
fn vector_smooth_handle_drags_preserve_opposite_length_and_anchor_translation() {
    let mut p = cubic();
    assert!(p.convert_anchor(1, AnchorMode::Smooth));
    let anchor = edit_point(&p, 1, PathPointKind::Anchor);
    let incoming = PathPointId { segment: 1, kind: PathPointKind::Control2 };
    let outgoing = PathPointId { segment: 2, kind: PathPointKind::Control1 };
    let untouched = edit_point(&p, 1, PathPointKind::Control1);
    let old_length = p.point_position(incoming).unwrap().distance(anchor);
    assert!(p.move_edit_point(outgoing, anchor + point(42.0, -80.0)));
    let incoming_vector = p.point_position(incoming).unwrap() - anchor;
    let outgoing_vector = p.point_position(outgoing).unwrap() - anchor;
    assert!((incoming_vector.distance(Point::default()) - old_length).abs() < 1e-8);
    assert!((incoming_vector.x * outgoing_vector.y - incoming_vector.y * outgoing_vector.x).abs() < 1e-8);
    assert!(incoming_vector.x * outgoing_vector.x + incoming_vector.y * outgoing_vector.y < 0.0);
    close(edit_point(&p, 1, PathPointKind::Control1), untouched);
    let outgoing_length = outgoing_vector.distance(Point::default());
    assert!(p.move_edit_point(incoming, anchor + point(-90.0, -16.0)));
    assert!((p.point_position(outgoing).unwrap().distance(anchor) - outgoing_length).abs() < 1e-8);
    let before_in = p.point_position(incoming).unwrap();
    let before_out = p.point_position(outgoing).unwrap();
    let delta = point(-41.0, 19.0);
    assert!(p.move_edit_point(PathPointId::anchor(1), anchor + delta));
    close(p.point_position(incoming).unwrap(), before_in + delta);
    close(p.point_position(outgoing).unwrap(), before_out + delta);
    assert_eq!(p.anchor_mode(1), Some(AnchorMode::Smooth));
    assert!(p.validate_anchor_modes());
    let before = p.clone();
    assert!(!p.convert_anchor(1, AnchorMode::Smooth));
    assert_eq!(p, before);
}

#[test]
fn vector_symmetric_handles_mirror_retract_and_release_to_corner() {
    let mut p = cubic();
    assert!(p.convert_anchor(1, AnchorMode::Symmetric));
    let anchor = edit_point(&p, 1, PathPointKind::Anchor);
    let incoming = PathPointId { segment: 1, kind: PathPointKind::Control2 };
    let outgoing = PathPointId { segment: 2, kind: PathPointKind::Control1 };
    for offset in [point(71.0, -32.0), Point::default(), point(-45.0, 88.0)] {
        assert!(p.move_edit_point(outgoing, anchor + offset));
        close(p.point_position(incoming).unwrap(), anchor - offset);
        assert!(p.validate_anchor_modes());
    }
    assert!(p.convert_anchor(1, AnchorMode::Corner));
    assert!(p.anchor_modes.is_empty());
    assert!(p.move_edit_point(outgoing, anchor + point(23.0, 47.0)));
    close(p.point_position(incoming).unwrap(), anchor);
    assert_eq!(p.anchor_mode(1), Some(AnchorMode::Corner));
}

#[test]
fn vector_closed_aliases_share_one_persistent_constraint() {
    let mut p = path(vec![
        PathSegment::MoveTo { point: Point::default() },
        PathSegment::CubicTo {
            control1: point(20.0, 20.0), control2: point(70.0, 50.0), point: point(100.0, 50.0)
        },
        PathSegment::CubicTo {
            control1: point(80.0, 100.0), control2: point(-20.0, 60.0), point: Point::default()
        },
        PathSegment::Close,
        PathSegment::MoveTo { point: point(300.0, 100.0) },
        PathSegment::CubicTo {
            control1: point(320.0, 80.0), control2: point(350.0, 180.0), point: point(390.0, 130.0)
        },
    ]);
    assert!(p.convert_anchor(2, AnchorMode::Symmetric));
    assert_eq!(p.anchor_modes.keys().copied().collect::<Vec<_>>(), vec![0]);
    assert_eq!(p.anchor_mode(0), Some(AnchorMode::Symmetric));
    assert_eq!(p.anchor_mode(2), Some(AnchorMode::Symmetric));
    let tail = p.segments[4..].to_vec();
    assert!(p.move_edit_point(
        PathPointId { segment: 2, kind: PathPointKind::Control2 },
        point(-45.0, 80.0)
    ));
    close(edit_point(&p, 1, PathPointKind::Control1), point(45.0, -80.0));
    assert_eq!(p.segments[4..], tail);
    assert!(p.validate_anchor_modes());
}

#[test]
fn vector_materializing_close_and_deleting_first_anchor_remaps_later_modes() {
    let mut p = VectorPath::polygon(
        &[point(0.0, 0.0), point(100.0, 0.0), point(100.0, 100.0)],
        true,
    );
    p.segments.extend(cubic().segments);
    assert!(p.convert_anchor(5, AnchorMode::Symmetric));
    assert!(p.convert_anchor(0, AnchorMode::Smooth));
    assert_eq!(p.segments[4], PathSegment::Close);
    assert_eq!(p.anchor_modes.keys().copied().collect::<Vec<_>>(), vec![0, 6]);
    assert_eq!(p.anchor_mode(3), Some(AnchorMode::Smooth));
    assert_eq!(p.anchor_mode(6), Some(AnchorMode::Symmetric));
    assert!(p.validate_anchor_modes());
    assert!(p.delete_anchor(0));
    assert_eq!(p.anchor_modes.keys().copied().collect::<Vec<_>>(), vec![5]);
    assert_eq!(p.anchor_mode(5), Some(AnchorMode::Symmetric));
    assert!(p.validate_anchor_modes());
}

#[test]
fn vector_subdivision_remaps_modes_and_preserves_geometry_when_symmetry_relaxes() {
    let mut p = cubic();
    p.segments.extend([
        PathSegment::MoveTo { point: point(400.0, 400.0) },
        PathSegment::LineTo { point: point(500.0, 400.0) },
    ]);
    assert!(p.convert_anchor(1, AnchorMode::Symmetric));
    assert!(p.convert_anchor(4, AnchorMode::Smooth));
    let original = p.clone();
    let split = 0.23;
    assert_eq!(p.split_segment(1, split), Some(PathPointId::anchor(1)));
    assert_eq!(p.anchor_mode(1), Some(AnchorMode::Smooth));
    assert_eq!(p.anchor_mode(2), Some(AnchorMode::Smooth));
    assert_eq!(p.anchor_mode(5), Some(AnchorMode::Smooth));
    assert_eq!(p.anchor_modes.len(), 3);
    let middle = p.point_position(PathPointId::anchor(1)).unwrap();
    for sample in 0..=40 {
        let t = sample as f64 / 40.0;
        close(evaluate(Point::default(), &p.segments[1], t), evaluate(Point::default(), &original.segments[1], t * split));
        close(evaluate(middle, &p.segments[2], t), evaluate(Point::default(), &original.segments[1], split + t * (1.0 - split)));
    }
    assert_eq!(p.segments[3..], original.segments[2..]);
    assert!(p.validate_anchor_modes());
}

#[test]
fn vector_delete_remaps_retained_modes_across_contours_and_removes_deleted_modes() {
    let mut p = cubic();
    p.segments.extend([
        PathSegment::MoveTo { point: point(400.0, 400.0) },
        PathSegment::LineTo { point: point(500.0, 400.0) },
        PathSegment::LineTo { point: point(600.0, 500.0) },
    ]);
    assert!(p.convert_anchor(1, AnchorMode::Smooth));
    assert!(p.convert_anchor(2, AnchorMode::Symmetric));
    assert!(p.convert_anchor(4, AnchorMode::Symmetric));
    assert!(p.delete_anchor(1));
    assert_eq!(p.anchor_modes.keys().copied().collect::<Vec<_>>(), vec![1, 3]);
    assert_eq!(p.anchor_mode(1), Some(AnchorMode::Symmetric));
    assert_eq!(p.anchor_mode(3), Some(AnchorMode::Symmetric));
    assert!(p.validate_anchor_modes());
    assert!(p.delete_anchor(0));
    assert_eq!(p.anchor_mode(0), Some(AnchorMode::Symmetric));
    assert!(p.delete_anchor(0));
    assert_eq!(p.anchor_modes.keys().copied().collect::<Vec<_>>(), vec![1]);
    assert_eq!(p.anchor_mode(1), Some(AnchorMode::Symmetric));
    assert!(p.validate_anchor_modes());
}

#[test]
fn vector_split_quadratic_has_independent_constrained_cubic_handles() {
    let mut p = path(vec![
        PathSegment::MoveTo { point: Point::default() },
        PathSegment::QuadraticTo { control: point(100.0, 200.0), point: point(200.0, 0.0) },
    ]);
    assert!(p.split_segment(1, 0.4).is_some());
    assert!(matches!(p.segments[1], PathSegment::CubicTo { .. }));
    assert!(matches!(p.segments[2], PathSegment::CubicTo { .. }));
    let start_control = edit_point(&p, 1, PathPointKind::Control1);
    let end_control = edit_point(&p, 2, PathPointKind::Control2);
    assert!(p.move_edit_point(
        PathPointId { segment: 2, kind: PathPointKind::Control1 },
        point(130.0, 160.0)
    ));
    close(edit_point(&p, 1, PathPointKind::Control1), start_control);
    close(edit_point(&p, 2, PathPointKind::Control2), end_control);
    assert!(p.validate_anchor_modes());
}

#[test]
fn vector_invalid_constraints_and_overflowing_mirror_edits_are_rejected() {
    let mut p = cubic();
    p.anchor_modes.insert(99, AnchorMode::Smooth);
    assert!(!p.validate_anchor_modes());
    p.anchor_modes.clear();
    assert!(p.convert_anchor(1, AnchorMode::Smooth));
    let PathSegment::CubicTo { control1, .. } = &mut p.segments[2] else { unreachable!() };
    *control1 = point(-100.0, -700.0);
    assert!(!p.validate_anchor_modes());

    let mut p = path(vec![
        PathSegment::MoveTo { point: Point::default() },
        PathSegment::CubicTo {
            control1: point(1e307, 0.0), control2: point(5e307, 0.0), point: point(1e308, 0.0)
        },
        PathSegment::CubicTo {
            control1: point(1.5e308, 0.0), control2: point(1.55e308, 0.0), point: point(1.6e308, 0.0)
        },
    ]);
    p.anchor_modes.insert(1, AnchorMode::Symmetric);
    assert!(p.validate_anchor_modes());
    let before = p.clone();
    assert!(!p.move_edit_point(
        PathPointId { segment: 2, kind: PathPointKind::Control1 },
        point(-1e308, 0.0)
    ));
    assert_eq!(p, before);
}

fn transformed_vector_editor() -> (Editor, NodeId) {
    let mut document = Document::new("Vector edits");
    let page = document.pages[0].id;
    let mut parent = Node::new(
        NodeKind::Frame,
        "Transformed frame",
        300.0,
        180.0,
        1000.0,
        800.0,
    );
    parent.rotation = -31.0;
    parent.flip_y = true;
    parent.affine_transform = Some(Transform {
        a: 1.8,
        c: 0.35,
        d: 0.65,
        ..Transform::IDENTITY
    });
    let parent = document.insert_node(page, parent).unwrap();
    let mut vector = Node::new(NodeKind::Vector, "Curve", 30.0, 45.0, 200.0, 120.0);
    vector.rotation = 27.0;
    vector.flip_x = true;
    vector.affine_transform = Some(Transform {
        a: 0.85,
        b: 0.15,
        c: -0.22,
        d: 1.3,
        e: 3.0,
        f: -5.0,
    });
    vector.path = cubic();
    let vector = document.insert_node(parent, vector).unwrap();
    (Editor::new(document), vector)
}

#[test]
fn vector_persistent_constraint_gesture_round_trips_with_one_undo() {
    let (mut editor, id) = transformed_vector_editor();
    let original = editor.document.to_json().unwrap();
    assert!(editor.convert_vector_anchor(id, 1, AnchorMode::Symmetric));
    assert_eq!(editor.history_len(), 1);
    let converted = editor.document.to_json().unwrap();
    let handle = PathPointId {
        segment: 2,
        kind: PathPointKind::Control1,
    };
    let world = editor.document.world_transform(id).unwrap();
    let start = world.apply(editor.document.nodes[&id].path.point_position(handle).unwrap());
    editor.begin_transaction("Drag mirrored tangent");
    for step in 1..=24 {
        assert!(editor.move_vector_point(
            id,
            handle,
            start + point(240.0, -120.0) * (step as f64 / 24.0),
        ));
        assert!(editor.document.nodes[&id].path.validate_anchor_modes());
    }
    assert_eq!(editor.history_len(), 1);
    assert!(editor.commit_transaction());
    assert_eq!(editor.history_len(), 2);
    let committed = editor.document.to_json().unwrap();
    let reloaded = Document::from_json(&committed).unwrap();
    assert_eq!(reloaded.to_json().unwrap(), committed);
    assert_eq!(reloaded.nodes[&id].path.anchor_mode(1), Some(AnchorMode::Symmetric));
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), converted);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), committed);
    assert!(editor.undo());
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), original);
}

#[test]
fn vector_gesture_is_one_undo_and_normalization_preserves_affine_world_geometry() {
    let (mut editor, id) = transformed_vector_editor();
    editor.selection = vec![id];
    let before = editor.document.nodes[&id].clone();
    let transform = editor.document.world_transform(id).unwrap();
    let world_before: Vec<_> = before
        .path
        .edit_points()
        .into_iter()
        .map(|p| (p.id, transform.apply(p.position)))
        .collect();
    let start = transform.apply(point(100.0, 50.0));
    let delta = point(-420.0, 75.0);
    editor.begin_transaction("Drag Bézier anchor");
    for step in 1..=32 {
        assert!(editor.move_vector_point(
            id,
            PathPointId::anchor(1),
            start + delta * (step as f64 / 32.0)
        ));
    }
    assert_eq!(editor.history_len(), 0);
    assert!(editor.commit_transaction());
    assert_eq!(editor.history_len(), 1);
    assert_eq!(editor.undo_label(), Some("Drag Bézier anchor"));
    let after = &editor.document.nodes[&id];
    assert_eq!(after.rotation, before.rotation);
    assert_eq!(after.flip_x, before.flip_x);
    assert_eq!(after.flip_y, before.flip_y);
    assert_eq!(after.affine_transform, before.affine_transform);
    let world = editor.document.world_transform(id).unwrap();
    for (point_id, position) in world_before {
        let attached = matches!(
            (point_id.segment, point_id.kind),
            (1, PathPointKind::Anchor | PathPointKind::Control2) | (2, PathPointKind::Control1)
        );
        close(
            world.apply(after.path.point_position(point_id).unwrap()),
            position + if attached { delta } else { Point::default() },
        );
    }
    let committed = after.clone();
    assert!(editor.undo());
    let mut restored = editor.document.nodes[&id].clone();
    restored.revision = before.revision;
    assert_eq!(restored, before);
    assert_eq!(editor.selection, vec![id]);
    assert!(editor.redo());
    let mut restored = editor.document.nodes[&id].clone();
    restored.revision = committed.revision;
    assert_eq!(restored, committed);
    editor.document.validate().unwrap();
}

#[test]
fn vector_cancel_restores_bounds_path_and_selection_after_multiple_edits() {
    let (mut editor, id) = transformed_vector_editor();
    editor.selection = vec![id];
    let before = editor.document.nodes[&id].clone();
    editor.begin_transaction("Cancelled curve edit");
    assert!(editor.move_vector_point(
        id,
        PathPointId {
            segment: 1,
            kind: PathPointKind::Control1
        },
        point(-2000.0, 3000.0)
    ));
    assert!(editor.split_vector_segment(id, 2, 0.35).is_some());
    assert!(editor.delete_vector_anchor(id, 1));
    editor.cancel_transaction();
    let mut restored = editor.document.nodes[&id].clone();
    restored.revision = before.revision;
    assert_eq!(restored, before);
    assert_eq!(editor.selection, vec![id]);
    assert_eq!(editor.history_len(), 0);
}

#[test]
fn vector_invalid_or_locked_edits_leave_document_and_history_unchanged() {
    let (mut editor, id) = transformed_vector_editor();
    let before = editor.document.clone();
    assert!(!editor.move_vector_point(id, PathPointId::anchor(0), point(f64::INFINITY, 2.0)));
    assert!(!editor.move_vector_point(id, PathPointId::anchor(500), point(1.0, 2.0)));
    assert!(editor.split_vector_segment(id, 1, -0.1).is_none());
    assert!(!editor.delete_vector_anchor(id, 500));
    assert!(!editor.convert_vector_anchor(id, 500, AnchorMode::Smooth));
    assert_eq!(editor.document, before);
    assert_eq!(editor.history_len(), 0);
    let parent = editor.document.nodes[&id].parent.unwrap();
    editor.document.nodes.get_mut(&parent).unwrap().locked = true;
    editor.refresh();
    let before = editor.document.clone();
    assert!(!editor.move_vector_point(id, PathPointId::anchor(0), point(1.0, 2.0)));
    assert!(!editor.delete_vector_anchor(id, 1));
    assert!(editor.split_vector_segment(id, 1, 0.5).is_none());
    assert_eq!(editor.document, before);
    assert_eq!(editor.history_len(), 0);
}

#[test]
fn vector_imported_arcs_quadratics_and_closed_contours_remain_editable_with_history() {
    let source = r##"<svg width="600" height="400" viewBox="10 20 300 200">
      <g transform="translate(20 30) rotate(19) skewX(12)">
        <path id="artwork" d="M10 20 Q60 100 110 20 A40 60 25 0 1 10 20 Z M160 20 C170 60 190 60 200 20"
          fill="#5178ed" fill-rule="evenodd" stroke="#123456" stroke-width="2"/>
      </g></svg>"##;
    let document = Document::from_svg(source, "Imported paths").unwrap();
    let id = document
        .nodes
        .values()
        .find(|n| {
            n.metadata
                .get("svg_id")
                .is_some_and(|value| value == "artwork")
        })
        .unwrap()
        .id;
    let mut editor = Editor::new(document);
    let before = editor.document.nodes[&id].clone();
    let anchor = before
        .path
        .edit_points()
        .into_iter()
        .find(|p| p.id.kind == PathPointKind::Anchor && p.id.segment != 0)
        .unwrap();
    let world = editor
        .document
        .world_transform(id)
        .unwrap()
        .apply(anchor.position);
    assert!(editor.move_vector_point(id, anchor.id, world + point(25.0, -35.0)));
    let changed = &editor.document.nodes[&id];
    close(
        editor
            .document
            .world_transform(id)
            .unwrap()
            .apply(changed.path.point_position(anchor.id).unwrap()),
        world + point(25.0, -35.0),
    );
    assert_eq!(changed.path.fill_rule, FillRule::EvenOdd);
    assert_eq!(changed.fills, before.fills);
    assert_eq!(changed.strokes, before.strokes);
    assert_eq!(changed.metadata, before.metadata);
    assert_eq!(
        changed
            .path
            .segments
            .iter()
            .filter(|segment| matches!(segment, PathSegment::MoveTo { .. }))
            .count(),
        2
    );
    assert_eq!(editor.history_len(), 1);
    assert!(editor.undo());
    assert_eq!(editor.document.nodes[&id].path, before.path);
    assert!(editor.redo());
    editor.document.validate().unwrap();
    let restored = Document::from_json(&editor.document.to_json().unwrap()).unwrap();
    assert_eq!(
        restored.to_json().unwrap(),
        editor.document.to_json().unwrap()
    );
}

#[test]
fn vector_normalization_updates_scene_bounds_for_anchor_edits_outside_old_extents() {
    let mut editor = Editor::default();
    let id = editor.add_node(NodeKind::Vector, 20.0, 30.0, 100.0, 100.0);
    editor.document.nodes.get_mut(&id).unwrap().path = VectorPath::polygon(
        &[point(0.0, 0.0), point(100.0, 0.0), point(100.0, 100.0)],
        false,
    );
    editor.refresh();
    editor.clear_history();
    assert!(editor.move_vector_point(id, PathPointId::anchor(1), point(-500.0, -300.0)));
    let bounds = editor.document.world_bounds(id).unwrap();
    assert!(bounds.inflate(1e-8).contains(point(-500.0, -300.0)));
    assert!(bounds.inflate(1e-8).contains(point(20.0, 30.0)));
    assert!(bounds.inflate(1e-8).contains(point(120.0, 130.0)));
    assert!(editor
        .scene
        .spatial
        .query(Rect::new(-501.0, -301.0, 2.0, 2.0))
        .contains(&id));
}
