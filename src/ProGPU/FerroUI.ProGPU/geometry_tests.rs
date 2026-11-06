//! The geometries of the backend against closed forms: bounds, fills, strokes, Boolean
//! combination, widening, transforms and the path measure.

use crate::geometry::{PathBuilder, PathData, ProGpuGeometry, ProGpuStreamGeometryBuilder};
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{GeometryCombineMode, GeometryPathSegment, IntersectionResult, PenLineCap, PenLineJoin, SweepDirection};
use ferroui_base::platform::{IGeometryContext, IGeometryImpl, IStreamGeometryBuilder};
use ferroui_base::{Matrix, Point, Rect, Size};

fn pen(thickness: f64, cap: PenLineCap) -> ImmutablePen {
    ImmutablePen::new(None, thickness, None, cap, PenLineJoin::Miter, 10.0)
}

/// The engine is linked statically: its geometry queries are always available.
fn available() -> bool {
    progpu_native_sys::abi_matches()
}

fn near(value: f64, expected: f64, tolerance: f64) -> bool {
    (value - expected).abs() <= tolerance
}

#[test]
fn rectangles_have_exact_bounds_fills_and_strokes() {
    if !available() {
        return;
    }
    let geometry = ProGpuGeometry::new(PathData::rectangle(Rect::new(10.0, 20.0, 100.0, 50.0)));
    assert_eq!(Rect::new(10.0, 20.0, 100.0, 50.0), geometry.bounds());
    assert!(geometry.fill_contains(Point::new(50.0, 40.0)));
    assert!(!geometry.fill_contains(Point::new(5.0, 40.0)));
    let pen = pen(4.0, PenLineCap::Flat);
    assert_eq!(Rect::new(8.0, 18.0, 104.0, 54.0), geometry.get_render_bounds(Some(&pen)));
    assert!(geometry.stroke_contains(Some(&pen), Point::new(11.0, 40.0)));
    assert!(!geometry.stroke_contains(Some(&pen), Point::new(50.0, 40.0)));
    assert!(near(geometry.contour_length(), 300.0, 1e-3), "length {}", geometry.contour_length());
    let (position, tangent) = geometry.try_get_point_and_tangent_at_distance(125.0).unwrap();
    assert!(near(position.x, 110.0, 1e-3) && near(position.y, 45.0, 1e-3));
    assert!(near(tangent.x, 0.0, 1e-6) && near(tangent.y, 1.0, 1e-6));
}

#[test]
fn ellipses_are_exact_arcs() {
    if !available() {
        return;
    }
    let circle = ProGpuGeometry::new(PathData::ellipse(Rect::new(0.0, 0.0, 100.0, 100.0)));
    let bounds = circle.bounds();
    assert!(near(bounds.x, 0.0, 1e-3) && near(bounds.y, 0.0, 1e-3) && near(bounds.width, 100.0, 1e-3), "{bounds:?}");
    // Lengths are sums of chords within half a unit of the curve (the tolerance of Skia's contour
    // measure, which the Skia backend has too): a little short of the circumference.
    let length = circle.contour_length();
    assert!(length < std::f64::consts::PI * 100.0 && near(length, std::f64::consts::PI * 100.0, 1.0), "length {length}");
    assert!(circle.fill_contains(Point::new(50.0, 50.0)));
    assert!(!circle.fill_contains(Point::new(5.0, 5.0)));
    let point = circle.try_get_point_at_distance(std::f64::consts::PI * 25.0).unwrap();
    assert!(near(point.x, 50.0, 0.2) && near(point.y, 100.0, 0.2), "a quarter turn clockwise from the right: {point:?}");
}

#[test]
fn combination_widening_and_transforms() {
    if !available() {
        return;
    }
    let a = ProGpuGeometry::new(PathData::rectangle(Rect::new(0.0, 0.0, 100.0, 100.0)));
    let b = ProGpuGeometry::new(PathData::rectangle(Rect::new(50.0, 50.0, 100.0, 100.0)));
    let inside = ProGpuGeometry::new(PathData::rectangle(Rect::new(10.0, 10.0, 10.0, 10.0)));
    assert_eq!(IntersectionResult::Intersects, a.get_fill_intersection_result(b.as_ref()));
    assert_eq!(IntersectionResult::FullyContains, a.get_fill_intersection_result(inside.as_ref()));
    let both = a.intersect(b.as_ref()).unwrap();
    assert_eq!(Rect::new(50.0, 50.0, 50.0, 50.0), both.bounds());
    let union = a.data().combine(b.data(), GeometryCombineMode::Union).unwrap();
    assert_eq!(Rect::new(0.0, 0.0, 150.0, 150.0), union.bounds());

    let line = ProGpuGeometry::new(PathData::line(Point::new(0.0, 0.0), Point::new(100.0, 0.0)));
    let square = pen(10.0, PenLineCap::Square);
    let widened = line.get_widened_geometry(&square);
    assert_eq!(Rect::new(-5.0, -5.0, 110.0, 10.0), widened.bounds());
    assert!(widened.fill_contains(Point::new(104.0, 0.0)) && !widened.fill_contains(Point::new(50.0, 6.0)));

    let moved = a.with_transform(Matrix::create_translation(10.0, 20.0));
    assert_eq!(Rect::new(10.0, 20.0, 100.0, 100.0), moved.bounds());
    assert!(moved.fill_contains(Point::new(105.0, 115.0)));
}

#[test]
fn stream_geometries_record_arcs_curves_and_pieces() {
    if !available() {
        return;
    }
    let mut builder = ProGpuStreamGeometryBuilder::new(None);
    builder.begin_figure(Point::new(0.0, 50.0), false);
    builder.arc_to(Point::new(100.0, 50.0), Size::new(50.0, 50.0), 0.0, false, SweepDirection::Clockwise, true);
    builder.cubic_bezier_to(Point::new(100.0, 100.0), Point::new(0.0, 100.0), Point::new(0.0, 50.0), true);
    builder.end_figure(false);
    let geometry = builder.to_geometry();
    let bounds = geometry.bounds();
    // The clockwise half turn from the left goes over the top, y from 0.
    assert!(near(bounds.y, 0.0, 1e-3) && near(bounds.x, 0.0, 1e-3) && near(bounds.width, 100.0, 1e-3), "{bounds:?}");
    let half = std::f64::consts::PI * 50.0;
    assert!(geometry.contour_length() > half, "the arc then the curve");
    let piece = geometry.try_get_segment(0.0, half, true).unwrap();
    assert!(near(piece.contour_length(), half, 0.5), "the arc alone: {}", piece.contour_length());
    let data = geometry.try_get_path_data().unwrap();
    let stroke = data.stroke.unwrap();
    assert!(matches!(stroke.segments[0], GeometryPathSegment::MoveTo(_)));
    assert!(stroke.segments.iter().filter(|segment| matches!(segment, GeometryPathSegment::CubicTo { .. })).count() >= 3);
    assert!(data.fill.is_none(), "an unfilled figure has no fill path");

    let mut path = PathBuilder::default();
    path.begin_figure(Point::new(0.0, 0.0), true);
    path.line_to(Point::new(10.0, 0.0), true);
    path.line_to(Point::new(10.0, 10.0), true);
    path.end_figure(true);
    let triangle = path.finish();
    assert_eq!(3, triangle.segments().len(), "a closed figure returns to its start explicitly");
}
