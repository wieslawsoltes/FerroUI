//! Port of upstream's `Shapes/PolylineTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::{Brushes, FillRule, PenLineCap, PenLineJoin, Points, Stretch};
use ferroui_base::{Point, Thickness};
use ferroui_controls::shapes::Polyline;
use ferroui_controls::Decorator;

fn base() -> TestBase {
    TestBase::new(r"Shapes\Polyline")
}

fn polyline_fill_rule(fill_rule: FillRule) {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(260.0);
    target.set_height(180.0);
    let child = Polyline::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(2.0);
    child.set_fill(Some(Brushes::orange_red()));
    child.set_points(Some(Points::from_items([
        Point::new(10.0, 170.0),
        Point::new(60.0, 20.0),
        Point::new(110.0, 170.0),
        Point::new(20.0, 70.0),
        Point::new(240.0, 70.0),
        Point::new(130.0, 170.0),
        Point::new(190.0, 20.0),
        Point::new(10.0, 170.0),
    ])));
    child.set_stretch(Stretch::Uniform);
    child.set_fill_rule(fill_rule);
    target.set_child(child);

    let test_name = format!(
        "Polyline_FillRule_{}",
        match fill_rule {
            FillRule::EvenOdd => "EvenOdd",
            FillRule::NonZero => "NonZero",
        }
    );
    t.render_to_file(&target, &test_name);
    t.compare_images(&test_name);
}

#[test]
fn polyline_fill_rule_even_odd() {
    polyline_fill_rule(FillRule::EvenOdd);
}

#[test]
fn polyline_fill_rule_non_zero() {
    polyline_fill_rule(FillRule::NonZero);
}

#[test]
fn polyline_fill_rule_no_fill() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(260.0);
    target.set_height(180.0);
    let child = Polyline::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(2.0);
    child.set_fill(None);
    child.set_points(Some(Points::from_items([
        Point::new(10.0, 170.0),
        Point::new(60.0, 20.0),
        Point::new(110.0, 170.0),
        Point::new(20.0, 70.0),
        Point::new(240.0, 70.0),
        Point::new(130.0, 170.0),
        Point::new(190.0, 20.0),
        Point::new(10.0, 170.0),
    ])));
    child.set_stretch(Stretch::Uniform);
    child.set_fill_rule(FillRule::EvenOdd);
    target.set_child(child);

    t.render_to_file(&target, "Polyline_FillRule_NoFill");
    t.compare_images("Polyline_FillRule_NoFill");
}

#[test]
fn polyline_1px_stroke() {
    let t = base();
    let polyline_points = Points::from_items([
        Point::new(0.0, 0.0),
        Point::new(5.0, 0.0),
        Point::new(6.0, -2.0),
        Point::new(7.0, 3.0),
        Point::new(8.0, -3.0),
        Point::new(9.0, 1.0),
        Point::new(10.0, 0.0),
        Point::new(15.0, 0.0),
    ]);

    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(400.0);
    target.set_height(200.0);
    let child = Polyline::new();
    child.set_stroke(Some(Brushes::brown()));
    child.set_points(Some(polyline_points));
    child.set_stretch(Stretch::Uniform);
    child.set_stroke_thickness(1.0);
    target.set_child(child);

    t.render_to_file(&target, "Polyline_1px_Stroke");
    t.compare_images("Polyline_1px_Stroke");
}

#[test]
fn polyline_10px_stroke_pen_line_join() {
    let t = base();
    let polyline_points = Points::from_items([
        Point::new(0.0, 0.0),
        Point::new(5.0, 0.0),
        Point::new(6.0, -2.0),
        Point::new(7.0, 3.0),
        Point::new(8.0, -3.0),
        Point::new(9.0, 1.0),
        Point::new(10.0, 0.0),
        Point::new(15.0, 0.0),
    ]);

    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(400.0);
    target.set_height(200.0);
    let child = Polyline::new();
    child.set_stroke(Some(Brushes::brown()));
    child.set_points(Some(polyline_points));
    child.set_stretch(Stretch::Uniform);
    child.set_stroke_join(PenLineJoin::Round);
    child.set_stroke_line_cap(PenLineCap::Round);
    child.set_stroke_thickness(10.0);
    target.set_child(child);

    t.render_to_file(&target, "Polyline_10px_Stroke_PenLineJoin");
    t.compare_images("Polyline_10px_Stroke_PenLineJoin");
}
