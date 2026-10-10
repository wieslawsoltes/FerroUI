//! Port of upstream's `Media/ConicGradientBrushTests.cs`.

use crate::media::relative_point_test_primitives_helper::RelativePointTestPrimitivesHelper;
use crate::test_base::TestBase;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, Color, Colors, ConicGradientBrush, DrawingContext, GradientSpreadMethod, GradientStop, IBrush,
    TranslateTransform,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Rect, Ref, RelativePoint, RelativeUnit,
    StyledElementImpl, Thickness, VisualImpl,
};
use ferroui_controls::{Border, Control, ControlImpl, Decorator};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Media\ConicGradientBrush")
}

/// `new GradientStop { Color = color, Offset = offset }`.
fn gradient_stop(color: Color, offset: f64) -> Ref<GradientStop> {
    let stop = GradientStop::new();
    stop.set_color(color);
    stop.set_offset(offset);
    stop
}

#[test]
fn conic_gradient_brush_red_blue() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = ConicGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "ConicGradientBrush_RedBlue");
    t.compare_images("ConicGradientBrush_RedBlue");
}

#[test]
fn conic_gradient_brush_red_blue_rotation() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = ConicGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    background.set_angle(90.0);
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "ConicGradientBrush_RedBlue_Rotation");
    t.compare_images("ConicGradientBrush_RedBlue_Rotation");
}

#[test]
fn conic_gradient_brush_red_blue_center() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = ConicGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    background.set_center(RelativePoint::new(0.25, 0.25, RelativeUnit::Relative));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "ConicGradientBrush_RedBlue_Center");
    t.compare_images("ConicGradientBrush_RedBlue_Center");
}

#[test]
fn conic_gradient_brush_red_blue_center_and_rotation() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = ConicGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    background.set_center(RelativePoint::new(0.25, 0.25, RelativeUnit::Relative));
    background.set_angle(90.0);
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "ConicGradientBrush_RedBlue_Center_and_Rotation");
    t.compare_images("ConicGradientBrush_RedBlue_Center_and_Rotation");
}

#[test]
fn conic_gradient_brush_red_blue_soft_edge() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = ConicGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 0.5));
    background.gradient_stops().add(gradient_stop(Colors::RED, 1.0));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "ConicGradientBrush_RedBlue_SoftEdge");
    t.compare_images("ConicGradientBrush_RedBlue_SoftEdge");
}

#[test]
fn conic_gradient_brush_umbrella() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = ConicGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::YELLOW, 0.1667));
    background.gradient_stops().add(gradient_stop(Colors::LIME, 0.3333));
    background.gradient_stops().add(gradient_stop(Colors::AQUA, 0.5000));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 0.6667));
    background.gradient_stops().add(gradient_stop(Colors::MAGENTA, 0.8333));
    background.gradient_stops().add(gradient_stop(Colors::RED, 1.0));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "ConicGradientBrush_Umbrella");
    t.compare_images("ConicGradientBrush_Umbrella");
}

#[test]
fn conic_gradient_brush_transform_applies_after_angle() {
    let t = base();
    // The angle belongs to the gradient itself, so the brush transform acts on the swept
    // pattern rather than the other way round: the output is the same gradient centred
    // 30px right and 20px down, and not one whose translation has been turned by the angle.
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = ConicGradientBrush::new();
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::YELLOW, 0.1667));
    background.gradient_stops().add(gradient_stop(Colors::LIME, 0.3333));
    background.gradient_stops().add(gradient_stop(Colors::AQUA, 0.5000));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 0.6667));
    background.gradient_stops().add(gradient_stop(Colors::MAGENTA, 0.8333));
    background.gradient_stops().add(gradient_stop(Colors::RED, 1.0));
    background.set_center(RelativePoint::new(100.0, 100.0, RelativeUnit::Absolute));
    background.set_angle(45.0);
    background.set_transform(Some(TranslateTransform::with_offset(30.0, 20.0).into()));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "ConicGradientBrush_Transform_Applies_After_Angle");
    t.compare_images("ConicGradientBrush_Transform_Applies_After_Angle");
}

#[test]
fn conic_gradient_brush_drawing_context() {
    let t = base();
    let brush = ConicGradientBrush::new();
    brush.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    brush.gradient_stops().add(gradient_stop(Colors::YELLOW, 0.1667));
    brush.gradient_stops().add(gradient_stop(Colors::LIME, 0.3333));
    brush.gradient_stops().add(gradient_stop(Colors::AQUA, 0.5000));
    brush.gradient_stops().add(gradient_stop(Colors::BLUE, 0.6667));
    brush.gradient_stops().add(gradient_stop(Colors::MAGENTA, 0.8333));
    brush.gradient_stops().add(gradient_stop(Colors::RED, 1.0));
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

    t.render_to_file(&target, "ConicGradientBrush_DrawingContext");
    t.compare_images("ConicGradientBrush_DrawingContext");
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

fn conic_gradient_brush_is_properly_mapped(relative: bool) {
    let t = base();
    let brush = ConicGradientBrush::new();
    brush.set_center(if relative {
        RelativePoint::CENTER
    } else {
        RelativePoint::new(128.0, 128.0, RelativeUnit::Absolute)
    });
    brush.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    brush.gradient_stops().add(gradient_stop(Colors::GREEN_YELLOW, 0.2));
    brush.gradient_stops().add(gradient_stop(Colors::MAGENTA, 0.5));
    brush.gradient_stops().add(gradient_stop(Colors::BLUE, 0.8));
    brush.gradient_stops().add(gradient_stop(Colors::RED, 1.0));
    brush.set_spread_method(GradientSpreadMethod::Repeat);
    brush.set_angle(270.0);

    let test_name = format!("ConicGradientBrushIsProperlyMapped_{:?}", brush.center().unit);
    t.render_to_file(RelativePointTestPrimitivesHelper::with_shadow(Some(brush.into()), !relative), &test_name);
    t.compare_images(&test_name);
}

#[test]
fn conic_gradient_brush_is_properly_mapped_false() {
    conic_gradient_brush_is_properly_mapped(false);
}

#[test]
fn conic_gradient_brush_is_properly_mapped_true() {
    conic_gradient_brush_is_properly_mapped(true);
}
