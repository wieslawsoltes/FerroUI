//! Port of upstream's `Shapes/PolygonTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::{Brushes, FillRule, Points, Stretch};
use ferroui_base::{Point, Thickness};
use ferroui_controls::shapes::Polygon;
use ferroui_controls::Decorator;

fn base() -> TestBase {
    TestBase::new(r"Shapes\Polygon")
}

fn polygon_fill_rule(fill_rule: FillRule) {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(220.0);
    target.set_height(220.0);
    let child = Polygon::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(2.0);
    child.set_fill(Some(Brushes::gold()));
    child.set_points(Some(Points::from_items([
        Point::new(50.0, 0.0),
        Point::new(21.0, 90.0),
        Point::new(98.0, 35.0),
        Point::new(2.0, 35.0),
        Point::new(79.0, 90.0),
    ])));
    child.set_stretch(Stretch::Uniform);
    child.set_fill_rule(fill_rule);
    target.set_child(child);

    let test_name = format!(
        "Polygon_FillRule_{}",
        match fill_rule {
            FillRule::EvenOdd => "EvenOdd",
            FillRule::NonZero => "NonZero",
        }
    );
    t.render_to_file(&target, &test_name);
    t.compare_images(&test_name);
}

#[test]
fn polygon_fill_rule_even_odd() {
    polygon_fill_rule(FillRule::EvenOdd);
}

#[test]
fn polygon_fill_rule_non_zero() {
    polygon_fill_rule(FillRule::NonZero);
}

#[test]
fn polygon_1px_stroke() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Polygon::new();
    child.set_stroke(Some(Brushes::dark_blue()));
    child.set_stretch(Stretch::Uniform);
    child.set_fill(Some(Brushes::violet()));
    child.set_points(Some(Points::from_items([
        Point::new(5.0, 0.0),
        Point::new(8.0, 8.0),
        Point::new(0.0, 3.0),
        Point::new(10.0, 3.0),
        Point::new(2.0, 8.0),
    ])));
    child.set_stroke_thickness(1.0);
    target.set_child(child);

    t.render_to_file(&target, "Polygon_1px_Stroke");
    t.compare_images("Polygon_1px_Stroke");
}

#[test]
fn polygon_non_uniform_fill() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(400.0);
    target.set_height(200.0);
    let child = Polygon::new();
    child.set_stroke(Some(Brushes::dark_blue()));
    child.set_stretch(Stretch::Fill);
    child.set_fill(Some(Brushes::violet()));
    child.set_points(Some(Points::from_items([
        Point::new(5.0, 0.0),
        Point::new(8.0, 8.0),
        Point::new(0.0, 3.0),
        Point::new(10.0, 3.0),
        Point::new(2.0, 8.0),
    ])));
    child.set_stroke_thickness(5.0);
    target.set_child(child);

    t.render_to_file(&target, "Polygon_NonUniformFill");
    t.compare_images("Polygon_NonUniformFill");
}
