//! Port of upstream's `Media/GeometryDrawingTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::{Color, EllipseGeometry, GeometryDrawing, Pen, SolidColorBrush};
use ferroui_base::Ref;

fn base() -> TestBase {
    TestBase::new(r"Media\GeometryDrawing")
}

fn create_geometry_drawing() -> Ref<GeometryDrawing> {
    let geometry_drawing = GeometryDrawing::new();
    let ellipse = EllipseGeometry::new();
    ellipse.set_radius_x(100.0);
    ellipse.set_radius_y(100.0);
    geometry_drawing.set_geometry(ellipse);
    geometry_drawing
}

#[test]
fn drawing_geometry_with_pen() {
    let _t = base();
    let geometry_drawing = create_geometry_drawing();
    geometry_drawing.set_pen(Some(
        Pen::with_brush(Some(SolidColorBrush::with_color(Color::from_argb(255, 0, 0, 0)).into()), 10.0).into(),
    ));

    assert_eq!(210.0, geometry_drawing.get_bounds().height);
    assert_eq!(210.0, geometry_drawing.get_bounds().width);
}

#[test]
fn drawing_geometry_without_pen() {
    let _t = base();
    let geometry_drawing = create_geometry_drawing();

    assert_eq!(200.0, geometry_drawing.get_bounds().height);
    assert_eq!(200.0, geometry_drawing.get_bounds().width);
}
