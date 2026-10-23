use crate::*;

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} differs from {b}");
}
fn point_close(a: Point, b: Point) {
    close(a.x, b.x);
    close(a.y, b.y);
}
fn layer<'a>(document: &'a Document, svg_id: &str) -> &'a Node {
    document
        .nodes
        .values()
        .find(|n| n.metadata.get("svg_id").is_some_and(|id| id == svg_id))
        .unwrap()
}

#[test]
fn svg_viewbox_and_nested_affine_groups_preserve_source_coordinates() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="200" viewBox="10 20 200 100">
      <g id="group" transform="translate(30 40) rotate(15)" opacity=".4" fill="#123456">
        <rect id="shape" x="5" y="7" width="80" height="20" transform="skewX(30)"/>
        <circle id="circle" cx="140" cy="25" r="10" fill="currentColor" color="rebeccapurple"/>
      </g></svg>"##;
    let document = Document::from_svg(source, "Affine artwork").unwrap();
    let shape = layer(&document, "shape");
    assert_eq!(shape.kind, NodeKind::Rectangle);
    assert_eq!(shape.fill_color(), Color::rgb(0x12, 0x34, 0x56));
    close(shape.opacity, 1.0);
    close(layer(&document, "group").opacity, 0.4);
    let expected = Transform::scale(2.0, 2.0)
        * Transform::translation(-10.0, -20.0)
        * Transform::translation(30.0, 40.0)
        * Transform::rotation(15.0)
        * Transform {
            c: 30_f64.to_radians().tan(),
            ..Transform::IDENTITY
        }
        * Transform::translation(5.0, 7.0);
    for point in shape.local_bounds().corners() {
        point_close(
            document.world_transform(shape.id).unwrap().apply(point),
            expected.apply(point),
        );
    }
    assert_eq!(
        layer(&document, "circle").fill_color(),
        Color::rgb(102, 51, 153)
    );
    assert_eq!(
        Document::from_json(&document.to_json().unwrap()).unwrap(),
        document
    );
}

#[test]
fn svg_linear_gradient_transforms_normals_and_inherits_stops() {
    let source = r##"<svg width="220" height="140"><defs>
      <linearGradient id="base"><stop offset="0" stop-color="#ff0000"/>
        <stop offset="100%" stop-color="blue" stop-opacity=".5"/></linearGradient>
      <linearGradient id="diagonal" href="#base" x2="1" y2="1"/>
      <radialGradient id="radial" cx="25%" cy="75%" r="40%" href="#base"/>
      </defs><rect id="linear" x="10" y="20" width="200" height="100" fill="url(#diagonal)"/>
      <circle id="radial-shape" cx="20" cy="20" r="10" fill="url(#radial)"/></svg>"##;
    let document = Document::from_svg(source, "Gradients").unwrap();
    let Paint::LinearGradient { start, end, stops } = &layer(&document, "linear").fills[0].paint
    else {
        panic!("expected gradient")
    };
    point_close(*start, Point::new(0.0, 0.0));
    // An object-space diagonal on a 2:1 rectangle retains its scalar field,
    // which differs from a physical-space diagonal after anisotropic scaling.
    point_close(*end, Point::new(0.4, 1.6));
    assert_eq!(stops.len(), 2);
    assert_eq!(stops[1].color, Color::rgba(0, 0, 255, 128));
    let Paint::RadialGradient { center, radius, .. } =
        &layer(&document, "radial-shape").fills[0].paint
    else {
        panic!("expected radial")
    };
    point_close(*center, Point::new(0.25, 0.75));
    close(*radius, 0.4);
}

#[test]
fn svg_paths_expand_relative_smooth_and_arc_commands_to_editable_segments() {
    let source = r##"<svg width="220" height="180">
      <path id="curve" d="m10 20 h30 v40 q10 20 20 0 t20 0 a20 10 25 0 1 40 10 z" fill-rule="evenodd"/>
      <rect id="rounded" x="0" y="0" width="70" height="40" rx="15" ry="5"/>
      <polygon id="polygon" points="1,2 3,4 5,2"/>
      <line id="line" x1="50" y1="60" x2="10" y2="20" stroke="black"/>
      </svg>"##;
    let document = Document::from_svg(source, "Paths").unwrap();
    let curve = layer(&document, "curve");
    assert_eq!(curve.kind, NodeKind::Vector);
    assert_eq!(curve.path.fill_rule, FillRule::EvenOdd);
    assert!(curve
        .path
        .segments
        .iter()
        .any(|s| matches!(s, PathSegment::CubicTo { .. })));
    assert_eq!(
        curve
            .path
            .segments
            .iter()
            .filter(|s| matches!(s, PathSegment::QuadraticTo { .. }))
            .count(),
        2
    );
    let world = curve
        .path
        .transformed(document.world_transform(curve.id).unwrap());
    let PathSegment::MoveTo { point } = world.segments[0] else {
        panic!("move expected")
    };
    point_close(point, Point::new(10.0, 20.0));
    let endpoint = world
        .segments
        .iter()
        .rev()
        .find_map(|s| match s {
            PathSegment::CubicTo { point, .. } | PathSegment::LineTo { point } => Some(*point),
            _ => None,
        })
        .unwrap();
    point_close(endpoint, Point::new(120.0, 70.0));
    assert_eq!(layer(&document, "rounded").kind, NodeKind::Vector);
    assert!(matches!(
        layer(&document, "polygon").path.segments.last(),
        Some(PathSegment::Close)
    ));
    let line = layer(&document, "line");
    assert!(line.fills.is_empty());
    assert_eq!(line.strokes.len(), 1);
    let world_line = line
        .path
        .transformed(document.world_transform(line.id).unwrap());
    assert_eq!(
        world_line.segments,
        vec![
            PathSegment::MoveTo {
                point: Point::new(50.0, 60.0)
            },
            PathSegment::LineTo {
                point: Point::new(10.0, 20.0)
            }
        ]
    );
}

#[test]
fn svg_css_cascade_and_unicode_runs_remain_editable() {
    let source = r##"<svg width="400" height="100"><style>
      rect { fill: red; } .accent { fill: blue !important; }
      #square { fill: green; } text.label { font-weight: 500; fill: #283848; }
      </style><rect id="square" class="accent" width="30" height="30" style="fill: orange"/>
      <text id="label" class="label" x="40" y="30" font-size="20">A &amp; <tspan font-weight="700" fill="red">🙂 café</tspan></text></svg>"##;
    let mut editor = Editor::new(Document::from_svg(source, "CSS").unwrap());
    assert_eq!(
        layer(&editor.document, "square").fill_color(),
        Color::rgb(0, 0, 255)
    );
    let text = layer(&editor.document, "label");
    assert_eq!(text.text, "A & 🙂 café");
    assert_eq!(text.font_weight, 500);
    assert_eq!(text.text_runs.len(), 2);
    assert_eq!(text.text_runs[0].color, None);
    assert_eq!(text.text_runs[0].font_weight, None);
    assert_eq!(text.text_runs[1].font_weight, Some(700));
    assert_eq!(
        &text.text[text.text_runs[1].start..text.text_runs[1].end],
        "🙂 café"
    );
    let id = text.id;
    editor.set_fill(id, Color::WHITE);
    assert_eq!(editor.document.nodes[&id].fill_color(), Color::WHITE);
    assert_eq!(editor.document.nodes[&id].text_runs[0].color, None);
    editor.document.validate().unwrap();
}

#[test]
fn svg_internal_use_and_symbol_copy_geometry_without_shared_edit_state() {
    let source = r##"<svg width="300" height="200"><defs>
      <symbol id="icon" viewBox="0 0 10 10"><path id="outline" d="M0 0H10V10H0Z"/></symbol>
      </defs><use id="first" href="#icon" x="20" y="30" width="40" height="20" fill="red"/>
      <use id="second" href="#icon" x="100" y="30" width="20" height="20" fill="blue"/></svg>"##;
    let document = Document::from_svg(source, "Symbols").unwrap();
    let outlines: Vec<_> = document
        .nodes
        .values()
        .filter(|n| n.name == "outline")
        .collect();
    assert_eq!(outlines.len(), 2);
    assert_ne!(outlines[0].id, outlines[1].id);
    let red = outlines
        .iter()
        .find(|n| n.fill_color() == Color::rgb(255, 0, 0))
        .unwrap();
    let blue = outlines
        .iter()
        .find(|n| n.fill_color() == Color::rgb(0, 0, 255))
        .unwrap();
    point_close(
        document.world_origin(red.id).unwrap(),
        Point::new(30.0, 30.0),
    );
    point_close(
        document.world_origin(blue.id).unwrap(),
        Point::new(100.0, 30.0),
    );
    close(document.world_bounds(red.id).unwrap().width, 20.0);
    document.validate().unwrap();
}

#[test]
fn svg_embedded_raster_is_preserved_and_external_images_rejected() {
    let data = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aGH0AAAAASUVORK5CYII=";
    let source = format!(
        r#"<svg width="100" height="80"><image id="image" x="5" y="10" width="40" height="30" opacity=".5" href="{data}" preserveAspectRatio="none"/></svg>"#
    );
    let document = Document::from_svg(&source, "Image").unwrap();
    let image = layer(&document, "image");
    assert_eq!(
        image.fills[0].paint,
        Paint::Image {
            href: data.into(),
            scale: ImageScale::Stretch,
            crop: None,
        }
    );
    close(image.opacity, 0.5);
    assert!(Document::from_svg(
        r#"<svg><image href="https://example.invalid/photo.png"/></svg>"#,
        "Bad"
    )
    .is_err());
    assert!(Document::from_svg(
        r#"<svg><image href="data:image/svg+xml;base64,PHN2Zy8+"/></svg>"#,
        "Bad"
    )
    .is_err());
    assert!(Document::from_svg(
        r#"<svg><image href="data:image/webp;base64,UklGRhAAAABXRUJQVlA4IAQAAAAwAQCdASoBAAEA"/></svg>"#,
        "Unsupported WebP"
    )
    .is_err());
}

#[test]
fn svg_import_selects_positions_and_undoes_the_entire_import_once() {
    let mut editor = Editor::default();
    let existing = editor.add_node(NodeKind::Rectangle, 100.0, 20.0, 200.0, 100.0);
    editor.selection = vec![existing];
    editor.clear_history();
    let before = editor.to_json().unwrap();
    let roots = editor
        .import_svg(
            r#"<svg width="60" height="30"><rect width="60" height="30"/></svg>"#,
            "Imported",
        )
        .unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(editor.selection, roots);
    assert_eq!(editor.history_len(), 1);
    let bounds = editor.document.world_bounds(roots[0]).unwrap();
    close(bounds.x, 364.0);
    close(bounds.y, 0.0);
    let imported = editor.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    assert_eq!(editor.selection, vec![existing]);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), imported);
    assert_eq!(editor.selection, roots);
}

#[test]
fn svg_bad_input_never_mutates_document_selection_or_history() {
    let bad = [
        "<svg><rect></svg>",
        "<!DOCTYPE svg [<!ENTITY x 'secret'>]><svg><text>&x;</text></svg>",
        "<svg><script>alert(1)</script></svg>",
        "<svg><rect onclick='run()'/></svg>",
        "<svg><foreignObject/></svg>",
        "<svg><rect fill='url(https://example.invalid/fill)'/></svg>",
        "<svg><style>@import 'https://example.invalid/a.css';</style></svg>",
        "<svg><rect width='-1'/></svg>",
        "<svg><rect width='NaN'/></svg>",
        "<svg><rect id='x'/><circle id='x'/></svg>",
        "<svg><defs><g id='a'><use href='#a'/></g></defs><use href='#a'/></svg>",
        "<svg><defs><linearGradient id='a' href='#b'/><linearGradient id='b' href='#a'/></defs><rect fill='url(#a)'/></svg>",
        "<svg><rect filter='url(#shadow)'/></svg>",
        "<svg><text><tspan x='10'>positioned</tspan></text></svg>",
    ];
    let mut editor = Editor::demo();
    editor.add_node(NodeKind::Ellipse, 1.0, 2.0, 3.0, 4.0);
    let before = editor.to_json().unwrap();
    let selection = editor.selection.clone();
    let history = editor.history_len();
    for source in bad {
        assert!(
            editor.import_svg(source, "Rejected").is_err(),
            "unexpected acceptance: {source}"
        );
        assert_eq!(editor.to_json().unwrap(), before);
        assert_eq!(editor.selection, selection);
        assert_eq!(editor.history_len(), history);
    }
}

#[test]
fn svg_failure_does_not_cancel_an_existing_gesture() {
    let mut editor = Editor::default();
    let id = editor.add_node(NodeKind::Rectangle, 0.0, 0.0, 20.0, 20.0);
    editor.clear_history();
    editor.begin_transaction("Gesture");
    editor.move_selection(25.0, 10.0);
    assert!(editor
        .import_svg("<svg><script/></svg>", "Invalid")
        .is_err());
    close(editor.document.nodes[&id].x, 25.0);
    assert!(editor.commit_transaction());
    assert_eq!(editor.history_len(), 1);
    editor.undo();
    close(editor.document.nodes[&id].x, 0.0);
}

#[test]
fn imported_affine_layers_keep_geometry_through_move_resize_group_and_ungroup() {
    let source = r#"<svg width="1000" height="1000"><g transform="matrix(1.4 .3 .6 -.8 200 300)">
      <rect id="box" x="20" y="30" width="50" height="40" transform="rotate(23)"/>
      <path id="vector" d="M100 100h40v30h-40z"/>
      </g></svg>"#;
    let mut editor = Editor::new(Document::from_svg(source, "Affine edits").unwrap());
    let box_id = layer(&editor.document, "box").id;
    let vector_id = layer(&editor.document, "vector").id;
    let original = editor.document.world_transform(box_id).unwrap();
    editor.selection = vec![box_id];
    editor.move_selection(12.0, -7.0);
    for point in [Point::default(), Point::new(50.0, 40.0)] {
        point_close(
            editor
                .document
                .world_transform(box_id)
                .unwrap()
                .apply(point),
            original.apply(point) + Point::new(12.0, -7.0),
        );
    }
    let before_resize = editor.document.world_transform(box_id).unwrap();
    editor.resize_node(box_id, 90.0, 60.0);
    for point in [Point::default(), Point::new(90.0, 60.0)] {
        point_close(
            editor
                .document
                .world_transform(box_id)
                .unwrap()
                .apply(point),
            before_resize.apply(point),
        );
    }
    let vector_transform = editor.document.world_transform(vector_id).unwrap();
    editor.resize_node(vector_id, 80.0, 60.0);
    close(
        editor.document.nodes[&vector_id]
            .path
            .bounds()
            .unwrap()
            .width,
        80.0,
    );
    close(
        editor.document.nodes[&vector_id]
            .path
            .bounds()
            .unwrap()
            .height,
        60.0,
    );
    point_close(
        editor.document.world_origin(vector_id).unwrap(),
        vector_transform.apply(Point::default()),
    );
    let transforms = [
        editor.document.world_transform(box_id).unwrap(),
        editor.document.world_transform(vector_id).unwrap(),
    ];
    editor.selection = vec![box_id, vector_id];
    editor.group_selection().unwrap();
    editor.ungroup_selection();
    for (id, transform) in [box_id, vector_id].into_iter().zip(transforms) {
        for point in [Point::default(), Point::new(9.0, 11.0)] {
            point_close(
                editor.document.world_transform(id).unwrap().apply(point),
                transform.apply(point),
            );
        }
    }
    editor.document.validate().unwrap();
}

#[test]
fn affine_scene_updates_hit_geometry_and_invalid_matrices_are_rejected() {
    let mut editor = Editor::default();
    let id = editor.add_node(NodeKind::Rectangle, 10.0, 20.0, 30.0, 40.0);
    editor.update_node(id, "Shear", |node| {
        node.affine_transform = Some(Transform {
            c: 1.0,
            e: 200.0,
            ..Transform::IDENTITY
        })
    });
    assert_eq!(editor.hit_test(Point::new(225.0, 25.0), 0.0), Some(id));
    assert_ne!(editor.hit_test(Point::new(15.0, 25.0), 0.0), Some(id));
    editor.undo();
    assert_eq!(editor.hit_test(Point::new(15.0, 25.0), 0.0), Some(id));
    editor.document.nodes.get_mut(&id).unwrap().affine_transform = Some(Transform {
        a: f64::NAN,
        ..Transform::IDENTITY
    });
    assert!(editor.document.validate().is_err());
}

#[test]
fn svg_nested_viewports_clip_children_after_aspect_mapping() {
    let document = Document::from_svg(r#"<svg width="200" height="200">
      <svg id="viewport" x="20" y="30" width="100" height="50" viewBox="0 0 10 10" preserveAspectRatio="xMaxYMin meet">
        <rect id="inside" width="10" height="10"/>
        <rect id="outside" x="12" width="10" height="10"/>
      </svg></svg>"#, "Nested").unwrap();
    let inside = layer(&document, "inside").id;
    let outside = layer(&document, "outside").id;
    point_close(
        document.world_origin(inside).unwrap(),
        Point::new(70.0, 30.0),
    );
    close(document.world_bounds(inside).unwrap().width, 50.0);
    let editor = Editor::new(document);
    assert_eq!(editor.hit_test(Point::new(80.0, 40.0), 0.0), Some(inside));
    assert_ne!(editor.hit_test(Point::new(135.0, 40.0), 0.0), Some(outside));
}

