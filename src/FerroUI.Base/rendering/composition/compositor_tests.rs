//! The compositor test fixture (the reference `CompositorTestsBase` /
//! `CompositorTestServices`) and the compositor tests that need no
//! controls: invalidation, clipping, lifetime, hit testing, child
//! synchronization, target rendering and readback.
//!
//! The reference tests build their trees from `Canvas`, `Border` and
//! `Decorator`. Those live in the controls crate, so here a visual that
//! fills its bounds stands in for `Border` and bounds are assigned directly
//! instead of being produced by layout.

use super::hit_testing::{CompositionHitTestAabbTree, PointCompositionHitTester};
use super::server::{ServerCompositionTarget, ServerCompositionVisual};
use super::{CompositingRenderer, Compositor, ElementComposition, ICompositionTargetDebugEvents};
use crate::layout::ILayoutRoot;
use crate::media::{
    Brushes, DrawingBrush, DrawingContext, Geometry, GeometryDrawing, IBrush, MediaContext, RectangleGeometry, Stretch,
    VisualBrush,
};
use crate::platform::LtrbRect;
use crate::rendering::testing::{ManualRenderLoop, MockPlatformRenderInterface};
use crate::rendering::{IHitTester, IPresentationSource, IRenderer};
use crate::threading::Dispatcher;
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// A visual that fills its bounds with its background, like a border.
#[repr(C)]
pub(crate) struct TestBorder {
    base: Visual,
    background: RefCell<Option<Rc<dyn IBrush>>>,
    custom_hit_test: Cell<Option<bool>>,
}

ferro_class!(TestBorder: Visual);
ferro_impl_classes!(TestBorder: FerroObjectImpl, StyledElementImpl);

impl VisualImpl for TestBorder {
    fn render(this: &Self, context: &mut DrawingContext) {
        let background = this.background.borrow().clone();
        if let Some(background) = background {
            context.fill_rectangle(&background, Rect::from_size(this.bounds().size()), 0.0);
        }
    }

    fn custom_hit_test(this: &Self, _point: Point) -> Option<bool> {
        this.custom_hit_test.get()
    }
}

impl TestBorder {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: Visual::construct(),
            background: RefCell::new(None),
            custom_hit_test: Cell::new(None),
        })
    }

    /// A border with a background at the given bounds.
    pub(crate) fn filled(x: f64, y: f64, width: f64, height: f64) -> Ref<Self> {
        let border = Self::new();
        border.set_background(Some(Brushes::red()));
        border.set_bounds(Rect::new(x, y, width, height));
        border
    }

    pub(crate) fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        *self.background.borrow_mut() = value;
        self.invalidate_visual();
    }

    fn add(&self, child: &Ref<TestBorder>) {
        self.visual_children().add(child.clone().upcast());
        // Panels report child changes through their logical children.
        self.invalidate_visual();
        if let Some(source) = self.presentation_source() {
            source.renderer().recalculate_children(self);
        }
    }

    fn remove(&self, child: &Ref<TestBorder>) {
        self.visual_children().remove(&child.clone().upcast());
        if let Some(source) = self.presentation_source() {
            source.renderer().recalculate_children(self);
        }
    }
}

struct TestSource {
    root: Ref<Visual>,
    renderer: RefCell<Option<Rc<CompositingRenderer>>>,
    client_size: Cell<Size>,
}

impl IPresentationSource for TestSource {
    fn root_visual(&self) -> Option<Ref<Visual>> {
        Some(self.root.clone())
    }
    fn render_scaling(&self) -> f64 {
        1.0
    }
    fn renderer(&self) -> Rc<dyn IRenderer> {
        self.renderer.borrow().clone().expect("the renderer is created with the source")
    }
    fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
        unimplemented!("not used by these tests")
    }
    fn hit_tester(&self) -> Rc<dyn IHitTester> {
        self.renderer.borrow().clone().expect("the renderer is created with the source")
    }
    fn input_root(&self) -> Rc<dyn crate::input::IInputRoot> {
        unimplemented!("not used by these tests")
    }
    fn client_size(&self) -> Size {
        self.client_size.get()
    }
}

#[derive(Default)]
struct DebugEvents {
    rects: RefCell<Vec<Rect>>,
    rendered_visuals: Cell<i32>,
    visited_visuals: Cell<i32>,
}

impl DebugEvents {
    fn reset(&self) {
        self.rects.borrow_mut().clear();
        self.rendered_visuals.set(0);
        self.visited_visuals.set(0);
    }
}

impl ICompositionTargetDebugEvents for DebugEvents {
    fn rendered_visuals(&self) -> i32 {
        self.rendered_visuals.get()
    }
    fn set_rendered_visuals(&self, value: i32) {
        self.rendered_visuals.set(value);
    }
    fn visited_visuals(&self) -> i32 {
        self.visited_visuals.get()
    }
    fn set_visited_visuals(&self, value: i32) {
        self.visited_visuals.set(value);
    }
    fn rect_invalidated(&self, rc: LtrbRect) {
        self.rects.borrow_mut().push(rc.to_rect());
    }
}

/// The reference `CompositorCanvas`: a compositor, a renderer over a root
/// visual, and a canvas-like visual as the only child of the root.
struct CompositorCanvas {
    // Declared first: dropped before the scopes below.
    canvas: Ref<TestBorder>,
    root: Ref<TestBorder>,
    renderer: Rc<CompositingRenderer>,
    source: Rc<TestSource>,
    compositor: Rc<Compositor>,
    events: Rc<DebugEvents>,
    render_loop: Arc<ManualRenderLoop>,
    render_interface: Rc<MockPlatformRenderInterface>,
    locator_scope: Rc<dyn crate::reactive::IDisposable>,
    _dispatcher_scope: crate::threading::UnitTestDispatcherScope,
}

impl CompositorCanvas {
    fn new() -> CompositorCanvas {
        Self::with_size(Size::new(1000.0, 1000.0))
    }

    fn with_size(size: Size) -> CompositorCanvas {
        let dispatcher_scope = Dispatcher::unit_test_scope();
        let (locator_scope, render_interface) = MockPlatformRenderInterface::install();
        let render_loop = ManualRenderLoop::new();
        let compositor = Compositor::with_scheduler(
            render_loop.clone(),
            None,
            true,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );

        let root = TestBorder::new();
        root.set_bounds(Rect::from_size(size));
        let source = Rc::new(TestSource {
            root: root.clone().upcast(),
            renderer: RefCell::new(None),
            client_size: Cell::new(size),
        });
        let as_source: Rc<dyn IPresentationSource> = source.clone();
        let weak: Weak<dyn IPresentationSource> = Rc::downgrade(&as_source);
        let renderer = CompositingRenderer::with_weak_root(weak, &compositor, Rc::new(Vec::new));
        *source.renderer.borrow_mut() = Some(renderer.clone());

        root.set_presentation_source_for_root_visual(Some(as_source));
        renderer.set_root(Some(root.clone().upcast()));
        renderer.start();

        let events = Rc::new(DebugEvents::default());
        renderer.composition_target().set_debug_events(Some(events.clone()));

        let canvas = TestBorder::new();
        canvas.set_bounds(Rect::from_size(size));
        let services = CompositorCanvas {
            canvas: canvas.clone(),
            root,
            renderer,
            source,
            compositor,
            events,
            render_loop,
            render_interface,
            locator_scope,
            _dispatcher_scope: dispatcher_scope,
        };
        services.root.add(&canvas);
        services.run_jobs();
        services.events.reset();
        services
    }

    fn run_jobs(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        self.render_loop.tick();
        Dispatcher::ui_thread().run_jobs(None);
    }

    fn assert_rects(&self, rects: &[Rect]) {
        self.run_jobs();
        let distinct = |rects: &[Rect]| {
            let mut texts: Vec<String> = rects.iter().map(|r| r.to_string()).collect();
            texts.sort();
            texts.dedup();
            texts
        };
        assert_eq!(distinct(rects), distinct(&self.events.rects.borrow()));
        self.events.rects.borrow_mut().clear();
    }

    fn assert_rendered_visuals(&self, visited_visuals: i32, render_visuals: i32) {
        self.run_jobs();
        assert_eq!(visited_visuals, self.events.visited_visuals.get());
        assert_eq!(render_visuals, self.events.rendered_visuals.get());
        self.events.rects.borrow_mut().clear();
    }

    fn assert_hit_test(&self, x: f64, y: f64, filter: Option<&dyn Fn(&Visual) -> bool>, expected: &[&Ref<TestBorder>]) {
        self.run_jobs();
        let tested = self.renderer.hit_test(Point::new(x, y), &self.root, filter);
        let expected: Vec<Ref<Visual>> = expected.iter().map(|v| (*v).clone().upcast()).collect();
        assert_eq!(expected, tested);
    }

    fn assert_hit_test_first(&self, x: f64, y: f64, filter: Option<&dyn Fn(&Visual) -> bool>, expected: Option<&Ref<TestBorder>>) {
        self.run_jobs();
        let tested = self.renderer.hit_test_first(Point::new(x, y), &self.root, filter);
        assert_eq!(expected.map(|v| v.clone().upcast::<Visual>()), tested);
    }

    fn server_visual(&self, visual: &Visual) -> Rc<ServerCompositionVisual> {
        let id = visual.composition_visual().expect("the visual is attached").server();
        self.compositor.server().get::<ServerCompositionVisual>(id).expect("the server visual exists")
    }
}

impl Drop for CompositorCanvas {
    fn drop(&mut self) {
        self.renderer.stop();
        self.root.set_presentation_source_for_root_visual(None);
        self.renderer.dispose();
        *self.source.renderer.borrow_mut() = None;
        self.locator_scope.dispose();
    }
}

// --- CompositorInvalidationTests ---------------------------------------------------

#[test]
fn control_should_invalidate_own_rect_when_added() {
    let s = CompositorCanvas::new();
    let control = TestBorder::filled(30.0, 50.0, 20.0, 10.0);
    s.canvas.add(&control);
    s.assert_rects(&[Rect::new(30.0, 50.0, 20.0, 10.0)]);
}

#[test]
fn control_should_invalidate_own_rect_when_removed() {
    let s = CompositorCanvas::new();
    let control = TestBorder::filled(30.0, 50.0, 20.0, 10.0);
    s.canvas.add(&control);
    s.run_jobs();
    s.events.rects.borrow_mut().clear();
    s.canvas.remove(&control);
    s.assert_rects(&[Rect::new(30.0, 50.0, 20.0, 10.0)]);
}

#[test]
fn sibling_controls_should_invalidate_union_rect_when_removed() {
    let s = CompositorCanvas::new();
    let control = TestBorder::filled(30.0, 10.0, 20.0, 10.0);
    let control2 = TestBorder::filled(30.0, 50.0, 20.0, 10.0);
    control2.set_background(Some(Brushes::blue()));
    s.canvas.add(&control);
    s.canvas.add(&control2);
    s.run_jobs();
    s.events.rects.borrow_mut().clear();
    s.canvas.remove(&control);
    s.canvas.remove(&control2);
    s.assert_rects(&[Rect::new(30.0, 10.0, 20.0, 50.0)]);
}

#[test]
fn control_should_invalidate_both_own_rects_when_moved() {
    let s = CompositorCanvas::new();
    let control = TestBorder::filled(30.0, 50.0, 20.0, 10.0);
    s.canvas.add(&control);
    s.run_jobs();
    s.events.rects.borrow_mut().clear();
    control.set_bounds(Rect::new(55.0, 50.0, 20.0, 10.0));
    s.assert_rects(&[Rect::new(30.0, 50.0, 20.0, 10.0), Rect::new(55.0, 50.0, 20.0, 10.0)]);
}

/// A decorator with a padding of 10 around a 20x10 border.
fn decorator_with_child() -> (Ref<TestBorder>, Ref<TestBorder>) {
    let control = TestBorder::new();
    control.set_bounds(Rect::new(30.0, 50.0, 40.0, 30.0));
    let child = TestBorder::filled(10.0, 10.0, 20.0, 10.0);
    control.add(&child);
    (control, child)
}

#[test]
fn control_should_invalidate_child_rects_when_moved() {
    let s = CompositorCanvas::new();
    let (control, _child) = decorator_with_child();
    s.canvas.add(&control);
    s.run_jobs();
    s.events.rects.borrow_mut().clear();
    control.set_bounds(Rect::new(55.0, 50.0, 40.0, 30.0));
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0), Rect::new(65.0, 60.0, 20.0, 10.0)]);
}

#[test]
fn control_should_invalidate_child_rects_when_becomes_invisible() {
    let s = CompositorCanvas::new();
    let (control, _child) = decorator_with_child();
    s.canvas.add(&control);
    s.run_jobs();
    s.events.rects.borrow_mut().clear();
    control.set_is_visible(false);
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0)]);
}

// --- CompositorInvalidationClippingTests -------------------------------------------

fn count_visuals(visual: &Visual) -> i32 {
    let mut count = 1;
    if let Some(children) = visual.visual_children_snapshot() {
        for child in children.iter() {
            count += count_visuals(child);
        }
    }
    count
}

// The root of the fixture (the reference tree has two visuals above the canvas).
const TOP_LEVEL_OVERHEAD: i32 = 1;

fn do_not_re_render_unaffected_visual_trees(
    clip_to_bounds: bool,
    clip_geometry: bool,
    canvas_has_content: bool,
    expected_visited_visuals_count: i32,
    expected_rendered_visuals_count: i32,
) {
    let s = CompositorCanvas::new();
    for (x, y) in [(0.0, 0.0), (30.0, 50.0)] {
        let border = TestBorder::filled(x, y, 20.0, 10.0);
        border.set_clip_to_bounds(clip_to_bounds);
        if clip_geometry {
            border.set_clip(RectangleGeometry::with_rect(Rect::from_size(Size::new(20.0, 10.0))).upcast::<crate::media::Geometry>());
        }
        s.canvas.add(&border);
    }

    if canvas_has_content {
        s.canvas.set_background(Some(Brushes::green()));
    }
    s.run_jobs();
    s.events.reset();

    assert_eq!(3 + TOP_LEVEL_OVERHEAD, count_visuals(&s.root), "the tree of the test is broken");

    // invalidate border1
    s.canvas.visual_children().snapshot()[0].set_is_visible(false);
    s.run_jobs();

    s.assert_rendered_visuals(expected_visited_visuals_count, expected_rendered_visuals_count);
}

#[test]
fn do_not_re_render_unaffected_visual_trees_cases() {
    // If canvas itself has no background, the second render won't draw any visuals at all, since
    // root visual's subtree bounds will exactly match the second visual
    do_not_re_render_unaffected_visual_trees(false, false, false, 1, 0);
    do_not_re_render_unaffected_visual_trees(true, false, false, 1, 0);
    do_not_re_render_unaffected_visual_trees(false, true, false, 1, 0);
    // If canvas has background, the second render will draw only the canvas visual itself
    // (and the root above it); every visual is visited.
    do_not_re_render_unaffected_visual_trees(false, false, true, 3 + TOP_LEVEL_OVERHEAD, 1 + TOP_LEVEL_OVERHEAD);
    do_not_re_render_unaffected_visual_trees(true, false, true, 3 + TOP_LEVEL_OVERHEAD, 1 + TOP_LEVEL_OVERHEAD);
    do_not_re_render_unaffected_visual_trees(false, true, true, 3 + TOP_LEVEL_OVERHEAD, 1 + TOP_LEVEL_OVERHEAD);
}

// --- CompositorLifetimeTests -------------------------------------------------------

#[test]
fn invalidate_visual_does_not_update_rendering_target_when_rendering_stopped() {
    let services = CompositorCanvas::with_size(Size::new(200.0, 200.0));
    let composition_target = services.renderer.composition_target().clone();
    assert!(composition_target.is_enabled());
    assert_eq!(Size::new(200.0, 200.0), composition_target.size());

    // Stop rendering and invalidate a visual: this should not result in an update
    services.renderer.stop();
    services.source.client_size.set(Size::new(300.0, 300.0));
    services.root.invalidate_visual();
    services.run_jobs();
    assert_eq!(Size::new(200.0, 200.0), composition_target.size());

    // Check that restarting rendering re-queues the pending invalidation
    services.renderer.start();
    services.run_jobs();
    assert_eq!(Size::new(300.0, 300.0), composition_target.size());
}

// --- hit testing (CompositorHitTestingTests, without controls) ----------------------

#[test]
fn hit_test_should_find_controls_at_point() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(450.0, 450.0, 100.0, 100.0);
    s.canvas.add(&border);

    s.assert_hit_test(500.0, 500.0, None, &[&border]);
    s.assert_hit_test(10.0, 10.0, None, &[]);
    s.assert_hit_test_first(500.0, 500.0, None, Some(&border));
    s.assert_hit_test_first(10.0, 10.0, None, None);
}

#[test]
fn hit_test_should_not_find_invisible_controls_at_point() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(450.0, 450.0, 100.0, 100.0);
    let child = TestBorder::filled(0.0, 0.0, 100.0, 100.0);
    border.add(&child);
    border.set_is_visible(false);
    s.canvas.add(&border);

    s.assert_hit_test(500.0, 500.0, None, &[]);
}

#[test]
fn hit_test_should_not_find_control_outside_point() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(450.0, 450.0, 100.0, 100.0);
    s.canvas.add(&border);

    s.assert_hit_test(10.0, 10.0, None, &[]);
    s.assert_hit_test(551.0, 500.0, None, &[]);
}

#[test]
fn hit_test_should_return_top_controls_first() {
    let s = CompositorCanvas::new();
    let bottom = TestBorder::filled(400.0, 400.0, 200.0, 200.0);
    let top = TestBorder::filled(450.0, 450.0, 100.0, 100.0);
    s.canvas.add(&bottom);
    s.canvas.add(&top);

    s.assert_hit_test(500.0, 500.0, None, &[&top, &bottom]);
    s.assert_hit_test_first(500.0, 500.0, None, Some(&top));
}

#[test]
fn hit_test_should_return_top_controls_first_with_z_index() {
    let s = CompositorCanvas::new();
    let a = TestBorder::filled(400.0, 400.0, 200.0, 200.0);
    a.set_z_index(1);
    let b = TestBorder::filled(450.0, 450.0, 100.0, 100.0);
    let c = TestBorder::filled(425.0, 425.0, 150.0, 150.0);
    c.set_z_index(2);
    s.canvas.add(&a);
    s.canvas.add(&b);
    s.canvas.add(&c);

    s.assert_hit_test(500.0, 500.0, None, &[&c, &a, &b]);

    // Changing the Z index reorders the composition children.
    b.set_z_index(3);
    s.assert_hit_test(500.0, 500.0, None, &[&b, &c, &a]);
}

#[test]
fn hit_test_should_find_control_translated_outside_parent_bounds() {
    let s = CompositorCanvas::new();
    let parent = TestBorder::filled(100.0, 100.0, 100.0, 100.0);
    let child = TestBorder::filled(150.0, 0.0, 50.0, 50.0);
    parent.add(&child);
    s.canvas.add(&parent);

    s.assert_hit_test(260.0, 110.0, None, &[&child]);

    // Clipping the parent to its bounds hides the child from hit testing.
    parent.set_clip_to_bounds(true);
    s.assert_hit_test(260.0, 110.0, None, &[]);
}

#[test]
fn hit_test_should_respect_geometry_clip() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(100.0, 100.0, 200.0, 200.0);
    border.set_clip(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 50.0, 50.0)).upcast::<crate::media::Geometry>());
    s.canvas.add(&border);

    s.assert_hit_test(120.0, 120.0, None, &[&border]);
    s.assert_hit_test(200.0, 200.0, None, &[]);
}

#[test]
fn hit_test_should_respect_filter() {
    let s = CompositorCanvas::new();
    let parent = TestBorder::filled(100.0, 100.0, 100.0, 100.0);
    let child = TestBorder::filled(0.0, 0.0, 50.0, 50.0);
    parent.add(&child);
    let other = TestBorder::filled(100.0, 100.0, 100.0, 100.0);
    s.canvas.add(&other);
    s.canvas.add(&parent);

    s.assert_hit_test(110.0, 110.0, None, &[&child, &parent, &other]);

    // A filtered-out visual excludes its subtree.
    let parent_visual: Ref<Visual> = parent.clone().upcast();
    let filter = move |v: &Visual| v.to_ref() != parent_visual;
    s.assert_hit_test(110.0, 110.0, Some(&filter), &[&other]);
    s.assert_hit_test_first(110.0, 110.0, Some(&filter), Some(&other));
}

#[test]
fn hit_test_should_respect_render_transform() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(100.0, 100.0, 100.0, 100.0);
    border.set_render_transform(Some(Rc::new(crate::media::immutable::ImmutableTransform::new(
        Matrix::create_translation(300.0, 0.0),
    ))));
    s.canvas.add(&border);

    s.assert_hit_test(150.0, 150.0, None, &[]);
    s.assert_hit_test(450.0, 150.0, None, &[&border]);
}

#[test]
fn hit_test_should_honour_custom_hit_test() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(100.0, 100.0, 100.0, 100.0);
    border.custom_hit_test.set(Some(false));
    s.canvas.add(&border);
    s.assert_hit_test(150.0, 150.0, None, &[]);

    // A custom hit test may hit outside of what the visual draws.
    let outside = TestBorder::new();
    outside.set_bounds(Rect::new(300.0, 300.0, 10.0, 10.0));
    outside.custom_hit_test.set(Some(true));
    s.canvas.add(&outside);
    s.assert_hit_test(600.0, 600.0, None, &[&outside]);
}

#[test]
fn hit_test_uses_the_child_index_for_many_children() {
    let s = CompositorCanvas::new();
    let mut borders = Vec::new();
    for i in 0..100 {
        let border = TestBorder::filled((i % 10) as f64 * 50.0, (i / 10) as f64 * 50.0, 40.0, 40.0);
        s.canvas.add(&border);
        borders.push(border);
    }
    // Two overlapping visuals in different buckets.
    let overlapping = TestBorder::filled(0.0, 0.0, 40.0, 40.0);
    s.canvas.add(&overlapping);

    s.assert_hit_test(10.0, 10.0, None, &[&overlapping, &borders[0]]);
    s.assert_hit_test_first(10.0, 10.0, None, Some(&overlapping));
    s.assert_hit_test(460.0, 460.0, None, &[&borders[99]]);
    s.assert_hit_test(45.0, 45.0, None, &[]);

    // Moving and removing children keeps the index up to date.
    borders[99].set_bounds(Rect::new(700.0, 700.0, 40.0, 40.0));
    s.assert_hit_test(460.0, 460.0, None, &[]);
    s.assert_hit_test(710.0, 710.0, None, &[&borders[99]]);
    s.canvas.remove(&overlapping);
    s.assert_hit_test(10.0, 10.0, None, &[&borders[0]]);

    let canvas_visual = s.canvas.composition_visual().unwrap();
    let tree = canvas_visual.hit_test_children.borrow();
    let tree = tree.as_ref().expect("the children are indexed");
    tree.validate();
    assert_eq!(4, tree.bucket_count());
}

// --- the hit test index ------------------------------------------------------------

#[test]
fn aabb_tree_orders_candidates_and_follows_changes() {
    let s = CompositorCanvas::new();
    let mut borders = Vec::new();
    for i in 0..70 {
        let border = TestBorder::filled(i as f64 * 10.0, 0.0, 30.0, 30.0);
        s.canvas.add(&border);
        borders.push(border);
    }
    s.run_jobs();
    let canvas_visual = s.canvas.composition_visual().unwrap();
    let revision = {
        let readback = s.compositor.server().readback();
        readback.next_read();
        readback.read_revision()
    };
    let mut tree = CompositionHitTestAabbTree::new(canvas_visual.children().clone());
    tree.validate();
    assert_eq!(3, tree.bucket_count());

    let query = |tree: &mut CompositionHitTestAabbTree, x: f64| {
        let mut results = Vec::new();
        tree.query::<PointCompositionHitTester>(&Point::new(x, 5.0), &mut results, revision);
        let children = canvas_visual.children();
        results.iter().map(|v| children.index_of(v).unwrap()).collect::<Vec<_>>()
    };
    // The bounds are fattened by one unit; candidates are topmost first.
    assert_eq!(vec![33, 32, 31, 30], query(&mut tree, 330.5));
    assert_eq!(vec![0], query(&mut tree, 5.0));
    assert!(query(&mut tree, 900.0).is_empty());

    let removed = canvas_visual.children().get(32);
    tree.remove(&removed);
    tree.validate();
    assert_eq!(vec![33, 31, 30], query(&mut tree, 330.5));

    tree.clear();
    assert_eq!(0, tree.bucket_count());
    assert!(query(&mut tree, 5.0).is_empty());
}

// --- child synchronization -----------------------------------------------------------

#[test]
fn composition_children_follow_visual_children_and_z_index() {
    let s = CompositorCanvas::new();
    let a = TestBorder::filled(0.0, 0.0, 10.0, 10.0);
    let b = TestBorder::filled(0.0, 0.0, 10.0, 10.0);
    let c = TestBorder::filled(0.0, 0.0, 10.0, 10.0);
    s.canvas.add(&a);
    s.canvas.add(&b);
    s.canvas.add(&c);
    s.run_jobs();

    let canvas_visual = s.canvas.composition_visual().unwrap();
    let order = |expected: &[&Ref<TestBorder>]| {
        let children = canvas_visual.children().items();
        assert_eq!(expected.len(), children.len());
        for (child, expected) in children.iter().zip(expected) {
            assert!(Rc::ptr_eq(child, &expected.composition_visual().unwrap()));
            assert!(Rc::ptr_eq(&child.parent().unwrap(), &canvas_visual));
        }
    };
    order(&[&a, &b, &c]);

    a.set_z_index(5);
    s.run_jobs();
    order(&[&b, &c, &a]);

    // The server sees the same order.
    let server_children = s.server_visual(&s.canvas).children().unwrap().list();
    let ids: Vec<_> = [&b, &c, &a].iter().map(|v| v.composition_visual().unwrap().server()).collect();
    for (child, id) in server_children.iter().zip(&ids) {
        assert!(Rc::ptr_eq(child, &s.compositor.server().get::<ServerCompositionVisual>(*id).unwrap()));
    }

    // A child composition visual comes last.
    let extra = s.compositor.create_solid_color_visual();
    ElementComposition::set_element_child_visual(&s.canvas, Some((*extra).clone()));
    s.run_jobs();
    let children = canvas_visual.children().items();
    assert_eq!(4, children.len());
    assert!(Rc::ptr_eq(&children[3], &extra));

    let removed_visual = b.composition_visual().unwrap();
    s.canvas.remove(&b);
    s.run_jobs();
    assert!(b.composition_visual().is_none());
    assert!(removed_visual.parent().is_none());
    assert_eq!(3, canvas_visual.children().count());
}

#[test]
fn detached_visuals_release_their_server_objects() {
    let s = CompositorCanvas::new();
    s.run_jobs();
    let baseline = s.compositor.server().object_count();

    let (control, _child) = decorator_with_child();
    s.canvas.add(&control);
    s.run_jobs();
    assert!(s.compositor.server().object_count() > baseline);

    s.canvas.remove(&control);
    s.run_jobs();
    // The disposal of dropped objects rides on a later batch.
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert_eq!(baseline, s.compositor.server().object_count());
}

// --- target rendering ----------------------------------------------------------------

#[test]
fn target_renders_only_the_dirty_area() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let first = TestBorder::filled(10.0, 10.0, 20.0, 20.0);
    let second = TestBorder::filled(500.0, 500.0, 20.0, 20.0);
    second.set_background(Some(Brushes::blue()));
    s.canvas.add(&first);
    s.canvas.add(&second);
    s.run_jobs();
    assert_eq!(1, log.count("DrawRectangle Red"), "{:?}", log.entries());
    assert_eq!(1, log.count("DrawRectangle Blue"));
    log.clear();

    // Nothing changed: nothing is drawn.
    s.run_jobs();
    assert!(log.entries().is_empty(), "{:?}", log.entries());

    // Only the changed visual is redrawn, clipped to its area.
    s.events.rects.borrow_mut().clear();
    second.set_background(Some(Brushes::green()));
    s.run_jobs();
    assert_eq!(0, log.count("DrawRectangle Red"), "{:?}", log.entries());
    assert_eq!(1, log.count("DrawRectangle Green"));
    assert!(log.entries().iter().any(|e| e.starts_with("PushClip 499, 499, 22, 22")), "{:?}", log.entries());
    s.assert_rects(&[Rect::new(500.0, 500.0, 20.0, 20.0)]);
}

#[test]
fn opacity_and_clip_are_pushed_around_the_content() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let border = TestBorder::filled(10.0, 10.0, 20.0, 20.0);
    border.set_opacity(0.5);
    border.set_clip_to_bounds(true);
    s.canvas.add(&border);
    s.run_jobs();

    let entries = log.entries();
    let position = |prefix: &str| entries.iter().position(|e| e.starts_with(prefix)).unwrap_or_else(|| panic!("{prefix}: {entries:?}"));
    let push_opacity = position("PushOpacity 0.5");
    let push_clip = position("PushClip 0, 0, 20, 20");
    let draw = position("DrawRectangle Red");
    let pop_opacity = position("PopOpacity");
    assert!(push_opacity < push_clip && push_clip < draw && draw < pop_opacity, "{entries:?}");
}

// --- readback ------------------------------------------------------------------------

#[test]
fn readback_reports_the_transform_and_bounds_of_the_last_frame() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(30.0, 50.0, 20.0, 10.0);
    s.canvas.add(&border);
    let visual = border.composition_visual().unwrap();

    // Nothing was rendered yet.
    s.compositor.server().readback().next_read();
    assert!(visual.try_get_valid_readback().is_none());

    s.run_jobs();
    s.compositor.server().readback().next_read();
    let readback = visual.try_get_valid_readback().expect("the visual was rendered");
    assert_eq!(Matrix::create_translation(30.0, 50.0), readback.matrix);
    assert_eq!(Some(LtrbRect::new(30.0, 50.0, 50.0, 60.0)), readback.transformed_subtree_bounds);
    assert!(readback.visible);
    assert_eq!(s.renderer.composition_target().id(), readback.target_id);

    let server_target =
        s.compositor.server().get::<ServerCompositionTarget>(s.renderer.composition_target().server()).unwrap();
    assert_eq!(server_target.id(), readback.target_id);

    // The reader keeps its revision until it asks for the next one.
    border.set_bounds(Rect::new(70.0, 50.0, 20.0, 10.0));
    s.run_jobs();
    assert_eq!(Matrix::create_translation(30.0, 50.0), visual.try_get_valid_readback().unwrap().matrix);
    s.compositor.server().readback().next_read();
    assert_eq!(Matrix::create_translation(70.0, 50.0), visual.try_get_valid_readback().unwrap().matrix);

    // An invisible visual has no valid readback.
    border.set_is_visible(false);
    s.run_jobs();
    s.compositor.server().readback().next_read();
    assert!(visual.try_get_valid_readback().is_none());
}

#[test]
fn scene_invalidated_is_raised_after_a_frame() {
    let s = CompositorCanvas::new();
    let raised = Rc::new(Cell::new(0));
    let r = raised.clone();
    let subscription = s.renderer.scene_invalidated(Rc::new(move |e| {
        assert_eq!(Rect::new(0.0, 0.0, 1000.0, 1000.0), e.dirty_rect());
        r.set(r.get() + 1);
    }));
    s.canvas.add(&TestBorder::filled(0.0, 0.0, 10.0, 10.0));
    s.run_jobs();
    assert_eq!(1, raised.get());
    subscription.dispose();
}

#[test]
fn paint_renders_immediately() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    s.canvas.add(&TestBorder::filled(0.0, 0.0, 10.0, 10.0));
    log.clear();
    // No dispatcher job and no render loop tick in between.
    s.renderer.paint(Rect::new(0.0, 0.0, 1000.0, 1000.0));
    assert_eq!(1, log.count("DrawRectangle Red"), "{:?}", log.entries());
}

// --- bitmap cache --------------------------------------------------------------------

#[test]
fn bitmap_cache_renders_the_subtree_once_and_redraws_from_the_layer() {
    use crate::media::{BitmapCache, CacheMode};

    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let (control, child) = decorator_with_child();
    let cache = BitmapCache::new();
    control.set_cache_mode(cache.clone().upcast::<CacheMode>());
    s.canvas.add(&control);
    s.run_jobs();

    let composition_visual = control.composition_visual().unwrap();
    let composition_cache = composition_visual.cache_mode().expect("the cache mode is synchronized");
    assert_eq!(1.0, composition_cache.render_at_scale());
    assert!(s.server_visual(&control).cache().is_some());
    // The subtree is drawn into the layer and the layer onto the target.
    assert_eq!(1, log.count("DrawRectangle Red"), "{:?}", log.entries());
    assert!(log.count("DrawBitmap 1 0, 0, 20, 10 10, 10, 20, 10") >= 1, "{:?}", log.entries());
    log.clear();

    // Moving the cached visual redraws the layer bitmap, not the subtree.
    control.set_bounds(Rect::new(100.0, 50.0, 40.0, 30.0));
    s.run_jobs();
    assert_eq!(0, log.count("DrawRectangle Red"), "{:?}", log.entries());
    assert!(log.count("DrawBitmap 1 0, 0, 20, 10 10, 10, 20, 10") >= 1, "{:?}", log.entries());
    log.clear();

    // A change inside the subtree re-renders the cache.
    child.set_background(Some(Brushes::blue()));
    s.run_jobs();
    assert_eq!(1, log.count("DrawRectangle Blue"), "{:?}", log.entries());

    // Changing the cache properties reaches the server and invalidates the cache.
    cache.set_render_at_scale(2.0);
    s.run_jobs();
    assert_eq!(2.0, composition_cache.render_at_scale());
    assert!(log.count("DrawBitmap 1 0, 0, 40, 20 10, 10, 20, 10") >= 1, "{:?}", log.entries());

    // Removing the cache mode removes the server cache.
    control.set_cache_mode(None::<Ref<CacheMode>>);
    control.invalidate_visual();
    s.run_jobs();
    assert!(composition_visual.cache_mode().is_none());
    assert!(s.server_visual(&control).cache().is_none());
}

// --- other visual classes ------------------------------------------------------------

#[test]
fn solid_color_and_acrylic_visuals_render() {
    use crate::media::{Colors, ImmutableExperimentalAcrylicMaterial};
    use crate::rendering::composition::CompositionExperimentalAcrylicVisual;

    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let solid = s.compositor.create_solid_color_visual();
    solid.set_size(Vector::new(30.0, 20.0));
    solid.set_offset(Vector3D::new(5.0, 5.0, 0.0));
    solid.set_color(Colors::BLUE);
    ElementComposition::set_element_child_visual(&s.canvas, Some((*solid).clone()));
    s.run_jobs();
    assert_eq!(Colors::BLUE, solid.color());
    assert_eq!(1, log.count("DrawRectangle Blue"), "{:?}", log.entries());
    s.assert_rects(&[Rect::new(5.0, 5.0, 30.0, 20.0)]);

    // An acrylic visual is a draw list visual with two more properties;
    // without acrylic support of the context only its draw list is drawn.
    let host = TestBorder::filled(100.0, 100.0, 10.0, 10.0);
    let acrylic = CompositionExperimentalAcrylicVisual::new(&s.compositor, &host);
    acrylic.set_material(ImmutableExperimentalAcrylicMaterial::default());
    acrylic.set_corner_radius(CornerRadius::uniform(4.0));
    acrylic.set_size(Vector::new(10.0, 10.0));
    assert!(CompositionExperimentalAcrylicVisual::from_visual(&acrylic).is_some());
    assert!(CompositionExperimentalAcrylicVisual::from_visual(&solid).is_none());
    ElementComposition::set_element_child_visual(&s.canvas, Some((**acrylic).clone()));
    s.run_jobs();
    assert_eq!(CornerRadius::uniform(4.0), acrylic.corner_radius());
    let server = s.compositor.server().get::<ServerCompositionVisual>(acrylic.server()).unwrap();
    let content = server
        .content_as::<crate::rendering::composition::server::ServerCompositionExperimentalAcrylicVisual>()
        .expect("the server visual is an acrylic visual");
    assert_eq!(CornerRadius::uniform(4.0), content.props().corner_radius());
    s.assert_rects(&[Rect::new(5.0, 5.0, 30.0, 20.0), Rect::new(0.0, 0.0, 10.0, 10.0)]);
}

// --- geometry hit testing --------------------------------------------------------------

#[test]
fn geometry_hit_test_finds_intersecting_controls() {
    let s = CompositorCanvas::new();
    let inside = TestBorder::filled(100.0, 100.0, 50.0, 50.0);
    let outside = TestBorder::filled(600.0, 600.0, 50.0, 50.0);
    s.canvas.add(&inside);
    s.canvas.add(&outside);
    s.run_jobs();

    let geometry = RectangleGeometry::with_rect(Rect::new(90.0, 90.0, 100.0, 100.0)).upcast::<crate::media::Geometry>();
    let tested = s.renderer.hit_test_geometry(&geometry, &s.root, None);
    assert_eq!(1, tested.len());
    assert_eq!(inside.clone().upcast::<Visual>(), tested[0].visual_hit);

    let first = s.renderer.hit_test_first_geometry(&geometry, &s.root, None).expect("a visual is hit");
    assert_eq!(inside.upcast::<Visual>(), first.visual_hit);

    let nothing = RectangleGeometry::with_rect(Rect::new(300.0, 300.0, 10.0, 10.0)).upcast::<crate::media::Geometry>();
    assert!(s.renderer.hit_test_geometry(&nothing, &s.root, None).is_empty());
    assert!(s.renderer.hit_test_first_geometry(&nothing, &s.root, None).is_none());
}

// --- DrawingBrushPropagationTests -----------------------------------------------------

fn drawing_brush_border(brush: &Ref<DrawingBrush>) -> Ref<TestBorder> {
    let border = TestBorder::new();
    border.set_background(Some(brush.into()));
    border.set_bounds(Rect::new(30.0, 50.0, 20.0, 10.0));
    border
}

fn rectangle_drawing(brush: Rc<dyn IBrush>, geometry: &Ref<RectangleGeometry>) -> Ref<GeometryDrawing> {
    let drawing = GeometryDrawing::new();
    drawing.set_brush(Some(brush));
    drawing.set_geometry(geometry.clone().upcast::<Geometry>());
    drawing
}

#[test]
fn mutating_geometry_inside_drawing_brush_invalidates_consumer() {
    let s = CompositorCanvas::new();

    let geometry = RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 20.0, 10.0));
    let brush = DrawingBrush::with_drawing(&rectangle_drawing(Brushes::red(), &geometry));
    s.canvas.add(&drawing_brush_border(&brush));
    s.run_jobs();
    s.events.rects.borrow_mut().clear();

    geometry.set_rect(Rect::new(0.0, 0.0, 30.0, 15.0));

    s.assert_rects(&[Rect::new(30.0, 50.0, 20.0, 10.0)]);
}

#[test]
fn replacing_drawing_invalidates_consumer() {
    let s = CompositorCanvas::new();

    let brush = DrawingBrush::with_drawing(&rectangle_drawing(
        Brushes::red(),
        &RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 20.0, 10.0)),
    ));
    s.canvas.add(&drawing_brush_border(&brush));
    s.run_jobs();
    s.events.rects.borrow_mut().clear();

    brush.set_drawing(&rectangle_drawing(
        Brushes::blue(),
        &RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 20.0, 10.0)),
    ));

    s.assert_rects(&[Rect::new(30.0, 50.0, 20.0, 10.0)]);
}

// --- scene brushes on a visual (not from upstream) --------------------------------------

#[test]
fn visual_brush_as_opacity_mask_is_a_server_side_scene_brush() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    // The visual of the brush is not part of the tree.
    let source = TestBorder::filled(0.0, 0.0, 8.0, 4.0);
    let mask = VisualBrush::with_visual(&source.clone().upcast::<Visual>());
    let border = TestBorder::filled(10.0, 10.0, 20.0, 20.0);
    border.set_opacity_mask(Some((&mask).into()));
    s.canvas.add(&border);
    s.run_jobs();

    let entries = log.entries();
    let position = |prefix: &str| entries.iter().position(|e| e.starts_with(prefix)).unwrap_or_else(|| panic!("{prefix}: {entries:?}"));
    let push_mask = position("PushOpacityMask brush");
    let draw = position("DrawRectangle Red none 0, 0, 20, 20");
    let pop_mask = position("PopOpacityMask");
    assert!(push_mask < draw && draw < pop_mask, "{entries:?}");

    // The mask of the server visual is the server-side counterpart of the
    // brush, with the rendering of the visual as its content.
    let content_of = |visual: &Ref<TestBorder>| {
        let mask = s.server_visual(visual).opacity_mask_brush().expect("the server visual has a mask");
        let content = mask.as_scene_brush().expect("a scene brush").create_content();
        (mask, content)
    };
    let (server_mask, content) = content_of(&border);
    let content = content.expect("the visual draws something");
    assert_eq!(content.rect(), Rect::new(0.0, 0.0, 8.0, 4.0));
    assert_eq!(server_mask.as_tile_brush().unwrap().stretch(), Stretch::Uniform);
    let content_log = crate::rendering::testing::DrawingLog::new();
    let mut context = crate::rendering::testing::MockDrawingContextImpl::new(content_log.clone());
    context.log_transforms = false;
    content.render(&mut context, None);
    assert!(
        content_log.entries().iter().any(|e| e == "DrawRectangle Red none 0, 0, 8, 4 shadows=0"),
        "{:?}",
        content_log.entries()
    );

    // A change of the brush records its content again and redraws what it
    // masks.
    s.events.rects.borrow_mut().clear();
    source.set_bounds(Rect::new(0.0, 0.0, 6.0, 6.0));
    mask.set_stretch(Stretch::Fill);
    s.assert_rects(&[Rect::new(10.0, 10.0, 20.0, 20.0)]);
    let (server_mask, content) = content_of(&border);
    assert_eq!(server_mask.as_tile_brush().unwrap().stretch(), Stretch::Fill);
    assert_eq!(content.expect("the visual draws something").rect(), Rect::new(0.0, 0.0, 6.0, 6.0));

    // Without a visual the brush has no content.
    mask.set_visual(None);
    s.run_jobs();
    assert!(content_of(&border).1.is_none());

    // Removing the mask releases the brush on the compositor.
    assert!(mask.is_on_compositor(&s.compositor));
    border.set_opacity_mask(None);
    s.run_jobs();
    assert!(!mask.is_on_compositor(&s.compositor));
    assert!(s.server_visual(&border).opacity_mask_brush().is_none());
}

#[test]
fn visual_brush_as_background_is_drawn_through_its_server_side_counterpart() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let source = TestBorder::filled(0.0, 0.0, 8.0, 4.0);
    let brush = VisualBrush::with_visual(&source.clone().upcast::<Visual>());
    let border = TestBorder::new();
    border.set_background(Some((&brush).into()));
    border.set_bounds(Rect::new(30.0, 50.0, 20.0, 10.0));
    s.canvas.add(&border);
    s.run_jobs();
    assert!(log.entries().iter().any(|e| e == "DrawRectangle brush none 0, 0, 20, 10 shadows=0"), "{:?}", log.entries());
    assert!(brush.is_on_compositor(&s.compositor));

    s.events.rects.borrow_mut().clear();
    brush.set_opacity(0.5);
    s.assert_rects(&[Rect::new(30.0, 50.0, 20.0, 10.0)]);

    s.canvas.remove(&border);
    s.run_jobs();
    assert!(!brush.is_on_compositor(&s.compositor));
}

// --- serialization queue (not from upstream) ---------------------------------------------

struct InertServerObject;

impl super::server::IServerObject for InertServerObject {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn std::any::Any> {
        self
    }
}

type PrepareAction = Box<dyn FnOnce(&Compositor)>;

/// Something a compositor serializes: counts how often, and may run an
/// action when it is about to be serialized.
struct SerializationProbe {
    server: super::server::ServerObjectId,
    serialized: Rc<Cell<u32>>,
    on_prepare: RefCell<Option<PrepareAction>>,
}

impl SerializationProbe {
    fn new(compositor: &Compositor, serialized: &Rc<Cell<u32>>, on_prepare: Option<PrepareAction>) -> Rc<Self> {
        Rc::new(SerializationProbe {
            server: compositor.create_server_object(|_, _| Rc::new(InertServerObject)),
            serialized: serialized.clone(),
            on_prepare: RefCell::new(on_prepare),
        })
    }
}

impl super::ICompositorSerializable for SerializationProbe {
    fn try_get_server(&self, _c: &Compositor) -> Option<super::server::ServerObjectId> {
        Some(self.server)
    }

    fn prepare_serialization(&self, c: &Compositor) {
        let action = self.on_prepare.borrow_mut().take();
        if let Some(action) = action {
            action(c);
        }
    }

    fn serialize_changes(&self, _c: &Compositor, _writer: &mut super::transport::BatchStreamWriter<'_>) {
        self.serialized.set(self.serialized.get() + 1);
    }
}

#[test]
fn an_object_created_while_another_is_serialized_is_serialized_with_the_same_batch() {
    let s = CompositorCanvas::new();
    let (early, late) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));

    // The queue holds the only reference to each of the first objects: they
    // are released once they have been serialized. An object of the same
    // type created after that, while a later object is serialized, is likely
    // to be given the memory of one of them; it must not be taken for an
    // object that is queued already.
    for _ in 0..8 {
        s.compositor.register_for_serialization(SerializationProbe::new(&s.compositor, &early, None));
    }
    let late_counter = late.clone();
    let creating = SerializationProbe::new(
        &s.compositor,
        &early,
        Some(Box::new(move |c: &Compositor| {
            for _ in 0..8 {
                c.register_for_serialization(SerializationProbe::new(c, &late_counter, None));
            }
        })),
    );
    s.compositor.register_for_serialization(creating);

    s.compositor.commit();
    assert_eq!(9, early.get());
    assert_eq!(8, late.get());

    // The batch is applied: the server objects were created before use.
    s.run_jobs();
}

/// Port of `CompositionAnimationTests.ExpressionAnimation_Requeues_Target_When_Another_Animation_Is_Invalidated_During_Evaluation`.
#[test]
fn expression_animation_requeues_target_when_another_animation_is_invalidated_during_evaluation() {
    use super::ICompositionObjectAnimations;

    let s = CompositorCanvas::new();
    let border = TestBorder::filled(0.0, 0.0, 10.0, 10.0);
    s.canvas.add(&border);
    s.run_jobs();

    let visual = ElementComposition::get_element_visual(&border).unwrap();
    let opacity_animation = visual.compositor().create_expression_animation_with("this.Target.RotationAngle * 0.1");
    let rotation_animation = visual.compositor().create_expression_animation_with("this.Target.Offset.X * 0.5");

    visual.start_animation("Opacity", &*opacity_animation);
    visual.start_animation("RotationAngle", &*rotation_animation);

    s.run_jobs();
    visual.set_offset(Vector3D::new(100.0, 0.0, 0.0));
    s.run_jobs();

    let server = s.compositor.server().get::<ServerCompositionVisual>(visual.server()).unwrap();
    assert_eq!(50.0, server.rotation_angle());
    assert_eq!(5.0, server.opacity());
}

/// Not from upstream: a custom visual receives the messages sent to it on
/// the render thread, draws through its handler, and its handler gets
/// animation frame updates while it asks for them.
#[test]
fn custom_visual_draws_through_its_handler() {
    use crate::media::immutable::ImmutableSolidColorBrush;
    use crate::media::{Colors, ImmediateDrawingContext};
    use crate::rendering::composition::{CompositionCustomVisualHandler, ICompositionCustomVisualHandler};
    use std::any::Any;

    #[derive(Default)]
    struct Handler {
        base: CompositionCustomVisualHandler,
        messages: RefCell<Vec<i32>>,
        frames: Cell<i32>,
        clip_contains: Cell<Option<bool>>,
        size: Cell<Option<Vector>>,
    }

    impl ICompositionCustomVisualHandler for Handler {
        fn handler_base(&self) -> &CompositionCustomVisualHandler {
            &self.base
        }

        fn on_message(&self, message: Rc<dyn Any>) {
            let value = *message.downcast_ref::<i32>().unwrap();
            self.messages.borrow_mut().push(value);
            if value == 1 {
                self.base.register_for_next_animation_frame_update();
            }
        }

        fn on_animation_frame_update(&self) {
            self.frames.set(self.frames.get() + 1);
            if self.frames.get() < 3 {
                self.base.register_for_next_animation_frame_update();
            }
            self.base.invalidate();
        }

        fn on_render(&self, drawing_context: &mut ImmediateDrawingContext<'_>) {
            self.clip_contains.set(Some(self.base.render_clip_contains(Point::new(1.0, 1.0))));
            let size = self.base.effective_size();
            self.size.set(Some(size));
            drawing_context.fill_rectangle(
                &ImmutableSolidColorBrush::new(Colors::GREEN),
                Rect::new(0.0, 0.0, size.x, size.y),
                0.0,
            );
        }
    }

    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let handler = Rc::new(Handler::default());
    let visual = s.compositor.create_custom_visual(handler.clone());
    visual.set_size(Vector::new(20.0, 10.0));
    ElementComposition::set_element_child_visual(&s.canvas, Some((*visual).clone()));
    visual.send_handler_message(Rc::new(1i32));
    visual.send_handler_message(Rc::new(2i32));
    assert!(handler.messages.borrow().is_empty());
    s.run_jobs();

    assert_eq!(*handler.messages.borrow(), [1, 2]);
    assert_eq!(handler.size.get(), Some(Vector::new(20.0, 10.0)));
    assert_eq!(1, log.count("DrawRectangle Green"), "{:?}", log.entries());
    assert_eq!(handler.clip_contains.get(), Some(true));

    // The handler asked for frames: it gets them until it stops asking.
    for _ in 0..5 {
        s.run_jobs();
    }
    assert_eq!(handler.frames.get(), 3);
    assert!(!s.compositor.server().animations().need_next_tick());

    // The render APIs are only available while the handler draws.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handler.base.render_clip_contains(Point::new(1.0, 1.0))
    }));
    assert!(result.is_err());
    // The other APIs are only available while the compositor renders.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler.base.effective_size()));
    assert!(result.is_err());
}

// --- Compositor.CreateCompositionVisualSnapshot --------------------------------------
//
// Not upstream tests: upstream covers the member in the headless rendering tests
// (`Should_Render_To_A_Compositor_Snapshot_Capture`), which need a rendering backend.

#[test]
fn create_composition_visual_snapshot_faults_for_a_visual_without_a_root() {
    let s = CompositorCanvas::new();
    let visual = s.compositor.create_container_visual();

    let task = s.compositor.create_composition_visual_snapshot(&visual, 1.0);

    assert!(task.is_faulted());
    assert_eq!(
        "the visual is not attached to a composition target",
        task.exception().expect("the task is faulted").to_string()
    );
}

#[test]
fn create_composition_visual_snapshot_completes_after_the_next_batch() {
    let s = CompositorCanvas::new();
    let control = TestBorder::filled(30.0, 50.0, 20.0, 10.0);
    s.canvas.add(&control);
    s.run_jobs();
    let visual = control.composition_visual().expect("the control is attached");

    let task = s.compositor.create_composition_visual_snapshot(&visual, 2.0);

    assert!(!task.is_completed());
    s.run_jobs();
    assert!(task.is_completed_successfully());
    assert!(matches!(task.take_result(), Some(Ok(_))));
}
