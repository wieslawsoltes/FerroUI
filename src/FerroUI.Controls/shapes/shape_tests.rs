use super::{Rectangle, Shape, ShapeImpl};
use crate::test_support_shapes::MockRenderInterfaceScope;
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, Brushes, DrawingContext, Geometry, GlyphRun, IBrush, IDrawingContextCore, IEffect, IPen,
    MediaCollection, PenLineCap, PenLineJoin, RectangleGeometry, RenderOptions, TextOptions,
};
use ferroui_base::platform::IGeometryImpl;
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

/// A rectangle with a geometry of its own (inset by five) and no update of it.
#[repr(C)]
struct InsetRectangle {
    base: Rectangle,
}

ferro_class!(InsetRectangle: Rectangle);
ferro_impl_classes!(
    InsetRectangle: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl ShapeImpl for InsetRectangle {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        Some(RectangleGeometry::with_rect(Rect::from_size(this.bounds().size()).deflate(5.0)).upcast())
    }
}

impl InsetRectangle {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Rectangle::construct() })
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
    fn draw_bitmap(&mut self, _source: &ferroui_base::utilities::RefCounted<ferroui_base::platform::SharedBitmapImpl>, _opacity: f64, _source_rect: Rect, _dest_rect: Rect) {}
    fn custom(&mut self, _custom: &std::sync::Arc<dyn ICustomDrawOperation>) {}
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

/// Section 9.88: a shape that makes its own geometry gives the geometry it has what it is now,
/// when a property the geometry is made of changes: the geometry is the same object, built
/// again, and the shape is measured again.
#[test]
fn a_shape_whose_geometry_properties_change_keeps_its_geometry() {
    use super::{Ellipse, Line};
    use ferroui_base::media::{EllipseGeometry, LineGeometry, RectangleGeometry};
    use ferroui_base::{Point, Rect, Size};

    let _scope = crate::test_support::test_scope();
    let _app = crate::test_support_shapes::MockRenderInterfaceScope::install();
    let arranged = |shape: &Shape, size: Size| {
        shape.measure(Size::INFINITY);
        shape.arrange(Rect::from_size(size));
    };

    // A rectangle that is resized, and whose radii change.
    let rectangle = Rectangle::new();
    rectangle.set_stroke_thickness(2.0);
    arranged(&rectangle, Size::new(100.0, 40.0));
    let geometry = rectangle.defining_geometry().expect("a geometry");
    let changes = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = changes.clone();
    geometry.changed(move || counted.set(counted.get() + 1)).detach();
    arranged(&rectangle, Size::new(140.0, 40.0));
    let kept = rectangle.defining_geometry().expect("a geometry");
    assert!(kept.ptr_eq(&geometry), "the same geometry");
    assert!(changes.get() >= 1, "the geometry says it changed");
    assert_eq!(kept.cast::<RectangleGeometry>().expect("a rectangle").rect(), Rect::new(1.0, 1.0, 138.0, 38.0));
    rectangle.set_radius_x(6.0);
    assert!(!rectangle.is_measure_valid());
    let kept = rectangle.defining_geometry().expect("a geometry");
    assert!(kept.ptr_eq(&geometry));
    assert_eq!(kept.cast::<RectangleGeometry>().expect("a rectangle").radius_x(), 6.0);

    // An ellipse that is resized.
    let ellipse = Ellipse::new();
    arranged(&ellipse, Size::new(50.0, 50.0));
    let geometry = ellipse.defining_geometry().expect("a geometry");
    arranged(&ellipse, Size::new(80.0, 60.0));
    let kept = ellipse.defining_geometry().expect("a geometry");
    assert!(kept.ptr_eq(&geometry));
    let stroke = ellipse.stroke_thickness();
    assert_eq!(
        kept.cast::<EllipseGeometry>().expect("an ellipse").rect(),
        Rect::from_size(Size::new(80.0, 60.0)).deflate(stroke / 2.0)
    );

    // A line whose end moves.
    let line = Line::new();
    line.set_end_point(Point::new(10.0, 10.0));
    line.measure(Size::INFINITY);
    let geometry = line.defining_geometry().expect("a geometry");
    line.set_end_point(Point::new(30.0, 5.0));
    assert!(!line.is_measure_valid());
    let kept = line.defining_geometry().expect("a geometry");
    assert!(kept.ptr_eq(&geometry));
    assert_eq!(kept.cast::<LineGeometry>().expect("a line").end_point(), Point::new(30.0, 5.0));

    // The indicator of a selection fits its radii to its size, in the geometry it keeps too.
    let indicator = crate::SelectionIndicator::new();
    indicator.set_radius_x(8.0);
    indicator.set_radius_y(8.0);
    arranged(&indicator, Size::new(3.0, 16.0));
    let geometry = indicator.defining_geometry().expect("a geometry");
    arranged(&indicator, Size::new(3.0, 24.0));
    let kept = indicator.defining_geometry().expect("a geometry");
    assert!(kept.ptr_eq(&geometry));
    let drawn = Rect::from_size(Size::new(3.0, 24.0)).deflate(indicator.stroke_thickness() / 2.0);
    let kept = kept.cast::<RectangleGeometry>().expect("a rectangle");
    assert_eq!(kept.rect(), drawn);
    assert_eq!(kept.radius_x(), drawn.width / 2.0, "the radius is fitted to the width");
    assert_eq!(kept.radius_y(), 8.0);
}

/// Section 9.88: an update describes one `create_defining_geometry`. A class that makes a
/// geometry of its own and says nothing of an update does not take the update of its base
/// class: its geometry is made again, by its own code, as before.
#[test]
fn a_shape_with_a_geometry_of_its_own_does_not_take_the_update_of_its_base_class() {
    use ferroui_base::Size;

    let _scope = crate::test_support::test_scope();
    let _app = crate::test_support_shapes::MockRenderInterfaceScope::install();
    let shape = InsetRectangle::new();
    shape.measure(Size::INFINITY);
    shape.arrange(Rect::from_size(Size::new(100.0, 40.0)));
    let first = shape.defining_geometry().expect("a geometry");
    assert_eq!(first.cast::<RectangleGeometry>().expect("a rectangle").rect(), Rect::new(5.0, 5.0, 90.0, 30.0));
    shape.measure(Size::INFINITY);
    shape.arrange(Rect::from_size(Size::new(140.0, 40.0)));
    let second = shape.defining_geometry().expect("a geometry");
    assert!(!second.ptr_eq(&first), "a new geometry, made by the class");
    assert_eq!(second.cast::<RectangleGeometry>().expect("a rectangle").rect(), Rect::new(5.0, 5.0, 130.0, 30.0));
}

