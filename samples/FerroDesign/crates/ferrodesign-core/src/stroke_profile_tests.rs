use super::*;
use crate::{Color, VectorNetwork};

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
fn stroke(profile: StrokeWidthProfile) -> Stroke {
    let mut stroke = Stroke::new(Color::rgb(24, 110, 230), 10.0);
    stroke.width_profile = profile;
    stroke
}
fn line() -> VectorPath {
    VectorPath::polygon(&[p(0.0, 0.0), p(100.0, 0.0)], false)
}
fn constant(value: f64) -> StrokeWidthProfile {
    StrokeWidthProfile::Custom {
        points: vec![StrokeWidthPoint::new(0.5, value)],
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn stroke_width_presets_have_the_published_anchors_and_explicit_linear_interpolation() {
    use StrokeWidthProfile::*;
    for (profile, anchors) in [
        (Uniform, vec![(0.0, 1.0), (0.4, 1.0), (1.0, 1.0)]),
        (Wedge, vec![(0.0, 1.0), (0.5, 0.5), (1.0, 0.0)]),
        (Taper, vec![(0.0, 1.0), (0.5, 0.625), (1.0, 0.25)]),
        (
            QuarterTaper,
            vec![
                (0.0, 0.25),
                (0.125, 0.625),
                (0.25, 1.0),
                (0.625, 0.625),
                (1.0, 0.25),
            ],
        ),
        (Eye, vec![(0.0, 0.0), (0.25, 0.5), (0.5, 1.0), (1.0, 0.0)]),
        (
            MirroredTaper,
            vec![(0.0, 0.25), (0.25, 0.625), (0.5, 1.0), (1.0, 0.25)],
        ),
    ] {
        profile.validate().unwrap();
        close(profile.maximum_multiplier(), 1.0);
        for (position, width) in anchors {
            close(profile.sample(position), width);
        }
        close(profile.sample(-10.0), profile.sample(0.0));
        close(profile.sample(10.0), profile.sample(1.0));
        assert!(profile.sample(f64::NAN).is_nan());
    }
}

#[test]
fn stroke_width_custom_points_extend_endpoints_and_reject_invalid_authored_order() {
    let custom = StrokeWidthProfile::Custom {
        points: vec![
            StrokeWidthPoint::new(0.25, 0.5),
            StrokeWidthPoint::new(0.75, 2.0),
        ],
    };
    custom.validate().unwrap();
    close(custom.maximum_multiplier(), 2.0);
    close(custom.sample(0.0), 0.5);
    close(custom.sample(0.5), 1.25);
    close(custom.sample(1.0), 2.0);
    assert!(!constant(1.0).is_uniform());
    close(constant(3.0).sample(0.0), 3.0);
    for points in [
        vec![],
        vec![StrokeWidthPoint::new(-0.1, 1.0)],
        vec![StrokeWidthPoint::new(1.1, 1.0)],
        vec![StrokeWidthPoint::new(0.0, -1.0)],
        vec![StrokeWidthPoint::new(0.0, f64::INFINITY)],
        vec![StrokeWidthPoint::new(f64::NAN, 1.0)],
        vec![
            StrokeWidthPoint::new(0.5, 1.0),
            StrokeWidthPoint::new(0.5, 2.0),
        ],
        vec![
            StrokeWidthPoint::new(1.0, 1.0),
            StrokeWidthPoint::new(0.0, 2.0),
        ],
        (0..=MAX_STROKE_WIDTH_POINTS)
            .map(|i| StrokeWidthPoint::new(i as f64 / MAX_STROKE_WIDTH_POINTS as f64, 1.0))
            .collect(),
    ] {
        let profile = StrokeWidthProfile::Custom { points };
        assert!(profile.validate().is_err());
        assert!(profile.maximum_multiplier().is_nan());
        assert!(profile.sample(0.5).is_nan());
    }
}

#[test]
fn stroke_width_json_keeps_legacy_uniform_defaults_and_custom_authorship() {
    let original = Stroke::new(Color::rgb(4, 5, 6), 7.0);
    let value = serde_json::to_value(&original).unwrap();
    assert!(value.get("width_profile").is_none());
    assert_eq!(serde_json::from_value::<Stroke>(value).unwrap(), original);
    for profile in [
        StrokeWidthProfile::Wedge,
        StrokeWidthProfile::QuarterTaper,
        constant(1.0),
        StrokeWidthProfile::Custom {
            points: vec![
                StrokeWidthPoint::new(0.0, 0.25),
                StrokeWidthPoint::new(0.3, 1.5),
                StrokeWidthPoint::new(1.0, 0.0),
            ],
        },
    ] {
        let stroke = stroke(profile);
        let json = serde_json::to_string(&stroke).unwrap();
        assert!(json.contains("width_profile"));
        assert_eq!(serde_json::from_str::<Stroke>(&json).unwrap(), stroke);
    }
}

#[test]
fn stroke_width_line_wedge_and_eye_have_real_nonuniform_silhouettes() {
    let path = line();
    let before = path.clone();
    let wedge =
        variable_width_path_outline(&path, &stroke(StrokeWidthProfile::Wedge), false).unwrap();
    assert!(wedge.contains(p(1.0, 4.8)));
    assert!(!wedge.contains(p(99.0, 1.0)));
    assert!(wedge.contains(p(50.0, 2.4)));
    assert!(!wedge.contains(p(50.0, 2.6)));
    let eye = variable_width_path_outline(&path, &stroke(StrokeWidthProfile::Eye), false).unwrap();
    assert!(eye.contains(p(50.0, 4.9)));
    assert!(!eye.contains(p(2.0, 1.0)));
    assert!(!eye.contains(p(98.0, 1.0)));
    assert_eq!(eye.fill_rule, FillRule::NonZero);
    assert_eq!(path, before);
}

#[test]
fn stroke_width_caps_use_the_endpoint_width_and_do_not_extend_zero_tips() {
    for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
        let mut style = stroke(StrokeWidthProfile::Wedge);
        style.cap = cap;
        let outline = variable_width_path_outline(&line(), &style, false).unwrap();
        assert_eq!(outline.contains(p(-4.0, 0.0)), cap != StrokeCap::Butt);
        assert_eq!(outline.contains(p(-4.0, 4.0)), cap == StrokeCap::Square);
        assert!(
            !outline.contains(p(100.1, 0.0)),
            "zero-width tips cannot acquire a cap"
        );
        let bounds = outline.tight_bounds().unwrap();
        close(bounds.right(), 100.0);
        if cap == StrokeCap::Square {
            close(bounds.x, -5.0);
        }
        if cap == StrokeCap::Butt {
            close(bounds.x, 0.0);
        }
    }
}

#[test]
fn stroke_width_miter_round_and_bevel_joins_remain_distinct() {
    let path = VectorPath::polygon(&[p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0)], false);
    for join in [StrokeJoin::Miter, StrokeJoin::Round, StrokeJoin::Bevel] {
        let mut style = stroke(constant(1.0));
        style.width = 20.0;
        style.join = join;
        let outline = variable_width_path_outline(&path, &style, false).unwrap();
        assert_eq!(outline.contains(p(109.0, -9.0)), join == StrokeJoin::Miter);
        assert_eq!(outline.contains(p(107.0, -7.0)), join != StrokeJoin::Bevel);
        assert!(outline.contains(p(101.0, 1.0)));
    }
    let path = VectorPath::polygon(&[p(0.0, 0.0), p(100.0, 0.0), p(0.0, 1.0)], false);
    let mut style = stroke(constant(1.0));
    style.width = 20.0;
    let outline = variable_width_path_outline(&path, &style, false).unwrap();
    assert!(
        outline.tight_bounds().unwrap().right() < 111.0,
        "a nearly reversed miter must bevel at the native 10x limit"
    );
}

#[test]
fn stroke_width_curve_subdivision_handles_cubics_quadratics_and_retains_source_commands() {
    for path in [
        VectorPath {
            segments: vec![
                PathSegment::MoveTo { point: p(0.0, 0.0) },
                PathSegment::CubicTo {
                    control1: p(0.0, 100.0),
                    control2: p(100.0, 100.0),
                    point: p(100.0, 0.0),
                },
            ],
            ..VectorPath::default()
        },
        VectorPath {
            segments: vec![
                PathSegment::MoveTo { point: p(0.0, 0.0) },
                PathSegment::QuadraticTo {
                    control: p(50.0, 150.0),
                    point: p(100.0, 0.0),
                },
            ],
            ..VectorPath::default()
        },
    ] {
        let before = path.clone();
        let outline =
            variable_width_path_outline(&path, &stroke(StrokeWidthProfile::Eye), false).unwrap();
        assert!(outline.contains(p(50.0, 75.0)));
        assert!(outline.contains(p(50.0, 79.7)));
        assert!(!outline.contains(p(50.0, 80.2)));
        assert!(outline.segments.len() < MAX_OUTPUT_COMMANDS);
        assert!(outline.segments.iter().all(|segment| matches!(
            segment,
            PathSegment::MoveTo { .. } | PathSegment::LineTo { .. } | PathSegment::Close
        )));
        assert_eq!(path, before);
    }
}

#[test]
fn stroke_width_narrow_custom_features_are_inserted_between_geometry_samples() {
    let profile = StrokeWidthProfile::Custom {
        points: vec![
            StrokeWidthPoint::new(0.0, 0.0),
            StrokeWidthPoint::new(0.499999, 0.0),
            StrokeWidthPoint::new(0.5, 2.0),
            StrokeWidthPoint::new(0.500001, 0.0),
            StrokeWidthPoint::new(1.0, 0.0),
        ],
    };
    let outline = variable_width_path_outline(&line(), &stroke(profile), false).unwrap();
    assert!(outline.contains(p(50.0, 9.5)));
    assert!(!outline.contains(p(49.99, 0.1)));
    close(outline.tight_bounds().unwrap().height, 20.0);
}

#[test]
fn stroke_width_profiles_repeat_per_contour_and_nonzero_pieces_preserve_holes() {
    let mut path = line();
    path.segments
        .extend(VectorPath::polygon(&[p(200.0, 0.0), p(300.0, 0.0)], false).segments);
    let outline =
        variable_width_path_outline(&path, &stroke(StrokeWidthProfile::Eye), false).unwrap();
    for x in [50.0, 250.0] {
        assert!(outline.contains(p(x, 4.9)));
    }
    for x in [1.0, 201.0, 150.0] {
        assert!(!outline.contains(p(x, 1.0)));
    }
    let mut square = VectorPath::polygon(
        &[p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)],
        true,
    );
    square.fill_rule = FillRule::EvenOdd;
    let outline = variable_width_path_outline(&square, &stroke(constant(1.0)), false).unwrap();
    assert_eq!(outline.fill_rule, FillRule::NonZero);
    assert!(!outline.contains(p(50.0, 50.0)));
    assert!(outline.contains(p(50.0, 2.0)));
    assert!(outline.contains(p(-4.0, -4.0)));
}

#[test]
fn stroke_width_inside_outside_return_one_doubled_band_and_open_paths_stay_centered() {
    let node = Node::new(NodeKind::Rectangle, "Box", 20.0, 30.0, 100.0, 100.0);
    let mut style = stroke(constant(1.0));
    let center = variable_width_outline(&node, &style).unwrap();
    close(center.tight_bounds().unwrap().x, -5.0);
    for align in [StrokeAlign::Inside, StrokeAlign::Outside] {
        style.align = align;
        let band = variable_width_outline(&node, &style).unwrap();
        close(band.tight_bounds().unwrap().x, -10.0);
        close(band.tight_bounds().unwrap().right(), 110.0);
        assert!(!band.contains(p(50.0, 50.0)));
    }
    let node = Node::new(NodeKind::Line, "Open", 20.0, 30.0, 100.0, 0.0);
    let outline = variable_width_outline(&node, &style).unwrap();
    close(outline.tight_bounds().unwrap().height, 10.0);
    assert!(variable_width_path_outline(&line(), &style, true).is_err());
}

#[test]
fn stroke_width_closed_seams_allow_unequal_endpoint_widths_and_affine_transforms_stay_external() {
    let mut node = Node::new(NodeKind::Rectangle, "Closed", 0.0, 0.0, 100.0, 80.0);
    let style = stroke(StrokeWidthProfile::Wedge);
    let outline = variable_width_outline(&node, &style).unwrap();
    assert!(!outline.segments.is_empty());
    assert!(!outline.contains(p(50.0, 40.0)));
    node.x = 1e6;
    node.y = -500.0;
    node.rotation = 37.0;
    node.flip_x = true;
    let before = node.clone();
    assert_eq!(variable_width_outline(&node, &style).unwrap(), outline);
    assert_eq!(node, before);
}

#[test]
fn stroke_width_ellipse_polygon_and_smoothed_box_share_the_prepared_path_api() {
    for kind in [
        NodeKind::Ellipse,
        NodeKind::Polygon,
        NodeKind::Star,
        NodeKind::Rectangle,
        NodeKind::Frame,
        NodeKind::Component,
        NodeKind::Instance,
        NodeKind::Section,
    ] {
        let mut node = Node::new(kind, "Primitive", 0.0, 0.0, 100.0, 80.0);
        node.corner_radius = 15.0;
        node.corner_smoothing = 0.7;
        let style = stroke(StrokeWidthProfile::MirroredTaper);
        let outline = variable_width_outline(&node, &style).unwrap();
        assert!(!outline.segments.is_empty(), "{kind:?}");
        assert!(outline.tight_bounds().unwrap().is_finite());
        if kind == NodeKind::Rectangle {
            assert_eq!(
                outline,
                variable_width_path_outline(&crate::corner_path(&node), &style, false).unwrap()
            );
        }
    }
}

#[test]
fn stroke_width_support_rejects_real_branches_but_ignores_nonstroked_fill_edges() {
    let mut network = VectorNetwork::default();
    let center = network.add_vertex(p(50.0, 50.0)).unwrap();
    let leaves = [p(0.0, 0.0), p(100.0, 0.0), p(50.0, 100.0)]
        .map(|point| network.add_vertex(point).unwrap());
    let edges = leaves.map(|leaf| network.connect(center, leaf).unwrap());
    let mut node = Node::new(NodeKind::Vector, "Branch", 0.0, 0.0, 100.0, 100.0);
    node.vector_network = Some(network);
    let style = stroke(StrokeWidthProfile::Eye);
    let before = node.clone();
    assert!(validate_variable_width_support(&node, &style).is_err());
    assert!(variable_width_outline(&node, &style).is_err());
    assert_eq!(node, before);
    assert!(validate_variable_width_support(&node, &stroke(StrokeWidthProfile::Uniform)).is_ok());
    node.vector_network
        .as_mut()
        .unwrap()
        .edges
        .get_mut(&edges[2])
        .unwrap()
        .stroke = false;
    validate_variable_width_support(&node, &style).unwrap();
    assert!(!variable_width_outline(&node, &style)
        .unwrap()
        .segments
        .is_empty());
}

#[test]
fn stroke_width_unsupported_combinations_fail_without_uniform_substitution() {
    let mut node = Node::new(NodeKind::Line, "Open", 0.0, 0.0, 100.0, 0.0);
    let style = stroke(StrokeWidthProfile::Eye);
    for kind in [NodeKind::Text, NodeKind::Boolean, NodeKind::Group] {
        node.kind = kind;
        assert!(validate_variable_width_support(&node, &style).is_err());
        assert!(variable_width_outline(&node, &style).is_err());
    }
    node.kind = NodeKind::Line;
    for arrow in [false, true] {
        let mut invalid = style.clone();
        if arrow {
            invalid.cap = StrokeCap::Arrow;
        } else {
            invalid.dash = vec![4.0, 2.0];
        }
        assert!(validate_variable_width_support(&node, &invalid).is_err());
        assert!(variable_width_path_outline(&line(), &invalid, false).is_err());
        invalid.width_profile = StrokeWidthProfile::Uniform;
        validate_variable_width_support(&node, &invalid).unwrap();
    }
}

#[test]
fn stroke_width_resource_and_coordinate_limits_fail_explicitly() {
    let too_many = VectorPath {
        segments: vec![PathSegment::MoveTo { point: p(0.0, 0.0) }; MAX_INPUT_COMMANDS + 1],
        ..VectorPath::default()
    };
    assert!(
        variable_width_path_outline(&too_many, &stroke(StrokeWidthProfile::Eye), false)
            .unwrap_err()
            .to_string()
            .contains("work limit")
    );
    let points: Vec<_> = (0..30_000)
        .map(|index| p(index as f64, (index % 2) as f64))
        .collect();
    let mut style = stroke(constant(1.0));
    style.join = StrokeJoin::Round;
    assert!(
        variable_width_path_outline(&VectorPath::polygon(&points, false), &style, false)
            .unwrap_err()
            .to_string()
            .contains("output limit")
    );
    let imprecise = VectorPath::polygon(&[p(1e16, 0.0), p(1e16 + 100.0, 0.0)], false);
    assert!(variable_width_path_outline(&imprecise, &style, false)
        .unwrap_err()
        .to_string()
        .contains("precision"));
    for width in [-1.0, f64::INFINITY, f64::NAN] {
        style.width = width;
        assert!(variable_width_path_outline(&line(), &style, false).is_err());
    }
    style.width = f64::MAX;
    style.width_profile = constant(2.0);
    assert!(variable_width_path_outline(&line(), &style, false).is_err());
}

#[test]
fn stroke_width_small_geometry_and_zero_profiles_are_finite_and_malformed_paths_fail() {
    for scale in [1e-9, 1e-6, 1.0, 1e6] {
        let mut style = stroke(StrokeWidthProfile::Eye);
        style.width *= scale;
        let path = VectorPath::polygon(&[p(0.0, 0.0), p(100.0 * scale, 0.0)], false);
        let outline = variable_width_path_outline(&path, &style, false).unwrap();
        let bounds = outline.tight_bounds().unwrap();
        assert!(bounds.is_finite());
        assert!((bounds.height / scale - 10.0).abs() < 1e-8);
    }
    assert!(
        variable_width_path_outline(&line(), &stroke(constant(0.0)), false)
            .unwrap()
            .segments
            .is_empty()
    );
    assert!(variable_width_path_outline(
        &VectorPath::default(),
        &stroke(StrokeWidthProfile::Eye),
        false
    )
    .unwrap()
    .segments
    .is_empty());
    for segments in [
        vec![PathSegment::LineTo {
            point: p(100.0, 0.0),
        }],
        vec![PathSegment::Close],
        vec![
            PathSegment::MoveTo { point: p(0.0, 0.0) },
            PathSegment::Close,
            PathSegment::LineTo {
                point: p(100.0, 0.0),
            },
        ],
        vec![PathSegment::MoveTo {
            point: p(f64::NAN, 0.0),
        }],
    ] {
        let path = VectorPath {
            segments,
            ..VectorPath::default()
        };
        assert!(
            variable_width_path_outline(&path, &stroke(StrokeWidthProfile::Eye), false).is_err()
        );
    }
}

#[test]
fn stroke_width_unrepresentable_custom_knots_fail_instead_of_dropping_the_feature() {
    let path = VectorPath::polygon(&[p(1e12, 0.0), p(1e12 + 100.0, 0.0)], false);
    let profile = StrokeWidthProfile::Custom {
        points: vec![
            StrokeWidthPoint::new(0.5, 0.0),
            StrokeWidthPoint::new(0.5 + f64::EPSILON, 2.0),
        ],
    };
    assert!(variable_width_path_outline(&path, &stroke(profile), false)
        .unwrap_err()
        .to_string()
        .contains("precision"));
}

#[test]
fn stroke_width_handle_sampling_shares_curve_lengths_knots_and_local_normals() {
    let mut node = Node::new(NodeKind::Vector, "Curve", 0.0, 0.0, 100.0, 100.0);
    node.path = VectorPath {
        segments: vec![
            PathSegment::MoveTo { point: p(0.0, 0.0) },
            PathSegment::CubicTo {
                control1: p(0.0, 100.0),
                control2: p(100.0, 100.0),
                point: p(100.0, 0.0),
            },
        ],
        ..VectorPath::default()
    };
    let style = stroke(StrokeWidthProfile::Eye);
    let before = node.clone();
    let contours = stroke_width_contours(&node, &style).unwrap();
    assert_eq!(contours.len(), 1);
    let contour = &contours[0];
    assert!(!contour.is_closed());
    assert!((contour.length() - 200.0).abs() < 0.05);
    assert!(contour.point_count() > 3 && contour.point_count() < MAX_FLAT_POINTS);
    let center = contour.sample(0.5).unwrap();
    close(center.point.x, 50.0);
    close(center.point.y, 75.0);
    assert!(center.tangent.distance(p(1.0, 0.0)) < 0.01);
    assert!(center.normal.distance(p(0.0, 1.0)) < 0.01);
    close(center.normal.distance(p(0.0, 0.0)), 1.0);
    close(dot(center.tangent, center.normal), 0.0);
    let (nearest, distance) = contour.nearest(p(50.0, 80.0)).unwrap();
    assert!((nearest.position - 0.5).abs() < 0.001);
    assert!((distance - 5.0).abs() < 0.05);
    assert_eq!(contour.sample(-1.0), contour.sample(0.0));
    assert_eq!(contour.sample(2.0), contour.sample(1.0));
    assert!(contour.sample(f64::NAN).is_none());
    assert!(contour.nearest(p(f64::INFINITY, 0.0)).is_none());
    assert_eq!(node, before);
    let preset = StrokeWidthProfile::QuarterTaper;
    let custom = StrokeWidthProfile::Custom {
        points: preset.control_points().to_vec(),
    };
    for i in 0..=100 {
        close(
            custom.sample(i as f64 / 100.0),
            preset.sample(i as f64 / 100.0),
        );
    }
}

#[test]
fn stroke_width_handle_projection_uses_transformed_distance_under_shear_and_reflection() {
    let mut node = Node::new(NodeKind::Vector, "L", 0.0, 0.0, 100.0, 100.0);
    node.path = VectorPath::polygon(&[p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0)], false);
    let contours = stroke_width_contours(&node, &stroke(StrokeWidthProfile::Eye)).unwrap();
    let contour = &contours[0];
    let transform = Transform {
        a: 0.1,
        b: 0.0,
        c: 5.0,
        d: 1.0,
        e: 7.0,
        f: -3.0,
    };
    let (location, distance) = contour
        .nearest_transformed(p(17.0, 2.0), transform)
        .unwrap();
    close(location.position, 0.5 + 0.5 / 520.0);
    close(location.point.x, 100.0);
    close(location.point.y, 100.0 / 520.0);
    close(location.normal.x, -1.0);
    close(location.normal.y, 0.0);
    close(distance, 5.0 * (1.0_f64 - 1.0 / 26.0).sqrt());
    // Inverse-transforming the query and using a local Euclidean nearest point
    // picks the wrong leg; the public transformed projection must not do that.
    close(contour.nearest(p(-150.0, 5.0)).unwrap().0.position, 0.0);
    let transform = Transform {
        a: -2.0,
        b: 0.0,
        c: 0.0,
        d: 0.5,
        e: 0.0,
        f: 0.0,
    };
    let (location, distance) = contour
        .nearest_transformed(p(-100.0, -20.0), transform)
        .unwrap();
    close(location.position, 0.25);
    close(location.point.x, 50.0);
    close(distance, 20.0);
    let collapsed = Transform {
        a: 0.0,
        b: 0.0,
        c: 0.0,
        d: 0.0,
        e: 1.0,
        f: 2.0,
    };
    assert!(contour
        .nearest_transformed(p(0.0, 0.0), collapsed)
        .is_none());
}
