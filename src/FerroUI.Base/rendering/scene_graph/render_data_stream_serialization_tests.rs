//! Port of `Rendering/SceneGraph/RenderDataStreamSerializationTests.cs`.
//!
//! The batch stream of the port is one growable buffer; it has no memory
//! pool of fixed-size segments, so `Round_Trip_Spanning_Multiple_Stream_Segments`
//! writes the same fifty rectangles without the 64-byte segments of upstream.

use super::scene_graph_test_support::{b, black, replay, rrect};
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{BoxShadows, Colors, IBrush};
use crate::rendering::composition::drawing::{RenderDataResource, RenderDataStream};
use crate::rendering::composition::transport::{BatchStreamData, BatchStreamReader, BatchStreamWriter};
use crate::{Matrix, Point, Rect};
use std::rc::Rc;

#[test]
fn round_trip_preserves_bounds() {
    let mut source = RenderDataStream::new();
    source.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    source.push_transform(Matrix::create_translation(40.0, 40.0));
    source.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    source.pop();

    let result = round_trip(&source);

    assert_eq!(source.calculate_bounds(), result.calculate_bounds());
    assert_eq!(Some(Rect::new(0.0, 0.0, 50.0, 50.0)), result.calculate_bounds());
}

#[test]
fn round_trip_preserves_hit_testing() {
    let mut source = RenderDataStream::new();
    source.push_clip(rrect(0.0, 0.0, 10.0, 10.0));
    source.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 100.0, 100.0), &BoxShadows::default());
    source.pop();

    let result = round_trip(&source);

    assert!(result.hit_test(Point::new(5.0, 5.0)));
    assert!(!result.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn round_trip_preserves_resource_references() {
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    let mut source = RenderDataStream::new();
    source.draw_rectangle(b(&brush), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());

    let result = round_trip(&source);

    // `DrawRectangle(brush, null, ..)` once, with the very brush.
    assert_eq!(replay(&result), ["DrawRectangle Red none 0, 0, 10, 10 shadows=0"]);
    match result.get_resource(0) {
        Some(RenderDataResource::Brush(replayed)) => assert!(Rc::ptr_eq(replayed, &brush)),
        _ => panic!("the brush did not survive the round trip"),
    }
}

#[test]
fn round_trip_of_empty_stream_produces_empty_stream() {
    let source = RenderDataStream::new();
    let result = round_trip(&source);

    assert_eq!(None, result.calculate_bounds());
}

#[test]
fn round_trip_spanning_multiple_stream_segments() {
    let mut source = RenderDataStream::new();
    for i in 0..50 {
        let i = f64::from(i);
        source.draw_rectangle(b(&black()), None, None, rrect(i, i, 1.0, 1.0), &BoxShadows::default());
    }

    let result = round_trip(&source);

    assert_eq!(source.calculate_bounds(), result.calculate_bounds());
}

fn round_trip(source: &RenderDataStream) -> RenderDataStream {
    let mut data = BatchStreamData::new();
    source.serialize_to(&mut BatchStreamWriter::new(&mut data));

    let mut result = RenderDataStream::new();
    let mut reader = BatchStreamReader::new(&mut data);
    result.deserialize_from(&mut reader, &|_| None);

    result
}
