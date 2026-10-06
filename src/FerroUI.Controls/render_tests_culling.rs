//! Port of `RenderTests_Culling.cs` (base unit tests). The trees are built
//! from canvases, panels and borders, so the tests live with the controls.
//!
//! The mocked platform drawing context of upstream
//! (`Mock.Of<IDrawingContextImpl>()`) is the recording drawing context of
//! the render test doubles, whose log is not read.

use crate::testing::{TestServices, UnitTestApplication};
use crate::{Border, Canvas, Control, ControlImpl, Panel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{DrawingContext, ITransform, PlatformDrawingContext, TranslateTransform};
use ferroui_base::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use ferroui_base::rendering::ImmediateRenderer;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, IntoRef, Rect, Ref, Size, StyledElementImpl,
    Thickness, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

fn translate(x: f64, y: f64) -> Option<Rc<dyn ITransform>> {
    Some(TranslateTransform::with_offset(x, y).into())
}

/// `new Canvas { Width = 100, Height = 100, ClipToBounds = true }`.
fn clipped_canvas() -> Ref<Canvas> {
    let container = Canvas::new();
    container.set_width(100.0);
    container.set_height(100.0);
    container.set_clip_to_bounds(true);
    container
}

#[test]
fn in_bounds_control_should_be_rendered() {
    let _app = UnitTestApplication::start(TestServices::mock_platform_render_interface());

    let target = TestControl::new();
    target.set_width(10.0);
    target.set_height(10.0);
    Canvas::set_left(&target, 98.0);
    Canvas::set_top(&target, 98.0);
    let container = clipped_canvas();
    container.children().add(target.clone());

    render(&container);

    assert!(target.rendered.get());
}

#[test]
fn out_of_bounds_control_should_not_be_rendered() {
    let _app = UnitTestApplication::start(TestServices::mock_platform_render_interface());

    let target = TestControl::new();
    target.set_width(10.0);
    target.set_height(10.0);
    target.set_clip_to_bounds(true);
    Canvas::set_left(&target, 110.0);
    Canvas::set_top(&target, 110.0);
    let container = clipped_canvas();
    container.children().add(target.clone());

    render(&container);

    assert!(!target.rendered.get());
}

#[test]
fn out_of_bounds_child_control_should_not_be_rendered() {
    let _app = UnitTestApplication::start(TestServices::mock_platform_render_interface());

    let target = TestControl::new();
    target.set_width(10.0);
    target.set_height(10.0);
    target.set_clip_to_bounds(true);
    Canvas::set_left(&target, 50.0);
    Canvas::set_top(&target, 50.0);
    let inner = Canvas::new();
    inner.set_width(100.0);
    inner.set_height(100.0);
    Canvas::set_left(&inner, 50.0);
    Canvas::set_top(&inner, 50.0);
    inner.children().add(target.clone());
    let container = clipped_canvas();
    container.children().add(inner);

    render(&container);

    assert!(!target.rendered.get());
}

#[test]
fn transformed_child_control_with_clip_to_bounds_true_should_be_rendered() {
    let _app = UnitTestApplication::start(TestServices::mock_platform_render_interface());

    let target = TestControl::new();
    target.set_width(100.0);
    target.set_height(20.0);
    target.set_clip_to_bounds(true);
    target.set_render_transform(translate(0.0, -30.0));
    let inner = Panel::new();
    inner.set_width(100.0);
    inner.set_height(20.0);
    inner.set_render_transform(translate(0.0, 30.0));
    inner.children().add(target.clone());
    let container = Panel::new();
    container.set_width(100.0);
    container.set_height(20.0);
    container.set_clip_to_bounds(true);
    container.children().add(inner);

    render(&container);

    assert!(target.rendered.get());
}

#[test]
fn render_transform_should_be_respected() {
    let _app = UnitTestApplication::start(TestServices::mock_platform_render_interface());

    let target = TestControl::new();
    target.set_width(10.0);
    target.set_height(10.0);
    Canvas::set_left(&target, 110.0);
    Canvas::set_top(&target, 110.0);
    target.set_render_transform(translate(-100.0, -100.0));
    let container = clipped_canvas();
    container.children().add(target.clone());

    render(&container);

    assert!(target.rendered.get());
}

#[test]
fn negative_margin_should_be_respected() {
    let _app = UnitTestApplication::start(TestServices::mock_platform_render_interface());

    let target = TestControl::new();
    target.set_width(10.0);
    target.set_height(10.0);
    target.set_margin(Thickness::new(-100.0, -100.0, 0.0, 0.0));
    let border = Border::new();
    border.set_margin(Thickness::new(100.0, 100.0, 0.0, 0.0));
    border.set_child(&target);
    let container = clipped_canvas();
    container.children().add(border);

    render(&container);

    assert!(target.rendered.get());
}

fn render(control: impl IntoRef<Control>) {
    let control = control.into_ref();
    let mut platform_impl = MockDrawingContextImpl::new(DrawingLog::new());
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut ctx = DrawingContext::new(&mut core);
    control.measure(Size::INFINITY);
    control.arrange(Rect::from_size(control.desired_size()));
    ImmediateRenderer::render(&mut ctx, &control.upcast());
    ctx.dispose();
}

#[repr(C)]
struct TestControl {
    base: Control,
    rendered: Cell<bool>,
}

ferro_class!(TestControl: Control);
ferro_impl_classes!(
    TestControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl VisualImpl for TestControl {
    fn render(this: &Self, _context: &mut DrawingContext) {
        this.rendered.set(true);
    }
}

impl TestControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct(), rendered: Cell::new(false) })
    }
}
