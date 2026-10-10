//! Port of upstream's `Media/RelativeTransformBrushTests.cs`.
//!
//! The relative transform of a brush applies in the unit space of the
//! painted bounds, before the absolute transform, so one brush instance
//! serves consumers of different sizes.

use crate::test_base::TestBase;
use ferroui_base::media::immutable::{
    ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableRadialGradientBrush, ImmutableTransform,
};
use ferroui_base::media::{
    Brushes, Colors, DrawingBrush, DrawingGroup, GeometryDrawing, GradientSpreadMethod, IBrush, RectangleGeometry,
    Stretch,
};
use ferroui_base::{Matrix, Rect, Ref};
use ferroui_controls::{Border, Canvas, Control};
use std::rc::Rc;

fn s_stops() -> [ImmutableGradientStop; 2] {
    [ImmutableGradientStop::new(0.0, Colors::RED), ImmutableGradientStop::new(1.0, Colors::BLUE)]
}

// A rotation about an off-center unit-space point: bounds-dependent once
// conjugated, and asymmetric enough that even a center-symmetric radial
// gradient shows it (a rotation about the center would be invisible there).
fn s_unit_matrix() -> Matrix {
    Matrix::create_translation(-0.3, -0.7)
        * Matrix::create_rotation(Matrix::to_radians(30.0))
        * Matrix::create_translation(0.3, 0.7)
}

fn base() -> TestBase {
    TestBase::new(r"Media\RelativeTransformBrush")
}

fn scene(children: &[Ref<Control>]) -> Ref<Canvas> {
    let canvas = Canvas::new();
    canvas.set_width(200.0);
    canvas.set_height(200.0);
    canvas.set_background(Some(Brushes::white()));

    for child in children {
        canvas.children().add(child);
    }

    canvas
}

fn filled(brush: &Rc<dyn IBrush>, left: f64, top: f64, width: f64, height: f64) -> Ref<Control> {
    let border = Border::new();
    border.set_background(Some(brush.clone()));
    border.set_width(width);
    border.set_height(height);
    Canvas::set_left(&border, left);
    Canvas::set_top(&border, top);
    border.upcast()
}

#[test]
fn linear_gradient_rotated_in_unit_space() {
    let t = base();
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableLinearGradientBrush::new(
        &s_stops(),
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        None,
        None,
        Some(Rc::new(ImmutableTransform::new(s_unit_matrix()))),
    ));

    t.render_to_file(scene(&[filled(&brush, 40.0, 70.0, 120.0, 60.0)]), "Linear_Gradient_Rotated_In_Unit_Space");
    t.compare_images("Linear_Gradient_Rotated_In_Unit_Space");
}

#[test]
fn radial_gradient_rotated_in_unit_space() {
    let t = base();
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableRadialGradientBrush::new(
        &s_stops(),
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        None,
        None,
        None,
        None,
        Some(Rc::new(ImmutableTransform::new(s_unit_matrix()))),
    ));

    t.render_to_file(scene(&[filled(&brush, 40.0, 70.0, 120.0, 60.0)]), "Radial_Gradient_Rotated_In_Unit_Space");
    t.compare_images("Radial_Gradient_Rotated_In_Unit_Space");
}

#[test]
fn shared_brush_on_two_different_bounds() {
    let t = base();
    // The gradient must follow each rect's own bounds, so the two fills
    // differ in shape while sharing one brush instance.
    let shared: Rc<dyn IBrush> = Rc::new(ImmutableLinearGradientBrush::new(
        &s_stops(),
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        None,
        None,
        Some(Rc::new(ImmutableTransform::new(s_unit_matrix()))),
    ));

    t.render_to_file(
        scene(&[filled(&shared, 40.0, 30.0, 120.0, 60.0), filled(&shared, 30.0, 110.0, 60.0, 90.0)]),
        "Shared_Brush_On_Two_Different_Bounds",
    );
    t.compare_images("Shared_Brush_On_Two_Different_Bounds");
}

#[test]
fn drawing_brush_rotated_in_unit_space() {
    let t = base();
    let group = DrawingGroup::new();
    let drawing = GeometryDrawing::new();
    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 20.0, 20.0)));
    drawing.set_brush(Some(Brushes::crimson()));
    group.children().add(drawing.upcast());
    let drawing = GeometryDrawing::new();
    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0)));
    drawing.set_brush(Some(Brushes::midnight_blue()));
    group.children().add(drawing.upcast());
    let drawing = GeometryDrawing::new();
    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(10.0, 10.0, 10.0, 10.0)));
    drawing.set_brush(Some(Brushes::midnight_blue()));
    group.children().add(drawing.upcast());
    let brush = DrawingBrush::with_drawing(group);
    brush.set_stretch(Stretch::Fill);
    brush.set_relative_transform(Some(Rc::new(ImmutableTransform::new(s_unit_matrix()))));
    let brush: Rc<dyn IBrush> = brush.into();

    t.render_to_file(scene(&[filled(&brush, 40.0, 70.0, 120.0, 60.0)]), "Drawing_Brush_Rotated_In_Unit_Space");
    t.compare_images("Drawing_Brush_Rotated_In_Unit_Space");
}

#[test]
fn relative_transform_applies_before_absolute() {
    let t = base();
    // A scale, not a translation: it does not commute with the relative
    // rotation, so applying the two the other way round moves the image far
    // enough to fail this golden. A translation stayed inside the tolerance.
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableLinearGradientBrush::new(
        &s_stops(),
        1.0,
        Some(Rc::new(ImmutableTransform::new(Matrix::create_scale(0.4, 0.4)))),
        None,
        GradientSpreadMethod::Pad,
        None,
        None,
        Some(Rc::new(ImmutableTransform::new(s_unit_matrix()))),
    ));

    t.render_to_file(scene(&[filled(&brush, 40.0, 70.0, 120.0, 60.0)]), "Relative_Transform_Applies_Before_Absolute");
    t.compare_images("Relative_Transform_Applies_Before_Absolute");
}
