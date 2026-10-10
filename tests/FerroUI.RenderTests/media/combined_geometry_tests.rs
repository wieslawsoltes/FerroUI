//! Port of upstream's `Media/CombinedGeometryTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::{
    Brushes, CombinedGeometry, EllipseGeometry, GeometryCombineMode, RectangleGeometry, RotateTransform,
};
use ferroui_base::{Point, Rect};
use ferroui_controls::shapes::Path;
use ferroui_controls::Border;

fn base() -> TestBase {
    TestBase::new(r"Media\CombinedGeometry")
}

fn geometry_combine_mode(mode: GeometryCombineMode) {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(Some(Brushes::white()));
    let path = Path::new();
    let data = CombinedGeometry::new();
    data.set_geometry_combine_mode(mode);
    data.set_geometry1(RectangleGeometry::with_rect(Rect::new(25.0, 25.0, 100.0, 100.0)));
    let ellipse = EllipseGeometry::new();
    ellipse.set_center(Point::new(125.0, 125.0));
    ellipse.set_radius_x(50.0);
    ellipse.set_radius_y(50.0);
    data.set_geometry2(ellipse);
    path.set_data(data);
    path.set_fill(Some(Brushes::blue()));
    path.set_stroke(Some(Brushes::red()));
    path.set_stroke_thickness(1.0);
    target.set_child(path);

    let mode_name = match mode {
        GeometryCombineMode::Union => "Union",
        GeometryCombineMode::Intersect => "Intersect",
        GeometryCombineMode::Xor => "Xor",
        GeometryCombineMode::Exclude => "Exclude",
    };
    let test_name = format!("GeometryCombineMode_{mode_name}");
    t.render_to_file(&target, &test_name);
    t.compare_images(&test_name);
}

#[test]
fn geometry_combine_mode_union() {
    geometry_combine_mode(GeometryCombineMode::Union);
}

#[test]
fn geometry_combine_mode_intersect() {
    geometry_combine_mode(GeometryCombineMode::Intersect);
}

#[test]
fn geometry_combine_mode_xor() {
    geometry_combine_mode(GeometryCombineMode::Xor);
}

#[test]
fn geometry_combine_mode_exclude() {
    geometry_combine_mode(GeometryCombineMode::Exclude);
}

#[test]
fn geometry1_transform() {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(Some(Brushes::white()));
    let path = Path::new();
    let data = CombinedGeometry::new();
    let rectangle = RectangleGeometry::with_rect(Rect::new(25.0, 25.0, 100.0, 100.0));
    rectangle.set_transform(RotateTransform::with_angle_and_center(45.0, 75.0, 75.0));
    data.set_geometry1(rectangle);
    let ellipse = EllipseGeometry::new();
    ellipse.set_center(Point::new(125.0, 125.0));
    ellipse.set_radius_x(50.0);
    ellipse.set_radius_y(50.0);
    data.set_geometry2(ellipse);
    path.set_data(data);
    path.set_fill(Some(Brushes::blue()));
    path.set_stroke(Some(Brushes::red()));
    path.set_stroke_thickness(1.0);
    target.set_child(path);

    t.render_to_file(&target, "Geometry1_Transform");
    t.compare_images("Geometry1_Transform");
}
