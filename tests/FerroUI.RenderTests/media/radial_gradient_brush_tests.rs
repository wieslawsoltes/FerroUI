//! Port of upstream's `Media/RadialGradientBrushTests.cs`.

use crate::media::relative_point_test_primitives_helper::RelativePointTestPrimitivesHelper;
use crate::test_base::TestBase;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, Color, Colors, DrawingContext, GradientSpreadMethod, GradientStop, IBrush, RadialGradientBrush,
    RotateTransform,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Rect, Ref, RelativePoint, RelativeScalar,
    RelativeUnit, StyledElementImpl, Thickness, VisualImpl,
};
use ferroui_controls::{Border, Control, ControlImpl, Decorator};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Media\RadialGradientBrush")
}

/// `new GradientStop { Color = color, Offset = offset }`.
fn gradient_stop(color: Color, offset: f64) -> Ref<GradientStop> {
    let stop = GradientStop::new();
    stop.set_color(color);
    stop.set_offset(offset);
    stop
}

#[test]
fn radial_gradient_brush_partial_cover() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = RadialGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::WHITE, 0.0));
    background.gradient_stops().add(gradient_stop(Color::parse("#00DD00").unwrap(), 0.7));
    background.set_gradient_origin(RelativePoint::new(0.7, 0.15, RelativeUnit::Relative));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "RadialGradientBrush_Partial_Cover");
    t.compare_images("RadialGradientBrush_Partial_Cover");
}

#[test]
fn radial_gradient_brush_red_blue() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = RadialGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "RadialGradientBrush_RedBlue");
    t.compare_images("RadialGradientBrush_RedBlue");
}

/// Tests using a GradientOrigin that falls inside of the circle described by Center/Radius.
#[test]
fn radial_gradient_brush_red_blue_offset_inside() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = RadialGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    background.set_gradient_origin(RelativePoint::new(0.25, 0.25, RelativeUnit::Relative));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "RadialGradientBrush_RedBlue_Offset_Inside");
    t.compare_images("RadialGradientBrush_RedBlue_Offset_Inside");
}

/// Tests using a GradientOrigin that falls outside of the circle described by Center/Radius.
#[test]
fn radial_gradient_brush_red_blue_offset_outside() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = RadialGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    background.set_gradient_origin(RelativePoint::new(0.1, 0.1, RelativeUnit::Relative));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "RadialGradientBrush_RedBlue_Offset_Outside");
    t.compare_images("RadialGradientBrush_RedBlue_Offset_Outside");
}

/// Tests using a GradientOrigin that falls inside of the circle described by Center/Radius.
#[test]
fn radial_gradient_brush_red_green_blue_offset_inside() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = RadialGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::GREEN, 0.5));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    background.set_gradient_origin(RelativePoint::new(0.25, 0.25, RelativeUnit::Relative));
    background.set_center(RelativePoint::new(0.5, 0.5, RelativeUnit::Relative));
    background.set_radius_x(RelativeScalar::MIDDLE);
    background.set_radius_y(RelativeScalar::MIDDLE);
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "RadialGradientBrush_RedGreenBlue_Offset_Inside");
    t.compare_images("RadialGradientBrush_RedGreenBlue_Offset_Inside");
}

/// Tests using a GradientOrigin that falls outside of the circle described by Center/Radius.
#[test]
fn radial_gradient_brush_red_green_blue_offset_outside() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = RadialGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::GREEN, 0.25));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    background.set_gradient_origin(RelativePoint::new(0.1, 0.1, RelativeUnit::Relative));
    background.set_center(RelativePoint::new(0.5, 0.5, RelativeUnit::Relative));
    background.set_radius_x(RelativeScalar::MIDDLE);
    background.set_radius_y(RelativeScalar::MIDDLE);
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "RadialGradientBrush_RedGreenBlue_Offset_Outside");
    t.compare_images("RadialGradientBrush_RedGreenBlue_Offset_Outside");
}

#[test]
fn radial_gradient_brush_drawing_context() {
    let t = base();
    let brush = RadialGradientBrush::new();
    brush.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    brush.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    let brush: Rc<dyn IBrush> = brush.into();

    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_child(DrawnControl::new(move |c| {
        c.draw_rectangle(Some(&brush), None, Rect::new(0.0, 0.0, 100.0, 100.0), 0.0, 0.0, &BoxShadows::default());
        let transform = c.push_transform(Matrix::create_translation(100.0, 100.0));
        c.draw_rectangle(Some(&brush), None, Rect::new(0.0, 0.0, 100.0, 100.0), 0.0, 0.0, &BoxShadows::default());
        c.pop(transform);
    }));

    t.render_to_file(&target, "RadialGradientBrush_DrawingContext");
    t.compare_images("RadialGradientBrush_DrawingContext");
}

#[repr(C)]
struct DrawnControl {
    base: Control,
    render: Box<dyn Fn(&mut DrawingContext)>,
}

ferro_class!(DrawnControl: Control);
ferro_impl_classes!(
    DrawnControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl DrawnControl {
    fn new(render: impl Fn(&mut DrawingContext) + 'static) -> Ref<Self> {
        instantiate(Self { base: Control::construct(), render: Box::new(render) })
    }
}

impl VisualImpl for DrawnControl {
    fn render(this: &Self, context: &mut DrawingContext) {
        (this.render)(context)
    }
}

fn radial_gradient_brush_is_properly_mapped(relative: bool, move_origin: bool) {
    let t = base();
    let center = if relative { RelativePoint::CENTER } else { RelativePoint::new(128.0, 128.0, RelativeUnit::Absolute) };
    let brush = RadialGradientBrush::new();
    brush.set_center(center);
    brush.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    brush.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    brush.set_spread_method(if move_origin { GradientSpreadMethod::Pad } else { GradientSpreadMethod::Repeat });
    brush.set_radius_x(if relative { RelativeScalar::MIDDLE } else { RelativeScalar::new(128.0, RelativeUnit::Absolute) });
    brush.set_radius_y(if relative { RelativeScalar::MIDDLE } else { RelativeScalar::new(64.0, RelativeUnit::Absolute) });
    brush.set_gradient_origin(if move_origin {
        if relative {
            RelativePoint::new(0.1, 0.1, RelativeUnit::Relative)
        } else {
            RelativePoint::new(32.0, 32.0, RelativeUnit::Absolute)
        }
    } else {
        center
    });

    let test_name = format!(
        "RadialGradientBrush_Is_Properly_Mapped_{}_{}",
        if relative { "Relative" } else { "Absolute" },
        if move_origin { "MovedOrigin" } else { "CenterOrigin" }
    );
    t.render_to_file(RelativePointTestPrimitivesHelper::with_shadow(Some(brush.into()), !relative), &test_name);
    t.compare_images(&test_name);
}

#[test]
fn radial_gradient_brush_is_properly_mapped_false_false() {
    radial_gradient_brush_is_properly_mapped(false, false);
}

#[test]
fn radial_gradient_brush_is_properly_mapped_false_true() {
    radial_gradient_brush_is_properly_mapped(false, true);
}

#[test]
fn radial_gradient_brush_is_properly_mapped_true_false() {
    radial_gradient_brush_is_properly_mapped(true, false);
}

#[test]
fn radial_gradient_brush_is_properly_mapped_true_true() {
    radial_gradient_brush_is_properly_mapped(true, true);
}

fn radial_gradient_brush_with_different_radius_is_properly_rotated(move_origin: bool) {
    let t = base();
    let brush = RadialGradientBrush::new();
    brush.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    brush.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    brush.set_gradient_origin(if move_origin {
        RelativePoint::new(0.1, 0.1, RelativeUnit::Relative)
    } else {
        RelativePoint::CENTER
    });
    brush.set_radius_y(RelativeScalar::new(0.25, RelativeUnit::Relative));
    brush.set_transform(Some(RotateTransform::with_angle(45.0).into()));
    brush.set_transform_origin(RelativePoint::CENTER);

    let test_name = format!(
        "RadialGradientBrush_With_Different_Radius_Is_Properly_Rotated_{}",
        if move_origin { "MovedOrigin" } else { "CenterOrigin" }
    );

    let target = Border::new();
    target.set_background(Some(brush.into()));
    target.set_width(256.0);
    target.set_height(256.0);
    target.set_min_height(256.0);
    t.render_to_file(target, &test_name);
    t.compare_images(&test_name);
}

#[test]
fn radial_gradient_brush_with_different_radius_is_properly_rotated_false() {
    radial_gradient_brush_with_different_radius_is_properly_rotated(false);
}

#[test]
fn radial_gradient_brush_with_different_radius_is_properly_rotated_true() {
    radial_gradient_brush_with_different_radius_is_properly_rotated(true);
}
