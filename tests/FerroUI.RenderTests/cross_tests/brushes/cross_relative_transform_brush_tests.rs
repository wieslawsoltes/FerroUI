//! Port of upstream's `CrossTests/Brushes/CrossRelativeTransformBrushTests.cs`.
//!
//! Upstream finds `Ramp64.png` next to the test assembly; here it is in the
//! `Assets` directory of the test crate.

use crate::cross_test_base::CrossTestBase;
use crate::cross_ui::*;
use crate::test_render_helper;
use ferroui_base::media::{BrushMappingMode, Colors, GradientStop, TileMode};
use ferroui_base::{Matrix, Point, Rect};

fn base() -> CrossTestBase {
    CrossTestBase::new("Media/RelativeTransformBrush")
}

// A rotation about an off-centre point of the unit square: bounds-dependent once conjugated
// into target space, and asymmetric enough to show on a centre-symmetric radial gradient,
// where a rotation about the centre would be invisible.
fn unit_rotation() -> Matrix {
    Matrix::create_translation(-0.3, -0.7)
        * Matrix::create_rotation(Matrix::to_radians(30.0))
        * Matrix::create_translation(0.3, 0.7)
}

#[test]
fn linear_gradient_relative_transform_should_work_as_expected() {
    let t = base();
    t.render_and_compare(
        scene(|ctx| {
            ctx.draw_rectangle(
                Some(&linear(Some(unit_rotation()), None).into()),
                None,
                Rect::new(40.0, 70.0, 120.0, 60.0),
            )
        }),
        "Linear_Gradient_Relative_Transform_Should_Work_As_Expected",
    );
}

#[test]
fn radial_gradient_relative_transform_should_work_as_expected() {
    let t = base();
    t.render_and_compare(
        scene(|ctx| {
            ctx.draw_rectangle(
                Some(&radial(Some(unit_rotation()), None).into()),
                None,
                Rect::new(40.0, 70.0, 120.0, 60.0),
            )
        }),
        "Radial_Gradient_Relative_Transform_Should_Work_As_Expected",
    );
}

#[test]
fn relative_transform_should_apply_before_transform() {
    let t = base();
    // A scale rather than a translation: it does not commute with the relative rotation, so the
    // two orders are far enough apart to be told apart by the comparison.
    t.render_and_compare(
        scene(|ctx| {
            ctx.draw_rectangle(
                Some(&linear(Some(unit_rotation()), Some(Matrix::create_scale(0.4, 0.4))).into()),
                None,
                Rect::new(40.0, 70.0, 120.0, 60.0),
            )
        }),
        "Relative_Transform_Should_Apply_Before_Transform",
    );
}

#[test]
fn relative_transform_should_follow_the_bounds_of_each_fill() {
    let t = base();
    // One brush instance, two differently sized rects: the unit space is each rect's own, so
    // the two fills must differ in shape.
    t.render_and_compare(
        scene(|ctx| {
            let shared: CrossBrush = linear(Some(unit_rotation()), None).into();
            ctx.draw_rectangle(Some(&shared), None, Rect::new(40.0, 30.0, 120.0, 60.0));
            ctx.draw_rectangle(Some(&shared), None, Rect::new(30.0, 110.0, 60.0, 90.0));
        }),
        "Relative_Transform_Should_Follow_The_Bounds_Of_Each_Fill",
    );
}

#[test]
fn solid_color_relative_transform_should_be_ignored() {
    let t = base();
    t.render_and_compare(
        scene(|ctx| {
            let mut brush = CrossSolidColorBrush::new(Colors::CRIMSON);
            brush.relative_transform = Some(unit_rotation());
            ctx.draw_rectangle(Some(&brush.into()), None, Rect::new(40.0, 70.0, 120.0, 60.0))
        }),
        "Solid_Color_Relative_Transform_Should_Be_Ignored",
    );
}

#[test]
fn drawing_brush_relative_transform_should_work_as_expected() {
    let t = base();
    t.render_and_compare(
        scene(|ctx| {
            ctx.draw_rectangle(
                Some(&checkerboard(Some(unit_rotation()), None).into()),
                None,
                Rect::new(40.0, 70.0, 120.0, 60.0),
            )
        }),
        "Drawing_Brush_Relative_Transform_Should_Work_As_Expected",
    );
}

#[test]
fn tiled_drawing_brush_relative_transform_should_work_as_expected() {
    let t = base();
    let mut brush = checkerboard(Some(unit_rotation()), None);
    brush.tile_mode = TileMode::Tile;
    brush.viewport = Rect::new(0.0, 0.0, 30.0, 30.0);
    brush.viewport_units = BrushMappingMode::Absolute;
    let brush: CrossBrush = brush.into();

    t.render_and_compare(
        scene(move |ctx| ctx.draw_rectangle(Some(&brush), None, Rect::new(40.0, 70.0, 120.0, 60.0))),
        "Tiled_Drawing_Brush_Relative_Transform_Should_Work_As_Expected",
    );
}

#[test]
fn image_brush_relative_transform_should_work_as_expected() {
    let t = base();
    t.render_and_compare(
        scene(|ctx| {
            let mut brush = CrossImageBrush::new(&ramp_image_path());
            brush.relative_transform = Some(unit_rotation());
            ctx.draw_rectangle(Some(&brush.into()), None, Rect::new(40.0, 70.0, 120.0, 60.0))
        }),
        "Image_Brush_Relative_Transform_Should_Work_As_Expected",
    );
}

// A smooth two-axis ramp rather than a line drawing: every pixel carries position, so a
// misplaced transform shows up as a colour shift over the whole fill instead of hiding in the
// resampling noise the two backends produce differently along hard edges.
#[test]
fn tiled_image_brush_relative_and_absolute_transform_should_work_as_expected() {
    let t = base();
    // Tiling and an absolute viewport put the tile offset and the destination translate into
    // the shader matrix, so this is where the slot the two brush transforms occupy matters.
    t.render_and_compare(
        scene(|ctx| {
            let mut brush = CrossImageBrush::new(&ramp_image_path());
            brush.tile_mode = TileMode::Tile;
            brush.viewport = Rect::new(0.0, 0.0, 40.0, 40.0);
            brush.viewport_units = BrushMappingMode::Absolute;
            brush.relative_transform = Some(unit_rotation());
            brush.transform = Some(Matrix::create_scale(0.6, 0.6));
            ctx.draw_rectangle(Some(&brush.into()), None, Rect::new(40.0, 70.0, 120.0, 60.0))
        }),
        "Tiled_Image_Brush_Relative_And_Absolute_Transform_Should_Work_As_Expected",
    );
}

fn ramp_image_path() -> String {
    test_render_helper::get_tests_directory()
        .join("Assets")
        .join("Ramp64.png")
        .to_str()
        .expect("the path is text")
        .to_string()
}

fn scene(render: impl Fn(&mut dyn ICrossDrawingContext) + 'static) -> CrossControl {
    let mut control = CrossFuncControl::new(render);
    control.width = 200.0;
    control.height = 200.0;
    control.background = CrossSolidColorBrush::new(Colors::WHITE).into();
    control
}

fn linear(relative: Option<Matrix>, absolute: Option<Matrix>) -> CrossLinearGradientBrush {
    let mut brush = CrossLinearGradientBrush::default();
    brush.gradient_stops.push(GradientStop::with_color_and_offset(Colors::RED, 0.0));
    brush.gradient_stops.push(GradientStop::with_color_and_offset(Colors::BLUE, 1.0));
    brush.start_point = Point::new(0.0, 0.0);
    brush.end_point = Point::new(1.0, 0.0);
    brush.relative_transform = relative;
    brush.transform = absolute;
    brush
}

fn radial(relative: Option<Matrix>, absolute: Option<Matrix>) -> CrossRadialGradientBrush {
    let mut brush = CrossRadialGradientBrush::default();
    brush.gradient_stops.push(GradientStop::with_color_and_offset(Colors::RED, 0.0));
    brush.gradient_stops.push(GradientStop::with_color_and_offset(Colors::BLUE, 1.0));
    brush.center = Point::new(0.5, 0.5);
    brush.gradient_origin = Point::new(0.5, 0.5);
    brush.radius_x = 0.5;
    brush.radius_y = 0.5;
    brush.relative_transform = relative;
    brush.transform = absolute;
    brush
}

fn checkerboard(relative: Option<Matrix>, absolute: Option<Matrix>) -> CrossDrawingBrush {
    let rectangle = |rect: Rect, brush: CrossSolidColorBrush| -> CrossDrawing {
        let mut drawing = CrossGeometryDrawing::new(CrossRectangleGeometry::new(rect));
        drawing.brush = brush.into();
        drawing.into()
    };
    let mut brush = CrossDrawingBrush::new(CrossDrawingGroup {
        children: vec![
            rectangle(Rect::new(0.0, 0.0, 20.0, 20.0), CrossSolidColorBrush::new(Colors::CRIMSON)),
            rectangle(Rect::new(0.0, 0.0, 10.0, 10.0), CrossSolidColorBrush::new(Colors::MIDNIGHT_BLUE)),
            rectangle(Rect::new(10.0, 10.0, 10.0, 10.0), CrossSolidColorBrush::new(Colors::MIDNIGHT_BLUE)),
        ],
    });
    brush.relative_transform = relative;
    brush.transform = absolute;
    brush
}
