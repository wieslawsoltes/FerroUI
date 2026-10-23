use crate::*;

fn paint(angle: f64) -> Paint {
    Paint::AngularGradient {
        center: Point::new(0.4, 0.6),
        angle,
        stops: vec![
            GradientStop { offset: 0.0, color: Color::rgba(240, 30, 60, 64) },
            GradientStop { offset: 0.45, color: Color::rgb(20, 210, 90) },
            GradientStop { offset: 0.45, color: Color::rgba(20, 90, 230, 128) },
            GradientStop { offset: 1.0, color: Color::rgb(70, 40, 220) },
        ],
    }
}

fn fixture() -> (Editor, NodeId) {
    let mut document = Document::new("Angular paint");
    let mut node = Node::new(NodeKind::Rectangle, "Angular", 10.0, 20.0, 180.0, 120.0);
    node.fills = vec![Fill { paint: paint(350.0), opacity: 0.8, visible: true }];
    node.strokes.push(Stroke { paint: paint(-90.0), ..Stroke::new(Color::BLACK, 4.0) });
    let id = document.insert_node(document.pages[0].id, node).unwrap();
    (Editor::new(document), id)
}

#[test]
fn angular_gradient_document_preserves_duplicate_stops_alpha_and_strokes() {
    let (editor, id) = fixture();
    let json = editor.to_json().unwrap();
    assert!(json.contains("angular_gradient"));
    let loaded = Document::from_json(&json).unwrap();
    assert_eq!(loaded.nodes[&id].fills, editor.document.nodes[&id].fills);
    assert_eq!(loaded.nodes[&id].strokes, editor.document.nodes[&id].strokes);
    assert_eq!(loaded.to_json().unwrap(), json);
    assert_eq!(loaded.nodes[&id].fills[0].paint.representative_color(), Color::rgba(240, 30, 60, 64));
}

#[test]
fn angular_gradient_scoped_edit_has_exact_undo_redo_and_unchanged_noop() {
    let (mut editor, id) = fixture();
    let before = editor.to_json().unwrap();
    editor.update_node(id, "Rotate angular paint", |node| {
        if let Paint::AngularGradient { angle, .. } = &mut node.fills[0].paint { *angle = 10.0; }
    });
    let after = editor.to_json().unwrap();
    assert_ne!(before, after);
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), after);
    let history = editor.history_len();
    editor.update_node(id, "Same angle", |node| {
        if let Paint::AngularGradient { angle, .. } = &mut node.fills[0].paint { *angle = 10.0; }
    });
    assert_eq!(editor.history_len(), history);
}

#[test]
fn angular_gradient_import_rejects_invalid_center_angle_and_stop_offsets() {
    let (editor, id) = fixture();
    for property in 0..6 {
        let mut document = editor.document.clone();
        let Paint::AngularGradient { center, angle, stops } = &mut document.nodes.get_mut(&id).unwrap().fills[0].paint else { unreachable!() };
        match property {
            0 => center.x = f64::NAN,
            1 => center.y = f64::INFINITY,
            2 => *angle = f64::NEG_INFINITY,
            3 => stops[0].offset = -0.1,
            4 => stops[0].offset = 1.1,
            _ => stops.clear(),
        }
        assert!(document.validate().is_err(), "invalid angular property {property} must be rejected");
    }
    let mut json: serde_json::Value = serde_json::from_str(&editor.to_json().unwrap()).unwrap();
    json["nodes"][id.to_string()]["fills"][0]["paint"]["angle"] = serde_json::json!("invalid");
    assert!(Document::from_json(&json.to_string()).is_err());
}

#[test]
fn angular_gradient_style_propagation_retains_bindings_and_manual_edit_detaches() {
    let (mut editor, id) = fixture();
    let style = editor.create_local_style_for_nodes(&[id], LocalStyleKind::Fill, "Brand/Angular", "").unwrap();
    let before = editor.to_json().unwrap();
    let mut value = editor.document.styles[&style].value.clone();
    let StyleValue::Fill { fills } = &mut value else { unreachable!() };
    fills[0].paint = paint(125.0);
    assert_eq!(editor.update_local_style(style, "Brand/Angular", "", value).unwrap().updated_nodes, 1);
    assert_eq!(editor.document.nodes[&id].fill_style, Some(style));
    let updated = editor.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), updated);
    editor.update_node(id, "Override angular center", |node| {
        if let Paint::AngularGradient { center, .. } = &mut node.fills[0].paint { center.x = 0.25; }
    });
    assert_eq!(editor.document.nodes[&id].fill_style, None);
}

#[test]
fn angular_gradient_rejected_style_update_preserves_document_redo_and_allocator() {
    let (mut editor, id) = fixture();
    let style = editor.create_local_style_for_nodes(&[id], LocalStyleKind::Fill, "Angular", "").unwrap();
    editor.update_node(id, "Move", |node| node.x += 1.0);
    assert!(editor.undo());
    let before = editor.to_json().unwrap();
    let history = editor.history_len();
    let mut value = editor.document.styles[&style].value.clone();
    let StyleValue::Fill { fills } = &mut value else { unreachable!() };
    let Paint::AngularGradient { angle, .. } = &mut fills[0].paint else { unreachable!() };
    *angle = f64::NAN;
    assert!(editor.update_local_style(style, "Angular", "", value).is_err());
    assert_eq!(editor.to_json().unwrap(), before);
    assert_eq!(editor.history_len(), history);
    assert!(editor.can_redo());
}

#[test]
fn angular_gradient_svg_approximation_is_bounded_explicit_and_preserves_alpha() {
    let (editor, _) = fixture();
    let report = editor.document.export_svg_report(editor.active_page, &[]);
    assert_eq!(report.warnings.len(), 1, "one layer warning covers fill and stroke");
    assert!(report.warnings[0].contains("1024 vector sectors"));
    let svg = roxmltree::Document::parse(&report.source).unwrap();
    let patterns: Vec<_> = svg.descendants().filter(|node| node.tag_name().name() == "pattern").collect();
    assert_eq!(patterns.len(), 2);
    for pattern in patterns {
        assert_eq!(pattern.attribute("data-ferrodesign-approximation"), Some("angular-gradient"));
        let sectors: Vec<_> = pattern.descendants().filter(|node| node.tag_name().name() == "path").collect();
        assert!((512..=1024).contains(&sectors.len()));
        assert!(sectors.iter().any(|sector| sector.attribute("fill-opacity").unwrap().parse::<f64>().unwrap() < 0.9));
        assert!(sectors.iter().any(|sector| sector.attribute("fill").unwrap().starts_with('#')));
        assert!(pattern.descendants().any(|node| node.attribute("shape-rendering") == Some("crispEdges")));
    }
}

#[test]
fn angular_gradient_svg_large_stop_stacks_have_fixed_geometry_budget() {
    let (mut editor, id) = fixture();
    let node = editor.document.nodes.get_mut(&id).unwrap();
    node.strokes.clear();
    let Paint::AngularGradient { stops, .. } = &mut node.fills[0].paint else { unreachable!() };
    *stops = (0..4000).map(|i| GradientStop { offset: i as f64 / 3999.0, color: Color::rgb((i % 255) as u8, 50, 150) }).collect();
    let report = editor.document.export_svg_report(editor.active_page, &[]);
    let svg = roxmltree::Document::parse(&report.source).unwrap();
    let pattern = svg.descendants().find(|node| node.attribute("data-ferrodesign-approximation") == Some("angular-gradient")).unwrap();
    assert_eq!(pattern.descendants().filter(|node| node.tag_name().name() == "path").count(), 1024);
    assert_eq!(report.warnings.len(), 1);
}

#[test]
fn angular_gradient_smart_animate_uses_shortest_angle_and_interpolates_stops() {
    let mut document = Document::new("Angular motion");
    let page = document.pages[0].id;
    let a = document.insert_node(page, Node::new(NodeKind::Frame, "A", 0.0, 0.0, 200.0, 200.0)).unwrap();
    let b = document.insert_node(page, Node::new(NodeKind::Frame, "B", 250.0, 0.0, 200.0, 200.0)).unwrap();
    for (parent, angle, center, color) in [(a, 350.0, 0.2, Color::BLACK), (b, 10.0, 0.8, Color::WHITE)] {
        let mut node = Node::new(NodeKind::Rectangle, "Angular", 0.0, 0.0, 100.0, 100.0);
        node.fills = vec![Fill {
            paint: Paint::AngularGradient { center: Point::new(center, 0.5), angle,
                stops: vec![GradientStop { offset: 0.0, color }, GradientStop { offset: 1.0, color }] },
            ..Fill::solid(Color::BLACK)
        }];
        document.insert_node(parent, node).unwrap();
    }
    let mut scene = PrototypeScene::new(
        PrototypeFrame::capture(&document, a, &[]).unwrap(),
        PrototypeFrame::capture(&document, b, &[]).unwrap(),
        Transition { kind: TransitionKind::SmartAnimate, duration_ms: 1000, easing: Easing::Linear },
    );
    scene.sample(500.0);
    let nodes: Vec<_> = scene.document.nodes.values().filter(|node| node.name == "Angular").collect();
    assert_eq!(nodes.len(), 1, "compatible angular paints morph instead of duplicating layers");
    let Paint::AngularGradient { center, angle, stops } = &nodes[0].fills[0].paint else { unreachable!() };
    assert_eq!(*center, Point::new(0.5, 0.5));
    assert_eq!(*angle, 360.0);
    assert_eq!(stops[0].color, Color::rgb(128, 128, 128));
}

#[test]
fn angular_gradient_svg_strokes_have_user_space_tiles_and_native_reference_centers() {
    for (kind, alignment, width, height, center) in [
        (NodeKind::Rectangle, StrokeAlign::Center, 100.0, 80.0, Point::new(14.0, 70.0)),
        (NodeKind::Rectangle, StrokeAlign::Inside, 100.0, 80.0, Point::new(20.0, 64.0)),
        (NodeKind::Rectangle, StrokeAlign::Outside, 100.0, 80.0, Point::new(8.0, 76.0)),
        (NodeKind::Line, StrokeAlign::Center, 100.0, 0.0, Point::new(14.0, 6.0)),
        (NodeKind::Line, StrokeAlign::Center, 0.0, 80.0, Point::new(-6.0, 70.0)),
    ] {
        let mut document = Document::new("Angular stroke bounds");
        let mut node = Node::new(kind, "Stroke", 0.0, 0.0, width, height);
        node.fills.clear();
        node.strokes = vec![Stroke { paint: Paint::AngularGradient {
            center: Point::new(0.2, 0.8), angle: 0.0,
            stops: vec![GradientStop { offset: 0.0, color: Color::BLACK },
                GradientStop { offset: 1.0, color: Color::WHITE }],
        }, align: alignment, ..Stroke::new(Color::BLACK, 20.0) }];
        document.insert_node(document.pages[0].id, node).unwrap();
        let report = document.export_svg_report(document.pages[0].id, &[]);
        let svg = roxmltree::Document::parse(&report.source).unwrap();
        let pattern = svg.descendants().find(|node| node.tag_name().name() == "pattern").unwrap();
        assert_eq!(pattern.attribute("patternUnits"), Some("userSpaceOnUse"));
        let number = |attribute| pattern.attribute(attribute).unwrap().parse::<f64>().unwrap();
        assert!(number("x") < -20.0 && number("y") < -20.0);
        assert!(number("width") > width + 40.0 && number("height") > height + 40.0);
        let data = pattern.descendants().find(|node| node.tag_name().name() == "path").unwrap().attribute("d").unwrap();
        assert!(data.starts_with(&format!("M{} {} L", center.x, center.y)),
            "{kind:?} {alignment:?} must resolve the relative center against native paint bounds: {data}");
    }
}
