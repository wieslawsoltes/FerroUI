use super::Polyline;
use crate::test_support::{test_scope, TestRoot};
use crate::test_support_shapes::MockRenderInterfaceScope;
use ferroui_base::media::{Brushes, FillRule, IBrush, Points, PolylineGeometry};
use ferroui_base::{Point, Ref, Size};
use std::rc::Rc;

fn red() -> Option<Rc<dyn IBrush>> {
    Some(Brushes::red())
}

fn triangle(fill: Option<Rc<dyn IBrush>>, fill_rule: FillRule) -> Ref<Polyline> {
    let target = Polyline::new();
    target.set_points(Some(Points::from_items([
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(20.0, 0.0),
    ])));
    target.set_fill(fill);
    target.set_fill_rule(fill_rule);
    target
}

#[test]
fn polyline_will_update_geometry_on_shapes_collection_content_change() {
    let _scope = test_scope();
    let _app = MockRenderInterfaceScope::install();
    let points = Points::new();

    let target = Polyline::new();
    target.set_points(Some(points.clone()));
    target.measure(Size::default());
    assert!(target.is_measure_valid());

    let root = TestRoot::with_child(&target);

    points.add(Point::default());

    assert!(!target.is_measure_valid());

    root.set_child(None);
}

#[test]
fn fill_rule_on_polyline_is_applied_to_defining_geometry() {
    let _app = MockRenderInterfaceScope::install();

    let target = triangle(red(), FillRule::NonZero);

    target.measure(Size::INFINITY);

    let geometry = target.defining_geometry().unwrap().cast::<PolylineGeometry>().expect("a polyline geometry");
    assert_eq!(FillRule::NonZero, geometry.fill_rule());
    assert!(geometry.is_filled());
}

#[test]
fn fill_rule_differs_between_even_odd_and_non_zero() {
    let _app = MockRenderInterfaceScope::install();

    let even_odd = triangle(red(), FillRule::EvenOdd);
    let non_zero = triangle(red(), FillRule::NonZero);

    even_odd.measure(Size::INFINITY);
    non_zero.measure(Size::INFINITY);

    let even_odd_geometry = even_odd.defining_geometry().unwrap().cast::<PolylineGeometry>().unwrap();
    let non_zero_geometry = non_zero.defining_geometry().unwrap().cast::<PolylineGeometry>().unwrap();
    assert_eq!(FillRule::EvenOdd, even_odd_geometry.fill_rule());
    assert_eq!(FillRule::NonZero, non_zero_geometry.fill_rule());
}

#[test]
fn when_fill_is_null_polyline_geometry_is_not_filled() {
    let _app = MockRenderInterfaceScope::install();

    let target = triangle(None, FillRule::NonZero);

    target.measure(Size::INFINITY);
    let geometry = target.defining_geometry().unwrap().cast::<PolylineGeometry>().expect("a polyline geometry");
    assert!(!geometry.is_filled());
}
