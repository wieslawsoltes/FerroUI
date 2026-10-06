//! Port of `Rendering/SceneGraph/RenderDataStreamLineHitTestTests.cs`.

use crate::media::{Brushes, IPen, Pen};
use crate::rendering::composition::drawing::{RenderDataResource, RenderDataStream};
use crate::Point;
use std::rc::Rc;

fn line_stream(pen: Rc<dyn IPen>, p1: Point, p2: Point) -> RenderDataStream {
    let mut stream = RenderDataStream::new();
    stream.draw_line(Some(RenderDataResource::Pen(pen.clone())), Some(RenderDataResource::Pen(pen)), p1, p2);
    stream
}

/// `new Pen(Brushes.Black, 3)`.
fn pen() -> Rc<dyn IPen> {
    Pen::with_brush(Some(Brushes::black()), 3.0).into()
}

#[test]
fn hit_test_should_be_true() {
    let stream = line_stream(pen(), Point::new(15.0, 10.0), Point::new(150.0, 73.0));

    let points_inside = [
        Point::new(14.0, 8.9),
        Point::new(15.0, 10.0),
        Point::new(30.0, 15.5),
        Point::new(30.0, 18.5),
        Point::new(150.0, 73.0),
        Point::new(151.0, 71.9),
    ];

    for point in points_inside {
        assert!(stream.hit_test(point), "{point}");
    }
}

#[test]
fn hit_test_should_be_false() {
    let stream = line_stream(pen(), Point::new(15.0, 10.0), Point::new(150.0, 73.0));

    let points_outside = [
        Point::new(14.0, 8.0),
        Point::new(14.0, 8.8),
        Point::new(30.0, 15.3),
        Point::new(30.0, 18.7),
        Point::new(151.0, 71.8),
        Point::new(155.0, 75.0),
    ];

    for point in points_outside {
        assert!(!stream.hit_test(point), "{point}");
    }
}
