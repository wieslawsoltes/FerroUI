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
        .find(|n| n.metadata.get("svg_id").is_some_and(|v| v == svg_id))
        .unwrap()
}

fn mask<'a>(document: &'a Document, reference: &str) -> &'a Node {
    document
        .nodes
        .values()
        .find(|n| {
            n.is_mask
                && n.metadata
                    .get("svg_mask_reference")
                    .is_some_and(|v| v == reference)
        })
        .unwrap()
}

fn xml_id<'a, 'input>(
    xml: &'a roxmltree::Document<'input>,
    id: &str,
) -> roxmltree::Node<'a, 'input> {
    xml.descendants()
        .find(|n| n.is_element() && n.attribute("id") == Some(id))
        .unwrap()
}

#[test]
fn svg_isolation_auto_isolate_opacity_and_explicit_inherit_keep_blend_scope() {
    let document = Document::from_svg(r#"<svg width="200" height="100">
        <g id="auto"><rect id="multiply" width="10" height="10" style="mix-blend-mode:multiply"/></g>
        <g id="isolate" style="isolation:isolate"><g id="inherit" style="isolation:inherit"><rect width="10" height="10"/></g></g>
        <g id="opacity" opacity=".4"><rect width="10" height="10"/></g>
        <g id="blend" style="mix-blend-mode:screen"><rect id="inherited-blend" width="10" height="10" style="mix-blend-mode:inherit"/></g>
        </svg>"#, "Isolation").unwrap();
    assert_eq!(layer(&document, "auto").blend_mode, BlendMode::PassThrough);
    assert_eq!(layer(&document, "isolate").blend_mode, BlendMode::Normal);
    assert_eq!(layer(&document, "inherit").blend_mode, BlendMode::Normal);
    assert_eq!(layer(&document, "opacity").blend_mode, BlendMode::Normal);
    close(layer(&document, "opacity").opacity, 0.4);
    assert_eq!(layer(&document, "multiply").blend_mode, BlendMode::Multiply);
    assert_eq!(
        layer(&document, "inherited-blend").blend_mode,
        BlendMode::Screen
    );
    assert!(document
        .nodes
        .values()
        .filter(|n| n.name == "SVG contents")
        .all(|n| n.blend_mode == BlendMode::PassThrough));
}

#[test]
fn svg_alpha_object_bounds_and_definition_inheritance_preserve_rotated_geometry() {
    let document = Document::from_svg(r##"<svg width="400" height="300"><defs fill="red">
        <mask id="alpha" mask-type="alpha" maskContentUnits="objectBoundingBox">
          <rect id="coverage" x=".25" y="0" width=".5" height="1" fill-opacity=".5"/>
        </mask></defs><g transform="translate(100 80) rotate(30)">
          <rect id="target" x="10" y="20" width="100" height="40" fill="blue" opacity=".4" mask="url(#alpha)"/>
        </g></svg>"##, "Alpha mask").unwrap();
    let target = layer(&document, "target");
    let parent = &document.nodes[&target.parent.unwrap()];
    assert_eq!(parent.blend_mode, BlendMode::Normal);
    close(parent.opacity, 0.4);
    close(target.opacity, 1.0);
    let transform = Transform::translation(100.0, 80.0) * Transform::rotation(30.0);
    point_close(
        document
            .world_transform(target.id)
            .unwrap()
            .apply(Point::new(100.0, 40.0)),
        transform.apply(Point::new(110.0, 60.0)),
    );
    let role = mask(&document, "alpha");
    assert_eq!(role.mask_type, MaskType::Alpha);
    assert!(role.visible && role.clip_content);
    close(role.width, 120.0);
    close(role.height, 48.0);
    point_close(
        document
            .world_transform(role.id)
            .unwrap()
            .apply(Point::default()),
        transform.apply(Point::new(0.0, 16.0)),
    );
    let coverage = layer(&document, "coverage");
    assert_eq!(coverage.fill_color(), Color::rgb(255, 0, 0));
    close(coverage.fills[0].opacity, 0.5);
    point_close(
        document
            .world_transform(coverage.id)
            .unwrap()
            .apply(Point::default()),
        transform.apply(Point::new(35.0, 20.0)),
    );
    point_close(
        document
            .world_transform(coverage.id)
            .unwrap()
            .apply(Point::new(0.5, 1.0)),
        transform.apply(Point::new(85.0, 60.0)),
    );
    assert_eq!(
        Document::from_json(&document.to_json().unwrap()).unwrap(),
        document
    );
}

#[test]
fn svg_user_space_mask_reuse_creates_independent_sources_in_each_reference_space() {
    let document = Document::from_svg(r##"<svg width="400" height="120"><defs>
      <mask id="shared" maskUnits="userSpaceOnUse" x="50" y="5" width="30" height="70" mask-type="alpha">
        <rect id="coverage" x="55" y="10" width="20" height="60" fill="white"/>
      </mask></defs>
      <rect id="first" x="60" y="20" width="50" height="40" mask="url(#shared)"/>
      <rect id="second" x="60" y="20" width="50" height="40" transform="translate(200 0)" mask="url(#shared)"/>
      </svg>"##, "Reusable masks").unwrap();
    let roles: Vec<_> = document.nodes.values().filter(|n| n.is_mask).collect();
    assert_eq!(roles.len(), 2);
    for (role, x) in roles.iter().zip([50.0, 250.0]) {
        point_close(
            document
                .world_transform(role.id)
                .unwrap()
                .apply(Point::default()),
            Point::new(x, 5.0),
        );
        close(role.width, 30.0);
        close(role.height, 70.0);
    }
    let sources: Vec<_> = document
        .nodes
        .values()
        .filter(|n| n.metadata.get("svg_id").is_some_and(|v| v == "coverage"))
        .collect();
    assert_eq!(sources.len(), 2);
    for (source, x) in sources.iter().zip([55.0, 255.0]) {
        point_close(
            document
                .world_transform(source.id)
                .unwrap()
                .apply(Point::default()),
            Point::new(x, 10.0),
        );
    }
    assert_ne!(roles[0].parent, roles[1].parent);
}

#[test]
fn svg_luminance_default_mask_mode_override_and_empty_mask_are_explicit() {
    let document = Document::from_svg(
        r##"<svg width="100" height="80"><defs>
       <mask id="luma"><rect width="100" height="80" fill="red" opacity=".5"/></mask>
       <mask id="empty" mask-type="alpha" display="none"><rect width="100" height="80"/></mask>
       </defs><rect width="100" height="80" mask="url(#luma)"/>
       <rect width="100" height="80" mask="url(#luma)" style="mask-mode:alpha"/>
       <rect width="100" height="80" mask="url(#empty)"/></svg>"##,
        "Mask modes",
    )
    .unwrap();
    let roles: Vec<_> = document.nodes.values().filter(|n| n.is_mask).collect();
    assert_eq!(
        roles.iter().map(|n| n.mask_type).collect::<Vec<_>>(),
        [MaskType::Luminance, MaskType::Alpha, MaskType::Alpha]
    );
    assert!(roles[2].visible);
    assert!(!document.nodes[&roles[2].children[0]].visible);
    assert!(Document::from_svg(r##"<svg><defs><mask id="m" color-interpolation="linearRGB"><rect width="10" height="10"/></mask></defs><rect width="10" height="10" mask="url(#m)"/></svg>"##, "Linear mask").unwrap_err().to_string().contains("linearRGB"));
}

#[test]
fn svg_clip_path_uses_geometry_and_clip_rule_instead_of_paint_stroke_or_opacity() {
    let document = Document::from_svg(r##"<svg width="200" height="120"><defs>
       <clipPath id="clip" clipPathUnits="objectBoundingBox" clip-rule="evenodd">
         <path id="ring" d="M0 0H1V1H0Z M.25 .25H.75V.75H.25Z" fill="none" stroke="red" stroke-width="40" opacity="0"/>
         <rect id="hidden" width="1" height="1" visibility="hidden"/>
       </clipPath></defs><rect id="target" x="20" y="30" width="100" height="40" clip-path="url(#clip)"/></svg>"##, "Geometry clip").unwrap();
    let role = mask(&document, "clip");
    assert_eq!(role.mask_type, MaskType::Vector);
    let ring = layer(&document, "ring");
    assert_eq!(ring.path.fill_rule, FillRule::EvenOdd);
    assert_eq!(ring.fill_color(), Color::WHITE);
    assert!(ring.strokes.is_empty());
    close(ring.opacity, 1.0);
    point_close(
        document
            .world_transform(ring.id)
            .unwrap()
            .apply(Point::new(0.25, 0.25)),
        Point::new(45.0, 40.0),
    );
    assert!(!layer(&document, "hidden").visible);
}

#[test]
fn svg_nested_clips_and_alpha_mask_intersect_in_a_bounded_sibling_scope() {
    let document = Document::from_svg(r##"<svg width="200" height="100"><defs>
       <clipPath id="inner"><rect width="50" height="100"/></clipPath>
       <clipPath id="outer"><rect width="100" height="100" clip-path="url(#inner)"/></clipPath>
       <mask id="alpha" mask-type="alpha"><rect width="100" height="100" fill="white"/></mask>
       </defs><rect id="clipped" width="100" height="100" clip-path="url(#outer)" mask="url(#alpha)"/>
       <rect id="unrelated" x="120" width="50" height="50"/></svg>"##, "Intersections").unwrap();
    let target = layer(&document, "clipped");
    let wrapper = &document.nodes[&target.parent.unwrap()];
    assert_eq!(wrapper.children.len(), 3);
    assert_eq!(
        document.nodes[&wrapper.children[0]].mask_type,
        MaskType::Vector
    );
    assert_eq!(
        document.nodes[&wrapper.children[1]].mask_type,
        MaskType::Alpha
    );
    assert_eq!(wrapper.children[2], target.id);
    assert_eq!(wrapper.blend_mode, BlendMode::Normal);
    assert_eq!(document.nodes.values().filter(|n| n.is_mask).count(), 3);
    assert_ne!(layer(&document, "unrelated").parent, Some(wrapper.id));
    document.validate().unwrap();
}

#[test]
fn svg_root_viewbox_mask_preserves_content_and_definition_coordinate_systems() {
    let document = Document::from_svg(r##"<svg width="200" height="100" viewBox="10 20 100 50" mask="url(#root-mask)">
        <defs><mask id="root-mask" maskUnits="userSpaceOnUse" x="10" y="20" width="100" height="50" mask-type="alpha">
        <rect id="coverage" x="10" y="20" width="50" height="50" fill="white"/></mask></defs>
        <rect id="content" x="10" y="20" width="100" height="50"/></svg>"##, "Root mask").unwrap();
    let coverage = layer(&document, "coverage");
    point_close(
        document
            .world_transform(coverage.id)
            .unwrap()
            .apply(Point::default()),
        Point::default(),
    );
    point_close(
        document
            .world_transform(coverage.id)
            .unwrap()
            .apply(Point::new(50.0, 50.0)),
        Point::new(100.0, 100.0),
    );
    let content = layer(&document, "content");
    point_close(
        document
            .world_transform(content.id)
            .unwrap()
            .apply(Point::new(100.0, 50.0)),
        Point::new(200.0, 100.0),
    );
}

#[test]
fn svg_mask_rejections_preserve_document_selection_revision_history_and_redo() {
    let sources = [
        r##"<svg><rect width="10" height="10" mask="url(#missing)"/></svg>"##,
        r##"<svg><defs><rect id="wrong"/></defs><rect width="10" height="10" mask="url(#wrong)"/></svg>"##,
        r##"<svg><defs><mask id="m"><rect width="10" height="10" mask="url(#m)"/></mask></defs><rect width="10" height="10" mask="url(#m)"/></svg>"##,
        r##"<svg><defs><clipPath id="a"><rect width="10" height="10" clip-path="url(#b)"/></clipPath><clipPath id="b"><circle r="10" clip-path="url(#a)"/></clipPath></defs><rect width="10" height="10" clip-path="url(#a)"/></svg>"##,
        r##"<svg><defs><g id="loop"><rect width="10" height="10" mask="url(#m)"/></g><mask id="m"><use href="#loop"/></mask></defs><rect width="10" height="10" mask="url(#m)"/></svg>"##,
        r##"<svg><defs><mask id="m" maskUnits="wrong"/></defs><rect width="10" height="10" mask="url(#m)"/></svg>"##,
        r##"<svg><defs><mask id="m" width="-1"/></defs><rect width="10" height="10" mask="url(#m)"/></svg>"##,
        r##"<svg xmlns:foreign="https://example.invalid"><defs><foreign:mask id="m"/></defs><rect width="10" height="10" mask="url(#m)"/></svg>"##,
        r##"<svg><rect width="10" height="10" mask="url(https://example.invalid/m.svg#m)"/></svg>"##,
        r##"<svg><style>rect{mask:url(#a),url(#b)}</style><rect width="10" height="10"/></svg>"##,
        r##"<svg><defs><clipPath id="c"><g><rect width="10" height="10"/></g></clipPath></defs><rect width="10" height="10" clip-path="url(#c)"/></svg>"##,
    ];
    let mut editor = Editor::default();
    editor.add_node(NodeKind::Rectangle, 0.0, 0.0, 30.0, 30.0);
    editor.add_node(NodeKind::Ellipse, 40.0, 0.0, 30.0, 30.0);
    assert!(editor.undo());
    let before = editor.to_json().unwrap();
    let selection = editor.selection.clone();
    let revision = editor.revision;
    let history = editor.history_len();
    for source in sources {
        assert!(
            editor.import_svg(source, "Rejected").is_err(),
            "accepted {source}"
        );
        assert_eq!(editor.to_json().unwrap(), before);
        assert_eq!(editor.selection, selection);
        assert_eq!(editor.revision, revision);
        assert_eq!(editor.history_len(), history);
        assert!(editor.can_redo());
    }
}

#[test]
fn svg_mask_import_is_one_undoable_edit() {
    let mut editor = Editor::default();
    editor.add_node(NodeKind::Rectangle, 10.0, 20.0, 30.0, 40.0);
    editor.clear_history();
    let before = editor.to_json().unwrap();
    editor.import_svg(r##"<svg width="100" height="80"><defs><mask id="m" mask-type="alpha"><rect width="50" height="80" fill="white"/></mask></defs><rect width="100" height="80" mask="url(#m)"/></svg>"##, "Masked artwork").unwrap();
    let after = editor.to_json().unwrap();
    assert_eq!(editor.history_len(), 1);
    assert_eq!(
        editor.document.nodes.values().filter(|n| n.is_mask).count(),
        1
    );
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), after);
}

fn native_masks() -> (Document, NodeId, NodeId, NodeId, NodeId) {
    let mut document = Document::new("Masks");
    let page = document.pages[0].id;
    let mut first = Node::new(NodeKind::Rectangle, "Alpha source", 0.0, 0.0, 100.0, 100.0)
        .with_fill(Color::rgba(255, 0, 0, 128));
    first.is_mask = true;
    let alpha = document.insert_node(page, first).unwrap();
    let first_content = document
        .insert_node(
            page,
            Node::new(NodeKind::Rectangle, "First content", 0.0, 0.0, 100.0, 100.0)
                .with_fill(Color::rgb(30, 90, 220)),
        )
        .unwrap();
    let mut second = Node::new(
        NodeKind::Ellipse,
        "Luminance source",
        10.0,
        10.0,
        80.0,
        80.0,
    )
    .with_fill(Color::rgb(255, 0, 0));
    second.is_mask = true;
    second.mask_type = MaskType::Luminance;
    let luma = document.insert_node(page, second).unwrap();
    let second_content = document
        .insert_node(
            page,
            Node::new(
                NodeKind::Rectangle,
                "Second content",
                20.0,
                20.0,
                60.0,
                60.0,
            )
            .with_fill(Color::rgb(30, 220, 90)),
        )
        .unwrap();
    (document, alpha, luma, first_content, second_content)
}

#[test]
fn svg_export_emits_cumulative_masks_with_explicit_modes_units_and_srgb() {
    let (document, alpha, luma, first, second) = native_masks();
    let report = document.export_svg_report(document.pages[0].id, &[]);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let xml = roxmltree::Document::parse(&report.source).unwrap();
    for (id, kind) in [(alpha, "alpha"), (luma, "luminance")] {
        let mask = xml_id(&xml, &format!("layer-mask-{id}"));
        assert_eq!(mask.attribute("mask-type"), Some(kind));
        assert_eq!(mask.attribute("maskUnits"), Some("userSpaceOnUse"));
        assert_eq!(mask.attribute("maskContentUnits"), Some("userSpaceOnUse"));
        assert_eq!(mask.attribute("color-interpolation"), Some("sRGB"));
        assert_eq!(
            xml_id(&xml, &format!("node-{id}"))
                .ancestors()
                .filter(|n| n.tag_name().name() == "mask")
                .count(),
            1
        );
    }
    assert_eq!(
        xml_id(&xml, &format!("node-{first}"))
            .ancestors()
            .filter(|n| n.attribute("mask").is_some())
            .count(),
        1
    );
    assert_eq!(
        xml_id(&xml, &format!("node-{second}"))
            .ancestors()
            .filter(|n| n.attribute("mask").is_some())
            .count(),
        2
    );
    let imported = Document::from_svg(&report.source, "Round trip").unwrap();
    assert_eq!(
        imported
            .nodes
            .values()
            .filter(|n| n.is_mask && n.mask_type == MaskType::Alpha)
            .count(),
        1
    );
    assert_eq!(
        imported
            .nodes
            .values()
            .filter(|n| n.is_mask && n.mask_type == MaskType::Luminance)
            .count(),
        1
    );
}

#[test]
fn svg_vector_mask_exports_opaque_fill_and_stroke_geometry_without_effects() {
    let (mut document, alpha, _, _, _) = native_masks();
    let mask = document.nodes.get_mut(&alpha).unwrap();
    mask.mask_type = MaskType::Vector;
    mask.opacity = 0.3;
    mask.strokes = vec![Stroke::new(Color::rgba(0, 0, 255, 128), 6.0)];
    mask.effects = vec![Effect {
        kind: EffectKind::LayerBlur,
        radius: 20.0,
        color: Color::BLACK,
        offset: Point::default(),
        spread: 0.0,
        visible: true,
    }];
    let source = document.export_svg(document.pages[0].id, &[]);
    let xml = roxmltree::Document::parse(&source).unwrap();
    let definition = xml_id(&xml, &format!("layer-mask-{alpha}"));
    assert_eq!(definition.attribute("mask-type"), Some("alpha"));
    assert_eq!(definition.attribute("data-ferro-mask-type"), Some("vector"));
    let group = xml_id(&xml, &format!("node-{alpha}"));
    assert_eq!(group.attribute("opacity"), Some("1"));
    assert!(group
        .descendants()
        .any(|n| n.attribute("stroke") == Some("#FFFFFF")
            && n.attribute("stroke-width") == Some("6")
            && n.attribute("stroke-opacity") == Some("1")));
    assert!(!source.contains("feGaussianBlur"));
    let imported = Document::from_svg(&source, "Vector mask round trip").unwrap();
    assert!(imported
        .nodes
        .values()
        .any(|n| n.is_mask && n.mask_type == MaskType::Vector));
    let untrusted = Document::from_svg(r##"<svg width="100" height="100"><defs>
        <mask id="hint" mask-type="alpha" data-ferro-mask-type="vector"><rect width="100" height="100" fill="white" opacity=".5"/></mask>
        </defs><rect width="100" height="100" mask="url(#hint)"/></svg>"##, "Untrusted vector hint").unwrap();
    assert_eq!(
        self::mask(&untrusted, "hint").mask_type,
        MaskType::Alpha,
        "An inert metadata hint cannot turn half-alpha coverage into opaque geometry"
    );
}

#[test]
fn svg_selection_retains_ancestor_and_sibling_mask_dependencies_without_unselected_paint() {
    let (mut document, alpha, luma, first, second) = native_masks();
    let page = document.pages[0].id;
    let container = document
        .insert_node(
            page,
            Node::new(NodeKind::Group, "Parent", 30.0, 40.0, 120.0, 120.0),
        )
        .unwrap();
    for (index, id) in [alpha, first, luma, second].into_iter().enumerate() {
        document.reparent(id, container, index).unwrap();
    }
    let source = document.export_svg(page, &[second]);
    let xml = roxmltree::Document::parse(&source).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.attribute("id") == Some(format!("node-{alpha}").as_str())));
    assert!(xml
        .descendants()
        .any(|n| n.attribute("id") == Some(format!("node-{luma}").as_str())));
    assert!(!xml
        .descendants()
        .any(|n| n.attribute("id") == Some(format!("node-{first}").as_str())));
    assert_eq!(
        xml_id(&xml, &format!("node-{second}"))
            .ancestors()
            .filter(|n| n.attribute("mask").is_some())
            .count(),
        2
    );
    assert_eq!(
        xml_id(&xml, &format!("node-{container}")).attribute("transform"),
        Some("matrix(1 0 0 1 30 40)")
    );
    assert_eq!(xml.root_element().attribute("viewBox"), Some("50 60 60 60"));
}

#[test]
fn svg_group_mask_does_not_escape_its_parent_and_external_blend_limit_is_reported() {
    let (mut document, alpha, luma, first, second) = native_masks();
    let page = document.pages[0].id;
    let mut group = Node::new(NodeKind::Group, "Contained masks", 0.0, 0.0, 100.0, 100.0);
    group.blend_mode = BlendMode::PassThrough;
    let group = document.insert_node(page, group).unwrap();
    for (index, id) in [alpha, luma, first].into_iter().enumerate() {
        document.reparent(id, group, index).unwrap();
    }
    document.nodes.get_mut(&first).unwrap().blend_mode = BlendMode::Multiply;
    let report = document.export_svg_report(page, &[]);
    let xml = roxmltree::Document::parse(&report.source).unwrap();
    assert_eq!(
        xml_id(&xml, &format!("node-{second}"))
            .ancestors()
            .filter(|n| n.attribute("mask").is_some())
            .count(),
        0
    );
    assert!(xml_id(&xml, &format!("node-{group}"))
        .attribute("style")
        .unwrap()
        .contains("isolation:auto"));
    assert!(report
        .warnings
        .iter()
        .any(|w| w.contains("external backdrop")));
}

#[test]
fn svg_export_normal_and_pass_through_have_distinct_explicit_isolation() {
    let mut document = Document::new("Isolation export");
    let page = document.pages[0].id;
    let mut normal = Node::new(NodeKind::Group, "Isolated", 0.0, 0.0, 20.0, 20.0);
    normal.blend_mode = BlendMode::Normal;
    let normal = document.insert_node(page, normal).unwrap();
    let mut pass = Node::new(NodeKind::Group, "Pass through", 30.0, 0.0, 20.0, 20.0);
    pass.blend_mode = BlendMode::PassThrough;
    let pass = document.insert_node(page, pass).unwrap();
    for parent in [normal, pass] {
        document
            .insert_node(
                parent,
                Node::new(NodeKind::Rectangle, "Paint", 0.0, 0.0, 20.0, 20.0),
            )
            .unwrap();
    }
    let source = document.export_svg(page, &[]);
    let xml = roxmltree::Document::parse(&source).unwrap();
    assert_eq!(
        xml_id(&xml, &format!("node-{normal}")).attribute("style"),
        Some("mix-blend-mode:normal;isolation:isolate")
    );
    assert_eq!(
        xml_id(&xml, &format!("node-{pass}")).attribute("style"),
        Some("mix-blend-mode:normal;isolation:auto")
    );
    let imported = Document::from_svg(&source, "Isolation round trip").unwrap();
    assert_eq!(
        layer(&imported, &format!("node-{normal}")).blend_mode,
        BlendMode::Normal
    );
    assert_eq!(
        layer(&imported, &format!("node-{pass}")).blend_mode,
        BlendMode::PassThrough
    );
}
