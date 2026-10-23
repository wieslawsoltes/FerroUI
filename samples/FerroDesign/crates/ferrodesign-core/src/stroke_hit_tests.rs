use super::*;
use crate::*;

fn line() -> Node {
    let mut node = Node::new(NodeKind::Line, "Wedge", 20.0, 30.0, 100.0, 0.0);
    node.fills.clear();
    node.strokes = vec![Stroke { width_profile: StrokeWidthProfile::Wedge,
        ..Stroke::new(Color::BLACK, 40.0) }];
    node
}

#[test]
fn variable_width_picking_uses_real_ink_and_retains_geometry_for_placement_and_paint() {
    let mut node = line();
    let hits = VariableStrokeHits::prepare(&node).unwrap();
    assert!(hits.hit(Point::new(10.0, 15.0), 0.0));
    assert!(!hits.hit(Point::new(90.0, 15.0), 0.0));
    assert!(hits.hit(Point::new(90.0, 2.5), 1.0));
    assert!(!node.hit_test_local(Point::new(90.0, 15.0), 0.0));
    let retained = hits.strokes[0].1.contours.as_ptr();
    for _ in 0..100 { assert!(hits.hit(Point::new(10.0, 15.0), 0.0)); }
    assert_eq!(retained, hits.strokes[0].1.contours.as_ptr());
    node.x += 50.0; node.rotation = 42.0; node.strokes[0].paint = Paint::Solid(Color::WHITE);
    assert!(hits.matches(&node));
    node.strokes[0].width_profile = StrokeWidthProfile::Eye;
    assert!(!hits.matches(&node));
}

#[test]
fn variable_width_scene_bounds_transform_ink_and_respect_frame_clip() {
    let mut document = Document::new("Affine variable ink");
    let page = document.pages[0].id;
    let mut node = line();
    let transform = Transform { a: 0.8, b: 0.4, c: -0.2, d: 1.3, e: 50.0, f: 60.0 };
    node.set_local_transform(transform);
    let id = document.insert_node(page, node).unwrap();
    let mut scene = SceneIndex::default(); scene.rebuild(&document, page);
    assert_eq!(scene.hit_test(&document, transform.apply(Point::new(10.0, 15.0)), 0.0, false), Some(id));
    assert_eq!(scene.hit_test(&document, transform.apply(Point::new(90.0, 15.0)), 0.0, false), None);
    document.nodes.get_mut(&id).unwrap().strokes[0].width_profile = StrokeWidthProfile::Uniform;
    scene.update_nodes(&document, page, &[id]);
    assert_eq!(scene.hit_test(&document, transform.apply(Point::new(90.0, 15.0)), 0.0, false), Some(id));

    let mut frame = Node::new(NodeKind::Frame, "Clip", 200.0, 100.0, 100.0, 100.0);
    frame.clip_content = true; frame.corner_radius = 20.0; frame.fills.clear();
    frame.strokes = vec![Stroke { align: StrokeAlign::Outside,
        width_profile: StrokeWidthProfile::Custom { points: vec![StrokeWidthPoint::new(0.0, 2.0)] },
        ..Stroke::new(Color::BLACK, 20.0) }];
    let frame = document.insert_node(page, frame).unwrap();
    let mut child = line(); child.x = -20.0; child.y = -5.0;
    let child = document.insert_node(frame, child).unwrap();
    scene.rebuild(&document, page);
    assert!(!scene.hit_test_candidates(&document, Point::new(260.0, 95.0), 0.0, false).contains(&child),
        "outside frame stroke must not extend the child's clip");
    assert!(!scene.hit_test_candidates(&document, Point::new(201.0, 101.0), 0.0, false).contains(&child),
        "rounded frame clip excludes the corner even when its stroke is wide");
}

#[test]
fn aligned_variable_width_hits_clip_the_doubled_band_only_once() {
    let mut node = Node::new(NodeKind::Rectangle, "Aligned", 0.0, 0.0, 100.0, 100.0);
    node.fills.clear();
    node.strokes = vec![Stroke { align: StrokeAlign::Inside,
        width_profile: StrokeWidthProfile::Custom { points: vec![StrokeWidthPoint::new(0.0, 2.0)] },
        ..Stroke::new(Color::BLACK, 10.0) }];
    let inside = VariableStrokeHits::prepare(&node).unwrap();
    assert!(inside.hit(Point::new(50.0, 18.0), 0.0));
    assert!(!inside.hit(Point::new(50.0, -5.0), 0.0));
    assert!(!inside.hit(Point::new(50.0, 25.0), 0.0));
    node.strokes[0].align = StrokeAlign::Outside;
    let outside = VariableStrokeHits::prepare(&node).unwrap();
    assert!(outside.hit(Point::new(50.0, -18.0), 0.0));
    assert!(!outside.hit(Point::new(50.0, 5.0), 0.0));
    assert!(!outside.hit(Point::new(50.0, -25.0), 0.0));
}
