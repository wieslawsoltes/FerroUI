//! Port of `Rendering/SceneGraph/RenderDataStreamBoundsTests.cs`.

use super::scene_graph_test_support::{b, black, black_pen, p, rrect, TestBitmapImpl, TestCustomOperation};
use crate::media::BoxShadows;
use crate::platform::IBitmapImpl;
use crate::rendering::composition::drawing::RenderDataStream;
use crate::{Matrix, Point, Rect};
use std::rc::Rc;

#[test]
fn empty_stream_has_null_bounds() {
    let stream = RenderDataStream::new();
    assert_eq!(None, stream.calculate_bounds());
}

#[test]
fn filled_rectangle_bounds_are_the_rectangle() {
    let mut stream = RenderDataStream::new();
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());

    assert_eq!(Some(Rect::new(0.0, 0.0, 10.0, 10.0)), stream.calculate_bounds());
}

#[test]
fn stroked_rectangle_bounds_are_inflated_by_half_thickness() {
    let pen = black_pen(4.0);

    let mut stream = RenderDataStream::new();
    stream.draw_rectangle(None, p(&pen), p(&pen), rrect(10.0, 10.0, 20.0, 20.0), &BoxShadows::default());

    assert_eq!(Some(Rect::new(8.0, 8.0, 24.0, 24.0)), stream.calculate_bounds());
}

#[test]
fn bounds_are_the_union_of_all_draws() {
    let mut stream = RenderDataStream::new();
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    stream.draw_rectangle(b(&black()), None, None, rrect(20.0, 20.0, 10.0, 10.0), &BoxShadows::default());

    assert_eq!(Some(Rect::new(0.0, 0.0, 30.0, 30.0)), stream.calculate_bounds());
}

#[test]
fn stroked_ellipse_bounds_are_inflated_by_thickness() {
    let pen = black_pen(4.0);

    let mut stream = RenderDataStream::new();
    stream.draw_ellipse(None, p(&pen), p(&pen), Rect::new(0.0, 0.0, 10.0, 10.0));

    assert_eq!(Some(Rect::new(-4.0, -4.0, 18.0, 18.0)), stream.calculate_bounds());
}

#[test]
fn bitmap_bounds_are_the_destination_rect() {
    let bitmap: Rc<dyn IBitmapImpl> = Rc::new(TestBitmapImpl);

    let mut stream = RenderDataStream::new();
    stream.draw_bitmap(Some(bitmap), 1.0, Rect::new(0.0, 0.0, 10.0, 10.0), Rect::new(5.0, 5.0, 20.0, 20.0));

    assert_eq!(Some(Rect::new(5.0, 5.0, 20.0, 20.0)), stream.calculate_bounds());
}

#[test]
fn custom_operation_bounds_are_used() {
    let operation = TestCustomOperation::with_bounds(Rect::new(1.0, 2.0, 3.0, 4.0));

    let mut stream = RenderDataStream::new();
    stream.draw_custom(Some(operation));

    assert_eq!(Some(Rect::new(1.0, 2.0, 3.0, 4.0)), stream.calculate_bounds());
}

#[test]
fn line_bounds_cover_the_segment() {
    let pen = black_pen(2.0);

    let mut stream = RenderDataStream::new();
    stream.draw_line(p(&pen), p(&pen), Point::new(0.0, 0.0), Point::new(100.0, 0.0));

    let bounds = stream.calculate_bounds().expect("the line has bounds");
    assert!(bounds.contains(Point::new(50.0, 0.0)));
}

#[test]
fn transform_is_applied_to_bounds() {
    let mut stream = RenderDataStream::new();
    stream.push_transform(Matrix::create_translation(50.0, 50.0));
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    stream.pop();

    assert_eq!(Some(Rect::new(50.0, 50.0, 10.0, 10.0)), stream.calculate_bounds());
}

#[test]
fn nested_transforms_compose_for_bounds() {
    let mut stream = RenderDataStream::new();
    stream.push_transform(Matrix::create_translation(20.0, 20.0));
    stream.push_transform(Matrix::create_scale(2.0, 2.0));
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    stream.pop();
    stream.pop();

    assert_eq!(Some(Rect::new(20.0, 20.0, 20.0, 20.0)), stream.calculate_bounds());
}

#[test]
fn clip_does_not_restrict_bounds() {
    let mut stream = RenderDataStream::new();
    stream.push_clip(rrect(0.0, 0.0, 5.0, 5.0));
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 100.0, 100.0), &BoxShadows::default());
    stream.pop();

    assert_eq!(Some(Rect::new(0.0, 0.0, 100.0, 100.0)), stream.calculate_bounds());
}

#[test]
fn opacity_push_is_transparent_to_bounds() {
    let mut stream = RenderDataStream::new();
    stream.push_opacity(0.5);
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    stream.pop();

    assert_eq!(Some(Rect::new(0.0, 0.0, 10.0, 10.0)), stream.calculate_bounds());
}

#[test]
fn empty_push_scope_contributes_nothing() {
    let mut stream = RenderDataStream::new();
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    stream.push_transform(Matrix::create_translation(1000.0, 1000.0));
    stream.pop();

    assert_eq!(Some(Rect::new(0.0, 0.0, 10.0, 10.0)), stream.calculate_bounds());
}

#[test]
fn bounds_handles_deeply_nested_scopes() {
    let mut stream = RenderDataStream::new();
    for _ in 0..100 {
        stream.push_opacity(0.5);
    }
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    for _ in 0..100 {
        stream.pop();
    }

    assert_eq!(Some(Rect::new(0.0, 0.0, 10.0, 10.0)), stream.calculate_bounds());
}
