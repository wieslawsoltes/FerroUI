use super::{Polygon, Polyline};
use crate::test_support::{test_scope, TestRoot};
use crate::test_support_shapes::MockRenderInterfaceScope;
use ferroui_base::media::{FillRule, Points, PolylineGeometry};
use ferroui_base::{Point, Size};

#[test]
fn polygon_will_update_geometry_on_shapes_collection_content_change() {
    let _scope = test_scope();
    let _app = MockRenderInterfaceScope::install();
    let points = Points::new();

    let target = Polygon::new();
    target.set_points(Some(points.clone()));
    target.measure(Size::default());
    assert!(target.is_measure_valid());

    let root = TestRoot::with_child(&target);

    points.add(Point::default());

    assert!(!target.is_measure_valid());

    root.set_child(None);
}

#[test]
fn fill_rule_on_polygon_is_applied_to_defining_geometry() {
    let _app = MockRenderInterfaceScope::install();

    let target = Polygon::new();
    target.set_points(Some(Points::from_items([
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(20.0, 0.0),
    ])));
    target.set_fill_rule(FillRule::NonZero);

    target.measure(Size::INFINITY);

    let geometry = target.defining_geometry().unwrap().cast::<PolylineGeometry>().expect("a polyline geometry");
    assert_eq!(FillRule::NonZero, geometry.fill_rule());
}

#[test]
fn polygon_equals_closed_polyline_bounds() {
    let _app = MockRenderInterfaceScope::install();

    let polyline = Polyline::new();
    polyline.set_points(Some(Points::from_items([
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
        Point::new(0.0, 0.0),
    ])));
    polyline.set_fill_rule(FillRule::NonZero);

    let polygon = Polygon::new();
    polygon.set_points(Some(Points::from_items([
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
    ])));
    polygon.set_fill_rule(FillRule::NonZero);

    polyline.measure(Size::INFINITY);
    polygon.measure(Size::INFINITY);

    assert_eq!(polygon.defining_geometry().unwrap().bounds(), polyline.defining_geometry().unwrap().bounds());
}
