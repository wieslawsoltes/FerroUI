//! Port of upstream's `Media/StreamGeometryTests.cs` of the Skia unit tests.

use crate::unit_tests::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::{StreamGeometry, TranslateTransform};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{Point, Rect};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn open_should_append_to_existing_figures() {
    let _app = start();

    let geometry = StreamGeometry::new();
    add_line(&geometry, Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    add_line(&geometry, Point::new(20.0, 20.0), Point::new(30.0, 30.0));

    assert_eq!(Rect::new(0.0, 0.0, 30.0, 30.0), geometry.bounds());
}

#[test]
fn open_should_raise_changed() {
    let _app = start();

    let geometry = StreamGeometry::new();
    add_line(&geometry, Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    let raised = Rc::new(Cell::new(false));
    let r = raised.clone();
    geometry.changed(move || r.set(true)).detach();

    add_line(&geometry, Point::new(20.0, 20.0), Point::new(30.0, 30.0));

    assert!(raised.get());
}

#[test]
fn open_should_update_transformed_geometry() {
    let _app = start();

    let geometry = StreamGeometry::new();
    geometry.set_transform(TranslateTransform::with_offset(50.0, 50.0));
    add_line(&geometry, Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    assert_eq!(Rect::new(50.0, 50.0, 10.0, 10.0), geometry.bounds());

    add_line(&geometry, Point::new(20.0, 20.0), Point::new(30.0, 30.0));
    assert_eq!(Rect::new(50.0, 50.0, 30.0, 30.0), geometry.bounds());
}

#[test]
fn open_should_update_contour_length() {
    let _app = start();

    let geometry = StreamGeometry::new();
    assert_eq!(0.0, geometry.contour_length());

    add_line(&geometry, Point::new(0.0, 0.0), Point::new(100.0, 0.0));
    assert_equal_precision(100.0, geometry.contour_length(), 3);
}

#[test]
fn open_should_not_affect_clone() {
    let _app = start();

    let geometry = StreamGeometry::new();
    add_line(&geometry, Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    let clone = geometry.clone_geometry();

    add_line(&geometry, Point::new(20.0, 20.0), Point::new(30.0, 30.0));

    assert_eq!(Rect::new(0.0, 0.0, 10.0, 10.0), clone.bounds());
    assert_eq!(Rect::new(0.0, 0.0, 30.0, 30.0), geometry.bounds());
}

#[test]
fn clone_should_keep_transform() {
    let _app = start();

    let geometry = StreamGeometry::parse("M10,190 l190,-190").unwrap();
    geometry.set_transform(TranslateTransform::with_offset(50.0, 50.0));

    let clone = geometry.clone_geometry();

    assert!(clone.transform().is_some());
    assert_eq!(geometry.transform().unwrap().value(), clone.transform().unwrap().value());
    assert_eq!(geometry.bounds(), clone.bounds());
}

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        mock_platform_render_interface().with_render_interface(Rc::new(PlatformRenderInterface::default())),
    )
}

fn add_line(geometry: &StreamGeometry, start: Point, end: Point) {
    let mut context = geometry.open();
    context.begin_figure(start, false);
    context.line_to(end, true);
    context.end_figure(false);
    context.dispose();
}

/// xUnit's `Assert.Equal(double expected, double actual, int precision)`:
/// both values rounded to `precision` decimal places (`Math.Round`, to even).
fn assert_equal_precision(expected: f64, actual: f64, precision: i32) {
    let factor = 10f64.powi(precision);
    let expected_rounded = (expected * factor).round_ties_even() / factor;
    let actual_rounded = (actual * factor).round_ties_even() / factor;

    assert_eq!(
        expected_rounded, actual_rounded,
        "Values differ at precision {precision}: expected {expected}, actual {actual}"
    );
}
