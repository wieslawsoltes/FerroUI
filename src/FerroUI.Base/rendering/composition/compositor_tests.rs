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
    /// Whether it has a composition visual of its own whatever it draws.
    layer: Cell<bool>,
    /// How far past its bounds it draws, on every side.
    overdraw: Cell<f64>,
}

ferro_class!(TestBorder: Visual);
ferro_impl_classes!(TestBorder: FerroObjectImpl, StyledElementImpl);

impl VisualImpl for TestBorder {
    fn render(this: &Self, context: &mut DrawingContext) {
        let background = this.background.borrow().clone();
        if let Some(background) = background {
            let size = this.bounds().size();
            let over = this.overdraw.get();
            let rect = Rect::new(0.0 - over, 0.0 - over, size.width + 2.0 * over, size.height + 2.0 * over);
            context.fill_rectangle(&background, rect, 0.0);
        }
    }

    fn custom_hit_test(this: &Self, _point: Point) -> Option<bool> {
        this.custom_hit_test.get()
    }

    fn requires_composition_visual(this: &Self) -> bool {
        this.layer.get()
    }
}

impl TestBorder {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: Visual::construct(),
            background: RefCell::new(None),
            custom_hit_test: Cell::new(None),
            layer: Cell::new(false),
            overdraw: Cell::new(0.0),
        })
    }

    /// A border with a background at the given bounds. What it draws is shown by the
    /// composition visual of the nearest visual above it that has one (section 9.42).
    pub(crate) fn filled(x: f64, y: f64, width: f64, height: f64) -> Ref<Self> {
        let border = Self::new();
        border.set_background(Some(Brushes::red()));
        border.set_bounds(Rect::new(x, y, width, height));
        border
    }

    /// A border with a background at the given bounds that has a composition visual of its
    /// own, as a visual that sets something the compositor applies.
    pub(crate) fn layer(x: f64, y: f64, width: f64, height: f64) -> Ref<Self> {
        let border = Self::filled(x, y, width, height);
        border.layer.set(true);
        border
    }

    pub(crate) fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        *self.background.borrow_mut() = value;
        self.invalidate_visual();
    }

    fn add<T: ObjectType + Upcast<Visual>>(&self, child: &Ref<T>) {
        self.visual_children().add(child.clone().upcast());
        // Panels report child changes through their logical children.
        self.invalidate_visual();
        if let Some(source) = self.presentation_source() {
            source.renderer().recalculate_children(self);
        }
    }

    fn remove<T: ObjectType + Upcast<Visual>>(&self, child: &Ref<T>) {
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

// SAFETY: the tests create and use the receiver on one thread; the impls
// only satisfy the thread-safety bound of the contract.
unsafe impl Send for DebugEvents {}
unsafe impl Sync for DebugEvents {}

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
    events: std::sync::Arc<DebugEvents>,
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
        Self::with_size_and_setup(size, |_| {})
    }

    /// A canvas whose render interface `setup` configures before the first
    /// frame (a backend context factory).
    fn with_size_and_setup(size: Size, setup: impl FnOnce(&MockPlatformRenderInterface)) -> CompositorCanvas {
        Self::with_options(size, None, setup)
    }

    /// A canvas whose compositor has `options`.
    fn with_options(
        size: Size,
        options: Option<crate::rendering::composition::CompositionOptions>,
        setup: impl FnOnce(&MockPlatformRenderInterface),
    ) -> CompositorCanvas {
        let dispatcher_scope = Dispatcher::unit_test_scope();
        let (locator_scope, render_interface) = MockPlatformRenderInterface::install();
        setup(render_interface.as_ref());
        let render_loop = ManualRenderLoop::new();
        let compositor = Compositor::with_scheduler(
            render_loop.clone(),
            None,
            true,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            options,
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
        let renderer = CompositingRenderer::with_weak_root(weak, &compositor, std::sync::Arc::new(Vec::new));
        *source.renderer.borrow_mut() = Some(renderer.clone());

        root.set_presentation_source_for_root_visual(Some(as_source));
        renderer.set_root(Some(root.clone().upcast()));
        renderer.start();

        let events = std::sync::Arc::new(DebugEvents::default());
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

    /// The server side of the composition visual of `visual`. A visual that had none gets it
    /// by the question (section 9.41), and the batch that carries it is sent first.
    fn server_visual(&self, visual: &Visual) -> Rc<ServerCompositionVisual> {
        let id = visual.composition_visual().expect("the visual is attached").server();
        if self.compositor.server().get::<ServerCompositionVisual>(id).is_none() {
            self.run_jobs();
        }
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

#[test]
fn counted_render_suppression_updates_composition_without_changing_authored_visibility() {
    let scene = CompositorCanvas::new();
    let child = TestBorder::filled(10.0, 10.0, 40.0, 30.0);
    child.set_opacity(0.4);
    scene.canvas.add(&child);
    scene.run_jobs();
    assert!(scene.server_visual(&child).visible());
    child.add_render_suppression_request();
    child.add_render_suppression_request();
    scene.run_jobs();
    assert!(!scene.server_visual(&child).visible());
    assert!(child.is_visible());
    assert_eq!(child.opacity(), 0.4);
    assert_eq!(child.bounds(), Rect::new(10.0, 10.0, 40.0, 30.0));
    child.remove_render_suppression_request();
    scene.run_jobs();
    assert!(!scene.server_visual(&child).visible());
    child.add_composition_hide_request();
    child.remove_render_suppression_request();
    scene.run_jobs();
    assert!(!child.is_render_suppressed());
    assert!(!scene.server_visual(&child).visible(), "another composition owner still hides it");
    child.remove_composition_hide_request();
    scene.run_jobs();
    assert!(scene.server_visual(&child).visible());
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
    let control = TestBorder::layer(30.0, 10.0, 20.0, 10.0);
    let control2 = TestBorder::layer(30.0, 50.0, 20.0, 10.0);
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

/// The decorator of [`decorator_with_child`] around a border that has a composition visual
/// of its own.
fn decorator_with_layer() -> (Ref<TestBorder>, Ref<TestBorder>) {
    let control = TestBorder::new();
    control.set_bounds(Rect::new(30.0, 50.0, 40.0, 30.0));
    let child = TestBorder::layer(10.0, 10.0, 20.0, 10.0);
    control.add(&child);
    (control, child)
}

/// What is set on a composition visual asked for after a pass stays: the visual was
/// synchronized when it was created, and the next pass has nothing of it to forward
/// (section 9.41).
#[test]
fn an_offset_set_on_a_composition_visual_asked_for_after_a_pass_survives_the_next_pass() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    assert!(!control.has_composition_visual());

    let control_visual: Rc<super::CompositionVisual> = (*control.composition_visual().expect("in a tree")).clone();
    assert_eq!(Vector3D::new(30.0, 50.0, 0.0), control_visual.offset());
    control_visual.set_offset(Vector3D::new(-210.0, 50.0, 0.0));
    s.run_jobs();
    assert_eq!(Vector3D::new(-210.0, 50.0, 0.0), control_visual.offset());
    assert!(Rc::ptr_eq(&composition_parent(&child), &control_visual));

    // A change of the framework side is forwarded as before.
    control.set_bounds(Rect::new(35.0, 50.0, 40.0, 30.0));
    s.run_jobs();
    assert_eq!(Vector3D::new(35.0, 50.0, 0.0), control_visual.offset());
}

/// The decorator of [`decorator_with_child`], clipping to its bounds: it draws nothing as the
/// other does, and has a composition visual of its own, which the other has not (section 9.41).
fn clipping_decorator_with_child() -> (Ref<TestBorder>, Ref<TestBorder>) {
    let (control, child) = decorator_with_child();
    control.set_clip_to_bounds(true);
    (control, child)
}

/// The decorator of [`clipping_decorator_with_child`] around a border that has a composition
/// visual of its own.
fn clipping_decorator_with_layer() -> (Ref<TestBorder>, Ref<TestBorder>) {
    let (control, child) = decorator_with_layer();
    control.set_clip_to_bounds(true);
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
    // Every visual of the test has a composition visual, as upstream; the drawings shown by
    // the composition visual above are in the tests of section 9.42.
    s.canvas.layer.set(true);
    for (x, y) in [(0.0, 0.0), (30.0, 50.0)] {
        let border = TestBorder::layer(x, y, 20.0, 10.0);
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

#[test]
fn visuals_removed_while_rendering_stopped_are_not_kept_by_renderer() {
    let services = CompositorCanvas::new();

    let kept = TestBorder::filled(30.0, 50.0, 20.0, 10.0);
    services.canvas.add(&kept);
    services.run_jobs();

    services.renderer.stop();

    let removed_child = TestBorder::new();
    removed_child.set_background(Some(Brushes::blue()));
    removed_child.set_bounds(Rect::new(0.0, 0.0, 20.0, 10.0));
    let removed = TestBorder::new();
    removed.set_bounds(Rect::new(100.0, 50.0, 20.0, 10.0));
    removed.add(&removed_child);
    services.canvas.add(&removed);
    kept.set_background(Some(Brushes::green()));
    services.run_jobs();

    assert!(services.renderer.has_pending_update_for_unit_tests(&removed));
    assert!(services.renderer.has_pending_update_for_unit_tests(&removed_child));

    services.canvas.remove(&removed);
    services.run_jobs();

    assert!(!services.renderer.has_pending_update_for_unit_tests(&removed));
    assert!(!services.renderer.has_pending_update_for_unit_tests(&removed_child));

    assert!(services.renderer.has_pending_update_for_unit_tests(&kept));
    assert!(services.renderer.has_pending_update_for_unit_tests(&services.canvas));

    services.renderer.start();
    services.run_jobs();

    assert!(!services.renderer.has_pending_update_for_unit_tests(&kept));
    assert!(!services.renderer.has_pending_update_for_unit_tests(&services.canvas));
    services.assert_hit_test(35.0, 55.0, None, &[&kept]);
    services.assert_hit_test(105.0, 55.0, None, &[]);
}

// --- CompositionChildVisualTests --------------------------------------------------

/// Clears its child visual when it leaves the tree, the usual counterpart to
/// setting one on attach.
#[repr(C)]
struct ClearingHost {
    base: Visual,
}

ferro_class!(ClearingHost: Visual);
ferro_impl_classes!(ClearingHost: FerroObjectImpl, StyledElementImpl);

impl VisualImpl for ClearingHost {
    fn on_detached_from_visual_tree(this: &Self, _e: &VisualTreeAttachmentEventArgs) {
        ElementComposition::set_element_child_visual(this, None);
    }
}

impl ClearingHost {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Visual::construct() })
    }
}

#[test]
fn child_visual_cleared_during_detach_can_reattach() {
    let s = CompositorCanvas::new();
    let child = s.compositor.create_container_visual();
    let host = ClearingHost::new();
    host.set_bounds(Rect::new(0.0, 0.0, 100.0, 100.0));
    s.canvas.add(&host);
    s.run_jobs();
    ElementComposition::set_element_child_visual(&host, Some(child.clone()));
    s.run_jobs();

    s.canvas.remove(&host);
    s.run_jobs();
    assert!(child.parent().is_none());

    s.canvas.add(&host);
    s.run_jobs();
    ElementComposition::set_element_child_visual(&host, Some(child.clone()));
    s.run_jobs();

    let element_visual = ElementComposition::get_element_visual(&host).expect("the host is attached");
    assert!(element_visual.children().contains(&child));
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
        let border = TestBorder::layer((i % 10) as f64 * 50.0, (i / 10) as f64 * 50.0, 40.0, 40.0);
        s.canvas.add(&border);
        borders.push(border);
    }
    // Two overlapping visuals in different buckets.
    let overlapping = TestBorder::layer(0.0, 0.0, 40.0, 40.0);
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

    // The canvas draws nothing and has no composition visual: the borders are children of the
    // composition visual of the root (section 9.41).
    assert!(!s.canvas.has_composition_visual());
    let canvas_visual = own_visual(&borders[0]).parent().expect("the composition visual of the root");
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
        let border = TestBorder::layer(i as f64 * 10.0, 0.0, 30.0, 30.0);
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
    let a = TestBorder::layer(0.0, 0.0, 10.0, 10.0);
    let b = TestBorder::layer(0.0, 0.0, 10.0, 10.0);
    let c = TestBorder::layer(0.0, 0.0, 10.0, 10.0);
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
    // Region clipping is on by default: the dirty area is clipped with a region.
    target_renders_only_the_dirty_area_clipped_with(CompositorCanvas::new(), "PushClipRegion");
}

#[test]
fn target_turned_off_region_clipping_renders_only_the_dirty_rectangle() {
    let options = crate::rendering::composition::CompositionOptions {
        use_region_dirty_rect_clipping: Some(false),
        ..Default::default()
    };
    let s = CompositorCanvas::with_options(Size::new(1000.0, 1000.0), Some(options), |_| {});
    target_renders_only_the_dirty_area_clipped_with(s, "PushClip 499, 499, 22, 22");
}

fn target_renders_only_the_dirty_area_clipped_with(s: CompositorCanvas, clip: &str) {
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
    assert!(log.entries().iter().any(|e| e.starts_with(clip)), "{:?}", log.entries());
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
    // The bounds before the transform of the visual: the area an effect of the visual is drawn
    // over, in its own coordinates, which its interactive effect normalizes the pointer to (#580).
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 20.0, 10.0)), readback.subtree_bounds);
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

        fn on_message(&self, message: std::sync::Arc<dyn Any + Send + Sync>) {
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
    // The tests run the server on their own thread and look at the handler:
    // the factory hands out the one made here.
    let server_handler = crate::utilities::ThreadBound::new(handler.clone());
    let visual = s
        .compositor
        .create_custom_visual(move || server_handler.into_inner() as Rc<dyn ICompositionCustomVisualHandler>);
    visual.set_size(Vector::new(20.0, 10.0));
    ElementComposition::set_element_child_visual(&s.canvas, Some((*visual).clone()));
    visual.send_handler_message(std::sync::Arc::new(1i32));
    visual.send_handler_message(std::sync::Arc::new(2i32));
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

// --- the life cycle of server object ids ---------------------------------------------
//
// Not from upstream, where a UI-thread object holds its server object: here
// it names it by an id, which is handed out again once the server has
// dropped the object.

impl CompositorCanvas {
    fn server_object(&self, id: super::server::ServerObjectId) -> Rc<ServerCompositionVisual> {
        self.compositor.server().get::<ServerCompositionVisual>(id).expect("the server visual exists")
    }
}

/// A composition target as a renderer makes one, over no surface.
fn bare_target(s: &CompositorCanvas) -> Rc<super::CompositionTarget> {
    s.compositor.create_composition_target(std::sync::Arc::new(Vec::new))
}

/// Disposes a target as a renderer does when its top level closes.
fn dispose_out_of_band(s: &CompositorCanvas, target: &super::CompositionTarget) {
    target.set_root(None);
    target.mark_disposed_out_of_band();
    MediaContext::instance().sync_dispose_composition_target(&s.compositor, target.server());
}

/// A top level that is closed before the compositor has committed: the
/// batch of the disposal overtakes the batch that would create the server
/// target. The target is created by the batch that disposes it, and keeps
/// its id until the UI-thread target is dropped.
#[test]
fn a_target_disposed_before_its_first_commit_is_created_disposed_and_keeps_its_id() {
    let s = CompositorCanvas::new();
    s.run_jobs();
    let baseline = s.compositor.server().object_count();

    let target = bare_target(&s);
    let id = target.server();
    assert!(s.compositor.server().get_object(id).is_none(), "the creation waits for a commit");

    dispose_out_of_band(&s, &target);

    let server_target = s.compositor.server().get::<ServerCompositionTarget>(id).expect("the disposal created the server target");
    assert!(server_target.is_disposed());
    assert_eq!(baseline + 1, s.compositor.server().object_count());

    // What is created while the target is alive gets other ids, and the
    // commit that follows does not create the target again.
    let visuals: Vec<_> = (0..4).map(|_| s.compositor.create_container_visual()).collect();
    assert!(visuals.iter().all(|visual| visual.server() != id));
    s.run_jobs();
    let with_visuals = s.compositor.server().object_count();
    let objects_of_a_visual = (with_visuals - baseline - 1) / 4;
    assert!(objects_of_a_visual >= 1);
    let same = s.compositor.server().get::<ServerCompositionTarget>(id).expect("the server target is kept");
    assert!(Rc::ptr_eq(&server_target, &same));
    drop((server_target, same));

    // The release rides on the batch after the drop; the id is handed out
    // for the batch after that one.
    drop(target);
    let before_release = s.compositor.create_container_visual();
    assert_ne!(id, before_release.server());
    s.run_jobs();
    assert!(s.compositor.server().get_object(id).is_none());
    assert_eq!(with_visuals - 1 + objects_of_a_visual, s.compositor.server().object_count());

    let _after_release = s.compositor.create_container_visual();
    s.run_jobs();
    assert_eq!(with_visuals - 1 + 2 * objects_of_a_visual, s.compositor.server().object_count());
    let reused = s.compositor.server().get_object(id).expect("the id is handed out again");
    assert!(reused.into_any_rc().downcast::<ServerCompositionTarget>().is_err());
}

/// A top level that was rendered and is closed: the server target is
/// disposed out of band and keeps its id until the UI-thread target is
/// dropped, so that nothing created meanwhile takes its place.
#[test]
fn a_target_disposed_out_of_band_keeps_its_id_until_it_is_dropped() {
    let s = CompositorCanvas::new();
    let target = bare_target(&s);
    let id = target.server();
    s.run_jobs();
    let baseline = s.compositor.server().object_count();
    let server_target = s.compositor.server().get::<ServerCompositionTarget>(id).expect("the commit created the server target");
    assert!(!server_target.is_disposed());

    dispose_out_of_band(&s, &target);
    assert!(server_target.is_disposed());
    drop(server_target);

    let visual = s.compositor.create_container_visual();
    assert_ne!(id, visual.server());
    s.run_jobs();
    let with_visual = s.compositor.server().object_count();
    assert!(with_visual > baseline);
    assert!(s.compositor.server().get::<ServerCompositionTarget>(id).is_some());

    drop(target);
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert!(s.compositor.server().get_object(id).is_none());
    assert_eq!(with_visual - 1, s.compositor.server().object_count());
}

/// A second window of a compositor, as the headless tests found it: a
/// target closed before its first commit, then a target that is shown.
#[test]
fn a_target_shown_after_another_one_was_closed_before_its_first_commit_is_created() {
    let s = CompositorCanvas::new();
    s.run_jobs();

    let closed = bare_target(&s);
    let closed_root = s.compositor.create_container_visual();
    closed.set_root(Some(closed_root.clone()));
    dispose_out_of_band(&s, &closed);
    drop((closed, closed_root));

    let shown = bare_target(&s);
    let shown_root = s.compositor.create_container_visual();
    shown.set_root(Some(shown_root.clone()));
    MediaContext::instance().immediate_render_requested(&s.compositor);
    s.run_jobs();

    let server_target = s.compositor.server().get::<ServerCompositionTarget>(shown.server()).expect("the second target exists");
    assert!(!server_target.is_disposed());
    assert!(s.compositor.server().get::<ServerCompositionVisual>(shown_root.server()).is_some());
    drop(server_target);
    dispose_out_of_band(&s, &shown);
}

/// An id is returned to the free list once, however often its release is
/// asked for: two objects never share one.
#[test]
fn an_id_released_twice_is_handed_out_once() {
    let s = CompositorCanvas::new();
    let visual = s.compositor.create_container_visual();
    let id = visual.server();
    s.run_jobs();

    s.compositor.dispose_on_next_batch(id);
    s.run_jobs();
    s.compositor.dispose_on_next_batch(id);
    s.run_jobs();
    // The drop of the UI-thread object asks for the release a third time.
    drop(visual);
    s.canvas.invalidate_visual();
    s.run_jobs();

    // An id that is in the free list twice goes to two of the objects
    // created next, and the server finds the second under an id in use.
    let visuals: Vec<_> = (0..4).map(|_| s.compositor.create_container_visual()).collect();
    s.run_jobs();
    let servers: Vec<_> = visuals.iter().map(|visual| s.server_object(visual.server())).collect();
    for (index, server) in servers.iter().enumerate() {
        assert!(servers[index + 1..].iter().all(|other| !Rc::ptr_eq(server, other)));
    }
}

// The tests of the WinUI visual building blocks use the fixture of this
// module.
#[path = "composition_winui_visual_tests.rs"]
mod composition_winui_visual_tests;

#[path = "composition_shape_tests.rs"]
mod composition_shape_tests;
// The tests of the WinUI layer and redirect visuals, the nine-grid and mask
// brushes and the remaining visual members use the fixture of this module.
#[path = "composition_winui_layer_tests.rs"]
mod composition_winui_layer_tests;
// The tests of the WinUI shadows and lights (phase 5) use the fixture of
// this module.
#[path = "composition_light_tests.rs"]
mod composition_light_tests;
// The tests of the drop shadows of an alpha and of the Z of the offset
// (phase 7) use the fixture of this module.
#[path = "composition_shadow_tests.rs"]
mod composition_shadow_tests;
// The tests of the theme shadows of elements (phase 7) use the fixture of
// this module.
#[path = "composition_theme_shadow_tests.rs"]
mod composition_theme_shadow_tests;
// The tests of the WinUI effect and backdrop brushes and the effect of
// layer visuals use the fixture of this module.
#[path = "composition_winui_effect_tests.rs"]
mod composition_winui_effect_tests;

// The tests of the custom effects of `Visual.Effect` (#455) use the fixture
// of this module.
#[path = "composition_custom_effect_tests.rs"]
mod composition_custom_effect_tests;

// The tests of the WinUI materials (winui-materials.md) use the fixture of
// this module.
#[path = "composition_winui_material_tests.rs"]
mod composition_winui_material_tests;

// The tests of the lighting and the effect graph of the composition port
// (winui-materials.md) use the fixture of this module.
#[path = "composition_winui_lighting_tests.rs"]
mod composition_winui_lighting_tests;

/// The prewarm of the GPU programs (#437): a registered graph is painted
/// through the painting of the effect brushes, so the drawing context makes
/// what a brush of the graph makes. For the acrylic material: the blur, the
/// luminosity and colour blend layers, and its two sources (the backdrop
/// and the noise) drawn as the stand-in image.
#[test]
fn the_prewarm_paints_an_effect_graph_through_the_effect_brushes() {
    use crate::platform::{AlphaFormat, IPlatformRenderInterface, PixelFormats};
    use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};

    let s = CompositorCanvas::new();
    let (effect, animatable) = crate::media::acrylic_effect(
        false,
        false,
        crate::media::Color::from_argb(38, 44, 44, 44),
        crate::media::Color::from_argb(245, 44, 44, 44),
        crate::media::Color::from_rgb(44, 44, 44),
    );
    let graph = Arc::new(super::effects::EffectGraph::load(&effect, &animatable).expect("the acrylic loads"));
    let stand_in = s.render_interface.load_bitmap_from_pixels(
        PixelFormats::RGBA8888,
        AlphaFormat::Premul,
        &[255u8; 16],
        PixelSize::new(2, 2),
        Vector::new(96.0, 96.0),
        8,
    );

    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    context.supports_layer_effects = true;
    super::brushes::paint_effect_graph_for_prewarm(
        s.compositor.server(),
        graph.clone(),
        Some(stand_in),
        &mut context,
        Rect::new(0.0, 0.0, 64.0, 64.0),
    );
    let entries = log.entries();
    assert_eq!(1, log.count("PushEffect"), "{entries:?}");
    assert_eq!(log.count("PushEffect"), log.count("PopEffect"), "{entries:?}");
    let blend = |mode: &str| entries.iter().filter(|e| e.starts_with("PushBlendLayer") && e.ends_with(mode)).count();
    assert_eq!((1, 1), (blend("Luminosity"), blend("Color")), "{entries:?}");
    assert_eq!(log.count("PushBlendLayer"), log.count("PopBlendLayer"), "{entries:?}");
    assert_eq!(log.count("PushLayer"), log.count("PopLayer"), "{entries:?}");
    assert_eq!(log.count("PushClip"), log.count("PopClip"), "{entries:?}");
    // The backdrop (inside the blur) and the noise.
    assert_eq!(2, log.count("DrawBitmap"), "{entries:?}");

    // Without a stand-in, the sources draw nothing; the rest is the same.
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    context.supports_layer_effects = true;
    super::brushes::paint_effect_graph_for_prewarm(
        s.compositor.server(),
        graph,
        None,
        &mut context,
        Rect::new(0.0, 0.0, 64.0, 64.0),
    );
    assert_eq!(0, log.count("DrawBitmap"), "{:?}", log.entries());
    assert_eq!(1, log.count("PushEffect"), "{:?}", log.entries());
}

/// The prewarm of the GPU programs by the composition target (#437).
mod effect_prewarm_tests {
    use super::*;
    use crate::platform::surfaces::IPlatformRenderSurface;
    use crate::platform::{
        IDrawingContextImpl, IDrawingContextLayerImpl, IOptionalFeatureProvider, IPlatformGraphicsContext,
        IPlatformRenderInterfaceContext, IRenderTarget, RenderTargetDrawingContextProperties, RenderTargetProperties,
        RenderTargetSceneInfo,
    };
    use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl, MockDrawingContextLayerImpl, MockRenderTarget};
    use std::any::{Any, TypeId};

    /// A backend context whose render targets prewarm into a log of their
    /// own.
    struct PrewarmingContext {
        log: DrawingLog,
        prewarm_log: DrawingLog,
    }

    impl IOptionalFeatureProvider for PrewarmingContext {
        fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
            None
        }
    }

    impl IPlatformRenderInterfaceContext for PrewarmingContext {
        fn create_render_target(&self, _surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
            Rc::new(PrewarmingTarget { inner: MockRenderTarget::new(self.log.clone()), prewarm_log: self.prewarm_log.clone() })
        }

        fn create_offscreen_render_target(
            &self,
            pixel_size: PixelSize,
            _scaling: Vector,
            _enable_text_antialiasing: bool,
        ) -> Rc<dyn IDrawingContextLayerImpl> {
            Rc::new(MockDrawingContextLayerImpl::new(self.log.clone(), pixel_size))
        }

        fn is_lost(&self) -> bool {
            false
        }

        fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
            None
        }

        fn dispose(&self) {}
    }

    struct PrewarmingTarget {
        inner: Rc<MockRenderTarget>,
        prewarm_log: DrawingLog,
    }

    impl IRenderTarget for PrewarmingTarget {
        fn properties(&self) -> RenderTargetProperties {
            self.inner.properties()
        }

        fn create_drawing_context(
            &self,
            scene_info: &RenderTargetSceneInfo,
        ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
            self.inner.create_drawing_context(scene_info)
        }

        fn prewarm(&self, scene_info: &RenderTargetSceneInfo, paint: &mut dyn FnMut(&mut dyn IDrawingContextImpl)) -> bool {
            self.prewarm_log.push(format!("Prewarm {} {}", scene_info.size.width, scene_info.size.height));
            let mut context = MockDrawingContextImpl::new(self.prewarm_log.clone());
            context.supports_effects = true;
            context.supports_layer_effects = true;
            context.log_effects = true;
            paint(&mut context);
            true
        }

        fn dispose(&self) {
            self.inner.dispose();
        }
    }

    #[test]
    fn a_target_prewarms_a_registered_effect_once_after_a_frame() {
        // A colour no other test registers.
        crate::rendering::composition::effects::register_effect_prewarm(
            "compositor_tests.a_target_prewarms_a_registered_effect_once_after_a_frame",
            crate::rendering::composition::effects::ColorSourceEffect {
                name: "Fill".into(),
                color: crate::media::Color::from_rgb(1, 2, 3),
            },
            &[],
        )
        .expect("a colour fill loads");
        let fill = "DrawRectangle #ff010203";

        let prewarm_log = DrawingLog::new();
        let shared = prewarm_log.clone();
        let s = CompositorCanvas::with_size_and_setup(Size::new(100.0, 100.0), move |render_interface| {
            render_interface.set_backend_context_factory(
                move |log: &DrawingLog, _graphics: Option<Rc<dyn IPlatformGraphicsContext>>| {
                    let context: Rc<dyn IPlatformRenderInterfaceContext> =
                        Rc::new(PrewarmingContext { log: log.clone(), prewarm_log: shared.clone() });
                    context
                },
            );
        });

        // The first frame was drawn: the effect is drawn once offscreen.
        assert_eq!(1, prewarm_log.count(fill), "{:?}", prewarm_log.entries());
        assert!(prewarm_log.count("Prewarm ") >= 1, "{:?}", prewarm_log.entries());
        // Not into the frames.
        assert_eq!(0, s.render_interface.log().count(fill), "{:?}", s.render_interface.log().entries());

        // The frames that follow do not draw it again.
        let child = TestBorder::filled(10.0, 10.0, 20.0, 20.0);
        s.canvas.add(&child);
        s.run_jobs();
        child.set_background(Some(Brushes::blue()));
        s.run_jobs();
        assert_eq!(1, prewarm_log.count(fill), "{:?}", prewarm_log.entries());
        assert_eq!(0, s.render_interface.log().count(fill));
    }

    /// The custom effects FerroUI ships are prewarmed by the first frame of a
    /// composition target (#455, E9): each type of the library registers its
    /// graph or its runtime shader when the registry is first read, and the
    /// target draws each once offscreen after its first frame, as the render
    /// walk draws it: a runtime shader pushed as an effect of the drawing
    /// context at the value its type registered, a graph around the image as
    /// its layers (the colour matrices, the convolutions, the drop shadow of
    /// the glow). The frames that follow draw none of them again.
    #[test]
    fn a_target_prewarms_the_custom_effects_of_the_library_after_its_first_frame() {
        crate::media::effects::library::register_library_effects();
        let shaders = [
            "pixelate",
            "scanline",
            "spotlight",
            "dissolve-transition",
            "mask-transition",
            "pointer-spotlight",
            "reactive-grid",
            "water-ripple",
            "burning-flame",
        ];
        let graphs = ["tint", "grayscale", "sepia", "saturation", "brightness-contrast", "invert", "glow", "sharpen", "edge-detect"];
        let names = crate::rendering::composition::effects::effect_prewarm_names();
        for name in shaders.iter().chain(graphs.iter()) {
            assert!(names.contains(&format!("custom-effect:{name}")), "{name}: {names:?}");
        }

        let prewarm_log = DrawingLog::new();
        let shared = prewarm_log.clone();
        let s = CompositorCanvas::with_size_and_setup(Size::new(100.0, 100.0), move |render_interface| {
            render_interface.set_backend_context_factory(
                move |log: &DrawingLog, _graphics: Option<Rc<dyn IPlatformGraphicsContext>>| {
                    let context: Rc<dyn IPlatformRenderInterfaceContext> =
                        Rc::new(PrewarmingContext { log: log.clone(), prewarm_log: shared.clone() });
                    context
                },
            );
        });

        let pushed_shader = |log: &DrawingLog, name: &str| {
            let marker = format!(" shader {name}(");
            log.entries().iter().filter(|entry| entry.starts_with("PushEffect ") && entry.contains(&marker)).count()
        };
        let entries = prewarm_log.entries();
        for name in shaders {
            assert_eq!(1, pushed_shader(&prewarm_log, name), "{name}: {entries:?}");
        }
        // A graph is drawn at the values of its description: the colour
        // matrix layers of grayscale (a saturation of 0) and sepia (an
        // intensity of 1); the tint (white), the saturation (1) and the
        // linear transfers of brightness and contrast and of invert are the
        // identity there and draw no layer, a colour matrix layer being one
        // program whatever its matrix. Sharpen and edge detect are
        // convolution layers, the glow a drop shadow. Other effects
        // registered by the process add to these.
        assert!(prewarm_log.count("PushColorMatrixLayer") >= 2, "{entries:?}");
        assert!(prewarm_log.count("PushConvolveMatrixLayer") >= 2, "{entries:?}");
        assert!(entries.iter().any(|entry| entry.starts_with("PushEffect ") && entry.contains("drop-shadow(")), "{entries:?}");
        assert_eq!(prewarm_log.count("PushEffect"), prewarm_log.count("PopEffect"), "{entries:?}");
        assert_eq!(prewarm_log.count("PushColorMatrixLayer"), prewarm_log.count("PopColorMatrixLayer"), "{entries:?}");
        // Nothing of it into the frames.
        for name in shaders {
            assert_eq!(0, pushed_shader(s.render_interface.log(), name), "{name}");
        }

        // The frames that follow prewarm nothing again.
        let child = TestBorder::filled(10.0, 10.0, 20.0, 20.0);
        s.canvas.add(&child);
        s.run_jobs();
        child.set_background(Some(Brushes::blue()));
        s.run_jobs();
        for name in shaders {
            assert_eq!(1, pushed_shader(&prewarm_log, name), "{name}");
        }
    }
}

/// Desktop-performance.md section 9.14: a visual invalidated whose drawing comes out the same
/// keeps its render data (not sent again), and the server still draws its area again.
#[test]
fn an_unchanged_drawing_is_not_sent_again_and_its_area_is_drawn_again() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(10.0, 10.0, 20.0, 20.0);
    s.canvas.add(&border);
    s.run_jobs();
    let comp = border.composition_visual().expect("a composition visual");
    let draw_list = comp.draw_list().expect("a drawing");

    s.events.rects.borrow_mut().clear();
    border.invalidate_visual();
    s.run_jobs();
    let after = comp.draw_list().expect("a drawing");
    assert!(Rc::ptr_eq(&draw_list, &after), "the same render data");
    s.assert_rects(&[Rect::new(10.0, 10.0, 20.0, 20.0)]);

    // A different drawing is sent.
    border.set_background(Some(Brushes::blue()));
    s.run_jobs();
    let changed = comp.draw_list().expect("a drawing");
    assert!(!Rc::ptr_eq(&draw_list, &changed));
}

// --- composition visuals kept across a leave-and-return (desktop-performance.md, 9.32) -----

/// The composition visual of `visual` as the visual tree sees it: asking for it here does not
/// hand it out.
fn own_visual(visual: &Visual) -> Rc<super::CompositionVisual> {
    (*visual.own_composition_visual().expect("the visual is attached")).clone()
}

fn draw_list_of(visual: &Rc<super::CompositionVisual>) -> Option<Rc<super::drawing::CompositionRenderData>> {
    super::CompositionDrawListVisual::from_visual(visual).expect("a draw list visual").draw_list()
}

fn own_server_visual(s: &CompositorCanvas, visual: &Visual) -> Rc<ServerCompositionVisual> {
    let id = own_visual(visual).server();
    s.compositor.server().get::<ServerCompositionVisual>(id).expect("the server visual exists")
}

#[test]
fn visual_back_before_the_update_pass_keeps_its_composition_visuals() {
    let s = CompositorCanvas::new();
    let (control, child) = clipping_decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    let objects = s.compositor.server().object_count();
    let control_visual = own_visual(&control);
    let child_visual = own_visual(&child);
    let control_server = own_server_visual(&s, &control);
    let child_server = own_server_visual(&s, &child);

    // What a virtualizing panel does when it recycles a row: out of the tree and back.
    s.canvas.remove(&control);
    assert!(control.composition_visual_departed());
    assert!(child.composition_visual_departed());
    // A visual that is out of the tree has no composition visual, kept or not.
    assert!(control.composition_visual().is_none());
    assert!(child.composition_visual().is_none());
    s.canvas.add(&control);
    assert!(!control.composition_visual_departed());
    assert!(!child.composition_visual_departed());
    assert!(Rc::ptr_eq(&control_visual, &own_visual(&control)));
    assert!(Rc::ptr_eq(&child_visual, &own_visual(&child)));

    s.events.rects.borrow_mut().clear();
    s.run_jobs();
    assert!(Rc::ptr_eq(&control_visual, &own_visual(&control)));
    assert!(Rc::ptr_eq(&child_visual, &own_visual(&child)));
    assert!(Rc::ptr_eq(&control_server, &own_server_visual(&s, &control)));
    assert!(Rc::ptr_eq(&child_server, &own_server_visual(&s, &child)));
    assert!(Rc::ptr_eq(&child_visual.parent().unwrap(), &control_visual));
    assert!(Rc::ptr_eq(&control_visual.parent().unwrap(), &own_visual(&s.root)));
    assert!(draw_list_of(&child_visual).is_some());
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert_eq!(objects, s.compositor.server().object_count());
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
}

#[test]
fn visual_back_in_the_tree_is_synchronized_with_what_changed_meanwhile() {
    let s = CompositorCanvas::new();
    let (control, child) = clipping_decorator_with_child();
    s.canvas.add(&control);
    s.run_jobs();
    let control_visual = own_visual(&control);

    s.canvas.remove(&control);
    // Out of the tree: it moves, turns translucent, and its child changes its drawing.
    control.set_bounds(Rect::new(100.0, 100.0, 40.0, 30.0));
    control.set_opacity(0.5);
    child.set_background(Some(Brushes::blue()));
    s.canvas.add(&control);
    s.events.rects.borrow_mut().clear();
    // The place it was shown at and the place it is shown at now.
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0), Rect::new(110.0, 110.0, 20.0, 10.0)]);

    assert!(Rc::ptr_eq(&control_visual, &own_visual(&control)));
    assert_eq!(0.5, own_server_visual(&s, &control).opacity());
    s.assert_hit_test(115.0, 115.0, None, &[&child]);
    s.assert_hit_test(45.0, 65.0, None, &[]);
}

#[test]
fn visual_not_back_by_the_update_pass_releases_its_composition_visuals() {
    let s = CompositorCanvas::new();
    s.run_jobs();
    let baseline = s.compositor.server().object_count();
    let (control, child) = clipping_decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    let control_visual = own_visual(&control);
    let child_visual = own_visual(&child);

    s.canvas.remove(&control);
    assert!(control.composition_visual_departed());
    s.events.rects.borrow_mut().clear();
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0)]);

    assert!(!control.composition_visual_departed());
    assert!(!child.composition_visual_departed());
    assert!(control.own_composition_visual().is_none());
    assert!(child.own_composition_visual().is_none());
    assert!(control_visual.parent().is_none());
    assert!(child_visual.parent().is_none());
    assert_eq!(0, control_visual.children().count());
    assert!(draw_list_of(&child_visual).is_none());
    assert!(!child_visual.is_departed());

    drop((control_visual, child_visual));
    // The disposal of dropped objects rides on a later batch.
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert_eq!(baseline, s.compositor.server().object_count());

    // It is attached again later with composition visuals of its own.
    s.canvas.add(&control);
    s.run_jobs();
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
}

#[test]
fn composition_visuals_kept_for_a_dropped_visual_are_released() {
    let s = CompositorCanvas::new();
    s.run_jobs();
    let baseline = s.compositor.server().object_count();
    let (control, child) = clipping_decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    let control_visual = own_visual(&control);
    let child_visual = own_visual(&child);
    let weak_control = control.downgrade();
    let weak_child = child.downgrade();

    s.canvas.remove(&control);
    drop((control, child));
    // The renderer keeps the composition visuals, not the visuals.
    assert!(weak_control.upgrade().is_none());
    assert!(weak_child.upgrade().is_none());
    // Nothing is hit through what is kept for a visual that is gone.
    assert!(s.renderer.hit_test(Point::new(45.0, 65.0), &s.root, None).is_empty());
    s.run_jobs();

    assert!(control_visual.parent().is_none());
    assert_eq!(0, control_visual.children().count());
    assert!(child_visual.parent().is_none());
    assert!(draw_list_of(&child_visual).is_none());
    // The composition visual and its children no longer refer to each other.
    assert_eq!(1, Rc::strong_count(&child_visual));
    drop((control_visual, child_visual));
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert_eq!(baseline, s.compositor.server().object_count());
}

#[test]
fn composition_visual_handed_out_is_released_when_its_visual_leaves() {
    let s = CompositorCanvas::new();
    let (control, child) = clipping_decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    // Something asked for the composition visual of the child: it may hold it or have changed it.
    let handed_out = child.composition_visual().unwrap();
    let control_visual = own_visual(&control);

    s.canvas.remove(&control);
    assert!(control.composition_visual_departed());
    assert!(!child.composition_visual_departed());
    assert!(handed_out.draw_list().is_none());
    s.canvas.add(&control);
    s.run_jobs();

    // The parent kept its own; the child has a new one, as upstream.
    assert!(Rc::ptr_eq(&control_visual, &own_visual(&control)));
    assert!(!Rc::ptr_eq(&handed_out, &own_visual(&child)));
    assert!(handed_out.parent().is_none());
    assert_eq!(1, control_visual.children().count());
    assert!(Rc::ptr_eq(&control_visual.children().get(0), &own_visual(&child)));
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
}

#[test]
fn composition_visual_is_not_kept_with_a_child_visual_or_while_rendering_is_stopped() {
    let s = CompositorCanvas::new();
    let host = TestBorder::layer(0.0, 0.0, 20.0, 20.0);
    let plain = TestBorder::layer(30.0, 0.0, 20.0, 20.0);
    s.canvas.add(&host);
    s.canvas.add(&plain);
    s.run_jobs();
    let extra = s.compositor.create_solid_color_visual();
    host.set_child_composition_visual(Some((*extra).clone()));
    s.run_jobs();

    s.canvas.remove(&host);
    assert!(!host.composition_visual_departed());
    assert!(host.own_composition_visual().is_none());
    s.run_jobs();

    // No update pass runs while the renderer is stopped, so nothing is kept for one.
    s.renderer.stop();
    s.canvas.remove(&plain);
    assert!(!plain.composition_visual_departed());
    assert!(plain.own_composition_visual().is_none());
    s.renderer.start();
    s.run_jobs();

    // And what was kept when the renderer stops is released there.
    s.canvas.add(&plain);
    s.run_jobs();
    let plain_visual = own_visual(&plain);
    s.canvas.remove(&plain);
    assert!(plain.composition_visual_departed());
    s.renderer.stop();
    assert!(!plain.composition_visual_departed());
    assert!(plain.own_composition_visual().is_none());
    assert!(draw_list_of(&plain_visual).is_none());
    s.renderer.start();
}

#[test]
fn visual_out_of_the_tree_is_not_hit_through_what_is_kept_for_it() {
    let s = CompositorCanvas::new();
    let (control, child) = clipping_decorator_with_layer();
    let under = TestBorder::layer(30.0, 50.0, 40.0, 30.0);
    s.canvas.add(&under);
    s.canvas.add(&control);
    s.assert_hit_test(45.0, 65.0, None, &[&child, &under]);
    s.assert_hit_test_first(45.0, 65.0, None, Some(&child));

    s.canvas.remove(&control);
    // Before the update pass: the composition visuals of the control and its child are kept,
    // and are hit tested as released ones.
    assert!(child.composition_visual_departed());
    let hit = s.renderer.hit_test(Point::new(45.0, 65.0), &s.root, None);
    assert_eq!(vec![under.clone().upcast::<Visual>()], hit);
    let first = s.renderer.hit_test_first(Point::new(45.0, 65.0), &s.root, None);
    assert_eq!(Some(under.clone().upcast::<Visual>()), first);

    s.canvas.add(&control);
    s.assert_hit_test(45.0, 65.0, None, &[&child, &under]);
}

/// Moves `child` from one parent to another in one pass and reports what was invalidated.
fn move_between_parents(hand_out: bool) -> Vec<String> {
    let s = CompositorCanvas::new();
    // The parents clip, so each has a composition visual of its own (section 9.41).
    let from = TestBorder::new();
    from.set_bounds(Rect::new(0.0, 0.0, 50.0, 50.0));
    from.set_clip_to_bounds(true);
    let to = TestBorder::new();
    to.set_bounds(Rect::new(100.0, 0.0, 50.0, 50.0));
    to.set_clip_to_bounds(true);
    let child = TestBorder::layer(10.0, 10.0, 20.0, 20.0);
    let grandchild = TestBorder::layer(5.0, 5.0, 5.0, 5.0);
    grandchild.set_background(Some(Brushes::blue()));
    child.add(&grandchild);
    from.add(&child);
    s.canvas.add(&from);
    s.canvas.add(&to);
    s.run_jobs();
    if hand_out {
        let _ = child.composition_visual();
        let _ = grandchild.composition_visual();
    }
    let child_visual = own_visual(&child);

    s.events.rects.borrow_mut().clear();
    from.remove(&child);
    assert_eq!(!hand_out, child.composition_visual_departed());
    to.add(&child);
    s.run_jobs();

    assert_eq!(!hand_out, Rc::ptr_eq(&child_visual, &own_visual(&child)));
    assert_eq!(0, own_visual(&from).children().count());
    assert_eq!(1, own_visual(&to).children().count());
    assert!(Rc::ptr_eq(&own_visual(&child).parent().unwrap(), &own_visual(&to)));
    assert!(Rc::ptr_eq(&own_visual(&grandchild).parent().unwrap(), &own_visual(&child)));
    let server_children = own_server_visual(&s, &to).children().unwrap().list();
    assert_eq!(1, server_children.len());
    assert!(Rc::ptr_eq(&server_children[0], &own_server_visual(&s, &child)));
    assert!(own_server_visual(&s, &from).children().is_none_or(|children| children.list().is_empty()));

    let mut rects: Vec<String> = s.events.rects.borrow().iter().map(|r| r.to_string()).collect();
    rects.sort();
    rects.dedup();
    s.assert_hit_test(117.0, 17.0, None, &[&grandchild, &child]);
    s.assert_hit_test(17.0, 17.0, None, &[]);
    rects
}

#[test]
fn visual_moved_to_another_parent_in_one_pass_keeps_its_composition_visuals() {
    let kept = move_between_parents(false);
    let released = move_between_parents(true);
    // The same is invalidated whether the composition visuals were kept or made again: where
    // the child was and where it is.
    assert_eq!(released, kept);
    assert!(kept.contains(&Rect::new(10.0, 10.0, 20.0, 20.0).to_string()), "{kept:?}");
    assert!(kept.contains(&Rect::new(110.0, 10.0, 20.0, 20.0).to_string()), "{kept:?}");
}

/// What the compositor redraws for a visual that is under another parent, at another place,
/// with the composition visual it had: where it was and where it is, and not the place its
/// old bounds name under its new parent.
#[test]
fn visual_moved_to_another_parent_and_place_redraws_where_it_was_and_where_it_is() {
    let s = CompositorCanvas::new();
    let from = TestBorder::new();
    from.set_bounds(Rect::new(0.0, 0.0, 50.0, 50.0));
    let to = TestBorder::new();
    to.set_bounds(Rect::new(100.0, 200.0, 50.0, 50.0));
    let child = TestBorder::layer(10.0, 10.0, 20.0, 20.0);
    from.add(&child);
    s.canvas.add(&from);
    s.canvas.add(&to);
    s.run_jobs();
    let child_visual = own_visual(&child);

    s.events.rects.borrow_mut().clear();
    from.remove(&child);
    child.set_bounds(Rect::new(20.0, 5.0, 20.0, 20.0));
    to.add(&child);
    s.assert_rects(&[Rect::new(10.0, 10.0, 20.0, 20.0), Rect::new(120.0, 205.0, 20.0, 20.0)]);
    assert!(Rc::ptr_eq(&child_visual, &own_visual(&child)));
}

#[test]
fn child_removed_while_its_parent_is_out_of_the_tree_releases_its_composition_visual() {
    let s = CompositorCanvas::new();
    let (control, child) = clipping_decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    let control_visual = own_visual(&control);
    let child_visual = own_visual(&child);

    s.canvas.remove(&control);
    control.remove(&child);
    s.canvas.add(&control);
    s.events.rects.borrow_mut().clear();
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0)]);

    assert!(Rc::ptr_eq(&control_visual, &own_visual(&control)));
    assert_eq!(0, control_visual.children().count());
    assert!(!child.composition_visual_departed());
    assert!(child.own_composition_visual().is_none());
    assert!(child_visual.parent().is_none());
    assert!(draw_list_of(&child_visual).is_none());
    s.assert_hit_test(45.0, 65.0, None, &[]);
}

#[test]
fn composition_visual_kept_for_one_compositor_is_not_used_with_another() {
    let s = CompositorCanvas::new();
    let control = TestBorder::layer(0.0, 0.0, 20.0, 20.0);
    s.canvas.add(&control);
    s.run_jobs();
    let kept = own_visual(&control);
    let other = Compositor::with_scheduler(
        ManualRenderLoop::new(),
        None,
        true,
        &MediaContext::instance().scheduler(),
        Dispatcher::ui_thread(),
        None,
        None,
    );

    s.canvas.remove(&control);
    assert!(control.composition_visual_departed());
    let created = control.attach_to_compositor(&other);
    assert!(!Rc::ptr_eq(&kept, &created));
    assert!(Rc::ptr_eq(created.compositor(), &other));
    assert!(!kept.is_departed());
    assert!(draw_list_of(&kept).is_none());
    control.detach_from_compositor();
    s.run_jobs();
}

/// A visual that leaves its tree, or ends a request it never made, is given no composition
/// extras: a visual without them is the one the synchronisation answers on its short paths.
#[test]
fn visual_that_leaves_its_tree_gains_no_composition_extras() {
    let s = CompositorCanvas::new();
    let (control, child) = clipping_decorator_with_child();
    s.canvas.add(&control);
    s.run_jobs();
    assert!(!control.has_composition_extras());

    s.canvas.remove(&control);
    s.run_jobs();
    assert!(!control.has_composition_extras());
    assert!(!child.has_composition_extras());

    control.remove_composition_hide_request();
    control.remove_composition_show_request();
    assert!(!control.has_composition_extras());
}

/// How many composition visuals a row costs when it is recycled (the counter check
/// `composition_work` of `scripts/perf/gate.py`; the counts are asserted with the feature
/// `perf-counters`): none is created for a row that is back in the same pass, the row that
/// stays out for an update pass has its own made again, and the row that is kept for a pool
/// stays out for as many passes as the pool has it and has none made.
#[test]
fn composition_work_of_a_row_taken_out_of_the_tree_and_put_back() {
    use crate::diagnostics::perf_counters::{self, PerfCounter};

    let s = CompositorCanvas::new();
    let (row, cell) = clipping_decorator_with_layer();
    s.canvas.add(&row);
    s.run_jobs();
    let objects = s.compositor.server().object_count();

    let before = perf_counters::snapshot();
    for _ in 0..8 {
        s.canvas.remove(&row);
        s.canvas.add(&row);
        s.run_jobs();
    }
    let work = perf_counters::snapshot().since(&before);
    assert_eq!(objects, s.compositor.server().object_count());
    assert!(cell.own_composition_visual().is_some());
    if perf_counters::ENABLED {
        assert_eq!(0, work.get(PerfCounter::CompositionVisualsCreated));
        assert_eq!(16, work.get(PerfCounter::CompositionVisualsKept));
    }

    let before = perf_counters::snapshot();
    s.canvas.remove(&row);
    s.run_jobs();
    s.canvas.add(&row);
    s.run_jobs();
    let work = perf_counters::snapshot().since(&before);
    assert_eq!(objects, s.compositor.server().object_count());
    if perf_counters::ENABLED {
        assert_eq!(2, work.get(PerfCounter::CompositionVisualsCreated));
        assert_eq!(0, work.get(PerfCounter::CompositionVisualsKept));
    }

    let before = perf_counters::snapshot();
    for _ in 0..8 {
        let kept = row.leave_tree_keeping_composition_visuals(|| s.canvas.remove(&row));
        for _ in 0..3 {
            s.canvas.invalidate_visual();
            s.run_jobs();
        }
        s.canvas.add(&row);
        drop(kept);
        s.run_jobs();
    }
    let work = perf_counters::snapshot().since(&before);
    assert_eq!(objects, s.compositor.server().object_count());
    if perf_counters::ENABLED {
        assert_eq!(0, work.get(PerfCounter::CompositionVisualsCreated));
        assert_eq!(16, work.get(PerfCounter::CompositionVisualsKept));
    }
}

// --- composition visuals kept for a pool (desktop-performance.md, 9.32) ---------------------

/// Takes `control` out of the canvas as a recycling panel takes a row out for its pool.
fn take_out_for_a_pool(s: &CompositorCanvas, control: &Ref<TestBorder>) -> Option<super::KeptCompositionVisuals> {
    control.leave_tree_keeping_composition_visuals(|| s.canvas.remove(control))
}

#[test]
fn visual_taken_out_for_a_pool_keeps_its_composition_visuals_across_update_passes() {
    let s = CompositorCanvas::new();
    let (control, child) = clipping_decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    let objects = s.compositor.server().object_count();
    let control_visual = own_visual(&control);
    let child_visual = own_visual(&child);
    let control_server = own_server_visual(&s, &control);
    let child_server = own_server_visual(&s, &child);

    let kept = take_out_for_a_pool(&s, &control).expect("the composition visuals are kept");
    assert_eq!(2, kept.len());
    assert!(!kept.is_empty());
    s.events.rects.borrow_mut().clear();
    // Where it was shown is redrawn, as for any visual that leaves.
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0)]);
    for _ in 0..3 {
        s.canvas.invalidate_visual();
        s.run_jobs();
    }

    // Still kept after update passes, as a subtree that is in no tree: nothing shows it and
    // nothing is hit through it.
    assert!(control.composition_visual_departed());
    assert!(child.composition_visual_departed());
    assert!(control.composition_visual().is_none());
    assert!(control_visual.parent().is_none());
    assert_eq!(0, own_visual(&s.root).children().count());
    assert!(Rc::ptr_eq(&child_visual.parent().unwrap(), &control_visual));
    assert!(draw_list_of(&child_visual).is_some());
    assert_eq!(objects, s.compositor.server().object_count());
    s.assert_hit_test(45.0, 65.0, None, &[]);

    // Out of the pool: it changed there, and is shown with the composition visuals it had.
    control.set_bounds(Rect::new(100.0, 100.0, 40.0, 30.0));
    s.canvas.add(&control);
    kept.release();
    assert!(!control.composition_visual_departed());
    assert!(!child.composition_visual_departed());
    s.events.rects.borrow_mut().clear();
    s.assert_rects(&[Rect::new(110.0, 110.0, 20.0, 10.0)]);
    assert!(Rc::ptr_eq(&control_visual, &own_visual(&control)));
    assert!(Rc::ptr_eq(&child_visual, &own_visual(&child)));
    assert!(Rc::ptr_eq(&control_server, &own_server_visual(&s, &control)));
    assert!(Rc::ptr_eq(&child_server, &own_server_visual(&s, &child)));
    assert!(Rc::ptr_eq(&control_visual.parent().unwrap(), &own_visual(&s.root)));
    assert_eq!(objects, s.compositor.server().object_count());
    s.assert_hit_test(115.0, 115.0, None, &[&child]);
    s.assert_hit_test(45.0, 65.0, None, &[]);
}

#[test]
fn composition_visuals_kept_for_a_pool_are_released_by_what_holds_them() {
    let s = CompositorCanvas::new();
    s.run_jobs();
    let baseline = s.compositor.server().object_count();
    let (control, child) = clipping_decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    let control_visual = own_visual(&control);
    let child_visual = own_visual(&child);

    let kept = take_out_for_a_pool(&s, &control).expect("the composition visuals are kept");
    s.run_jobs();
    assert!(control.composition_visual_departed());

    // Dropping it is releasing it.
    drop(kept);
    assert!(!control.composition_visual_departed());
    assert!(!child.composition_visual_departed());
    assert!(control.own_composition_visual().is_none());
    assert!(child.own_composition_visual().is_none());
    assert!(child_visual.parent().is_none());
    assert_eq!(0, control_visual.children().count());
    assert!(draw_list_of(&child_visual).is_none());
    assert!(!child_visual.is_departed());
    drop((control_visual, child_visual));
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert_eq!(baseline, s.compositor.server().object_count());

    // It is attached again later with composition visuals of its own.
    s.canvas.add(&control);
    s.run_jobs();
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
}

#[test]
fn composition_visuals_kept_for_a_pool_outlive_no_visual() {
    let s = CompositorCanvas::new();
    s.run_jobs();
    let baseline = s.compositor.server().object_count();
    let (control, child) = clipping_decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    let child_visual = own_visual(&child);
    let weak_control = control.downgrade();

    let kept = take_out_for_a_pool(&s, &control).expect("the composition visuals are kept");
    s.run_jobs();
    drop((control, child));
    // What is kept refers to the composition visuals, not to the visuals.
    assert!(weak_control.upgrade().is_none());
    assert!(child_visual.is_departed());

    kept.release();
    assert!(!child_visual.is_departed());
    assert!(child_visual.parent().is_none());
    assert!(draw_list_of(&child_visual).is_none());
    s.canvas.invalidate_visual();
    s.run_jobs();
    // The composition visual and its parent no longer refer to each other.
    assert_eq!(1, Rc::strong_count(&child_visual));
    drop(child_visual);
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert_eq!(baseline, s.compositor.server().object_count());
}

#[test]
fn child_that_left_a_pooled_visual_releases_its_composition_visual_with_the_pool() {
    let s = CompositorCanvas::new();
    let (control, child) = clipping_decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    let control_visual = own_visual(&control);
    let child_visual = own_visual(&child);

    let kept = take_out_for_a_pool(&s, &control).expect("the composition visuals are kept");
    s.run_jobs();
    control.remove(&child);
    s.canvas.add(&control);
    drop(kept);
    s.run_jobs();

    assert!(Rc::ptr_eq(&control_visual, &own_visual(&control)));
    assert_eq!(0, control_visual.children().count());
    assert!(!child.composition_visual_departed());
    assert!(child.own_composition_visual().is_none());
    assert!(child_visual.parent().is_none());
    assert!(draw_list_of(&child_visual).is_none());
    s.assert_hit_test(45.0, 65.0, None, &[]);
}

#[test]
fn visuals_taken_out_inside_one_another_are_kept_by_the_innermost() {
    let s = CompositorCanvas::new();
    let first = TestBorder::layer(0.0, 0.0, 20.0, 20.0);
    let second = TestBorder::layer(30.0, 0.0, 20.0, 20.0);
    let third = TestBorder::layer(60.0, 0.0, 20.0, 20.0);
    for control in [&first, &second, &third] {
        s.canvas.add(control);
    }
    s.run_jobs();
    let second_visual = own_visual(&second);

    let mut inner = None;
    let outer = first.leave_tree_keeping_composition_visuals(|| {
        s.canvas.remove(&first);
        inner = take_out_for_a_pool(&s, &second);
        // The outer one goes on after the inner one.
        s.canvas.remove(&third);
    });
    assert_eq!(Some(2), outer.as_ref().map(super::KeptCompositionVisuals::len));
    assert_eq!(Some(1), inner.as_ref().map(super::KeptCompositionVisuals::len));
    s.run_jobs();

    drop(outer);
    assert!(!first.composition_visual_departed());
    assert!(!third.composition_visual_departed());
    assert!(second.composition_visual_departed());
    s.canvas.add(&second);
    drop(inner);
    s.run_jobs();
    assert!(Rc::ptr_eq(&second_visual, &own_visual(&second)));

    // After the scopes a visual that leaves is released in the next update pass again.
    s.canvas.remove(&second);
    assert!(second.composition_visual_departed());
    s.run_jobs();
    assert!(!second.composition_visual_departed());
    assert!(second.own_composition_visual().is_none());
}

#[test]
fn nothing_is_kept_for_a_pool_outside_of_a_tree_or_while_rendering_is_stopped() {
    let s = CompositorCanvas::new();
    let (control, child) = clipping_decorator_with_child();

    // In no tree: the departure runs, once, and nothing is kept.
    let mut runs = 0;
    assert!(control.leave_tree_keeping_composition_visuals(|| runs += 1).is_none());
    assert_eq!(1, runs);

    s.canvas.add(&control);
    s.run_jobs();
    s.renderer.stop();
    assert!(take_out_for_a_pool(&s, &control).is_none());
    assert!(!control.composition_visual_departed());
    assert!(control.own_composition_visual().is_none());
    assert!(child.own_composition_visual().is_none());
    s.renderer.start();

    // What a pool keeps is not released when rendering stops: the pool holds it, and the
    // visual is shown with it once it is back.
    s.canvas.add(&control);
    s.run_jobs();
    let control_visual = own_visual(&control);
    let kept = take_out_for_a_pool(&s, &control).expect("the composition visuals are kept");
    s.renderer.stop();
    assert!(control.composition_visual_departed());
    s.renderer.start();
    s.canvas.add(&control);
    drop(kept);
    s.run_jobs();
    assert!(Rc::ptr_eq(&control_visual, &own_visual(&control)));
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
}

#[test]
fn composition_visuals_kept_for_a_pool_are_not_used_by_a_disposed_renderer() {
    let s = CompositorCanvas::new();
    let (control, child) = clipping_decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    let child_visual = own_visual(&child);

    let kept = take_out_for_a_pool(&s, &control).expect("the composition visuals are kept");
    s.renderer.dispose();
    // The pool still decides: nothing is released behind it.
    assert!(child.composition_visual_departed());
    drop(kept);
    assert!(!child.composition_visual_departed());
    assert!(child.own_composition_visual().is_none());
    assert!(draw_list_of(&child_visual).is_none());
}

// --- layers that hold a frame back (desktop-performance.md, section 9.33) ---

/// The canvas with mock layers that hold a frame back, its log and its
/// server target.
fn canvas_with_held_frames() -> (CompositorCanvas, crate::rendering::testing::DrawingLog, Rc<ServerCompositionTarget>) {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let target = s
        .compositor
        .server()
        .get::<ServerCompositionTarget>(s.renderer.composition_target().server())
        .expect("the target of the renderer exists");
    log.hold_layer_frames_back(true);
    (s, log, target)
}

#[test]
fn a_frame_a_layer_holds_back_is_presented_by_the_tick_after() {
    let (s, log, target) = canvas_with_held_frames();
    assert!(!target.has_pending_frame());

    s.canvas.add(&TestBorder::filled(30.0, 50.0, 20.0, 10.0));
    Dispatcher::ui_thread().run_jobs(None);
    let batch = s.compositor.commit();
    log.clear();
    s.render_loop.tick();
    // The tick drew its frame into the layer and presented the one before.
    assert_eq!(1, log.count("Blit frame 0 of 1"), "{:?}", log.entries());
    assert_eq!(0, log.count("CompletePendingFrame"));
    assert!(target.has_pending_frame());
    assert!(batch.processed().is_completed());
    assert!(!batch.rendered().is_completed());

    // Nothing changes: the next tick presents the frame, and only then is
    // the batch rendered.
    log.clear();
    s.render_loop.tick();
    assert_eq!(vec!["CompletePendingFrame"], log.entries().into_iter().filter(|e| e.starts_with("Complete")).collect::<Vec<_>>());
    assert_eq!(1, log.count("Blit frame 1 of 1"), "{:?}", log.entries());
    assert_eq!(0, log.count("FillRectangle"), "the tick presents, it draws nothing: {:?}", log.entries());
    assert!(!target.has_pending_frame());
    assert!(batch.rendered().is_completed());

    // The stream has ended: the ticks that follow present nothing.
    log.clear();
    s.render_loop.tick();
    assert_eq!(0, log.count("Blit"), "{:?}", log.entries());
}

#[test]
fn the_batches_of_a_stream_of_held_frames_are_rendered_a_tick_later() {
    let (s, log, target) = canvas_with_held_frames();

    s.canvas.add(&TestBorder::filled(30.0, 50.0, 20.0, 10.0));
    Dispatcher::ui_thread().run_jobs(None);
    let first = s.compositor.commit();
    s.render_loop.tick();
    assert!(!first.rendered().is_completed());

    // The next frame of the stream presents the first one.
    s.canvas.add(&TestBorder::filled(130.0, 50.0, 20.0, 10.0));
    Dispatcher::ui_thread().run_jobs(None);
    let second = s.compositor.commit();
    log.clear();
    s.render_loop.tick();
    assert_eq!(1, log.count("Blit frame 1 of 2"), "{:?}", log.entries());
    assert_eq!(0, log.count("CompletePendingFrame"), "a tick that draws completes the frame before it by itself");
    assert!(first.rendered().is_completed());
    assert!(!second.rendered().is_completed());
    assert!(target.has_pending_frame());

    s.render_loop.tick();
    assert!(second.rendered().is_completed());
    assert!(!target.has_pending_frame());
}

#[test]
fn a_synchronous_render_presents_the_frame_a_layer_holds_back() {
    let (s, log, target) = canvas_with_held_frames();

    s.canvas.add(&TestBorder::filled(30.0, 50.0, 20.0, 10.0));
    Dispatcher::ui_thread().run_jobs(None);
    let batch = s.compositor.commit();
    log.clear();
    s.compositor.render_on_this_thread();
    let presented: Vec<String> =
        log.entries().into_iter().filter(|e| e.starts_with("Blit") || e.starts_with("Complete")).collect();
    assert_eq!(vec!["Blit frame 0 of 1", "CompletePendingFrame", "Blit frame 1 of 1"], presented);
    assert!(!target.has_pending_frame());
    assert!(batch.rendered().is_completed());
}

#[test]
fn no_batch_waits_for_a_held_frame_once_rendering_is_stopped() {
    let (s, _log, target) = canvas_with_held_frames();

    s.canvas.add(&TestBorder::filled(30.0, 50.0, 20.0, 10.0));
    Dispatcher::ui_thread().run_jobs(None);
    s.render_loop.tick();
    assert!(target.has_pending_frame());

    // No tick presents the frame any more: a batch is answered at once.
    s.compositor.server().stop_rendering();
    let batch = s.compositor.commit();
    s.render_loop.tick();
    assert!(batch.rendered().is_completed());
}

// --- the rarely set properties of a visual (not from upstream) ------------------------

/// The properties of a visual that few visuals set are in a second block,
/// which a visual of an element that is only placed and drawn never makes,
/// on either side.
#[test]
fn a_visual_that_sets_no_rare_property_has_no_second_block() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(10.0, 20.0, 100.0, 50.0);
    s.canvas.add(&border);
    s.run_jobs();

    let visual = border.composition_visual().expect("the border is attached");
    assert!(!visual.visual_props().has_rare_properties());
    assert!(!s.server_visual(&border).visual_props().has_rare_properties());

    // A read without the block is the value the property is made with.
    assert_eq!(visual.scale(), Vector3D::new(1.0, 1.0, 1.0));
    assert_eq!(visual.transform_matrix(), Matrix::IDENTITY);
    assert_eq!(visual.rotation_angle(), 0.0);
    assert!(visual.clip().is_none());
    let server = s.server_visual(&border);
    assert_eq!(server.visual_props().scale(), Vector3D::new(1.0, 1.0, 1.0));
    assert_eq!(server.visual_props().transform_matrix(), Matrix::IDENTITY);
}

/// A write of the value a rare property is made with does not make the
/// block: the synchronization of an element writes its clip and its
/// transform whether they changed or not.
#[test]
fn a_write_of_the_initial_value_of_a_rare_property_makes_no_second_block() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(10.0, 20.0, 100.0, 50.0);
    s.canvas.add(&border);
    s.run_jobs();

    let visual = border.composition_visual().expect("the border is attached");
    visual.set_scale(Vector3D::new(1.0, 1.0, 1.0));
    visual.set_transform_matrix(Matrix::IDENTITY);
    visual.set_rotation_angle(0.0);
    visual.set_clip(None);
    s.run_jobs();

    assert!(!visual.visual_props().has_rare_properties());
    assert!(!s.server_visual(&border).visual_props().has_rare_properties());
}

/// A rare property that is given another value is read back as that value on
/// both sides, and keeps what it is given after that, the initial value
/// included.
#[test]
fn a_rare_property_that_is_set_is_kept_on_both_sides() {
    let s = CompositorCanvas::new();
    let border = TestBorder::filled(10.0, 20.0, 100.0, 50.0);
    s.canvas.add(&border);
    s.run_jobs();

    let visual = border.composition_visual().expect("the border is attached");
    visual.set_scale(Vector3D::new(2.0, 3.0, 1.0));
    visual.set_rotation_angle(0.5);
    s.run_jobs();

    assert!(visual.visual_props().has_rare_properties());
    assert_eq!(visual.scale(), Vector3D::new(2.0, 3.0, 1.0));
    assert_eq!(visual.rotation_angle(), 0.5);
    // The properties of the block that were not set are as they are made.
    assert_eq!(visual.transform_matrix(), Matrix::IDENTITY);
    assert_eq!(visual.rotation_axis(), Vector3D::new(0.0, 0.0, 1.0));

    let server = s.server_visual(&border);
    assert!(server.visual_props().has_rare_properties());
    assert_eq!(server.visual_props().scale(), Vector3D::new(2.0, 3.0, 1.0));
    assert_eq!(server.visual_props().rotation_angle(), 0.5);
    assert_eq!(server.visual_props().transform_matrix(), Matrix::IDENTITY);

    visual.set_scale(Vector3D::new(1.0, 1.0, 1.0));
    s.run_jobs();
    assert_eq!(visual.scale(), Vector3D::new(1.0, 1.0, 1.0));
    assert_eq!(s.server_visual(&border).visual_props().scale(), Vector3D::new(1.0, 1.0, 1.0));
}

// --- composition visuals on demand (desktop-performance.md, 9.41) --------------------------

/// The composition visual `visual` is a child of.
fn composition_parent(visual: &Visual) -> Rc<super::CompositionVisual> {
    own_visual(visual).parent().expect("the composition visual is in the composition tree")
}

#[test]
fn a_visual_that_draws_nothing_and_sets_nothing_has_no_composition_visual() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();

    assert!(s.root.has_composition_visual(), "the root has one");
    assert!(!s.canvas.has_composition_visual());
    assert!(!control.has_composition_visual());
    // What draws is shown by the nearest visual above it that has a composition visual, at its
    // place in it.
    assert!(Rc::ptr_eq(&composition_parent(&child), &own_visual(&s.root)));
    assert_eq!(Vector3D::new(40.0, 60.0, 0.0), own_visual(&child).offset());
    assert_eq!(1, own_visual(&s.root).children().count());
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
    s.assert_hit_test(35.0, 55.0, None, &[]);
}

#[test]
fn a_visual_gets_its_composition_visual_when_it_sets_what_the_compositor_applies() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();

    control.set_opacity(0.5);
    s.run_jobs();
    assert!(control.has_composition_visual());
    assert_eq!(0.5, own_visual(&control).opacity());
    assert!(Rc::ptr_eq(&composition_parent(&child), &own_visual(&control)));
    assert_eq!(Vector3D::new(10.0, 10.0, 0.0), own_visual(&child).offset());

    // One that sets it before it is shown has it from its first pass.
    let (clipping, inner) = clipping_decorator_with_layer();
    s.canvas.add(&clipping);
    s.run_jobs();
    assert!(clipping.has_composition_visual());
    assert!(Rc::ptr_eq(&composition_parent(&inner), &own_visual(&clipping)));

    // Hidden, it hides what is under it.
    let (hidden, covered) = decorator_with_layer();
    hidden.set_bounds(Rect::new(200.0, 200.0, 40.0, 30.0));
    s.canvas.add(&hidden);
    s.assert_hit_test(215.0, 215.0, None, &[&covered]);
    hidden.set_is_visible(false);
    s.assert_hit_test(215.0, 215.0, None, &[]);
    assert!(hidden.has_composition_visual());
}

#[test]
fn asking_for_the_composition_visual_creates_it_in_its_place() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    assert!(!control.has_composition_visual());

    // Before any pass: it is where it would be had the visual had it all along, and so is
    // that of its visual parent, which what is read from it (`Parent`, `Offset`) describes.
    let control_visual: Rc<super::CompositionVisual> = (*control.composition_visual().expect("in a tree")).clone();
    // What is under it is hit as it was, before a frame is rendered with the new ones.
    assert_eq!(vec![child.clone().upcast::<Visual>()], s.renderer.hit_test(Point::new(45.0, 65.0), &s.root, None));
    assert_eq!(Some(child.clone().upcast::<Visual>()), s.renderer.hit_test_first(Point::new(45.0, 65.0), &s.root, None));
    assert_eq!(vec![child.clone().upcast::<Visual>()], s.renderer.hit_test(Point::new(15.0, 15.0), &control, None));
    assert_eq!(Some(child.clone().upcast::<Visual>()), s.renderer.hit_test_first(Point::new(15.0, 15.0), &control, None));
    assert!(s.renderer.hit_test(Point::new(35.0, 55.0), &s.root, None).is_empty());
    assert!(s.canvas.has_composition_visual());
    assert!(Rc::ptr_eq(&composition_parent(&control), &own_visual(&s.canvas)));
    assert!(Rc::ptr_eq(&composition_parent(&s.canvas), &own_visual(&s.root)));
    assert!(Rc::ptr_eq(&composition_parent(&child), &control_visual));
    assert_eq!(Vector3D::new(30.0, 50.0, 0.0), control_visual.offset());
    assert_eq!(Vector3D::new(10.0, 10.0, 0.0), own_visual(&child).offset());
    assert_eq!(1, own_visual(&s.root).children().count());

    s.events.rects.borrow_mut().clear();
    s.run_jobs();
    assert!(Rc::ptr_eq(&control_visual, &own_visual(&control)));
    s.assert_hit_test(45.0, 65.0, None, &[&child]);

    // One asked for before the first pass of its visual is placed by that pass.
    let (early, inner) = decorator_with_layer();
    early.set_bounds(Rect::new(200.0, 200.0, 40.0, 30.0));
    s.canvas.add(&early);
    let early_visual: Rc<super::CompositionVisual> = (*early.composition_visual().expect("in a tree")).clone();
    s.run_jobs();
    assert!(Rc::ptr_eq(&early_visual, &own_visual(&early)));
    assert!(Rc::ptr_eq(&composition_parent(&early), &own_visual(&s.canvas)));
    assert!(Rc::ptr_eq(&composition_parent(&inner), &early_visual));
    assert_eq!(Vector3D::new(200.0, 200.0, 0.0), early_visual.offset());
    s.assert_hit_test(215.0, 215.0, None, &[&inner]);
}

#[test]
fn what_is_under_a_new_composition_visual_is_hit_before_the_first_frame_with_it() {
    let s = CompositorCanvas::new();
    let outer = TestBorder::new();
    outer.set_bounds(Rect::new(100.0, 100.0, 200.0, 200.0));
    let (control, child) = decorator_with_child();
    outer.add(&control);
    s.canvas.add(&outer);
    // Enough children for the hit test index of the visual that shows them.
    for index in 0..40 {
        s.canvas.add(&TestBorder::filled(400.0 + 10.0 * f64::from(index), 0.0, 5.0, 5.0));
    }
    s.run_jobs();
    assert!(!control.has_composition_visual());
    assert!(!outer.has_composition_visual());
    s.assert_hit_test(145.0, 165.0, None, &[&child]);

    // In the last frame the child was shown by the visual above the two: a hit test reads that
    // frame, and finds the child through the two new ones until a frame has them.
    let hit = vec![child.clone().upcast::<Visual>()];
    let first = Some(child.clone().upcast::<Visual>());
    let _control_visual = control.composition_visual().expect("in a tree");
    assert!(control.has_composition_visual());
    assert!(outer.has_composition_visual());
    assert_eq!(hit, s.renderer.hit_test(Point::new(145.0, 165.0), &s.root, None));
    assert_eq!(first, s.renderer.hit_test_first(Point::new(145.0, 165.0), &s.root, None));
    assert_eq!(hit, s.renderer.hit_test(Point::new(45.0, 65.0), &outer, None));
    assert_eq!(first, s.renderer.hit_test_first(Point::new(45.0, 65.0), &outer, None));
    assert_eq!(hit, s.renderer.hit_test(Point::new(15.0, 15.0), &control, None));
    assert_eq!(first, s.renderer.hit_test_first(Point::new(15.0, 15.0), &control, None));
    // The two draw nothing, and the filter is asked for them as before.
    assert!(s.renderer.hit_test(Point::new(135.0, 155.0), &s.root, None).is_empty());
    let control_as_visual: Ref<Visual> = control.clone().upcast();
    let not_the_control = move |visual: &Visual| !std::ptr::eq(visual, &*control_as_visual);
    assert!(s.renderer.hit_test(Point::new(145.0, 165.0), &s.root, Some(&not_the_control)).is_empty());

    // With the frame, they are hit as every other.
    s.run_jobs();
    assert_eq!(hit, s.renderer.hit_test(Point::new(145.0, 165.0), &s.root, None));
    assert_eq!(first, s.renderer.hit_test_first(Point::new(145.0, 165.0), &s.root, None));
    assert_eq!(hit, s.renderer.hit_test(Point::new(15.0, 15.0), &control, None));
    assert_eq!(first, s.renderer.hit_test_first(Point::new(15.0, 15.0), &control, None));
}

#[test]
fn what_is_under_a_visual_without_a_composition_visual_follows_it() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    s.events.rects.borrow_mut().clear();

    control.set_bounds(Rect::new(55.0, 50.0, 40.0, 30.0));
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0), Rect::new(65.0, 60.0, 20.0, 10.0)]);
    assert!(!control.has_composition_visual());
    assert_eq!(Vector3D::new(65.0, 60.0, 0.0), own_visual(&child).offset());
    s.assert_hit_test(70.0, 65.0, None, &[&child]);

    // With many below it, it gets a composition visual, which is the one thing a move writes.
    let many = TestBorder::new();
    many.set_bounds(Rect::new(300.0, 300.0, 400.0, 20.0));
    let mut cells = Vec::new();
    for i in 0..12 {
        let cell = TestBorder::layer(i as f64 * 30.0, 0.0, 20.0, 20.0);
        many.add(&cell);
        cells.push(cell);
    }
    s.canvas.add(&many);
    s.run_jobs();
    assert!(!many.has_composition_visual());
    assert_eq!(Vector3D::new(330.0, 300.0, 0.0), own_visual(&cells[1]).offset());

    many.set_bounds(Rect::new(300.0, 400.0, 400.0, 20.0));
    s.run_jobs();
    assert!(many.has_composition_visual());
    assert_eq!(Vector3D::new(300.0, 400.0, 0.0), own_visual(&many).offset());
    for (i, cell) in cells.iter().enumerate() {
        assert_eq!(Vector3D::new(i as f64 * 30.0, 0.0, 0.0), own_visual(cell).offset());
        assert!(Rc::ptr_eq(&composition_parent(cell), &own_visual(&many)));
    }
    s.assert_hit_test(335.0, 405.0, None, &[&cells[1]]);
    s.assert_hit_test(335.0, 305.0, None, &[]);
}

#[test]
fn children_of_a_visual_without_a_composition_visual_are_shown_in_their_order() {
    let s = CompositorCanvas::new();
    let (first, first_child) = decorator_with_layer();
    let (second, second_child) = decorator_with_layer();
    s.canvas.add(&first);
    s.canvas.add(&second);
    s.assert_hit_test(45.0, 65.0, None, &[&second_child, &first_child]);
    assert!(!first.has_composition_visual() && !second.has_composition_visual());

    // The Z order of the visuals without one orders what is under them.
    first.set_z_index(1);
    s.assert_hit_test(45.0, 65.0, None, &[&first_child, &second_child]);
    assert!(!first.has_composition_visual());

    // A child added to one of them is shown after its siblings, before what follows it.
    let added = TestBorder::layer(10.0, 10.0, 20.0, 10.0);
    second.add(&added);
    s.assert_hit_test(45.0, 65.0, None, &[&first_child, &added, &second_child]);

    // One removed (by the collection alone) is not shown any more.
    second.visual_children().remove(&second_child.clone().upcast());
    s.assert_hit_test(45.0, 65.0, None, &[&first_child, &added]);
    s.run_jobs();
    assert_eq!(2, own_visual(&s.root).children().count());

    // A visual without one that leaves takes what is under it along.
    s.canvas.remove(&first);
    s.assert_hit_test(45.0, 65.0, None, &[&added]);
    s.run_jobs();
    assert_eq!(1, own_visual(&s.root).children().count());
}

#[test]
fn hit_tests_ask_the_filter_for_the_visuals_without_a_composition_visual() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_child();
    let other = TestBorder::filled(40.0, 60.0, 20.0, 10.0);
    s.canvas.add(&other);
    s.canvas.add(&control);
    s.assert_hit_test(45.0, 65.0, None, &[&child, &other]);

    // What the filter rejects is rejected with what is under it.
    let control_visual: Ref<Visual> = control.clone().upcast();
    let not_the_control = move |visual: &Visual| !std::ptr::eq(visual, &*control_visual);
    s.assert_hit_test(45.0, 65.0, Some(&not_the_control), &[&other]);
    s.assert_hit_test_first(45.0, 65.0, Some(&not_the_control), Some(&other));
    assert!(!control.has_composition_visual());

    // A hit test under a visual without a composition visual is in its coordinates and finds
    // what is under it alone.
    let tested = s.renderer.hit_test(Point::new(15.0, 15.0), &control, None);
    assert_eq!(vec![child.clone().upcast::<Visual>()], tested);
    assert_eq!(Some(child.clone().upcast::<Visual>()), s.renderer.hit_test_first(Point::new(15.0, 15.0), &control, None));
    assert!(s.renderer.hit_test(Point::new(5.0, 5.0), &control, None).is_empty());
    assert!(s.renderer.hit_test(Point::new(15.0, 15.0), &control, Some(&not_the_control)).is_empty());
    assert!(!control.has_composition_visual(), "a hit test creates no composition visual");
}

#[test]
fn a_visual_without_a_composition_visual_that_leaves_and_returns_keeps_what_is_under_it() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_layer();
    s.canvas.add(&control);
    s.run_jobs();
    let child_visual = own_visual(&child);
    let objects = s.compositor.server().object_count();

    // Back in the same pass: the composition visual below is the one it was.
    s.canvas.remove(&control);
    s.canvas.add(&control);
    s.run_jobs();
    assert!(!control.has_composition_visual());
    assert!(Rc::ptr_eq(&child_visual, &own_visual(&child)));
    assert!(Rc::ptr_eq(&composition_parent(&child), &own_visual(&s.root)));
    assert_eq!(Vector3D::new(40.0, 60.0, 0.0), child_visual.offset());
    assert_eq!(objects, s.compositor.server().object_count());
    s.assert_hit_test(45.0, 65.0, None, &[&child]);

    // Out for a pass: it is released, and what is shown when it is back is new.
    s.canvas.remove(&control);
    s.run_jobs();
    assert_eq!(0, own_visual(&s.root).children().count());
    assert!(child_visual.parent().is_none());
    drop(child_visual);
    s.assert_hit_test(45.0, 65.0, None, &[]);
    s.canvas.add(&control);
    s.run_jobs();
    assert!(!control.has_composition_visual());
    // The disposal of dropped objects rides on a later batch.
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert_eq!(objects, s.compositor.server().object_count());
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
}

// --- drawing items (desktop-performance.md, 9.42) -------------------------------------------

fn visuals(borders: &[&Ref<TestBorder>]) -> Vec<Ref<Visual>> {
    borders.iter().map(|border| (*border).clone().upcast()).collect()
}

/// What `visual` drew, as the composition visual above it shows it.
fn item_of(visual: &Visual) -> Rc<super::CompositionDrawItem> {
    visual.composition_item().expect("what the visual drew is an item")
}

/// The visuals whose drawings the composition visual of `visual` shows as items, in the order
/// of the items.
fn item_elements(visual: &Visual) -> Vec<Ref<Visual>> {
    let holder = own_visual(visual);
    let Some(items) = holder.draw_items() else { return Vec::new() };
    (0..items.count()).filter_map(|index| items.item_at(index)).filter_map(|item| item.element()).collect()
}

/// The items the server side of the composition visual of `visual` shows.
fn server_items(s: &CompositorCanvas, visual: &Visual) -> Vec<Rc<super::server::ServerDrawItem>> {
    own_server_visual(s, visual).content().draw_items().map(|items| items.items()).unwrap_or_default()
}

/// Where the child visuals of the server side of the composition visual of `visual` are among
/// its items.
fn server_child_positions(s: &CompositorCanvas, visual: &Visual) -> Vec<u32> {
    own_server_visual(s, visual).content().draw_items().map(|items| items.child_positions()).unwrap_or_default()
}

/// What `log` has of rectangles and rectangular clips, in its order: the brush of a rectangle,
/// `PushClip` with its rectangle, and the `PopClip` of such a clip.
fn drawn(log: &crate::rendering::testing::DrawingLog) -> Vec<String> {
    let mut clips = Vec::new();
    let mut drawn = Vec::new();
    for entry in log.entries().iter() {
        if let Some(rest) = entry.strip_prefix("DrawRectangle ") {
            drawn.extend(rest.split(' ').next().map(str::to_string));
        } else if entry.starts_with("PushClip ") {
            clips.push(true);
            drawn.push(entry.clone());
        } else if entry == "PushClipRegion" || entry.starts_with("PushRoundedClip ") || entry.starts_with("PushGeometryClip") {
            clips.push(false);
        } else if entry == "PopClip" && clips.pop().unwrap_or(false) {
            drawn.push(entry.clone());
        }
    }
    drawn
}

/// The brushes of the rectangles `log` has, in its order.
fn rectangles(log: &crate::rendering::testing::DrawingLog) -> Vec<String> {
    drawn(log).into_iter().filter(|entry| !entry.starts_with("PushClip ") && entry != "PopClip").collect()
}

#[test]
fn a_visual_that_draws_is_an_item_of_the_composition_visual_above_it() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_child();
    s.canvas.add(&control);
    s.run_jobs();

    assert!(!s.canvas.has_composition_visual());
    assert!(!control.has_composition_visual());
    assert!(!child.has_composition_visual());
    assert!(control.composition_item().is_none(), "it draws nothing");
    let root_visual = own_visual(&s.root);
    let item = item_of(&child);
    assert!(item.is_held_by(&root_visual));
    assert!(Rc::ptr_eq(&root_visual, &item.holder().expect("the item is shown")));
    assert_eq!((40.0, 60.0), item.offset());
    assert_eq!(None, item.clip());
    assert_eq!(visuals(&[&child]), item_elements(&s.root));
    assert_eq!(0, root_visual.children().count());
    let on_server = server_items(&s, &s.root);
    assert_eq!(1, on_server.len());
    assert_eq!((40.0, 60.0), on_server[0].offset());
    assert_eq!(None, on_server[0].clip());
    assert!(on_server[0].data().is_some());
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
    s.assert_hit_test_first(45.0, 65.0, None, Some(&child));
    s.assert_hit_test(35.0, 55.0, None, &[]);
    s.events.rects.borrow_mut().clear();

    // A visual above it that draws is an item before it: the one it has stays.
    control.set_background(Some(Brushes::blue()));
    s.assert_rects(&[Rect::new(30.0, 50.0, 40.0, 30.0)]);
    assert!(!control.has_composition_visual());
    assert_eq!((30.0, 50.0), item_of(&control).offset());
    assert!(Rc::ptr_eq(&item, &item_of(&child)));
    assert_eq!(visuals(&[&control, &child]), item_elements(&s.root));
    assert_eq!(2, server_items(&s, &s.root).len());
    assert!(Rc::ptr_eq(&on_server[0], &server_items(&s, &s.root)[1]));
    s.assert_hit_test(45.0, 65.0, None, &[&child, &control]);
    s.assert_hit_test(35.0, 55.0, None, &[&control]);

    // One that draws nothing any more has no item.
    control.set_background(None);
    s.assert_rects(&[Rect::new(30.0, 50.0, 40.0, 30.0)]);
    assert!(control.composition_item().is_none());
    assert!(!control.has_composition_visual());
    assert_eq!(visuals(&[&child]), item_elements(&s.root));
    assert_eq!(1, server_items(&s, &s.root).len());
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
    assert_eq!(0, root_visual.children().count());
}

#[test]
fn an_item_is_clipped_to_the_size_of_a_visual_that_clips_to_its_bounds() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let border = TestBorder::filled(30.0, 50.0, 20.0, 10.0);
    border.overdraw.set(5.0);
    border.set_clip_to_bounds(true);
    s.canvas.add(&border);
    log.clear();
    s.assert_rects(&[Rect::new(30.0, 50.0, 20.0, 10.0)]);

    // It has no visual children: the clip is one of its drawing alone.
    assert!(!border.has_composition_visual());
    let item = item_of(&border);
    assert_eq!(Some((20.0, 10.0)), item.clip());
    assert_eq!(Some((20.0, 10.0)), server_items(&s, &s.root)[0].clip());
    assert_eq!(vec!["PushClip 0, 0, 20, 10", "Red", "PopClip"], drawn(&log));
    s.assert_hit_test(35.0, 55.0, None, &[&border]);
    s.assert_hit_test(27.0, 47.0, None, &[]);
    s.events.rects.borrow_mut().clear();

    // Without the clip all of the drawing is shown, and hit.
    border.set_clip_to_bounds(false);
    log.clear();
    s.assert_rects(&[Rect::new(30.0, 50.0, 20.0, 10.0), Rect::new(25.0, 45.0, 30.0, 20.0)]);
    assert!(Rc::ptr_eq(&item, &item_of(&border)));
    assert_eq!(None, item.clip());
    assert_eq!(None, server_items(&s, &s.root)[0].clip());
    assert_eq!(vec!["Red"], drawn(&log));
    s.assert_hit_test(27.0, 47.0, None, &[&border]);

    // With a visual child the clip is one of what is under the visual too: it is the clip of
    // a composition visual, which shows the drawing from here on.
    border.set_clip_to_bounds(true);
    s.run_jobs();
    assert!(!border.has_composition_visual());
    let inner = TestBorder::filled(5.0, 2.0, 5.0, 5.0);
    inner.set_background(Some(Brushes::blue()));
    border.add(&inner);
    s.run_jobs();
    assert!(border.has_composition_visual());
    assert!(border.composition_item().is_none());
    assert!(item_elements(&s.root).is_empty());
    assert!(server_items(&s, &s.root).is_empty());
    assert!(draw_list_of(&own_visual(&border)).is_some());
    let inner_item = item_of(&inner);
    assert!(inner_item.is_held_by(&own_visual(&border)));
    assert_eq!((5.0, 2.0), inner_item.offset());
    assert_eq!(1, server_items(&s, &border).len());
    s.assert_hit_test(27.0, 47.0, None, &[]);
    s.assert_hit_test(36.0, 53.0, None, &[&inner, &border]);
}

#[test]
fn items_and_child_visuals_are_painted_and_hit_in_the_order_of_the_visuals() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let below = TestBorder::filled(40.0, 60.0, 20.0, 10.0);
    let layer = TestBorder::layer(40.0, 60.0, 20.0, 10.0);
    layer.set_background(Some(Brushes::blue()));
    let above = TestBorder::filled(40.0, 60.0, 20.0, 10.0);
    above.set_background(Some(Brushes::green()));
    s.canvas.add(&below);
    s.canvas.add(&layer);
    s.canvas.add(&above);
    log.clear();
    s.run_jobs();

    let root_visual = own_visual(&s.root);
    assert_eq!(vec!["Red", "Blue", "Green"], drawn(&log));
    assert_eq!(visuals(&[&below, &above]), item_elements(&s.root));
    assert_eq!(1, root_visual.children().count());
    assert_eq!(1, root_visual.draw_items().expect("items").position(0));
    assert_eq!(vec![1], server_child_positions(&s, &s.root));
    s.assert_hit_test(45.0, 65.0, None, &[&above, &layer, &below]);
    s.assert_hit_test_first(45.0, 65.0, None, Some(&above));

    // What the filter rejects is passed over, an item as a composition visual.
    let above_visual: Ref<Visual> = above.clone().upcast();
    let not_above = move |visual: &Visual| !std::ptr::eq(visual, &*above_visual);
    s.assert_hit_test(45.0, 65.0, Some(&not_above), &[&layer, &below]);
    s.assert_hit_test_first(45.0, 65.0, Some(&not_above), Some(&layer));
    let layer_visual: Ref<Visual> = layer.clone().upcast();
    let not_layer = move |visual: &Visual| !std::ptr::eq(visual, &*layer_visual);
    s.assert_hit_test(45.0, 65.0, Some(&not_layer), &[&above, &below]);

    // The Z order of the visuals orders the items and the child visuals alike.
    below.set_z_index(1);
    log.clear();
    s.assert_hit_test(45.0, 65.0, None, &[&below, &above, &layer]);
    assert_eq!(vec!["Blue", "Green", "Red"], drawn(&log));
    assert_eq!(visuals(&[&above, &below]), item_elements(&s.root));
    assert_eq!(0, root_visual.draw_items().expect("items").position(0));
    assert_eq!(vec![0], server_child_positions(&s, &s.root));

    // An item added below one that has a composition visual is painted before it.
    let first = TestBorder::filled(40.0, 60.0, 20.0, 10.0);
    first.set_background(Some(Brushes::yellow()));
    first.set_z_index(-1);
    s.canvas.add(&first);
    log.clear();
    s.assert_hit_test(45.0, 65.0, None, &[&below, &above, &layer, &first]);
    assert_eq!(vec!["Yellow", "Blue", "Green", "Red"], drawn(&log));
    assert_eq!(vec![1], server_child_positions(&s, &s.root));

    // Without the child visual the items are all there is.
    s.canvas.remove(&layer);
    log.clear();
    s.assert_hit_test(45.0, 65.0, None, &[&below, &above, &first]);
    assert_eq!(vec!["Yellow", "Green", "Red"], drawn(&log));
    assert_eq!(0, root_visual.children().count());
    assert!(server_child_positions(&s, &s.root).is_empty());
}

#[test]
fn a_geometry_hits_the_items_whose_bounds_it_meets() {
    let s = CompositorCanvas::new();
    let first = TestBorder::filled(0.0, 0.0, 10.0, 10.0);
    let second = TestBorder::filled(0.0, 10.0, 10.0, 10.0);
    let layer = TestBorder::layer(0.0, 20.0, 10.0, 10.0);
    layer.set_background(Some(Brushes::blue()));
    s.canvas.add(&first);
    s.canvas.add(&second);
    s.canvas.add(&layer);
    s.run_jobs();
    assert_eq!(visuals(&[&first, &second]), item_elements(&s.root));
    let hit = |x: f64, y: f64, width: f64, height: f64| -> Vec<Ref<Visual>> {
        let geometry =
            RectangleGeometry::with_rect(Rect::new(x, y, width, height)).upcast::<crate::media::Geometry>();
        s.renderer.hit_test_geometry(&geometry, &s.root, None).into_iter().map(|hit| hit.visual_hit).collect()
    };

    // A geometry that only touches what an item drew does not hit it, as it does not hit a
    // composition visual it only touches.
    assert_eq!(visuals(&[&second]), hit(5.0, 10.0, 10.0, 10.0));
    assert_eq!(visuals(&[&first]), hit(5.0, 0.0, 10.0, 10.0));
    assert_eq!(visuals(&[&layer, &second, &first]), hit(5.0, 5.0, 10.0, 20.0));
    assert!(hit(10.0, 0.0, 10.0, 30.0).is_empty());

    // The item is where the layout put it.
    second.set_bounds(Rect::new(0.0, 40.0, 10.0, 10.0));
    s.run_jobs();
    assert!(hit(5.0, 10.0, 10.0, 10.0).is_empty());
    assert_eq!(visuals(&[&second]), hit(5.0, 45.0, 10.0, 10.0));
}

#[test]
fn items_among_many_child_visuals_are_hit_in_the_order_of_the_visuals() {
    let s = CompositorCanvas::new();
    let place = |i: usize| TestBorder::layer((i % 10) as f64 * 50.0, (i / 10) as f64 * 50.0, 40.0, 40.0);
    let mut layers = Vec::new();
    for i in 0..20 {
        layers.push(place(i));
        s.canvas.add(&layers[i]);
    }
    let between = TestBorder::filled(0.0, 0.0, 40.0, 40.0);
    s.canvas.add(&between);
    for i in 20..40 {
        layers.push(place(i));
        s.canvas.add(&layers[i]);
    }
    let over = TestBorder::layer(0.0, 0.0, 40.0, 40.0);
    s.canvas.add(&over);
    let top = TestBorder::filled(0.0, 0.0, 40.0, 40.0);
    s.canvas.add(&top);
    let first = TestBorder::filled(0.0, 0.0, 40.0, 40.0);
    first.set_z_index(-1);
    s.canvas.add(&first);
    s.run_jobs();

    // The child visuals are looked up in their index, and the items between them are tested
    // where they are painted (section 9.42).
    let root_visual = own_visual(&s.root);
    assert_eq!(visuals(&[&first, &between, &top]), item_elements(&s.root));
    assert_eq!(41, root_visual.children().count());
    s.assert_hit_test(10.0, 10.0, None, &[&top, &over, &between, &layers[0], &first]);
    assert!(root_visual.hit_test_children.borrow().is_some(), "the children are indexed");
    s.assert_hit_test_first(10.0, 10.0, None, Some(&top));
    s.assert_hit_test(10.0, 110.0, None, &[&layers[20]]);
    s.assert_hit_test_first(10.0, 110.0, None, Some(&layers[20]));
    s.assert_hit_test(45.0, 45.0, None, &[]);
    s.assert_hit_test_first(45.0, 45.0, None, None);

    // The first that the filter accepts, an item or a child visual.
    let without = |rejected: &[&Ref<TestBorder>]| {
        let rejected = visuals(rejected);
        move |visual: &Visual| !rejected.iter().any(|rejected| std::ptr::eq(visual, &**rejected))
    };
    s.assert_hit_test_first(10.0, 10.0, Some(&without(&[&top])), Some(&over));
    s.assert_hit_test_first(10.0, 10.0, Some(&without(&[&top, &over])), Some(&between));
    s.assert_hit_test_first(10.0, 10.0, Some(&without(&[&top, &over, &between])), Some(&layers[0]));
    s.assert_hit_test_first(10.0, 10.0, Some(&without(&[&top, &over, &between, &layers[0]])), Some(&first));
    s.assert_hit_test(10.0, 10.0, Some(&without(&[&over, &layers[0]])), &[&top, &between, &first]);

    // An item where no child visual is, between child visuals that the index leaves out.
    between.set_bounds(Rect::new(700.0, 700.0, 40.0, 40.0));
    s.assert_hit_test(710.0, 710.0, None, &[&between]);
    s.assert_hit_test_first(710.0, 710.0, None, Some(&between));
    s.assert_hit_test(10.0, 10.0, None, &[&top, &over, &layers[0], &first]);

    // A geometry, as a point.
    let hit = |x: f64, y: f64, width: f64, height: f64| -> Vec<Ref<Visual>> {
        let geometry =
            RectangleGeometry::with_rect(Rect::new(x, y, width, height)).upcast::<crate::media::Geometry>();
        s.renderer.hit_test_geometry(&geometry, &s.root, None).into_iter().map(|hit| hit.visual_hit).collect()
    };
    assert_eq!(visuals(&[&top, &over, &layers[1], &layers[0], &first]), hit(30.0, 10.0, 30.0, 10.0));
    assert_eq!(visuals(&[&between]), hit(690.0, 690.0, 20.0, 20.0));
}

#[test]
fn the_items_of_a_holder_with_many_are_hit_through_the_bounds_of_their_chunks() {
    use crate::platform::LtrbRect;

    let s = CompositorCanvas::new();
    let mut items = Vec::new();
    for i in 0..100 {
        let item = TestBorder::filled((i % 10) as f64 * 50.0, (i / 10) as f64 * 50.0, 40.0, 40.0);
        s.canvas.add(&item);
        items.push(item);
    }
    let overlapping = TestBorder::filled(0.0, 0.0, 40.0, 40.0);
    s.canvas.add(&overlapping);
    s.run_jobs();
    let holder = own_visual(&s.root);
    let list = || holder.draw_items().expect("the root shows items");
    assert_eq!(101, item_elements(&s.root).len());

    s.assert_hit_test(10.0, 10.0, None, &[&overlapping, &items[0]]);
    s.assert_hit_test_first(10.0, 10.0, None, Some(&overlapping));
    s.assert_hit_test(460.0, 460.0, None, &[&items[99]]);
    s.assert_hit_test_first(460.0, 460.0, None, Some(&items[99]));
    s.assert_hit_test(45.0, 45.0, None, &[]);
    s.assert_hit_test_first(45.0, 45.0, None, None);
    // Thirty-two items that follow each other have bounds: the first three rows and two of
    // the fourth, and the last five items.
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 490.0, 190.0)), list().chunk_bounds(0));
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 490.0, 490.0)), list().chunk_bounds(3));
    assert_eq!(None, list().chunk_bounds(4));

    // An item that moves is where it is, and the bounds of its chunk with it.
    s.canvas.remove(&overlapping);
    s.run_jobs();
    assert_eq!(Some(LtrbRect::new(300.0, 450.0, 490.0, 490.0)), list().chunk_bounds(3));
    items[99].set_bounds(Rect::new(700.0, 700.0, 40.0, 40.0));
    s.assert_hit_test(460.0, 460.0, None, &[]);
    s.assert_hit_test(710.0, 710.0, None, &[&items[99]]);
    s.assert_hit_test_first(710.0, 710.0, None, Some(&items[99]));
    assert_eq!(Some(LtrbRect::new(300.0, 450.0, 740.0, 740.0)), list().chunk_bounds(3));
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 490.0, 190.0)), list().chunk_bounds(0));

    // One that draws something else.
    items[40].set_bounds(Rect::new(0.0, 200.0, 48.0, 48.0));
    s.assert_hit_test(45.0, 245.0, None, &[&items[40]]);
    // One that clips to its bounds is hit inside them.
    items[41].set_clip_to_bounds(true);
    s.assert_hit_test(60.0, 210.0, None, &[&items[41]]);
    s.assert_hit_test(95.0, 210.0, None, &[]);

    // The items after one that left are others of their chunks.
    s.canvas.remove(&items[5]);
    s.assert_hit_test(260.0, 10.0, None, &[]);
    s.assert_hit_test(310.0, 10.0, None, &[&items[6]]);
    s.assert_hit_test(710.0, 710.0, None, &[&items[99]]);
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 490.0, 190.0)), list().chunk_bounds(0));
    assert_eq!(Some(LtrbRect::new(350.0, 450.0, 740.0, 740.0)), list().chunk_bounds(3));
    let added = TestBorder::filled(900.0, 900.0, 40.0, 40.0);
    s.canvas.add(&added);
    s.assert_hit_test(910.0, 910.0, None, &[&added]);
    s.assert_hit_test_first(910.0, 910.0, None, Some(&added));

    // A geometry that meets the bounds of one chunk.
    let hit = |x: f64, y: f64, width: f64, height: f64| -> Vec<Ref<Visual>> {
        let geometry =
            RectangleGeometry::with_rect(Rect::new(x, y, width, height)).upcast::<crate::media::Geometry>();
        s.renderer.hit_test_geometry(&geometry, &s.root, None).into_iter().map(|hit| hit.visual_hit).collect()
    };
    assert_eq!(visuals(&[&added, &items[99]]), hit(730.0, 730.0, 180.0, 180.0));
    assert_eq!(visuals(&[&items[11], &items[10], &items[1], &items[0]]), hit(30.0, 30.0, 30.0, 30.0));

    // Few items are tested one by one, without bounds of chunks.
    for item in &items[20..] {
        s.canvas.remove(item);
    }
    s.assert_hit_test(10.0, 10.0, None, &[&items[0]]);
    s.assert_hit_test(910.0, 910.0, None, &[&added]);
    assert_eq!(20, item_elements(&s.root).len());
    assert!(!list().has_hit_bounds());
}

#[test]
fn an_item_follows_its_visual_and_the_visuals_above_it_with_the_drawing_it_has() {
    use crate::diagnostics::perf_counters::{self, PerfCounter};

    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_child();
    s.canvas.add(&control);
    s.run_jobs();
    let item = item_of(&child);
    let data = item.data().expect("a drawing");
    let on_server = server_items(&s, &s.root)[0].clone();
    let data_on_server = on_server.data().expect("a drawing");
    let objects = s.compositor.server().object_count();
    s.events.rects.borrow_mut().clear();

    // The visual moves: its item has another offset, and nothing is drawn or sent again.
    let before = perf_counters::snapshot();
    child.set_bounds(Rect::new(15.0, 10.0, 20.0, 10.0));
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0), Rect::new(45.0, 60.0, 20.0, 10.0)]);
    let work = perf_counters::snapshot().since(&before);
    if perf_counters::ENABLED {
        assert_eq!(0, work.get(PerfCounter::VisualsRecorded));
        assert_eq!(0, work.get(PerfCounter::CompositionVisualsCreated));
    }
    assert!(Rc::ptr_eq(&item, &item_of(&child)));
    assert!(Rc::ptr_eq(&data, &item.data().expect("a drawing")));
    assert_eq!((45.0, 60.0), item.offset());
    assert!(Rc::ptr_eq(&on_server, &server_items(&s, &s.root)[0]));
    assert_eq!((45.0, 60.0), on_server.offset());
    assert!(Rc::ptr_eq(&data_on_server, &on_server.data().expect("a drawing")));
    assert_eq!(objects, s.compositor.server().object_count());
    s.assert_hit_test(50.0, 65.0, None, &[&child]);
    s.assert_hit_test(42.0, 65.0, None, &[]);

    // A visual above it that has no composition visual moves: the same.
    control.set_bounds(Rect::new(55.0, 50.0, 40.0, 30.0));
    s.assert_rects(&[Rect::new(45.0, 60.0, 20.0, 10.0), Rect::new(70.0, 60.0, 20.0, 10.0)]);
    assert!(!control.has_composition_visual());
    assert_eq!((70.0, 60.0), item.offset());
    assert_eq!((70.0, 60.0), on_server.offset());
    assert!(Rc::ptr_eq(&data, &item.data().expect("a drawing")));
    s.assert_hit_test(75.0, 65.0, None, &[&child]);
    s.assert_hit_test(50.0, 65.0, None, &[]);

    // Under a visual that has a composition visual the item is where it is in that one, which
    // is what moves.
    let (clipping, inner) = clipping_decorator_with_child();
    clipping.set_bounds(Rect::new(200.0, 200.0, 40.0, 30.0));
    s.canvas.add(&clipping);
    s.run_jobs();
    let inner_item = item_of(&inner);
    assert!(inner_item.is_held_by(&own_visual(&clipping)));
    assert_eq!((10.0, 10.0), inner_item.offset());
    assert_eq!(visuals(&[&child]), item_elements(&s.root));
    assert_eq!(visuals(&[&inner]), item_elements(&clipping));
    s.events.rects.borrow_mut().clear();
    clipping.set_bounds(Rect::new(300.0, 200.0, 40.0, 30.0));
    s.assert_rects(&[Rect::new(210.0, 210.0, 20.0, 10.0), Rect::new(310.0, 210.0, 20.0, 10.0)]);
    assert_eq!((10.0, 10.0), inner_item.offset());
    s.assert_hit_test(315.0, 215.0, None, &[&inner]);
    s.assert_hit_test(215.0, 215.0, None, &[]);
}

#[test]
fn an_item_moves_to_the_composition_visual_a_visual_above_it_gets() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_child();
    s.canvas.add(&control);
    s.run_jobs();
    let item = item_of(&child);
    let data = item.data().expect("a drawing");
    s.events.rects.borrow_mut().clear();

    control.set_opacity(0.5);
    s.run_jobs();
    assert!(control.has_composition_visual());
    assert!(Rc::ptr_eq(&item, &item_of(&child)));
    assert!(Rc::ptr_eq(&data, &item.data().expect("a drawing")));
    assert!(item.is_held_by(&own_visual(&control)));
    assert_eq!((10.0, 10.0), item.offset());
    assert!(item_elements(&s.root).is_empty());
    assert!(server_items(&s, &s.root).is_empty());
    assert_eq!(visuals(&[&child]), item_elements(&control));
    let on_server = server_items(&s, &control);
    assert_eq!(1, on_server.len());
    assert_eq!((10.0, 10.0), on_server[0].offset());
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0)]);
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
    s.assert_hit_test(35.0, 55.0, None, &[]);

    // The new one moves, and the item with it.
    control.set_bounds(Rect::new(55.0, 50.0, 40.0, 30.0));
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0), Rect::new(65.0, 60.0, 20.0, 10.0)]);
    assert_eq!((10.0, 10.0), item.offset());
    s.assert_hit_test(70.0, 65.0, None, &[&child]);
}

#[test]
fn a_visual_that_is_an_item_gets_a_composition_visual_with_its_drawing() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_child();
    s.canvas.add(&control);
    s.run_jobs();
    let item = item_of(&child);
    let data = item.data().expect("a drawing");

    // Asked for: the drawing of the item is the one of the composition visual.
    let child_visual: Rc<super::CompositionVisual> = (*child.composition_visual().expect("in a tree")).clone();
    assert!(child.composition_item().is_none());
    assert!(item.data().is_none());
    assert!(Rc::ptr_eq(&data, &draw_list_of(&child_visual).expect("the drawing of the item")));
    // It is hit where it is before a frame is rendered with the composition visual.
    let hit = vec![child.clone().upcast::<Visual>()];
    let first = Some(child.clone().upcast::<Visual>());
    assert_eq!(hit, s.renderer.hit_test(Point::new(45.0, 65.0), &s.root, None));
    assert_eq!(first, s.renderer.hit_test_first(Point::new(45.0, 65.0), &s.root, None));
    assert_eq!(hit, s.renderer.hit_test(Point::new(15.0, 15.0), &control, None));
    assert_eq!(first, s.renderer.hit_test_first(Point::new(15.0, 15.0), &control, None));
    assert_eq!(hit, s.renderer.hit_test(Point::new(5.0, 5.0), &child, None));
    assert!(s.renderer.hit_test(Point::new(35.0, 55.0), &s.root, None).is_empty());

    s.events.rects.borrow_mut().clear();
    s.assert_rects(&[Rect::new(40.0, 60.0, 20.0, 10.0)]);
    assert!(item_elements(&s.root).is_empty());
    assert!(server_items(&s, &s.root).is_empty());
    assert!(Rc::ptr_eq(&composition_parent(&child), &own_visual(&control)));
    assert_eq!(Vector3D::new(10.0, 10.0, 0.0), child_visual.offset());
    assert!(Rc::ptr_eq(&data, &draw_list_of(&child_visual).expect("the drawing of the item")));
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
    s.assert_hit_test_first(45.0, 65.0, None, Some(&child));

    // It is not an item again for as long as it is in the tree.
    child.set_background(Some(Brushes::blue()));
    s.run_jobs();
    assert!(child.composition_item().is_none());
    assert!(Rc::ptr_eq(&child_visual, &own_visual(&child)));
    assert!(item_elements(&control).is_empty());

    // One that sets what the compositor applies gets its composition visual in the update
    // pass, with the drawing it has.
    let other = TestBorder::filled(200.0, 200.0, 20.0, 10.0);
    s.canvas.add(&other);
    s.run_jobs();
    let other_data = item_of(&other).data().expect("a drawing");
    s.events.rects.borrow_mut().clear();
    other.set_opacity(0.5);
    s.assert_rects(&[Rect::new(200.0, 200.0, 20.0, 10.0)]);
    assert!(other.has_composition_visual());
    assert!(other.composition_item().is_none());
    assert!(Rc::ptr_eq(&other_data, &draw_list_of(&own_visual(&other)).expect("the drawing of the item")));
    assert_eq!(0.5, own_visual(&other).opacity());
    s.assert_hit_test(205.0, 205.0, None, &[&other]);

    // Hidden, it is neither shown nor hit.
    let hidden = TestBorder::filled(300.0, 300.0, 20.0, 10.0);
    s.canvas.add(&hidden);
    s.assert_hit_test(305.0, 305.0, None, &[&hidden]);
    s.events.rects.borrow_mut().clear();
    hidden.set_is_visible(false);
    s.assert_rects(&[Rect::new(300.0, 300.0, 20.0, 10.0)]);
    s.assert_hit_test(305.0, 305.0, None, &[]);
    assert!(hidden.composition_item().is_none());
}

#[test]
fn an_item_of_a_visual_that_leaves_and_returns_is_kept_as_a_composition_visual_is() {
    let s = CompositorCanvas::new();
    let (control, child) = decorator_with_child();
    s.canvas.add(&control);
    s.run_jobs();
    let item = item_of(&child);
    let data = item.data().expect("a drawing");
    let on_server = server_items(&s, &s.root)[0].clone();
    let objects = s.compositor.server().object_count();

    // Back in the same pass: the item is the one it was, and nothing of it is sent.
    s.canvas.remove(&control);
    assert!(item.is_departed());
    // A visual that is out of the tree is not hit through what is kept for it.
    assert!(s.renderer.hit_test(Point::new(45.0, 65.0), &s.root, None).is_empty());
    assert!(s.renderer.hit_test_first(Point::new(45.0, 65.0), &s.root, None).is_none());
    s.canvas.add(&control);
    assert!(!item.is_departed());
    s.run_jobs();
    assert!(Rc::ptr_eq(&item, &item_of(&child)));
    assert!(Rc::ptr_eq(&data, &item.data().expect("a drawing")));
    assert!(item.is_held_by(&own_visual(&s.root)));
    assert!(Rc::ptr_eq(&on_server, &server_items(&s, &s.root)[0]));
    assert_eq!(objects, s.compositor.server().object_count());
    s.assert_hit_test(45.0, 65.0, None, &[&child]);

    // Out for a pass: it is released, and what is shown when the visual is back is new.
    s.canvas.remove(&control);
    s.run_jobs();
    assert!(child.composition_item().is_none());
    assert!(item.data().is_none());
    assert!(item_elements(&s.root).is_empty());
    assert!(server_items(&s, &s.root).is_empty());
    s.assert_hit_test(45.0, 65.0, None, &[]);
    s.canvas.add(&control);
    s.run_jobs();
    let item = item_of(&child);
    assert_eq!((40.0, 60.0), item.offset());
    // The disposal of dropped objects rides on a later batch.
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert_eq!(objects, s.compositor.server().object_count());
    s.assert_hit_test(45.0, 65.0, None, &[&child]);

    // Taken out for a pool: it is kept for as many passes as the pool has it, and is not shown.
    let data = item.data().expect("a drawing");
    let kept = take_out_for_a_pool(&s, &control);
    assert!(kept.is_some());
    for _ in 0..3 {
        s.canvas.invalidate_visual();
        s.run_jobs();
    }
    assert!(item.is_departed());
    assert!(Rc::ptr_eq(&item, &child.composition_item().expect("kept")));
    assert!(item_elements(&s.root).is_empty());
    assert!(server_items(&s, &s.root).is_empty());
    s.assert_hit_test(45.0, 65.0, None, &[]);
    s.canvas.add(&control);
    drop(kept);
    s.run_jobs();
    assert!(!item.is_departed());
    assert!(Rc::ptr_eq(&item, &item_of(&child)));
    assert!(Rc::ptr_eq(&data, &item.data().expect("a drawing")));
    assert_eq!(visuals(&[&child]), item_elements(&s.root));
    assert_eq!(1, server_items(&s, &s.root).len());
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert_eq!(objects, s.compositor.server().object_count());
    s.assert_hit_test(45.0, 65.0, None, &[&child]);

    // What the pool lets go of while the visual is out is released.
    let kept = take_out_for_a_pool(&s, &control);
    s.run_jobs();
    assert!(child.composition_item().is_some());
    drop(kept);
    s.run_jobs();
    assert!(child.composition_item().is_none());
    assert!(item.data().is_none());
    s.canvas.add(&control);
    s.run_jobs();
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert_eq!(objects, s.compositor.server().object_count());
    s.assert_hit_test(45.0, 65.0, None, &[&child]);
}

/// The row of `composition_work_of_a_row_taken_out_of_the_tree_and_put_back` with a cell that
/// is an item: the one composition visual of the row is what is kept, and made again.
#[test]
fn composition_work_of_a_row_of_items_taken_out_of_the_tree_and_put_back() {
    use crate::diagnostics::perf_counters::{self, PerfCounter};

    let s = CompositorCanvas::new();
    let (row, cell) = clipping_decorator_with_child();
    s.canvas.add(&row);
    s.run_jobs();
    s.canvas.invalidate_visual();
    s.run_jobs();
    let objects = s.compositor.server().object_count();
    let item = item_of(&cell);

    let before = perf_counters::snapshot();
    for _ in 0..8 {
        s.canvas.remove(&row);
        s.canvas.add(&row);
        s.run_jobs();
    }
    let work = perf_counters::snapshot().since(&before);
    assert_eq!(objects, s.compositor.server().object_count());
    assert!(Rc::ptr_eq(&item, &item_of(&cell)));
    assert!(!cell.has_composition_visual());
    if perf_counters::ENABLED {
        assert_eq!(0, work.get(PerfCounter::CompositionVisualsCreated));
        assert_eq!(8, work.get(PerfCounter::CompositionVisualsKept));
        assert_eq!(8, work.get(PerfCounter::DrawingItemsKept));
    }

    let before = perf_counters::snapshot();
    s.canvas.remove(&row);
    s.run_jobs();
    s.canvas.add(&row);
    s.run_jobs();
    s.canvas.invalidate_visual();
    s.run_jobs();
    let work = perf_counters::snapshot().since(&before);
    assert_eq!(objects, s.compositor.server().object_count());
    assert!(!cell.has_composition_visual());
    if perf_counters::ENABLED {
        assert_eq!(1, work.get(PerfCounter::CompositionVisualsCreated));
        assert_eq!(0, work.get(PerfCounter::CompositionVisualsKept));
        assert_eq!(0, work.get(PerfCounter::DrawingItemsKept));
    }

    let item = item_of(&cell);
    let before = perf_counters::snapshot();
    for _ in 0..8 {
        let kept = row.leave_tree_keeping_composition_visuals(|| s.canvas.remove(&row));
        for _ in 0..3 {
            s.canvas.invalidate_visual();
            s.run_jobs();
        }
        s.canvas.add(&row);
        drop(kept);
        s.run_jobs();
    }
    let work = perf_counters::snapshot().since(&before);
    assert_eq!(objects, s.compositor.server().object_count());
    assert!(Rc::ptr_eq(&item, &item_of(&cell)));
    if perf_counters::ENABLED {
        assert_eq!(0, work.get(PerfCounter::CompositionVisualsCreated));
        assert_eq!(8, work.get(PerfCounter::CompositionVisualsKept));
        assert_eq!(8, work.get(PerfCounter::DrawingItemsKept));
    }
}

/// A holder that is drawn again keeps the lists of what is below it, and whatever changes what
/// is below it has them made again (not from upstream; a counter check `composition_work` of
/// `scripts/perf/gate.py`: the counts are asserted with the feature `perf-counters`, and a
/// debug build without it lists a holder that is drawn again and fails when a list changed).
#[test]
fn composition_work_of_a_holder_that_is_drawn_again() {
    use crate::diagnostics::perf_counters::{self, PerfCounter};

    let s = CompositorCanvas::new();
    let holder = TestBorder::layer(0.0, 0.0, 400.0, 400.0);
    // Without a composition visual: what is under it is shown by the holder.
    let group = TestBorder::new();
    group.set_bounds(Rect::new(10.0, 10.0, 300.0, 300.0));
    let first = TestBorder::filled(0.0, 0.0, 20.0, 10.0);
    let layer = TestBorder::layer(0.0, 20.0, 20.0, 10.0);
    layer.set_background(Some(Brushes::blue()));
    let second = TestBorder::filled(0.0, 40.0, 20.0, 10.0);
    second.set_background(Some(Brushes::green()));
    let third = TestBorder::filled(350.0, 350.0, 20.0, 10.0);
    third.set_background(Some(Brushes::yellow()));
    group.add(&first);
    group.add(&layer);
    group.add(&second);
    holder.add(&group);
    holder.add(&third);
    s.canvas.add(&holder);
    s.run_jobs();
    let holder_visual = own_visual(&holder);
    let positions = || {
        let items = holder_visual.draw_items().expect("items");
        (0..holder_visual.children().count()).map(|child| items.position(child)).collect::<Vec<_>>()
    };
    assert_eq!(visuals(&[&first, &second, &third]), item_elements(&holder));
    assert_eq!(vec![1], positions());

    // The holder is drawn again after `change`: how many composition visuals listed what is
    // below them.
    let listed = |change: &dyn Fn()| {
        let before = perf_counters::snapshot();
        change();
        holder.invalidate_visual();
        s.run_jobs();
        perf_counters::snapshot().since(&before).get(PerfCounter::CompositionChildrenListed)
    };
    let assert_listed = |expected, count| {
        if perf_counters::ENABLED {
            assert_eq!(expected, count);
        }
    };

    // Drawn again, and an item of it drawn again: the lists are the ones it has.
    for _ in 0..8 {
        assert_listed(0, listed(&|| {}));
    }
    assert_listed(0, listed(&|| second.set_background(Some(Brushes::red()))));
    assert_eq!(visuals(&[&first, &second, &third]), item_elements(&holder));

    // A child that leaves it, comes back and moves, without a notice to the renderer.
    let third_visual: Ref<Visual> = third.clone().upcast();
    assert_listed(1, listed(&|| {
        holder.visual_children().remove(&third_visual);
    }));
    assert_eq!(visuals(&[&first, &second]), item_elements(&holder));
    assert_listed(1, listed(&|| holder.visual_children().add(third_visual.clone())));
    assert_eq!(visuals(&[&first, &second, &third]), item_elements(&holder));
    assert_listed(1, listed(&|| holder.visual_children().move_item(1, 0)));
    assert_eq!(visuals(&[&third, &first, &second]), item_elements(&holder));
    assert_eq!(vec![2], positions());
    assert_listed(0, listed(&|| {}));

    // A visual under one that has no composition visual leaves and comes back.
    assert_listed(1, listed(&|| group.remove(&second)));
    assert_eq!(visuals(&[&third, &first]), item_elements(&holder));
    assert_listed(1, listed(&|| group.add(&second)));
    assert_eq!(visuals(&[&third, &first, &second]), item_elements(&holder));
    assert_listed(0, listed(&|| {}));

    // The Z order of what is below.
    assert_listed(1, listed(&|| layer.set_z_index(-1)));
    assert_eq!(visuals(&[&third, &first, &second]), item_elements(&holder));
    assert_eq!(vec![1], positions());
    assert_listed(0, listed(&|| {}));

    // An item that is no longer drawn, and is drawn again.
    assert_listed(1, listed(&|| second.set_background(None)));
    assert_eq!(visuals(&[&third, &first]), item_elements(&holder));
    assert_listed(1, listed(&|| second.set_background(Some(Brushes::green()))));
    assert_eq!(visuals(&[&third, &first, &second]), item_elements(&holder));
    assert_listed(0, listed(&|| {}));

    // A child with a composition visual that comes and leaves without a notice.
    let extra = TestBorder::layer(100.0, 100.0, 20.0, 10.0);
    let extra_visual: Ref<Visual> = extra.clone().upcast();
    listed(&|| holder.visual_children().add(extra_visual.clone()));
    assert_eq!(2, holder_visual.children().count());
    assert!(Rc::ptr_eq(&composition_parent(&extra), &holder_visual));
    assert_listed(0, listed(&|| {}));
    assert_listed(1, listed(&|| {
        holder.visual_children().remove(&extra_visual);
    }));
    assert_eq!(1, holder_visual.children().count());
    assert_listed(0, listed(&|| {}));

    // An item that becomes a composition visual, and the visual above it with it: that one
    // shows what is below it from then on.
    listed(&|| {
        first.composition_visual().expect("in a tree");
    });
    assert_eq!(visuals(&[&third]), item_elements(&holder));
    assert_eq!(visuals(&[&second]), item_elements(&group));
    assert_eq!(1, holder_visual.children().count());
    assert!(Rc::ptr_eq(&composition_parent(&group), &holder_visual));
    assert!(Rc::ptr_eq(&composition_parent(&first), &own_visual(&group)));
    assert!(Rc::ptr_eq(&composition_parent(&layer), &own_visual(&group)));
    assert_listed(0, listed(&|| {}));
}

#[test]
fn removed_items_invalidate_their_own_rects() {
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
    // Not the union of the two, which is what the bounds of one composition visual would give.
    s.assert_rects(&[Rect::new(30.0, 10.0, 20.0, 10.0), Rect::new(30.0, 50.0, 20.0, 10.0)]);
    assert!(server_items(&s, &s.root).is_empty());
}

#[test]
fn only_the_items_in_the_dirty_area_are_drawn_again() {
    use crate::diagnostics::perf_counters::{self, PerfCounter};

    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let near = TestBorder::filled(10.0, 10.0, 20.0, 20.0);
    let far = TestBorder::filled(500.0, 500.0, 20.0, 20.0);
    far.set_background(Some(Brushes::blue()));
    s.canvas.add(&near);
    s.canvas.add(&far);
    log.clear();
    s.run_jobs();
    assert_eq!(vec!["Red", "Blue"], drawn(&log));
    s.events.rects.borrow_mut().clear();

    // A drawing that changed: the item alone is drawn, with the one composition visual.
    let before = perf_counters::snapshot();
    log.clear();
    near.set_background(Some(Brushes::green()));
    s.assert_rects(&[Rect::new(10.0, 10.0, 20.0, 20.0)]);
    assert_eq!(vec!["Green"], drawn(&log));
    let work = perf_counters::snapshot().since(&before);
    if perf_counters::ENABLED {
        assert_eq!(1, work.get(PerfCounter::RenderItemsDrawn));
        assert_eq!(1, work.get(PerfCounter::VisualsRecordedAsItems));
    }

    // The same drawing drawn again (section 9.14): its area alone.
    log.clear();
    far.invalidate_visual();
    s.assert_rects(&[Rect::new(500.0, 500.0, 20.0, 20.0)]);
    assert_eq!(vec!["Blue"], drawn(&log));

    // One over the other: both are drawn, in their order.
    log.clear();
    far.set_bounds(Rect::new(20.0, 20.0, 20.0, 20.0));
    s.assert_rects(&[Rect::new(500.0, 500.0, 20.0, 20.0), Rect::new(20.0, 20.0, 20.0, 20.0)]);
    assert_eq!(vec!["Green", "Blue"], drawn(&log));
}

#[test]
fn a_bitmap_cache_is_rendered_again_when_an_item_below_it_changes() {
    use crate::media::{BitmapCache, CacheMode};

    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let (control, child) = decorator_with_child();
    let other = TestBorder::filled(0.0, 0.0, 5.0, 5.0);
    other.set_background(Some(Brushes::blue()));
    control.add(&other);
    // A visual below the cached one that has no composition visual and shows an item.
    let nested = TestBorder::new();
    nested.set_bounds(Rect::new(30.0, 20.0, 10.0, 10.0));
    let deep = TestBorder::filled(2.0, 2.0, 5.0, 5.0);
    deep.set_background(Some(Brushes::green()));
    nested.add(&deep);
    control.add(&nested);
    control.set_cache_mode(BitmapCache::new().upcast::<CacheMode>());
    s.canvas.add(&control);
    s.run_jobs();
    assert!(control.has_composition_visual());
    assert_eq!(visuals(&[&child, &other, &deep]), item_elements(&control));
    assert_eq!(vec!["Red", "Blue", "Green"], drawn(&log));
    log.clear();

    // Nothing changed: the cache is drawn, not what is under it.
    s.canvas.invalidate_visual();
    s.run_jobs();
    assert!(drawn(&log).is_empty(), "{:?}", log.entries());
    log.clear();

    // An item that moves: the cache is rendered again where the item was and is, with the
    // items that are there.
    child.set_bounds(Rect::new(15.0, 10.0, 20.0, 10.0));
    s.run_jobs();
    assert_eq!(vec!["Red"], rectangles(&log));
    log.clear();

    // One that moves with a visual above it: what is cached is of another size, and all of it
    // is rendered again.
    nested.set_bounds(Rect::new(30.0, 15.0, 10.0, 10.0));
    s.run_jobs();
    assert_eq!(vec!["Red", "Blue", "Green"], rectangles(&log));
    assert_eq!(1, log.count("DrawBitmap 1 0, 0, 37, 22 0, 0, 37, 22"));
    log.clear();

    // One that is removed.
    control.remove(&child);
    s.run_jobs();
    assert_eq!(vec!["Green"], rectangles(&log));
    log.clear();

    // One that is added: it is the last of the items.
    control.add(&child);
    s.run_jobs();
    assert_eq!(visuals(&[&other, &deep, &child]), item_elements(&control));
    assert_eq!(vec!["Green", "Red"], rectangles(&log));
    log.clear();

    // One whose drawing is another one.
    other.set_background(Some(Brushes::yellow()));
    s.run_jobs();
    assert_eq!(vec!["Yellow"], rectangles(&log));
}
