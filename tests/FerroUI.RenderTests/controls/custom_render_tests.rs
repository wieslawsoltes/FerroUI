//! Port of upstream's `Controls/CustomRenderTests.cs`.
//!
//! A `using` block around a pushed state of the drawing context is a scope
//! of the context here (`DrawingContext::scope`), which pops the state
//! where the block ends.

use crate::test_base::TestBase;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, Color, DrawingContext, EllipseGeometry, GradientStop, IBrush, LinearGradientBrush};
use ferroui_base::*;
use ferroui_controls::{Border, Control, ControlImpl, Decorator};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Controls\CustomRender")
}

#[test]
fn clip() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_child(CustomRenderer::new(|control, context| {
        let red: Rc<dyn IBrush> = Brushes::red();
        let blue: Rc<dyn IBrush> = Brushes::blue();

        context.fill_rectangle(&red, Rect::from_size(control.bounds().size()), 4.0);

        {
            let state = context.push_clip(Rect::from_size(control.bounds().size()).deflate(10.0));
            let mut context = context.scope(state);
            context.fill_rectangle(&blue, Rect::from_size(control.bounds().size()), 4.0);
        }
    }));

    t.render_to_file(&target, "Clip");
    t.compare_images("Clip");
}

#[test]
fn geometry_clip() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_child(CustomRenderer::new(|control, context| {
        let red: Rc<dyn IBrush> = Brushes::red();
        let blue: Rc<dyn IBrush> = Brushes::blue();
        let clip = EllipseGeometry::with_rect(Rect::from_size(control.bounds().size()));

        context.fill_rectangle(&red, Rect::from_size(control.bounds().size()), 4.0);

        {
            let state = context.push_geometry_clip(&clip.upcast());
            let mut context = context.scope(state);
            context.fill_rectangle(&blue, Rect::from_size(control.bounds().size()), 4.0);
        }
    }));

    t.render_to_file(&target, "GeometryClip");
    t.compare_images("GeometryClip");
}

// Upstream's test of this name pushes a rectangle clip, as `Clip_With_Transform` does: the two have the same
// body.
#[test]
fn geometry_clip_with_transform() {
    let t = base();
    let target = Border::new();
    target.set_background(Some(Brushes::white()));
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_child(CustomRenderer::new(|_control, context| {
        let red: Rc<dyn IBrush> = Brushes::red();
        let blue: Rc<dyn IBrush> = Brushes::blue();

        {
            let transform = context.push_transform(Matrix::create_translation(100.0, 100.0));
            let mut context = context.scope(transform);
            let clip = context.push_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
            let mut context = context.scope(clip);
            context.fill_rectangle(&blue, Rect::new(0.0, 0.0, 200.0, 200.0), 0.0);
        }

        context.fill_rectangle(&red, Rect::new(0.0, 0.0, 100.0, 100.0), 0.0);
    }));

    t.render_to_file(&target, "GeometryClip_With_Transform");
    t.compare_images("GeometryClip_With_Transform");
}

#[test]
fn clip_with_transform() {
    let t = base();
    let target = Border::new();
    target.set_background(Some(Brushes::white()));
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_child(CustomRenderer::new(|_control, context| {
        let red: Rc<dyn IBrush> = Brushes::red();
        let blue: Rc<dyn IBrush> = Brushes::blue();

        {
            let transform = context.push_transform(Matrix::create_translation(100.0, 100.0));
            let mut context = context.scope(transform);
            let clip = context.push_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
            let mut context = context.scope(clip);
            context.fill_rectangle(&blue, Rect::new(0.0, 0.0, 200.0, 200.0), 0.0);
        }

        context.fill_rectangle(&red, Rect::new(0.0, 0.0, 100.0, 100.0), 0.0);
    }));

    t.render_to_file(&target, "Clip_With_Transform");
    t.compare_images("Clip_With_Transform");
}

#[test]
fn opacity() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_child(CustomRenderer::new(|control, context| {
        let red: Rc<dyn IBrush> = Brushes::red();
        let blue: Rc<dyn IBrush> = Brushes::blue();

        context.fill_rectangle(&red, Rect::from_size(control.bounds().size()), 4.0);

        {
            let state = context.push_opacity(0.5);
            let mut context = context.scope(state);
            context.fill_rectangle(&blue, Rect::from_size(control.bounds().size()).deflate(10.0), 4.0);
        }
    }));

    t.render_to_file(&target, "Opacity");
    t.compare_images("Opacity");
}

#[test]
fn opacity_mask() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_child(CustomRenderer::new(|control, context| {
        let red: Rc<dyn IBrush> = Brushes::red();
        let blue: Rc<dyn IBrush> = Brushes::blue();
        let mask = LinearGradientBrush::new();
        mask.set_start_point(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
        mask.set_end_point(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative));
        mask.gradient_stops().add(GradientStop::with_color_and_offset(Color::from_uint32(0xffffffff), 0.0));
        mask.gradient_stops().add(GradientStop::with_color_and_offset(Color::from_uint32(0x00ffffff), 1.0));
        let mask: Rc<dyn IBrush> = mask.into();

        context.fill_rectangle(&red, Rect::from_size(control.bounds().size()), 4.0);

        {
            let state = context.push_opacity_mask(&mask, Rect::from_size(control.bounds().size()));
            let mut context = context.scope(state);
            context.fill_rectangle(&blue, Rect::from_size(control.bounds().size()).deflate(10.0), 4.0);
        }
    }));

    t.render_to_file(&target, "OpacityMask");
    t.compare_images("OpacityMask");
}

type Render = dyn Fn(&CustomRenderer, &mut DrawingContext);

#[repr(C)]
struct CustomRenderer {
    base: Control,
    render: Box<Render>,
}

ferro_class!(CustomRenderer: Control);
ferro_impl_classes!(CustomRenderer: FerroObjectImpl, StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl CustomRenderer {
    fn new(render: impl Fn(&CustomRenderer, &mut DrawingContext) + 'static) -> Ref<Self> {
        instantiate(Self { base: Control::construct(), render: Box::new(render) })
    }
}

impl VisualImpl for CustomRenderer {
    fn render(this: &Self, context: &mut DrawingContext) {
        (this.render)(this, context)
    }
}
