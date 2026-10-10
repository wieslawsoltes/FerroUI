//! Port of upstream's `Media/DrawingContextTests.cs`.

use crate::test_base::{test_font_family, CompareOptions, TestBase};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::effects::{DropShadowEffect, IEffect};
use ferroui_base::media::{
    BoxShadows, Brushes, Colors, DrawingContext, DrawingGroup, DrawingImage, FlowDirection, FormattedText, IBrush,
    IPen, Pen, Stretch, Typeface,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::*;
use ferroui_controls::{Border, Control, ControlImpl, Image};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Media\DrawingContext")
}

#[test]
fn should_render_lines_and_text() {
    let t = base();
    let target = Border::new();
    target.set_width(300.0);
    target.set_height(300.0);
    target.set_background(Some(Brushes::white()));
    target.set_child(RenderControl::new());

    t.render_to_file(&target, "Should_Render_LinesAndText");
    t.compare_images_with("Should_Render_LinesAndText", CompareOptions { skip_immediate: true, ..Default::default() });
}

#[test]
fn should_render_drawing_group_with_effect() {
    let t = base();
    let group = DrawingGroup::new();
    {
        let mut context = group.open();
        let white: Rc<dyn IBrush> = Brushes::white();
        context.draw_rectangle(Some(&white), None, Rect::new(0.0, 0.0, 100.0, 100.0), 0.0, 0.0, &BoxShadows::default());
        let effect = DropShadowEffect::new();
        effect.set_blur_radius(10.0);
        effect.set_color(Colors::BLACK);
        effect.set_opacity(1.0);
        let effect: Rc<dyn IEffect> = effect.into();
        let state = context.push_effect(&effect, Rect::new(0.0, 0.0, 100.0, 100.0));
        {
            let red: Rc<dyn IBrush> = Brushes::red();
            context.draw_rectangle(Some(&red), None, Rect::new(20.0, 20.0, 60.0, 60.0), 0.0, 0.0, &BoxShadows::default());
        }
        context.pop(state);
        context.dispose();
    }

    let target = Border::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_background(Some(Brushes::white()));
    let image = Image::new();
    let source = DrawingImage::new();
    source.set_drawing(group);
    image.set_source(Some(source.into()));
    image.set_stretch(Stretch::None);
    image.set_vertical_alignment(VerticalAlignment::Center);
    image.set_horizontal_alignment(HorizontalAlignment::Center);
    image.set_clip_to_bounds(false);
    target.set_child(image);

    t.render_to_file(&target, "Should_Render_DrawingGroup_With_Effect");
    t.compare_images("Should_Render_DrawingGroup_With_Effect");
}

#[repr(C)]
struct RenderControl {
    base: Control,
}

ferro_class!(RenderControl: Control);
ferro_impl_classes!(
    RenderControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl VisualImpl for RenderControl {
    fn render(_this: &Self, context: &mut DrawingContext) {
        let pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::light_gray()), 10.0).into();
        RenderControl::render_line1(context, &pen);
        RenderControl::render_line2(context, &pen);
        RenderControl::render_line3(context, &pen);
        RenderControl::render_line4(context, &pen);

        RenderControl::render_line1(context, &Pen::with_brush(Some(Brushes::red()), 1.0).into());
        RenderControl::render_a_text(context, Point::new(50.0, 20.0));
        RenderControl::render_line2(context, &Pen::with_brush(Some(Brushes::orange()), 1.0).into());
        RenderControl::render_a_text(context, Point::new(50.0, -50.0));
        RenderControl::render_line3(context, &Pen::with_brush(Some(Brushes::yellow()), 1.0).into());
        RenderControl::render_a_text(context, Point::new(0.0, 0.0));
        RenderControl::render_line4(context, &Pen::with_brush(Some(Brushes::green()), 1.0).into());
    }
}

impl RenderControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct() })
    }

    // `s_typeface`, a static of the class upstream.
    fn typeface() -> Typeface {
        Typeface::new(test_font_family())
    }

    fn render_line1(context: &mut DrawingContext, pen: &Rc<dyn IPen>) {
        context.draw_line(pen, Point::new(100.0, 100.0), Point::new(200.0, 100.0));
    }

    fn render_line2(context: &mut DrawingContext, pen: &Rc<dyn IPen>) {
        context.draw_line(pen, Point::new(200.0, 100.0), Point::new(200.0, 200.0));
    }

    fn render_line3(context: &mut DrawingContext, pen: &Rc<dyn IPen>) {
        context.draw_line(pen, Point::new(200.0, 200.0), Point::new(100.0, 200.0));
    }

    fn render_line4(context: &mut DrawingContext, pen: &Rc<dyn IPen>) {
        context.draw_line(pen, Point::new(100.0, 200.0), Point::new(100.0, 100.0));
    }

    fn render_a_text(context: &mut DrawingContext, point: Point) {
        let state = context.push_opacity(0.7);
        {
            context.draw_text(
                &FormattedText::new(
                    "any text to render",
                    CultureInfo::current_culture(),
                    FlowDirection::LeftToRight,
                    Self::typeface(),
                    12.0,
                    Some(Brushes::black()),
                ),
                point,
            );
        }
        context.pop(state);
    }
}
