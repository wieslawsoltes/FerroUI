use crate::*;

fn profile(width: f64, multiplier: f64, cap: StrokeCap) -> Stroke {
    Stroke {
        width_profile: StrokeWidthProfile::Custom {
            points: vec![StrokeWidthPoint::new(0.0, multiplier)],
        },
        cap,
        ..Stroke::new(Color::rgb(37, 99, 235), width)
    }
}

fn line(x: f64, y: f64, width: f64, stroke: Stroke) -> Node {
    let mut node = Node::new(NodeKind::Line, "Profiled line", x, y, width, 0.0);
    node.fills.clear();
    node.strokes = vec![stroke];
    node
}

fn view_box(source: &str) -> Rect {
    let xml = roxmltree::Document::parse(source).unwrap();
    let values: Vec<f64> = xml
        .root_element()
        .attribute("viewBox")
        .unwrap()
        .split_whitespace()
        .map(|value| value.parse().unwrap())
        .collect();
    assert_eq!(values.len(), 4);
    Rect::new(values[0], values[1], values[2], values[3])
}

fn contains(bounds: Rect, point: Point) {
    // SVG numbers are serialized to four decimal places. The samples are well
    // inside the known shape; this allowance is only for affine serialization.
    assert!(
        bounds.inflate(0.0002).contains(point),
        "{bounds:?} crops painted point {point:?}"
    );
}

#[test]
fn svg_variable_stroke_viewport_contains_wide_round_caps() {
    let mut document = Document::new("Wide caps");
    let page = document.pages[0].id;
    let id = document
        .insert_node(
            page,
            line(50.0, 25.0, 100.0, profile(20.0, 3.0, StrokeCap::Round)),
        )
        .unwrap();
    let before = document.to_json().unwrap();
    for selection in [vec![id], Vec::new()] {
        let report = document.export_svg_report(page, &selection);
        assert!(report.warnings.is_empty());
        let bounds = view_box(&report.source);
        for point in [
            Point::new(21.0, 25.0),
            Point::new(179.0, 25.0),
            Point::new(100.0, -4.0),
            Point::new(100.0, 54.0),
        ] {
            contains(bounds, point);
        }
        assert!(report.source.contains("fill-rule=\"nonzero\""));
        assert!(report.source.contains("stroke=\"none\""));
    }
    assert_eq!(document.to_json().unwrap(), before);
}

#[test]
fn svg_variable_stroke_viewport_contains_the_outside_aligned_band() {
    let mut document = Document::new("Outside profile");
    let page = document.pages[0].id;
    let mut node = Node::new(NodeKind::Rectangle, "Outside", 10.0, 20.0, 80.0, 50.0);
    node.corner_radius = 0.0;
    node.fills.clear();
    let mut stroke = profile(10.0, 3.0, StrokeCap::Butt);
    stroke.align = StrokeAlign::Outside;
    node.strokes = vec![stroke];
    let id = document.insert_node(page, node).unwrap();
    let report = document.export_svg_report(page, &[id]);
    assert!(report.warnings.is_empty());
    let bounds = view_box(&report.source);
    for point in [
        Point::new(-19.0, -9.0),
        Point::new(119.0, -9.0),
        Point::new(119.0, 99.0),
        Point::new(-19.0, 99.0),
    ] {
        contains(bounds, point);
    }
    assert!(report.source.contains(&format!("width-mask-{id}-0")));
}

#[test]
fn svg_profiled_descendant_overflow_uses_full_affine_ancestry() {
    let mut document = Document::new("Affine profile overflow");
    let page = document.pages[0].id;
    let mut parent = Node::new(NodeKind::Frame, "Affine parent", 100.0, 80.0, 60.0, 40.0);
    parent.fills.clear();
    parent.strokes.clear();
    parent.clip_content = false;
    parent.rotation = 17.0;
    parent.affine_transform = Some(Transform {
        a: -1.2,
        b: 0.3,
        c: 0.4,
        d: 0.9,
        e: 0.0,
        f: 0.0,
    });
    let parent = document.insert_node(page, parent).unwrap();
    let child = document
        .insert_node(
            parent,
            line(-10.0, 20.0, 100.0, profile(12.0, 4.0, StrokeCap::Square)),
        )
        .unwrap();
    let transform = document.world_transform(child).unwrap();
    let points = [
        Point::new(-23.0, -23.0),
        Point::new(123.0, -23.0),
        Point::new(123.0, 23.0),
        Point::new(-23.0, 23.0),
    ];
    let before = document.to_json().unwrap();
    for selection in [vec![parent], vec![child], vec![parent, child], Vec::new()] {
        let report = document.export_svg_report(page, &selection);
        assert!(report.warnings.is_empty());
        let bounds = view_box(&report.source);
        for point in points {
            contains(bounds, transform.apply(point));
        }
    }
    assert_eq!(document.to_json().unwrap(), before);
}

#[test]
fn svg_profiled_descendant_clip_preserves_the_authored_frame_viewport() {
    let mut document = Document::new("Clipped profile");
    let page = document.pages[0].id;
    let mut frame = Node::new(NodeKind::Frame, "Viewport", 30.0, 40.0, 100.0, 60.0);
    frame.fills.clear();
    frame.strokes.clear();
    frame.clip_content = true;
    let frame = document.insert_node(page, frame).unwrap();
    document
        .insert_node(
            frame,
            line(-50.0, 30.0, 200.0, profile(20.0, 4.0, StrokeCap::Square)),
        )
        .unwrap();
    let report = document.export_svg_report(page, &[frame]);
    assert_eq!(view_box(&report.source), Rect::new(30.0, 40.0, 100.0, 60.0));
    assert!(report
        .source
        .contains(&format!("clip-path=\"url(#clip-{frame})\"")));
}

#[test]
fn svg_content_clip_contains_descendants_but_not_the_frames_outside_stroke() {
    let mut document = Document::new("Content clip and outside stroke");
    let page = document.pages[0].id;
    let mut frame = Node::new(NodeKind::Frame, "Clipped frame", 10.0, 20.0, 80.0, 50.0);
    frame.corner_radius = 0.0;
    frame.clip_content = true;
    let mut stroke = profile(10.0, 3.0, StrokeCap::Butt);
    stroke.align = StrokeAlign::Outside;
    frame.strokes = vec![stroke];
    let frame = document.insert_node(page, frame).unwrap();
    let mut child = Node::new(
        NodeKind::Rectangle,
        "Overflowing child",
        -10.0,
        -10.0,
        100.0,
        70.0,
    );
    child.fills = vec![Fill::solid(Color::rgb(239, 68, 68))];
    child.strokes.clear();
    let child = document.insert_node(frame, child).unwrap();
    let report = document.export_svg_report(page, &[frame]);
    assert!(report.warnings.is_empty());
    let xml = roxmltree::Document::parse(&report.source).unwrap();
    let frame_clip = format!("url(#clip-{frame})");
    let stroke = xml
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("fill") == Some("#2563EB")
                && node.attribute("stroke") == Some("none")
        })
        .unwrap();
    assert!(
        !stroke
            .ancestors()
            .any(|node| node.attribute("clip-path") == Some(frame_clip.as_str())),
        "A frame's own outside stroke must survive its child-content clip"
    );
    let child_id = format!("node-{child}");
    let child = xml
        .descendants()
        .find(|node| node.attribute("id") == Some(child_id.as_str()))
        .unwrap();
    assert!(
        child
            .ancestors()
            .any(|node| node.attribute("clip-path") == Some(frame_clip.as_str())),
        "The same content clip must still contain the overflowing child"
    );
    contains(view_box(&report.source), Point::new(-19.0, -9.0));
    contains(view_box(&report.source), Point::new(119.0, 99.0));
}

#[test]
fn svg_uniform_frame_does_not_gain_the_conservative_mask_allocation_margin() {
    let mut document = Document::new("Uniform viewport compatibility");
    let page = document.pages[0].id;
    let mut frame = Node::new(NodeKind::Frame, "Viewport", 30.0, 40.0, 100.0, 60.0);
    frame.clip_content = false;
    frame.strokes = vec![Stroke::new(Color::BLACK, 12.0)];
    let frame = document.insert_node(page, frame).unwrap();
    let mut child = Node::new(NodeKind::Rectangle, "Contained", 10.0, 10.0, 30.0, 20.0);
    child.strokes = vec![Stroke::new(Color::BLACK, 10.0)];
    document.insert_node(frame, child).unwrap();
    assert_eq!(
        view_box(&document.export_svg(page, &[frame])),
        Rect::new(30.0, 40.0, 100.0, 60.0)
    );
}

#[test]
fn svg_layer_mask_region_contains_profiles_larger_than_a_base_width_heuristic() {
    let mut document = Document::new("Large profiled mask");
    let page = document.pages[0].id;
    let mut mask = line(100.0, 100.0, 20.0, profile(2.0, 100.0, StrokeCap::Round));
    mask.is_mask = true;
    mask.mask_type = MaskType::Alpha;
    let mask = document.insert_node(page, mask).unwrap();
    document
        .insert_node(
            page,
            line(100.0, 100.0, 20.0, profile(2.0, 100.0, StrokeCap::Round)),
        )
        .unwrap();
    let report = document.export_svg_report(page, &[]);
    assert!(report.warnings.is_empty());
    let xml = roxmltree::Document::parse(&report.source).unwrap();
    let id = format!("layer-mask-{mask}");
    let mask = xml
        .descendants()
        .find(|node| node.attribute("id") == Some(id.as_str()))
        .unwrap();
    let value = |name| mask.attribute(name).unwrap().parse::<f64>().unwrap();
    let bounds = Rect::new(value("x"), value("y"), value("width"), value("height"));
    for point in [
        Point::new(1.0, 100.0),
        Point::new(219.0, 100.0),
        Point::new(110.0, 1.0),
        Point::new(110.0, 199.0),
    ] {
        contains(bounds, point);
        contains(view_box(&report.source), point);
    }
}

#[test]
fn svg_variable_stroke_bounds_ignore_disabled_and_zero_opacity_entries() {
    let mut document = Document::new("Visible profile bounds");
    let page = document.pages[0].id;
    let mut node = line(0.0, 0.0, 100.0, profile(12.0, 3.0, StrokeCap::Butt));
    let mut disabled = profile(12.0, 100.0, StrokeCap::Round);
    disabled.visible = false;
    let mut transparent = disabled.clone();
    transparent.visible = true;
    transparent.opacity = 0.0;
    node.strokes.extend([disabled, transparent]);
    let id = document.insert_node(page, node).unwrap();
    assert_eq!(
        view_box(&document.export_svg(page, &[id])),
        Rect::new(0.0, -18.0, 100.0, 36.0)
    );
}
