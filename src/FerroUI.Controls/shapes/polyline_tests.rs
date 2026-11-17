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

#[test]
fn polyline_should_invalidate_geometry_when_points_collection_changes() {
    let _app = MockRenderInterfaceScope::install();

    let points = Points::from_items([Point::new(0.0, 0.0)]);
    let target = Polyline::new();
    target.set_points(Some(points.clone()));
    target.measure(Size::INFINITY);

    let geometry = target.defining_geometry();
    assert!(geometry.is_some());
    let invalidated = Rc::new(std::cell::Cell::new(false));
    let i = invalidated.clone();
    geometry.unwrap().changed(move || i.set(true)).detach();

    points.add(Point::new(10.0, 10.0));

    assert!(invalidated.get());
}

/// Section 9.88: a polyline given another collection of points keeps its geometry, which takes
/// the points, follows the new collection and is built again.
#[test]
fn a_polyline_given_other_points_keeps_its_geometry_which_takes_them() {
    let _scope = test_scope();
    let _app = MockRenderInterfaceScope::install();

    let target = triangle(None, FillRule::EvenOdd);
    target.measure(Size::INFINITY);
    let geometry = target.defining_geometry().expect("a geometry");
    let changes = Rc::new(std::cell::Cell::new(0));
    let counted = changes.clone();
    geometry.changed(move || counted.set(counted.get() + 1)).detach();

    let other = Points::from_items([Point::new(0.0, 0.0), Point::new(30.0, 5.0)]);
    target.set_points(Some(other.clone()));
    assert!(!target.is_measure_valid());
    assert_eq!(changes.get(), 1, "the geometry says it changed");
    target.measure(Size::INFINITY);
    let kept = target.defining_geometry().expect("a geometry");
    assert!(kept.ptr_eq(&geometry), "the same geometry");
    let polyline = kept.cast::<PolylineGeometry>().expect("a polyline geometry");
    assert_eq!(polyline.points().to_vec(), other.to_vec());

    // It follows the collection it was given, and no longer the one it had. (A shape in a tree
    // listens to its geometry and is measured again:
    // `polyline_will_update_geometry_on_shapes_collection_content_change`.)
    other.add(Point::new(40.0, 0.0));
    assert_eq!(changes.get(), 2);

    // Without points the geometry is let go, as before; with points again there is a new one.
    target.set_points(None);
    target.measure(Size::INFINITY);
    let empty = target.defining_geometry().expect("a geometry");
    assert!(!empty.ptr_eq(&geometry));
    assert_eq!(empty.cast::<PolylineGeometry>().expect("a polyline geometry").points().len(), 0);
}

/// The geometry of a polyline is made of its own collection of points, not of a copy.
#[test]
fn the_geometry_of_a_polyline_has_the_collection_of_the_polyline() {
    let _scope = test_scope();
    let _app = MockRenderInterfaceScope::install();

    let points = Points::from_items([Point::new(0.0, 0.0), Point::new(10.0, 10.0)]);
    let target = Polyline::new();
    target.set_points(Some(points.clone()));
    target.set_fill(red());
    target.measure(Size::INFINITY);
    let geometry = target.defining_geometry().and_then(|g| g.cast::<PolylineGeometry>()).expect("a polyline geometry");
    assert!(geometry.is_filled());
    points.add(Point::new(20.0, 0.0));
    assert_eq!(geometry.points().len(), 3);
}

