//! Port of upstream's `CrossTests/Brushes/RadialGradientBrushTests.cs`
//! (the class `CrossRadialGradientBrushTests`).

use crate::cross_test_base::CrossTestBase;
use crate::cross_ui::*;
use ferroui_base::media::{BrushMappingMode, Colors, GradientSpreadMethod, GradientStop};
use ferroui_base::{Matrix, Point, Rect};
use std::rc::Rc;

fn base() -> CrossTestBase {
    CrossTestBase::new("Media/RadialGradientBrush")
}

#[test]
fn transform_should_work_as_expected() {
    let t = base();
    let mut root = CrossControl::new();
    root.children.push({
        let mut child = CrossFuncControl::new(|ctx| {
            let geo: CrossGeometry =
                CrossEllipseGeometry::new(Rect::new(3.430200000000003, 29.019099999999998, 42.7692, 19.6732)).into();
            ctx.draw_geometry(Some(&CrossSolidColorBrush::new(Colors::MAGENTA).into()), None, &geo);
            let mut brush = CrossRadialGradientBrush::default();
            brush.radius_x = 12.289;
            brush.radius_y = 12.289;
            brush.gradient_origin = Point::new(15.116, 63.965);
            brush.center = Point::new(15.116, 63.965);
            brush.mapping_mode = BrushMappingMode::Absolute;
            brush.spread_method = GradientSpreadMethod::Pad;
            brush.gradient_stops.push(GradientStop::with_color_and_offset(Colors::BLACK, 0.0));
            brush.gradient_stops.push(GradientStop::with_color_and_offset(Colors::TRANSPARENT, 1.0));
            brush.transform = Some(Matrix::new(1.664, 0.0, 0.0, 0.75621371, -0.06567275, -10.272));
            ctx.draw_geometry(Some(&brush.into()), None, &geo);
        });
        child.width = 48.0;
        child.height = 48.0;
        child.render_transform = Matrix::create_scale(4.0, 4.0);
        Rc::new(child)
    });
    root.width = 256.0;
    root.height = 256.0;
    root.background = CrossSolidColorBrush::new(Colors::WHITE).into();

    t.render_and_compare(root, "Transform_Should_Work_As_Expected");
}
