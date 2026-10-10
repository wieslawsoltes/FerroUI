//! Port of upstream's `Media/GeometryGroupTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::{Brushes, EllipseGeometry, FillRule, GeometryGroup, RectangleGeometry, RotateTransform};
use ferroui_base::{Point, Rect};
use ferroui_controls::shapes::Path;
use ferroui_controls::Border;

fn base() -> TestBase {
    TestBase::new(r"Media\GeometryGroup")
}

fn fill_rule_stroke(fill_rule: FillRule) {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(Some(Brushes::white()));
    let path = Path::new();
    let group = GeometryGroup::new();
    group.set_fill_rule(fill_rule);
    group.children().add(RectangleGeometry::with_rect(Rect::new(25.0, 25.0, 100.0, 100.0)).upcast());
    let ellipse = EllipseGeometry::new();
    ellipse.set_center(Point::new(125.0, 125.0));
    ellipse.set_radius_x(50.0);
    ellipse.set_radius_y(50.0);
    group.children().add(ellipse.upcast());
    path.set_data(group);
    path.set_fill(Some(Brushes::blue()));
    path.set_stroke(Some(Brushes::red()));
    path.set_stroke_thickness(1.0);
    target.set_child(path);

    let fill_rule_name = match fill_rule {
        FillRule::EvenOdd => "EvenOdd",
        FillRule::NonZero => "NonZero",
    };
    let test_name = format!("FillRule_Stroke_{fill_rule_name}");
    t.render_to_file(&target, &test_name);
    t.compare_images(&test_name);
}

#[test]
fn fill_rule_stroke_even_odd() {
    fill_rule_stroke(FillRule::EvenOdd);
}

#[test]
fn fill_rule_stroke_non_zero() {
    fill_rule_stroke(FillRule::NonZero);
}

#[test]
fn child_transform() {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(Some(Brushes::white()));
    let path = Path::new();
    let group = GeometryGroup::new();
    let rectangle = RectangleGeometry::with_rect(Rect::new(25.0, 25.0, 100.0, 100.0));
    rectangle.set_transform(RotateTransform::with_angle_and_center(45.0, 75.0, 75.0));
    group.children().add(rectangle.upcast());
    let ellipse = EllipseGeometry::new();
    ellipse.set_center(Point::new(125.0, 125.0));
    ellipse.set_radius_x(50.0);
    ellipse.set_radius_y(50.0);
    group.children().add(ellipse.upcast());
    path.set_data(group);
    path.set_fill(Some(Brushes::blue()));
    path.set_stroke(Some(Brushes::red()));
    path.set_stroke_thickness(1.0);
    target.set_child(path);

    t.render_to_file(&target, "Child_Transform");
    t.compare_images("Child_Transform");
}
