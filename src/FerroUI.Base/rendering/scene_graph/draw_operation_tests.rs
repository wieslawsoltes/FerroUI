//! Port of `Rendering/SceneGraph/DrawOperationTests.cs`.
//!
//! The fixture `CompositorTestServices` is the test compositor of the
//! composition tests (a compositor over a manual render loop and the mock
//! render interface). `TestContext.ForceRender` commits and runs one frame
//! of that render loop. The bitmap's `RefCountable` reference count is the
//! strong count of its handle: the recorded reference is shared by the
//! client render data and the server render data it is sent to, as the
//! `IRef` of upstream is.
//!
//! The tracking brushes and pens of upstream are their own server-side
//! counterparts (`GetForCompositor` returns `this`). A server-side object
//! of the port is an object of the server, so these doubles count the
//! reference calls and hand the counterpart of an inner brush or pen
//! through. `ImmutableTrackingPen` derives from `ImmutablePen` upstream; here
//! it wraps one and is seen as one (`IPen::as_immutable_pen`).

use super::scene_graph_test_support::{black, TestBitmapImpl, TestCustomOperation, TestGeometryImpl};
use crate::media::immutable::ImmutablePen;
use crate::media::{
    BoxShadows, Brushes, DrawingContext, IBrush, IDashStyle, IImmutableBrush, IPen, ITransform, Pen, PenLineCap,
    PenLineJoin, SolidColorBrush,
};
use crate::platform::{IBitmapImpl, IGeometryImpl};
use crate::rendering::composition::drawing::{
    CompositionRenderData, ICompositionRenderResource, RenderDataDrawingContext, ServerCompositionRenderData,
};
use crate::rendering::composition::server::ServerObjectId;
use crate::rendering::composition::test_compositor::TestCompositor;
use crate::rendering::composition::Compositor;
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{Matrix, Point, Rect, RelativePoint, RoundedRect};
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

/// Declares one test per data row of a parameterized test.
macro_rules! theory {
    ($func:ident: $($name:ident($($arg:expr),* $(,)?));+ $(;)?) => {
        $(
            #[test]
            fn $name() {
                $func($($arg),*)
            }
        )+
    };
}

struct TestContext {
    services: TestCompositor,
    context: RenderDataDrawingContext,
}

impl TestContext {
    fn new() -> TestContext {
        let services = TestCompositor::new();
        let context = RenderDataDrawingContext::new(Some(services.compositor.clone()));
        TestContext { services, context }
    }

    /// Draws through a drawing context over the recording context.
    fn draw(&mut self, draw: impl FnOnce(&mut DrawingContext<'_>)) {
        let mut context = DrawingContext::new(&mut self.context);
        draw(&mut context);
    }

    fn get_render_results(&mut self) -> Option<Rc<CompositionRenderData>> {
        self.context.get_render_results()
    }

    fn force_render(&self) {
        self.services.compositor.commit();
        self.services.render_loop.tick();
    }

    /// The bounds of the server-side render data, after a frame.
    fn server_bounds(&self, render_data: &CompositionRenderData) -> Option<Rect> {
        self.services
            .server::<ServerCompositionRenderData>(render_data.server())
            .bounds()
            .map(|bounds| bounds.to_rect())
    }

    fn get_bounds(&mut self) -> Option<Rect> {
        let render_data = self.get_render_results()?;
        self.force_render();
        self.server_bounds(&render_data)
    }
}

fn rounded(x: f64, y: f64, width: f64, height: f64) -> RoundedRect {
    RoundedRect::from_rect(Rect::new(x, y, width, height))
}

#[test]
fn empty_bounds_remain_empty() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| c.draw_rectangle(Some(&black()), None, Rect::default(), 0.0, 0.0, &BoxShadows::default()));

    assert_eq!(None, ctx.get_bounds());
}

#[allow(clippy::too_many_arguments)]
fn rectangle_bounds_are_snapped_to_pixels(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale_x: f64,
    scale_y: f64,
    pen_thickness: f64,
    expected_x: f64,
    expected_y: f64,
    expected_width: f64,
    expected_height: f64,
) {
    let mut ctx = TestContext::new();
    let pen: Rc<dyn IPen> = Rc::new(ImmutablePen::with_brush(Some(Brushes::black()), pen_thickness));
    ctx.draw(|c| {
        let state = c.push_transform(Matrix::create_scale(scale_x, scale_y));
        c.draw_rectangle(None, Some(&pen), Rect::new(x, y, width, height), 0.0, 0.0, &BoxShadows::default());
        c.pop(state);
    });

    let bounds = ctx.get_bounds().expect("the rectangle has bounds");
    assert_eq!(Rect::new(expected_x, expected_y, expected_width, expected_height), bounds);
}

theory!(rectangle_bounds_are_snapped_to_pixels:
    rectangle_bounds_are_snapped_to_pixels_1(10.0, 10.0, 10.0, 10.0, 1.0, 1.0, 1.0, 9.0, 9.0, 12.0, 12.0);
    rectangle_bounds_are_snapped_to_pixels_2(10.0, 10.0, 10.0, 10.0, 1.0, 1.0, 2.0, 9.0, 9.0, 12.0, 12.0);
    rectangle_bounds_are_snapped_to_pixels_3(10.0, 10.0, 10.0, 10.0, 1.5, 1.5, 1.0, 14.0, 14.0, 17.0, 17.0));

fn image_node_releases_reference_to_bitmap_on_dispose(dispose_before_commit: bool) {
    let bitmap: Rc<dyn IBitmapImpl> = Rc::new(TestBitmapImpl);

    let mut ctx = TestContext::new();
    ctx.draw(|c| c.draw_bitmap(&bitmap, 1.0, Rect::new(1.0, 1.0, 1.0, 1.0), Rect::new(1.0, 1.0, 1.0, 1.0)));
    let render_data = ctx.get_render_results().expect("something was drawn");
    assert_eq!(2, Rc::strong_count(&bitmap));
    if dispose_before_commit {
        render_data.dispose();
        assert_eq!(1, Rc::strong_count(&bitmap));
        ctx.force_render();
        assert_eq!(1, Rc::strong_count(&bitmap));
    } else {
        ctx.force_render();
        assert_eq!(2, Rc::strong_count(&bitmap));

        // Refs ownership is transferred to server-side render data
        render_data.dispose();
        assert_eq!(2, Rc::strong_count(&bitmap));

        ctx.force_render();
        assert_eq!(1, Rc::strong_count(&bitmap));
    }
}

theory!(image_node_releases_reference_to_bitmap_on_dispose:
    image_node_releases_reference_to_bitmap_on_dispose_1(false);
    image_node_releases_reference_to_bitmap_on_dispose_2(true));

#[test]
fn hit_test_on_geometry_node_with_zero_transform_does_not_throw() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let state = c.push_transform(Matrix::default());
        let geometry: Arc<dyn IGeometryImpl> = TestGeometryImpl::new();
        c.draw_geometry_impl(Some(&black()), None, &geometry);
        c.pop(state);
    });

    assert!(!ctx.get_render_results().expect("something was drawn").hit_test(Point::default()));
}

#[test]
fn hit_test_rectangle_node_with_transform_hits() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let state = c.push_transform(Matrix::create_translation(20.0, 20.0));
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.pop(state);
    });

    assert!(ctx.get_render_results().expect("something was drawn").hit_test(Point::new(25.0, 25.0)));
}

#[test]
fn empty_push_pop_sequence_produces_no_results() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let transform = c.push_transform(Matrix::create_translation(20.0, 20.0));
        let opacity = c.push_opacity(1.0);
        c.pop(opacity);
        c.pop(transform);
    });

    assert!(ctx.get_render_results().is_none());
}

#[test]
fn hit_test_through_push_transform_maps_coordinates() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let state = c.push_transform(Matrix::create_translation(50.0, 50.0));
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.pop(state);
    });

    let rd = ctx.get_render_results().expect("something was drawn");
    assert!(rd.hit_test(Point::new(55.0, 55.0)));
    assert!(!rd.hit_test(Point::new(5.0, 5.0)));
}

#[test]
fn hit_test_through_push_clip_restricts_to_clip_region() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let state = c.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 100.0, 100.0), &BoxShadows::default());
        c.pop(state);
    });

    let rd = ctx.get_render_results().expect("something was drawn");
    assert!(rd.hit_test(Point::new(5.0, 5.0)));
    assert!(!rd.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn hit_test_through_nested_transforms_composes_outer_to_inner() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let outer = c.push_transform(Matrix::create_translation(20.0, 20.0));
        let inner = c.push_transform(Matrix::create_scale(2.0, 2.0));
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.pop(inner);
        c.pop(outer);
    });

    let rd = ctx.get_render_results().expect("something was drawn");
    assert!(rd.hit_test(Point::new(30.0, 30.0)));
    assert!(!rd.hit_test(Point::new(5.0, 5.0)));
    assert!(!rd.hit_test(Point::new(45.0, 45.0)));
}

#[test]
fn bounds_union_across_multiple_top_level_draws() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.draw_rounded_rectangle(Some(&black()), None, rounded(20.0, 20.0, 10.0, 10.0), &BoxShadows::default());
    });

    assert_eq!(Some(Rect::new(0.0, 0.0, 30.0, 30.0)), ctx.get_bounds());
}

#[test]
fn bounds_reflect_push_transform_translation() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let state = c.push_transform(Matrix::create_translation(50.0, 50.0));
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.pop(state);
    });

    assert_eq!(Some(Rect::new(50.0, 50.0, 10.0, 10.0)), ctx.get_bounds());
}

#[test]
fn identity_push_transform_does_not_wrap_children() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let state = c.push_transform(Matrix::IDENTITY);
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.pop(state);
    });

    let rd = ctx.get_render_results().expect("something was drawn");
    ctx.force_render();
    assert_eq!(Some(Rect::new(0.0, 0.0, 10.0, 10.0)), ctx.server_bounds(&rd));
    assert!(rd.hit_test(Point::new(5.0, 5.0)));
}

#[test]
fn opacity_one_push_does_not_wrap_children() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let state = c.push_opacity(1.0);
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.pop(state);
    });

    let rd = ctx.get_render_results().expect("something was drawn");
    ctx.force_render();
    assert_eq!(Some(Rect::new(0.0, 0.0, 10.0, 10.0)), ctx.server_bounds(&rd));
    assert!(rd.hit_test(Point::new(5.0, 5.0)));
}

#[test]
fn empty_push_between_real_draws_does_not_affect_siblings() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        let state = c.push_transform(Matrix::create_translation(100.0, 100.0));
        c.pop(state);
        c.draw_rounded_rectangle(Some(&black()), None, rounded(40.0, 40.0, 10.0, 10.0), &BoxShadows::default());
    });

    let rd = ctx.get_render_results().expect("something was drawn");
    ctx.force_render();
    assert_eq!(Some(Rect::new(0.0, 0.0, 50.0, 50.0)), ctx.server_bounds(&rd));
    assert!(rd.hit_test(Point::new(5.0, 5.0)));
    assert!(rd.hit_test(Point::new(45.0, 45.0)));
    assert!(!rd.hit_test(Point::new(25.0, 25.0)));
}

#[test]
fn no_op_inner_push_inside_real_outer_push_is_stripped() {
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let outer = c.push_transform(Matrix::create_translation(20.0, 20.0));
        let inner = c.push_opacity(1.0);
        c.pop(inner);
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.pop(outer);
    });

    let rd = ctx.get_render_results().expect("something was drawn");
    ctx.force_render();
    assert_eq!(Some(Rect::new(20.0, 20.0, 10.0, 10.0)), ctx.server_bounds(&rd));
    assert!(rd.hit_test(Point::new(25.0, 25.0)));
    assert!(!rd.hit_test(Point::new(5.0, 5.0)));
}

#[test]
fn non_immutable_brush_is_add_refed_once_and_released_on_dispose() {
    let brush = TrackingBrush::new();
    let as_brush: Rc<dyn IBrush> = brush.clone();
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        c.draw_rounded_rectangle(Some(&as_brush), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default())
    });
    let rd = ctx.get_render_results().expect("something was drawn");

    assert_eq!(1, brush.add_ref_count.get());
    assert_eq!(0, brush.release_count.get());

    rd.dispose();
    assert_eq!(1, brush.add_ref_count.get());
    assert_eq!(1, brush.release_count.get());
}

#[test]
fn non_immutable_brush_used_multiple_times_is_add_refed_once() {
    let brush = TrackingBrush::new();
    let as_brush: Rc<dyn IBrush> = brush.clone();
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        c.draw_rounded_rectangle(Some(&as_brush), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.draw_rounded_rectangle(Some(&as_brush), None, rounded(20.0, 20.0, 10.0, 10.0), &BoxShadows::default());
        c.draw_ellipse(Some(&as_brush), None, Rect::new(40.0, 40.0, 10.0, 10.0));
    });
    let rd = ctx.get_render_results().expect("something was drawn");

    assert_eq!(1, brush.add_ref_count.get());

    rd.dispose();
    assert_eq!(1, brush.release_count.get());
}

#[test]
fn non_immutable_pen_is_add_refed_once_and_released_on_dispose() {
    let pen = TrackingPen::new();
    let as_pen: Rc<dyn IPen> = pen.clone();
    let mut ctx = TestContext::new();
    ctx.draw(|c| c.draw_line(&as_pen, Point::new(0.0, 0.0), Point::new(10.0, 10.0)));
    let rd = ctx.get_render_results().expect("something was drawn");

    assert_eq!(1, pen.add_ref_count.get());
    assert_eq!(0, pen.release_count.get());

    rd.dispose();
    assert_eq!(1, pen.release_count.get());
}

#[test]
fn immutable_brush_is_not_registered_as_server_resource() {
    let brush = ImmutableTrackingBrush::new();
    let as_brush: Rc<dyn IBrush> = brush.clone();
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        c.draw_rounded_rectangle(Some(&as_brush), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default())
    });
    let rd = ctx.get_render_results().expect("something was drawn");

    assert_eq!(0, brush.add_ref_count.get());

    rd.dispose();
}

#[test]
fn immutable_pen_is_not_registered_as_server_resource() {
    let pen = ImmutableTrackingPen::new();
    let as_pen: Rc<dyn IPen> = pen.clone();
    let mut ctx = TestContext::new();
    ctx.draw(|c| c.draw_line(&as_pen, Point::new(0.0, 0.0), Point::new(10.0, 10.0)));
    let rd = ctx.get_render_results().expect("something was drawn");

    assert_eq!(0, pen.add_ref_count.get());

    rd.dispose();
}

#[test]
fn custom_draw_operation_disposed_when_render_data_disposed_before_commit() {
    let op = TestCustomOperation::with_bounds(Rect::new(0.0, 0.0, 10.0, 10.0));
    let custom: Rc<dyn ICustomDrawOperation> = op.clone();
    let mut ctx = TestContext::new();
    ctx.draw(|c| c.custom(&custom));
    let rd = ctx.get_render_results().expect("something was drawn");

    assert_eq!(0, op.dispose_count.get());

    rd.dispose();
    assert_eq!(1, op.dispose_count.get());
}

#[test]
fn push_opacity_mask_brush_is_add_refed_once_and_released_on_dispose() {
    let brush = TrackingBrush::new();
    let as_brush: Rc<dyn IBrush> = brush.clone();
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let state = c.push_opacity_mask(&as_brush, Rect::new(0.0, 0.0, 100.0, 100.0));
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.pop(state);
    });
    let rd = ctx.get_render_results().expect("something was drawn");

    assert_eq!(1, brush.add_ref_count.get());
    assert_eq!(0, brush.release_count.get());

    rd.dispose();
    assert_eq!(1, brush.release_count.get());
}

#[test]
fn push_opacity_mask_hit_test_recurses_into_children() {
    let brush: Rc<dyn IBrush> = TrackingBrush::new();
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        let state = c.push_opacity_mask(&brush, Rect::new(0.0, 0.0, 100.0, 100.0));
        c.draw_rounded_rectangle(Some(&black()), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.pop(state);
    });
    let rd = ctx.get_render_results().expect("something was drawn");

    assert!(rd.hit_test(Point::new(5.0, 5.0)));
    assert!(!rd.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn geometry_node_add_refs_brush_and_pen() {
    let brush = TrackingBrush::new();
    let pen = TrackingPen::new();
    let (as_brush, as_pen): (Rc<dyn IBrush>, Rc<dyn IPen>) = (brush.clone(), pen.clone());
    let geometry: Arc<dyn IGeometryImpl> = TestGeometryImpl::new();
    let mut ctx = TestContext::new();
    ctx.draw(|c| c.draw_geometry_impl(Some(&as_brush), Some(&as_pen), &geometry));
    let rd = ctx.get_render_results().expect("something was drawn");

    assert_eq!(1, brush.add_ref_count.get());
    assert_eq!(1, pen.add_ref_count.get());

    rd.dispose();
    assert_eq!(1, brush.release_count.get());
    assert_eq!(1, pen.release_count.get());
}

#[test]
fn geometry_node_hit_test_uses_fill_contains_when_brush_set() {
    let geom_mock: Arc<dyn IGeometryImpl> = TestGeometryImpl::with_fill(&[Point::new(5.0, 5.0)]);

    let mut ctx = TestContext::new();
    ctx.draw(|c| c.draw_geometry_impl(Some(&black()), None, &geom_mock));
    let rd = ctx.get_render_results().expect("something was drawn");

    assert!(rd.hit_test(Point::new(5.0, 5.0)));
    assert!(!rd.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn geometry_node_hit_test_uses_stroke_contains_when_pen_set() {
    let pen: Rc<dyn IPen> = Rc::new(ImmutablePen::with_brush(Some(Brushes::black()), 1.0));
    let geom_mock: Arc<dyn IGeometryImpl> = TestGeometryImpl::with_stroke_pen(&pen, &[Point::new(5.0, 5.0)]);

    let mut ctx = TestContext::new();
    ctx.draw(|c| c.draw_geometry_impl(None, Some(&pen), &geom_mock));
    let rd = ctx.get_render_results().expect("something was drawn");

    assert!(rd.hit_test(Point::new(5.0, 5.0)));
    assert!(!rd.hit_test(Point::new(50.0, 50.0)));
}

#[test]
fn two_distinct_brushes_are_add_refed_separately() {
    let brush1 = TrackingBrush::new();
    let brush2 = TrackingBrush::new();
    let (as_brush1, as_brush2): (Rc<dyn IBrush>, Rc<dyn IBrush>) = (brush1.clone(), brush2.clone());
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        c.draw_rounded_rectangle(Some(&as_brush1), None, rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
        c.draw_rounded_rectangle(Some(&as_brush2), None, rounded(20.0, 20.0, 10.0, 10.0), &BoxShadows::default());
    });
    let rd = ctx.get_render_results().expect("something was drawn");

    assert_eq!(1, brush1.add_ref_count.get());
    assert_eq!(1, brush2.add_ref_count.get());

    rd.dispose();
    assert_eq!(1, brush1.release_count.get());
    assert_eq!(1, brush2.release_count.get());
}

#[test]
fn brush_and_pen_on_same_draw_are_both_add_refed_once() {
    let brush = TrackingBrush::new();
    let pen = TrackingPen::new();
    let (as_brush, as_pen): (Rc<dyn IBrush>, Rc<dyn IPen>) = (brush.clone(), pen.clone());
    let mut ctx = TestContext::new();
    ctx.draw(|c| {
        c.draw_rounded_rectangle(Some(&as_brush), Some(&as_pen), rounded(0.0, 0.0, 10.0, 10.0), &BoxShadows::default())
    });
    let rd = ctx.get_render_results().expect("something was drawn");

    assert_eq!(1, brush.add_ref_count.get());
    assert_eq!(1, pen.add_ref_count.get());

    rd.dispose();
    assert_eq!(1, brush.release_count.get());
    assert_eq!(1, pen.release_count.get());
}

/// The render resource of a mutable brush or pen whose counterpart the
/// tracking doubles hand through.
fn inner_resource<'a>(resource: Option<&'a dyn ICompositionRenderResource>) -> &'a dyn ICompositionRenderResource {
    resource.expect("a mutable object has a render resource")
}

struct TrackingBrush {
    add_ref_count: Cell<i32>,
    release_count: Cell<i32>,
    inner: Rc<dyn IBrush>,
}

impl TrackingBrush {
    fn new() -> Rc<TrackingBrush> {
        Rc::new(TrackingBrush {
            add_ref_count: Cell::new(0),
            release_count: Cell::new(0),
            inner: SolidColorBrush::new().into(),
        })
    }
}

impl IBrush for TrackingBrush {
    fn opacity(&self) -> f64 {
        1.0
    }
    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }
    fn transform_origin(&self) -> RelativePoint {
        RelativePoint::default()
    }
    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_composition_render_resource(&self) -> Option<&dyn ICompositionRenderResource> {
        Some(self)
    }
}

impl ICompositionRenderResource for TrackingBrush {
    fn add_ref_on_compositor(&self, c: &Rc<Compositor>) {
        self.add_ref_count.set(self.add_ref_count.get() + 1);
        inner_resource(self.inner.as_composition_render_resource()).add_ref_on_compositor(c);
    }
    fn release_on_compositor(&self, c: &Rc<Compositor>) {
        self.release_count.set(self.release_count.get() + 1);
        inner_resource(self.inner.as_composition_render_resource()).release_on_compositor(c);
    }
    fn get_for_compositor(&self, c: &Compositor) -> ServerObjectId {
        inner_resource(self.inner.as_composition_render_resource()).get_for_compositor(c)
    }
}

struct TrackingPen {
    add_ref_count: Cell<i32>,
    release_count: Cell<i32>,
    inner: Rc<dyn IPen>,
}

impl TrackingPen {
    fn new() -> Rc<TrackingPen> {
        Rc::new(TrackingPen {
            add_ref_count: Cell::new(0),
            release_count: Cell::new(0),
            inner: Pen::with_brush(Some(Brushes::black()), 1.0).into(),
        })
    }
}

impl IPen for TrackingPen {
    fn brush(&self) -> Option<Rc<dyn IBrush>> {
        Some(Brushes::black())
    }
    fn dash_style(&self) -> Option<Rc<dyn IDashStyle>> {
        None
    }
    fn line_cap(&self) -> PenLineCap {
        PenLineCap::Flat
    }
    fn line_join(&self) -> PenLineJoin {
        PenLineJoin::Miter
    }
    fn miter_limit(&self) -> f64 {
        10.0
    }
    fn thickness(&self) -> f64 {
        1.0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_composition_render_resource(&self) -> Option<&dyn ICompositionRenderResource> {
        Some(self)
    }
    fn into_immutable_pen(self: Rc<Self>) -> Rc<ImmutablePen> {
        self.inner.clone().into_immutable_pen()
    }
}

impl ICompositionRenderResource for TrackingPen {
    fn add_ref_on_compositor(&self, c: &Rc<Compositor>) {
        self.add_ref_count.set(self.add_ref_count.get() + 1);
        inner_resource(self.inner.as_composition_render_resource()).add_ref_on_compositor(c);
    }
    fn release_on_compositor(&self, c: &Rc<Compositor>) {
        self.release_count.set(self.release_count.get() + 1);
        inner_resource(self.inner.as_composition_render_resource()).release_on_compositor(c);
    }
    fn get_for_compositor(&self, c: &Compositor) -> ServerObjectId {
        inner_resource(self.inner.as_composition_render_resource()).get_for_compositor(c)
    }
}

struct ImmutableTrackingBrush {
    add_ref_count: Cell<i32>,
    inner: Rc<dyn IBrush>,
}

impl ImmutableTrackingBrush {
    fn new() -> Rc<ImmutableTrackingBrush> {
        Rc::new(ImmutableTrackingBrush { add_ref_count: Cell::new(0), inner: SolidColorBrush::new().into() })
    }
}

impl IBrush for ImmutableTrackingBrush {
    fn opacity(&self) -> f64 {
        1.0
    }
    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }
    fn transform_origin(&self) -> RelativePoint {
        RelativePoint::default()
    }
    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_immutable_brush(&self) -> Option<&dyn IImmutableBrush> {
        Some(self)
    }
    fn as_composition_render_resource(&self) -> Option<&dyn ICompositionRenderResource> {
        Some(self)
    }
}

impl IImmutableBrush for ImmutableTrackingBrush {}

impl ICompositionRenderResource for ImmutableTrackingBrush {
    fn add_ref_on_compositor(&self, _c: &Rc<Compositor>) {
        self.add_ref_count.set(self.add_ref_count.get() + 1);
    }
    fn release_on_compositor(&self, _c: &Rc<Compositor>) {}
    fn get_for_compositor(&self, c: &Compositor) -> ServerObjectId {
        inner_resource(self.inner.as_composition_render_resource()).get_for_compositor(c)
    }
}

struct ImmutableTrackingPen {
    add_ref_count: Cell<i32>,
    base: ImmutablePen,
    inner: Rc<dyn IPen>,
}

impl ImmutableTrackingPen {
    fn new() -> Rc<ImmutableTrackingPen> {
        Rc::new(ImmutableTrackingPen {
            add_ref_count: Cell::new(0),
            base: ImmutablePen::with_brush(Some(Brushes::black()), 1.0),
            inner: Pen::with_brush(Some(Brushes::black()), 1.0).into(),
        })
    }
}

impl IPen for ImmutableTrackingPen {
    fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.base.brush()
    }
    fn dash_style(&self) -> Option<Rc<dyn IDashStyle>> {
        self.base.dash_style()
    }
    fn line_cap(&self) -> PenLineCap {
        self.base.line_cap()
    }
    fn line_join(&self) -> PenLineJoin {
        self.base.line_join()
    }
    fn miter_limit(&self) -> f64 {
        self.base.miter_limit()
    }
    fn thickness(&self) -> f64 {
        self.base.thickness()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_immutable_pen(&self) -> Option<&ImmutablePen> {
        Some(&self.base)
    }
    fn as_composition_render_resource(&self) -> Option<&dyn ICompositionRenderResource> {
        Some(self)
    }
    fn into_immutable_pen(self: Rc<Self>) -> Rc<ImmutablePen> {
        Rc::new(self.base.clone())
    }
}

impl ICompositionRenderResource for ImmutableTrackingPen {
    fn add_ref_on_compositor(&self, _c: &Rc<Compositor>) {
        self.add_ref_count.set(self.add_ref_count.get() + 1);
    }
    fn release_on_compositor(&self, _c: &Rc<Compositor>) {}
    fn get_for_compositor(&self, c: &Compositor) -> ServerObjectId {
        inner_resource(self.inner.as_composition_render_resource()).get_for_compositor(c)
    }
}
