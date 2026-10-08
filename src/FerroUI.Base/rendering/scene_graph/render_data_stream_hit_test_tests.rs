//! Port of `Rendering/SceneGraph/RenderDataStreamHitTestTests.cs`.

use super::scene_graph_test_support::{
    b, black, black_pen, p, rrect, TestBitmapImpl, TestCustomOperation, TestGeometryImpl,
};
use crate::media::BoxShadows;
use crate::platform::IBitmapImpl;
use crate::rendering::composition::drawing::{RenderDataResource, RenderDataStream};
use crate::{Matrix, Point, Rect};

#[test]
fn filled_rectangle_is_hit_inside_and_missed_outside() {
    let mut stream = RenderDataStream::new();
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 100.0, 100.0), &BoxShadows::default());

    assert!(stream.hit_test(Point::new(50.0, 50.0)));
    assert!(!stream.hit_test(Point::new(150.0, 150.0)));
}

#[test]
fn stroked_rectangle_is_hit_on_border_and_missed_in_hollow_center() {
    let pen = black_pen(4.0);

    let mut stream = RenderDataStream::new();
    stream.draw_rectangle(None, p(&pen), p(&pen), rrect(0.0, 0.0, 100.0, 100.0), &BoxShadows::default());

    assert!(stream.hit_test(Point::new(0.0, 50.0)));
    assert!(!stream.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn line_is_hit_along_its_length() {
    let pen = black_pen(4.0);

    let mut stream = RenderDataStream::new();
    stream.draw_line(p(&pen), p(&pen), Point::new(0.0, 0.0), Point::new(100.0, 0.0));

    assert!(stream.hit_test(Point::new(50.0, 1.0)));
    assert!(!stream.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn filled_ellipse_is_hit_at_center_and_missed_at_corner() {
    let mut stream = RenderDataStream::new();
    stream.draw_ellipse(b(&black()), None, None, Rect::new(0.0, 0.0, 100.0, 100.0));

    assert!(stream.hit_test(Point::new(50.0, 50.0)));
    assert!(!stream.hit_test(Point::new(2.0, 2.0)));
}

#[test]
fn geometry_is_hit_via_fill_contains() {
    let geometry = TestGeometryImpl::with_fill(&[Point::new(5.0, 5.0)]);

    let mut stream = RenderDataStream::new();
    stream.draw_geometry(b(&black()), None, None, Some(RenderDataResource::GeometryImpl(geometry)));

    assert!(stream.hit_test(Point::new(5.0, 5.0)));
    assert!(!stream.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn bitmap_is_hit_within_its_destination_rect() {
    let bitmap: std::sync::Arc<crate::platform::SharedBitmapImpl> = std::sync::Arc::new(TestBitmapImpl);

    let mut stream = RenderDataStream::new();
    stream.draw_bitmap(Some(bitmap), 1.0, Rect::new(0.0, 0.0, 10.0, 10.0), Rect::new(20.0, 20.0, 30.0, 30.0));

    assert!(stream.hit_test(Point::new(25.0, 25.0)));
    assert!(!stream.hit_test(Point::new(5.0, 5.0)));
}

#[test]
fn custom_operation_hit_test_is_delegated() {
    let operation = TestCustomOperation::with_hits(&[Point::new(5.0, 5.0)]);

    let mut stream = RenderDataStream::new();
    stream.draw_custom(Some(operation));

    assert!(stream.hit_test(Point::new(5.0, 5.0)));
    assert!(!stream.hit_test(Point::new(99.0, 99.0)));
}

#[test]
fn clip_restricts_the_hit_region() {
    let mut stream = RenderDataStream::new();
    stream.push_clip(rrect(0.0, 0.0, 10.0, 10.0));
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 100.0, 100.0), &BoxShadows::default());
    stream.pop();

    assert!(stream.hit_test(Point::new(5.0, 5.0)));
    assert!(!stream.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn geometry_clip_restricts_the_hit_region() {
    let geometry = TestGeometryImpl::with_fill(&[Point::new(5.0, 5.0)]);

    let mut stream = RenderDataStream::new();
    stream.push_geometry_clip(Some(RenderDataResource::GeometryImpl(geometry)));
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 100.0, 100.0), &BoxShadows::default());
    stream.pop();

    assert!(stream.hit_test(Point::new(5.0, 5.0)));
    assert!(!stream.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn transform_maps_hit_test_coordinates() {
    let mut stream = RenderDataStream::new();
    stream.push_transform(Matrix::create_translation(50.0, 50.0));
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    stream.pop();

    assert!(stream.hit_test(Point::new(55.0, 55.0)));
    assert!(!stream.hit_test(Point::new(5.0, 5.0)));
}

#[test]
fn nested_transforms_compose() {
    let mut stream = RenderDataStream::new();
    stream.push_transform(Matrix::create_translation(20.0, 20.0));
    stream.push_transform(Matrix::create_scale(2.0, 2.0));
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    stream.pop();
    stream.pop();

    assert!(stream.hit_test(Point::new(30.0, 30.0)));
    assert!(!stream.hit_test(Point::new(5.0, 5.0)));
}

#[test]
fn singular_transform_excludes_its_scope() {
    let mut stream = RenderDataStream::new();
    stream.push_transform(Matrix::default());
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 100.0, 100.0), &BoxShadows::default());
    stream.pop();

    assert!(!stream.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn opacity_push_is_transparent_to_hit_testing() {
    let mut stream = RenderDataStream::new();
    stream.push_opacity(0.5);
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 100.0, 100.0), &BoxShadows::default());
    stream.pop();

    assert!(stream.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn scope_state_is_restored_after_pop() {
    let mut stream = RenderDataStream::new();
    stream.push_transform(Matrix::create_translation(1000.0, 1000.0));
    stream.pop();
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());

    assert!(stream.hit_test(Point::new(5.0, 5.0)));
}

#[test]
fn hit_test_handles_deeply_nested_scopes() {
    let mut stream = RenderDataStream::new();
    for _ in 0..100 {
        stream.push_opacity(0.5);
    }
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    for _ in 0..100 {
        stream.pop();
    }

    assert!(stream.hit_test(Point::new(5.0, 5.0)));
    assert!(!stream.hit_test(Point::new(50.0, 50.0)));
}
