use super::*;
use crate::{Document, Fill, Style, StyleValue, Transform, Variable, VariableValue};
use std::collections::BTreeMap;

fn gradient() -> LinearGradientEdit {
    LinearGradientEdit {
        start: Point::new(0.1, 0.5),
        end: Point::new(0.9, 0.5),
        stops: vec![
            GradientStop {
                offset: 0.0,
                color: Color::BLACK,
            },
            GradientStop {
                offset: 1.0,
                color: Color::WHITE,
            },
        ],
    }
}

fn fixture() -> (Editor, NodeId) {
    let mut document = Document::new("Native linear-gradient authoring");
    let mut node = Node::new(NodeKind::Rectangle, "Gradient", 20.0, 30.0, 200.0, 100.0);
    node.fills = vec![
        Fill::solid(Color::rgb(40, 70, 160)),
        Fill {
            paint: gradient().paint(),
            opacity: 0.75,
            visible: true,
        },
    ];
    node.fill_style = Some(900);
    // Deliberately unresolved imported metadata: applying this color variable
    // would replace the whole fill stack. A real gradient edit must detach it;
    // no-op edits and undo must preserve the exact portable snapshot.
    node.variable_bindings.insert("fill".into(), 901);
    document.styles.insert(
        900,
        Style {
            id: 900,
            name: "Two fills".into(),
            description: String::new(),
            value: StyleValue::Fill {
                fills: node.fills.clone(),
            },
        },
    );
    document.variables.insert(
        901,
        Variable {
            id: 901,
            name: "Base".into(),
            collection: "Colors".into(),
            values: BTreeMap::from([(
                "Default".into(),
                VariableValue::Color(Color::rgb(40, 70, 160)),
            )]),
        },
    );
    document.variable_mode = "Default".into();
    let id = document.insert_node(document.pages[0].id, node).unwrap();
    document.next_id = 902;
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    (editor, id)
}

#[test]
fn linear_gradient_changes_only_one_fill_and_detaches_metadata_only_on_change() {
    let (mut editor, id) = fixture();
    let before = editor.document.to_json().unwrap();
    let first = editor.document.nodes[&id].fills[0].clone();
    let mut value = gradient();
    assert!(!editor.set_linear_gradient(id, 1, value.clone()).unwrap());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.history_len(), 0);
    value.end = Point::new(0.75, 0.8);
    assert!(editor.set_linear_gradient(id, 1, value).unwrap());
    let node = &editor.document.nodes[&id];
    assert_eq!(node.fills[0], first);
    assert_eq!(node.fills[1].opacity, 0.75);
    assert_eq!(node.fill_style, None);
    assert!(!node.variable_bindings.contains_key("fill"));
    assert!(node.overridden_properties.contains("fills"));
    assert_eq!(
        (node.width, node.height, node.x, node.y),
        (200.0, 100.0, 20.0, 30.0)
    );
    let after = editor.document.to_json().unwrap();
    assert_eq!(editor.history_len(), 1);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(!editor.set_linear_gradient(id, 1, gradient()).unwrap());
    assert!(editor.can_redo());
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
    editor.document.validate().unwrap();
}

#[test]
fn invalid_gradient_edits_fail_atomically_and_locked_targets_retain_redo() {
    let (mut editor, id) = fixture();
    let mut changed = gradient();
    changed.start.x = 0.2;
    editor.set_linear_gradient(id, 1, changed.clone()).unwrap();
    assert!(editor.undo());
    let before = editor.document.to_json().unwrap();
    assert!(editor
        .set_linear_gradient(u64::MAX, 1, changed.clone())
        .is_err());
    assert!(editor.set_linear_gradient(id, 0, changed.clone()).is_err());
    assert!(editor.set_linear_gradient(id, 9, changed.clone()).is_err());
    for property in 0..5 {
        let mut invalid = gradient();
        match property {
            0 => invalid.start.x = f64::NAN,
            1 => invalid.end.y = 1000.1,
            2 => invalid.stops[0].offset = -0.1,
            3 => {
                invalid.stops.pop();
            }
            _ => invalid.stops = vec![invalid.stops[0].clone(); MAX_CANVAS_GRADIENT_STOPS + 1],
        }
        assert!(editor.set_linear_gradient(id, 1, invalid).is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(editor.can_redo());
    }
    editor.document.nodes.get_mut(&id).unwrap().locked = true;
    editor.refresh();
    let locked = editor.document.to_json().unwrap();
    assert!(!editor.set_linear_gradient(id, 1, changed).unwrap());
    assert_eq!(editor.document.to_json().unwrap(), locked);
    assert!(editor.can_redo());
}

#[test]
fn canvas_gradient_bounds_match_only_supported_box_geometries_and_affine_projection() {
    for kind in [
        NodeKind::Rectangle,
        NodeKind::Ellipse,
        NodeKind::Frame,
        NodeKind::Component,
        NodeKind::Instance,
        NodeKind::Section,
    ] {
        let node = Node::new(kind, "Bounds", 50.0, 70.0, 200.0, 100.0);
        let bounds = linear_gradient_canvas_bounds(&node).unwrap();
        assert_eq!(bounds, Rect::new(0.0, 0.0, 200.0, 100.0));
        assert_eq!(
            gradient().local_endpoints(bounds),
            [Point::new(20.0, 50.0), Point::new(180.0, 50.0)]
        );
    }
    for kind in [
        NodeKind::Vector,
        NodeKind::Polygon,
        NodeKind::Star,
        NodeKind::Text,
        NodeKind::Group,
        NodeKind::Boolean,
        NodeKind::Line,
    ] {
        assert!(linear_gradient_canvas_bounds(&Node::new(
            kind,
            "Unsupported",
            0.0,
            0.0,
            100.0,
            100.0
        ))
        .is_none());
    }
    let mut node = Node::new(NodeKind::Rectangle, "Empty", 0.0, 0.0, 0.0, 100.0);
    assert!(linear_gradient_canvas_bounds(&node).is_none());
    node.width = f64::INFINITY;
    assert!(linear_gradient_canvas_bounds(&node).is_none());
    let transform = Transform {
        a: -1.2,
        b: 0.3,
        c: 0.4,
        d: 0.9,
        e: 20.0,
        f: 30.0,
    };
    let axis = transform.apply_vector(Point::new(160.0, 0.0));
    assert!((gradient_offset_delta(axis * 0.25, axis).unwrap() - 0.25).abs() < 1e-12);
    assert_eq!(
        gradient_offset_delta(Point::new(1.0, 0.0), Point::default()),
        None
    );
    assert_eq!(
        gradient_offset_delta(Point::new(f64::INFINITY, 0.0), axis),
        None
    );
}

#[test]
fn gradient_stop_authoring_preserves_source_order_ties_and_limits() {
    let mut value = gradient();
    value.stops = vec![
        GradientStop {
            offset: 1.0,
            color: Color::WHITE,
        },
        GradientStop {
            offset: 0.5,
            color: Color::rgb(255, 0, 0),
        },
        GradientStop {
            offset: 0.0,
            color: Color::BLACK,
        },
        GradientStop {
            offset: 0.5,
            color: Color::rgb(0, 0, 255),
        },
    ];
    let original = value.stops.clone();
    let new = value.add_stop(0.5).unwrap();
    assert_eq!(new, original.len());
    assert_eq!(&value.stops[..new], &original);
    assert_eq!(
        value.stops[new].color,
        Color::rgb(0, 0, 255),
        "Stable equal-offset ordering chooses the right-hand stop"
    );
    assert!(value.move_stop(1, 0.8).unwrap());
    assert_eq!(value.stops[1].color, Color::rgb(255, 0, 0));
    assert_eq!(value.stops[0], original[0]);
    assert_eq!(value.stops[2], original[2]);
    let before = value.clone();
    assert!(value.move_stop(1, f64::NAN).is_err());
    assert!(value.add_stop(-0.1).is_err());
    assert!(value.remove_stop(99).is_err());
    assert_eq!(value, before);
    let mut two = gradient();
    assert!(two.remove_stop(0).is_err());
    assert_eq!(two, gradient());
    let index = two.add_stop(0.25).unwrap();
    assert_eq!(two.stops[index].color, Color::rgb(64, 64, 64));
    two.remove_stop(index).unwrap();
    assert_eq!(two, gradient());
}

#[test]
fn gradient_translation_clamps_endpoints_together_without_changing_extent() {
    let value = gradient();
    let moved = value.translated(Point::new(0.2, -0.15)).unwrap();
    assert_eq!(moved.start, Point::new(0.30000000000000004, 0.35));
    assert_eq!(moved.end, Point::new(1.1, 0.35));
    let bounded = value.translated(Point::new(10000.0, -10000.0)).unwrap();
    assert_eq!(bounded.end.x, 1000.0);
    assert_eq!(bounded.start.y, -1000.0);
    assert!(((bounded.end.x - bounded.start.x) - (value.end.x - value.start.x)).abs() < 1e-10);
    assert_eq!(bounded.stops, value.stops);
    assert!(value.translated(Point::new(f64::NAN, 0.0)).is_none());
    for invalid in [
        LinearGradientEdit {
            start: Point::new(f64::NAN, 0.5),
            ..value.clone()
        },
        LinearGradientEdit {
            start: Point::new(-1001.0, 0.5),
            end: Point::new(1001.0, 0.5),
            ..value.clone()
        },
        LinearGradientEdit {
            stops: Vec::new(),
            ..value.clone()
        },
    ] {
        assert!(
            invalid.translated(Point::new(1.0, 1.0)).is_none(),
            "Invalid public snapshots cannot panic in clamp"
        );
    }
    let mut node = Node::new(
        NodeKind::Rectangle,
        "Oversized imported paint",
        0.0,
        0.0,
        100.0,
        100.0,
    );
    node.fills = vec![Fill {
        paint: Paint::LinearGradient {
            start: value.start,
            end: value.end,
            stops: vec![value.stops[0].clone(); MAX_CANVAS_GRADIENT_STOPS + 1],
        },
        opacity: 1.0,
        visible: true,
    }];
    assert!(LinearGradientEdit::from_node(&node, 0).is_err());
}

fn native_gradient(geometry: CanvasGradientGeometry) -> CanvasGradientEdit {
    CanvasGradientEdit {
        geometry,
        stops: gradient().stops,
    }
}

#[test]
fn radial_canvas_geometry_uses_both_native_relative_radii_and_translates_without_radius_change() {
    let bounds = Rect::new(5.0, 7.0, 200.0, 80.0);
    let value = native_gradient(CanvasGradientGeometry::Radial {
        center: Point::new(0.3, 0.6),
        radius: 0.25,
    });
    assert_eq!(
        value.local_endpoints(bounds),
        [Point::new(65.0, 55.0), Point::new(115.0, 55.0)]
    );
    let quarter = value.local_ring(bounds, 0.25).unwrap();
    assert!((quarter.x - 65.0).abs() < 1e-9);
    assert!((quarter.y - 75.0).abs() < 1e-9);
    let moved = value
        .moved_endpoint(true, Point::new(10.0, 32.0), bounds)
        .unwrap();
    assert_eq!(
        moved.geometry,
        CanvasGradientGeometry::Radial {
            center: Point::new(0.3, 0.6),
            radius: 0.5
        }
    );
    let translated = value.translated(Point::new(-0.25, 0.125)).unwrap();
    let CanvasGradientGeometry::Radial { center, radius } = translated.geometry else {
        panic!()
    };
    assert!((center.x - 0.05).abs() < 1e-9);
    assert_eq!(center.y, 0.725);
    assert_eq!(radius, 0.25);
    assert_eq!(translated.stops, value.stops);
}

#[test]
fn angular_canvas_geometry_uses_local_pixels_clockwise_angles_and_no_authored_radius() {
    let bounds = Rect::new(10.0, 20.0, 240.0, 80.0);
    let value = native_gradient(CanvasGradientGeometry::Angular {
        center: Point::new(0.25, 0.5),
        angle: -90.0,
    });
    let axis = value.local_endpoints(bounds);
    assert_eq!(axis[0], Point::new(70.0, 60.0));
    assert!((axis[1].x - 30.0).abs() < 1e-9);
    assert!((axis[1].y - 60.0).abs() < 1e-9);
    let stop = value.local_stop(bounds, 0.25);
    assert!((stop.x - 70.0).abs() < 1e-9);
    assert!((stop.y - 20.0).abs() < 1e-9);
    let moved = value
        .moved_endpoint(true, Point::new(40.0, -40.0), bounds)
        .unwrap();
    let CanvasGradientGeometry::Angular { angle, center } = moved.geometry else {
        panic!()
    };
    assert!(angle.abs() < 1e-9);
    assert_eq!(center, Point::new(0.25, 0.5));
    assert!(matches!(moved.paint(), Paint::AngularGradient { .. }));
}

#[test]
fn signed_gradient_angles_cross_the_seam_and_survive_large_direction_magnitudes() {
    let ray = |degrees: f64, scale: f64| {
        let angle = degrees.to_radians();
        Point::new(angle.sin() * scale, -angle.cos() * scale)
    };
    let forward = gradient_angle_delta(ray(359.0, 1e200), ray(1.0, 1e200))
        .unwrap()
        .to_degrees();
    let backward = gradient_angle_delta(ray(1.0, 1e200), ray(359.0, 1e200))
        .unwrap()
        .to_degrees();
    assert!((forward - 2.0).abs() < 1e-9);
    assert!((backward + 2.0).abs() < 1e-9);
    assert_eq!(gradient_angle_delta(Point::default(), ray(0.0, 1.0)), None);
    assert_eq!(
        gradient_angle_delta(ray(0.0, 1.0), Point::new(f64::NAN, 0.0)),
        None
    );
}

#[test]
fn all_native_canvas_gradient_kinds_preserve_history_style_binding_and_other_fills() {
    for geometry in [
        CanvasGradientGeometry::Linear {
            start: Point::new(0.1, 0.5),
            end: Point::new(0.9, 0.5),
        },
        CanvasGradientGeometry::Radial {
            center: Point::new(0.35, 0.6),
            radius: 0.4,
        },
        CanvasGradientGeometry::Angular {
            center: Point::new(0.35, 0.6),
            angle: 713.25,
        },
    ] {
        let (mut editor, id) = fixture();
        let original = native_gradient(geometry);
        editor.document.nodes.get_mut(&id).unwrap().fills[1].paint = original.paint();
        editor.refresh();
        let before = editor.document.to_json().unwrap();
        let unaffected = editor.document.nodes[&id].fills[0].clone();
        let changed = original.translated(Point::new(0.1, -0.2)).unwrap();
        assert!(editor.set_canvas_gradient(id, 1, changed.clone()).unwrap());
        assert_eq!(editor.document.nodes[&id].fills[0], unaffected);
        assert_eq!(editor.document.nodes[&id].fill_style, None);
        assert!(!editor.document.nodes[&id]
            .variable_bindings
            .contains_key("fill"));
        assert!(editor.document.nodes[&id]
            .overridden_properties
            .contains("fills"));
        assert_eq!(editor.history_len(), 1);
        let after = editor.document.to_json().unwrap();
        assert!(editor.undo());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(!editor.set_canvas_gradient(id, 1, original).unwrap());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(editor.can_redo());
        assert!(editor.redo());
        assert_eq!(editor.document.to_json().unwrap(), after);
        assert_eq!(
            CanvasGradientEdit::from_node(&editor.document.nodes[&id], 1).unwrap(),
            changed
        );
    }
}

#[test]
fn invalid_radial_angular_and_stale_kind_gradient_edits_are_atomic_with_real_redo() {
    let (mut editor, id) = fixture();
    let original = native_gradient(CanvasGradientGeometry::Radial {
        center: Point::new(0.5, 0.5),
        radius: 0.5,
    });
    editor.document.nodes.get_mut(&id).unwrap().fills[1].paint = original.paint();
    editor.refresh();
    editor
        .set_canvas_gradient(id, 1, original.translated(Point::new(0.1, 0.0)).unwrap())
        .unwrap();
    assert!(editor.undo());
    let before = editor.document.to_json().unwrap();
    for geometry in [
        CanvasGradientGeometry::Radial {
            center: Point::new(0.5, 0.5),
            radius: -0.1,
        },
        CanvasGradientGeometry::Radial {
            center: Point::new(0.5, 0.5),
            radius: f64::INFINITY,
        },
        CanvasGradientGeometry::Radial {
            center: Point::new(0.5, 0.5),
            radius: 1000.1,
        },
        CanvasGradientGeometry::Angular {
            center: Point::new(0.5, 0.5),
            angle: 0.0,
        },
        CanvasGradientGeometry::Angular {
            center: Point::new(0.5, 0.5),
            angle: f64::NAN,
        },
        CanvasGradientGeometry::Angular {
            center: Point::new(0.5, 0.5),
            angle: 360_001.0,
        },
    ] {
        assert!(editor
            .set_canvas_gradient(id, 1, native_gradient(geometry))
            .is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(editor.can_redo());
    }
    assert!(editor.redo());
}

#[test]
fn radial_angular_stop_edits_preserve_source_order_ties_and_exact_kind() {
    for geometry in [
        CanvasGradientGeometry::Radial {
            center: Point::new(0.5, 0.5),
            radius: 0.0,
        },
        CanvasGradientGeometry::Angular {
            center: Point::new(0.5, 0.5),
            angle: -270.5,
        },
    ] {
        let mut value = native_gradient(geometry);
        value.stops[0].color = Color {
            r: 10,
            g: 20,
            b: 30,
            a: 40,
        };
        value.stops[1].color = Color {
            r: 110,
            g: 120,
            b: 130,
            a: 140,
        };
        assert_eq!(value.add_stop(0.5).unwrap(), 2);
        assert_eq!(
            value.stops[2].color,
            Color {
                r: 60,
                g: 70,
                b: 80,
                a: 90
            }
        );
        assert!(value.move_stop(2, 1.0).unwrap());
        assert_eq!(value.geometry, geometry);
        assert_eq!(value.stops[1].offset, 1.0);
        assert_eq!(value.stops[2].offset, 1.0);
        value.remove_stop(0).unwrap();
        assert_eq!(
            value.stops[0].color,
            Color {
                r: 110,
                g: 120,
                b: 130,
                a: 140
            }
        );
        let before = value.clone();
        assert!(value.remove_stop(0).is_err());
        assert_eq!(value, before);
        value.validate().unwrap();
    }
}
