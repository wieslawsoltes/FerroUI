use super::{Shape, ShapeImpl};
use crate::test_support_shapes::MockRenderInterfaceScope;
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, Brushes, DrawingContext, Geometry, GlyphRun, IBrush, IDrawingContextCore, IEffect, IPen,
    MediaCollection, PenLineCap, PenLineJoin, RectangleGeometry, RenderOptions, TextOptions,
};
use ferroui_base::platform::{IBitmapImpl, IGeometryImpl};
use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Point, Rect, Ref, RoundedRect,
    StyledElementImpl, VisualImpl,
};
use std::rc::Rc;
use std::sync::Arc;

#[repr(C)]
struct TestShape {
    base: Shape,
}

ferro_class!(TestShape: Shape);
ferro_impl_classes!(
    TestShape: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl ShapeImpl for TestShape {
    fn create_defining_geometry(_this: &Self) -> Option<Ref<Geometry>> {
        Some(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 20.0, 20.0)).upcast())
    }
}

impl TestShape {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Shape::construct() })
    }
}

#[derive(Default)]
struct RecordingDrawingContext {
    last_pen: Option<Rc<dyn IPen>>,
}

impl IDrawingContextCore for RecordingDrawingContext {
    fn draw_line_core(&mut self, _pen: &Rc<dyn IPen>, _p1: Point, _p2: Point) {}

    fn draw_geometry_impl_core(
        &mut self,
        _brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        _geometry: &Arc<dyn IGeometryImpl>,
    ) {
        self.last_pen = pen.cloned();
    }

    fn draw_rectangle_core(
        &mut self,
        _brush: Option<&Rc<dyn IBrush>>,
        _pen: Option<&Rc<dyn IPen>>,
        _rrect: RoundedRect,
        _box_shadows: &BoxShadows,
    ) {
    }

    fn draw_ellipse_core(&mut self, _brush: Option<&Rc<dyn IBrush>>, _pen: Option<&Rc<dyn IPen>>, _rect: Rect) {}
    fn draw_bitmap(&mut self, _source: &Rc<dyn IBitmapImpl>, _opacity: f64, _source_rect: Rect, _dest_rect: Rect) {}
    fn custom(&mut self, _custom: &Rc<dyn ICustomDrawOperation>) {}
    fn draw_glyph_run(&mut self, _foreground: Option<&Rc<dyn IBrush>>, _glyph_run: &Rc<GlyphRun>) {}
    fn push_clip_core(&mut self, _rect: Rect) {}
    fn push_rounded_clip_core(&mut self, _rect: RoundedRect) {}
    fn push_geometry_clip_core(&mut self, _clip: &Ref<Geometry>) {}
    fn push_opacity_core(&mut self, _opacity: f64) {}
    fn push_opacity_mask_core(&mut self, _mask: &Rc<dyn IBrush>, _bounds: Rect) {}
    fn push_transform_core(&mut self, _matrix: Matrix) {}
    fn push_render_options_core(&mut self, _render_options: RenderOptions) {}
    fn push_text_options_core(&mut self, _text_options: TextOptions) {}
    fn push_effect_core(&mut self, _effect: &Rc<dyn IEffect>, _bounds: Rect) {}
    fn pop_clip_core(&mut self) {}
    fn pop_geometry_clip_core(&mut self) {}
    fn pop_opacity_core(&mut self) {}
    fn pop_opacity_mask_core(&mut self) {}
    fn pop_transform_core(&mut self) {}
    fn pop_render_options_core(&mut self) {}
    fn pop_text_options_core(&mut self) {}
    fn pop_effect_core(&mut self) {}
    fn dispose_core(&mut self) {}
}

fn render_and_get_pen(shape: &Shape) -> Option<Rc<dyn IPen>> {
    let mut core = RecordingDrawingContext::default();
    {
        let mut context = DrawingContext::new(&mut core);
        shape.render(&mut context);
    }
    core.last_pen
}

fn black() -> Option<Rc<dyn IBrush>> {
    Some(Brushes::black())
}

#[test]
fn stroke_miter_limit_default_is_applied_to_pen() {
    let _app = MockRenderInterfaceScope::install();
    let shape = TestShape::new();
    shape.set_stroke_thickness(4.0);
    shape.set_stroke(black());

    let pen = render_and_get_pen(&shape);

    let pen = pen.expect("a pen");
    assert_eq!(10.0, pen.miter_limit());
}

#[test]
fn stroke_miter_limit_update_refreshes_pen() {
    let _app = MockRenderInterfaceScope::install();
    let shape = TestShape::new();
    shape.set_stroke_thickness(4.0);
    shape.set_stroke(black());

    render_and_get_pen(&shape);
    shape.set_stroke_miter_limit(2.0);
    let pen = render_and_get_pen(&shape);

    let pen = pen.expect("a pen");
    assert_eq!(2.0, pen.miter_limit());
}

#[test]
fn stroke_thickness_is_applied_to_pen() {
    let _app = MockRenderInterfaceScope::install();
    let shape = TestShape::new();
    shape.set_stroke_thickness(6.0);
    shape.set_stroke(black());

    let pen = render_and_get_pen(&shape);

    let pen = pen.expect("a pen");
    assert_eq!(6.0, pen.thickness());
}

#[test]
fn stroke_line_cap_and_join_are_applied_to_pen() {
    let _app = MockRenderInterfaceScope::install();
    let shape = TestShape::new();
    shape.set_stroke(black());
    shape.set_stroke_thickness(4.0);
    shape.set_stroke_line_cap(PenLineCap::Round);
    shape.set_stroke_join(PenLineJoin::Bevel);

    let pen = render_and_get_pen(&shape);

    let pen = pen.expect("a pen");
    assert_eq!(PenLineCap::Round, pen.line_cap());
    assert_eq!(PenLineJoin::Bevel, pen.line_join());
}

#[test]
fn stroke_dash_array_and_offset_are_applied_to_pen() {
    let _app = MockRenderInterfaceScope::install();
    let shape = TestShape::new();
    shape.set_stroke(black());
    shape.set_stroke_thickness(4.0);
    shape.set_stroke_dash_array(Some(MediaCollection::from_items([1.0, 2.0, 3.0])));
    shape.set_stroke_dash_offset(1.5);

    let pen = render_and_get_pen(&shape);

    let pen = pen.expect("a pen");
    let dash_style = pen.dash_style().expect("a dash style");
    let dashes = dash_style.dashes().expect("dashes");
    assert_eq!(3, dashes.len());
    assert_eq!(1.0, dashes[0]);
    assert_eq!(2.0, dashes[1]);
    assert_eq!(3.0, dashes[2]);
    assert_eq!(1.5, dash_style.offset());
}

#[test]
fn no_stroke_produces_no_pen() {
    let _app = MockRenderInterfaceScope::install();
    let shape = TestShape::new();
    shape.set_stroke(None);
    shape.set_stroke_thickness(4.0);

    let pen = render_and_get_pen(&shape);

    assert!(pen.is_none());
}
