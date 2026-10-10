//! Port of upstream's `Media/LinearGradientBrushTests.cs`.

use crate::media::relative_point_test_primitives_helper::RelativePointTestPrimitivesHelper;
use crate::test_base::TestBase;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, Color, Colors, DrawingContext, GradientSpreadMethod, GradientStop, IBrush, LinearGradientBrush,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Rect, Ref, RelativePoint, RelativeUnit,
    StyledElementImpl, Thickness, VisualImpl,
};
use ferroui_controls::{Border, Control, ControlImpl, Decorator};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Media\LinearGradientBrush")
}

/// `new GradientStop { Color = color, Offset = offset }`.
fn gradient_stop(color: Color, offset: f64) -> Ref<GradientStop> {
    let stop = GradientStop::new();
    stop.set_color(color);
    stop.set_offset(offset);
    stop
}

#[test]
fn linear_gradient_brush_red_blue_horizontal_fill() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = LinearGradientBrush::new();
    background.set_start_point(RelativePoint::new(0.0, 0.5, RelativeUnit::Relative));
    background.set_end_point(RelativePoint::new(1.0, 0.5, RelativeUnit::Relative));
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "LinearGradientBrush_RedBlue_Horizontal_Fill");
    t.compare_images("LinearGradientBrush_RedBlue_Horizontal_Fill");
}

#[test]
fn linear_gradient_brush_red_blue_vertical_fill() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    let background = LinearGradientBrush::new();
    background.set_start_point(RelativePoint::new(0.5, 0.0, RelativeUnit::Relative));
    background.set_end_point(RelativePoint::new(0.5, 1.0, RelativeUnit::Relative));
    background.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    background.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    child.set_background(Some(background.into()));
    target.set_child(child);

    t.render_to_file(&target, "LinearGradientBrush_RedBlue_Vertical_Fill");
    t.compare_images("LinearGradientBrush_RedBlue_Vertical_Fill");
}

#[test]
fn linear_gradient_brush_drawing_context() {
    let t = base();
    let brush = LinearGradientBrush::new();
    brush.set_start_point(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
    brush.set_end_point(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative));
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

    t.render_to_file(&target, "LinearGradientBrush_DrawingContext");
    t.compare_images("LinearGradientBrush_DrawingContext");
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

fn linear_gradient_brush_is_properly_mapped(relative: bool) {
    let t = base();
    let brush = LinearGradientBrush::new();
    brush.set_start_point(if relative {
        RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)
    } else {
        RelativePoint::new(50.0, 0.0, RelativeUnit::Absolute)
    });
    brush.set_end_point(if relative {
        RelativePoint::new(1.0, 1.0, RelativeUnit::Relative)
    } else {
        RelativePoint::new(150.0, 0.0, RelativeUnit::Absolute)
    });
    brush.gradient_stops().add(gradient_stop(Colors::RED, 0.0));
    brush.gradient_stops().add(gradient_stop(Colors::BLUE, 1.0));
    brush.set_spread_method(GradientSpreadMethod::Repeat);

    let test_name = format!("LinearGradientBrushIsProperlyMapped_{:?}", brush.start_point().unit);
    t.render_to_file(RelativePointTestPrimitivesHelper::new(Some(brush.into())), &test_name);
    t.compare_images(&test_name);
}

#[test]
fn linear_gradient_brush_is_properly_mapped_false() {
    linear_gradient_brush_is_properly_mapped(false);
}

#[test]
fn linear_gradient_brush_is_properly_mapped_true() {
    linear_gradient_brush_is_properly_mapped(true);
}
