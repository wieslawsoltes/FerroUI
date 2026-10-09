use super::web_render_target::{get_render_target, remove_render_target, unregister_canvas};
use super::{
    BrowserRenderSurface, BrowserSharedRenderLoop, BrowserSurface, BrowserSurfaceShared, RenderStatistics,
    RenderWorker,
};
use crate::browser_app_builder::BrowserRenderingMode;
use crate::interop::canvas_helper::CanvasSurface;
use crate::interop::{thread_proxy, JsObject};
use ferroui_base::media::MediaContext;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext, IPlatformGraphicsReadyStateFeature,
};
use ferroui_base::rendering::composition::server::ServerCompositor;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, Size};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

/// The surface of a view that is rendered through a render target of the
/// page (a WebGL context or a 2D canvas).
///
/// The object belongs to the thread of the user interface. What the thread
/// that renders needs of the canvas is in the two objects it shares with
/// that thread: [`BrowserSurfaceShared`] (the id of the render target, the
/// size, whether the target exists and its kind) and the
/// [`BrowserRenderSurface`] over it, which is the surface the compositor
/// draws to.
pub struct RenderTargetBrowserSurface {
    base: BrowserSurface,
    shared: Arc<BrowserSurfaceShared>,
    render_surface: Arc<BrowserRenderSurface>,
    /// Whether [`dispose`](Self::dispose) has run.
    disposed: Cell<bool>,
}

impl RenderTargetBrowserSurface {
    /// `thread_id` is what the canvas was created with: 0 when this thread
    /// kept the canvas and has its render target.
    fn new(js_surface: CanvasSurface, thread_id: i32) -> Rc<Self> {
        let target_id = js_surface.target_id();
        let shared = BrowserSurfaceShared::new();
        // Known to every thread by the id from here on: the worker of a
        // render thread reports the target under it, and may already have.
        shared.register(target_id);
        if thread_id == 0 {
            // This thread owns the target, and its script created it with
            // the canvas: it publishes that the target exists and its kind.
            // Upstream asks its registry for the target at each frame; here
            // the answer has to be where a second thread can read it.
            if let Some(target) = get_render_target(target_id) {
                shared.set_target_kind(target.kind());
            }
        }
        let gpu: Arc<dyn IPlatformGraphics> = Arc::new(BrowserPlatformGraphics { shared: shared.clone() });
        let compositor = if thread_id == 0 {
            Compositor::with_scheduler(
                BrowserSharedRenderLoop::render_loop(),
                Some(gpu),
                false,
                &MediaContext::instance().scheduler(),
                Dispatcher::ui_thread(),
                None,
                None,
            )
        } else {
            // The canvas belongs to the render thread, and so does everything
            // that draws to it: the compositor is confined to that thread
            // (the render-thread mode, this thread never renders, a loop
            // that runs in the background). This thread commits batches and
            // waits for a frame at the synchronous points. Upstream creates
            // one kind of compositor in both of its modes, with a timer that
            // does not run in the background (see `DEVIATIONS.md`).
            Compositor::with_render_thread(
                BrowserSharedRenderLoop::render_loop(),
                Some(gpu),
                false,
                &MediaContext::instance().scheduler(),
                Dispatcher::ui_thread(),
                None,
                None,
            )
        };

        let render_surface = BrowserRenderSurface::new(shared.clone());
        let this = Rc::new(Self {
            base: BrowserSurface::new(js_surface, compositor),
            shared,
            render_surface,
            disposed: Cell::new(false),
        });
        if let Some((w, h, s)) = this.base.initial_size() {
            this.on_size_changed(w, h, s);
        }
        this
    }

    /// Creates a canvas in `container` with a render target of the first of
    /// `modes` the browser supports, and the surface over it.
    pub fn create(container: &JsObject, modes: &[BrowserRenderingMode], top_level_id: i32) -> Rc<Self> {
        let modes: Vec<i32> = modes.iter().map(|m| *m as i32).collect();
        // 0 while the render loop of the page ticks on this thread: the
        // render target is created and used here. When it ticks on the
        // render thread, the script transfers the control of the canvas to
        // the worker of that thread, at once or when the thread has
        // reported itself. A render thread that does not run the loop of
        // the page (somebody else started it) gets no canvas of a view.
        let thread_id =
            if BrowserSharedRenderLoop::renders_on_render_thread() { RenderWorker::canvas_thread_id() } else { 0 };
        let js = CanvasSurface::create_render_target_surface(container, &modes, top_level_id, thread_id);
        Self::new(js, thread_id)
    }

    /// The compositor that renders to the surface.
    pub fn compositor(&self) -> Rc<Compositor> {
        self.base.compositor()
    }

    /// The scaling from logical units to device pixels.
    pub fn scaling(&self) -> f64 {
        self.base.scaling()
    }

    /// The size in logical units.
    pub fn client_size(&self) -> Size {
        self.base.client_size()
    }

    /// The size in device pixels.
    pub fn render_size(&self) -> PixelSize {
        self.base.render_size()
    }

    /// Whether the surface has a size it can be rendered at.
    pub fn is_valid(&self) -> bool {
        self.base.is_valid()
    }

    /// Subscribes to changes of the client size.
    pub fn size_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.base.size_changed(handler)
    }

    /// Subscribes to changes of the scaling.
    pub fn scaling_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.base.scaling_changed(handler)
    }

    /// The surfaces a render backend can draw to: the render surface of the
    /// canvas. It is handed out from the start and is not ready until the
    /// thread that owns the render target has published it (upstream hands
    /// out nothing until then).
    pub fn get_render_surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        let surface: Arc<dyn IPlatformRenderSurface> = self.render_surface.clone();
        vec![surface]
    }

    /// What the thread of the user interface and the thread that renders
    /// share about the canvas.
    pub fn shared(&self) -> &Arc<BrowserSurfaceShared> {
        &self.shared
    }

    /// The canvas changed its size or its scaling.
    pub fn on_size_changed(&self, pixel_width: f64, pixel_height: f64, dpr: f64) {
        // Where the thread that renders reads it at the start of a frame.
        self.shared.on_size_changed(pixel_width, pixel_height, dpr);
        self.base.on_size_changed(pixel_width, pixel_height, dpr);
    }

    /// Releases the canvas surface.
    ///
    /// Upstream posts a job to the compositor, which takes its server
    /// compositor out of the render loop, and then destroys the surface of
    /// the script side. A job is not what this port can use: it runs with a
    /// batch, and the batch of a closed view is the one that is likely never
    /// applied (the compositor is dropped as soon as the media context has
    /// committed it, and its graph is released in place of the next frame);
    /// a job does not run either while the graphics are not ready. So the
    /// thread that renders is given the work directly ([`release_canvas`]):
    ///
    /// - this thread, when it renders the view itself: the work is done
    ///   here, before the call returns;
    /// - the render thread, when the compositor is confined to it: the work
    ///   is queued for that thread, which enters the graph under the
    ///   compositor lock. Whether it gets there before or after the graph is
    ///   released does not matter: a graph that is released has released
    ///   what it drew with, in its own graphics context.
    ///
    /// The top-level is disposed before its renderer: the composition target
    /// of the view is disposed after this call, in a batch of its own that
    /// this thread waits for. The work does not depend on which of the two
    /// reaches the thread that renders first.
    ///
    /// A canvas that never reached the render thread (the view was closed
    /// before that thread had reported itself) is let go of here: the script
    /// of this thread still holds it back. When the work cannot be queued
    /// for a render thread that runs, the canvas is only marked as disposed
    /// and stays alive in the worker, unused.
    ///
    /// A surface that is dropped without this call makes it itself: a view
    /// that is let go of without being closed releases what it drew with in
    /// the same way, and not when its compositor happens to be dropped.
    pub fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        // Nothing waits for the first frame of a closed view.
        self.shared.forget_first_frame();
        // A target reported under the id from here on belongs to no canvas.
        self.shared.unregister();
        let shared = self.shared.clone();
        // This thread created the canvas, and its script is the one to tell.
        let page_thread = thread_proxy::current_thread();
        let compositor = self.base.compositor();
        if compositor.is_confined_to_render_thread() {
            let server = compositor.locked_server();
            let posted = RenderWorker::post({
                let shared = shared.clone();
                move || {
                    let released = server.try_with(|server| release_canvas(server, &shared, page_thread));
                    if released.is_none() {
                        // The graph is gone, and what it drew with went
                        // with it.
                        release_canvas_objects(&shared, page_thread);
                    }
                }
            });
            if !posted {
                if RenderWorker::thread_id() == 0 {
                    release_canvas_objects(&shared, page_thread);
                } else {
                    shared.dispose();
                }
            }
        } else {
            compositor.with_server(|server| release_canvas(server, &shared, page_thread));
        }
        self.base.dispose();
    }
}

impl Drop for RenderTargetBrowserSurface {
    fn drop(&mut self) {
        // Not while the thread unwinds: the release calls into the script
        // and into the graph, and a second panic would end the module.
        if !std::thread::panicking() {
            self.dispose();
        }
    }
}

/// What the thread that renders does when a view is closed. Runs under the
/// compositor lock, outside a frame.
///
/// 1. With the graphics context of the canvas current, the render targets
///    and layers that are still there and the backend context (Skia's
///    context, which holds the objects of the graphics interface it drew
///    with) are released
///    ([`release_gpu_resources`](ServerCompositor::release_gpu_resources)).
///    A thread that renders several canvases has several contexts, and an
///    object that is deleted while another context is current is deleted
///    there, where the same number names something else; with no context
///    current the call fails in the script of the module. The composition
///    target of the view, when it is disposed after this, finds nothing to
///    release.
/// 2. The compositor stops rendering
///    ([`stop_rendering`](ServerCompositor::stop_rendering)): it still
///    applies the batches of the view that is being closed, and asks for no
///    tick. Upstream's job takes the server compositor out of the render
///    loop here.
/// 3. The rest is [`release_canvas_objects`].
fn release_canvas(server: &ServerCompositor, shared: &Arc<BrowserSurfaceShared>, page_thread: usize) {
    server.release_gpu_resources();
    server.stop_rendering();
    release_canvas_objects(shared, page_thread);
}

/// What the thread that renders does with the canvas of a closed view once
/// nothing draws to it:
///
/// 1. the canvas is marked as disposed: its surface is of no kind and its
///    graphics is not ready from here on, so nothing creates a backend
///    context or a render target for it again;
/// 2. the render target leaves the table of this thread;
/// 3. the script of the thread that created the canvas (`page_thread`) is
///    told ([`unregister_canvas`]): directly when that is this thread,
///    through its event loop otherwise. It releases the target of the script
///    side, or sends `unregisterCanvas` to the worker that has it.
fn release_canvas_objects(shared: &Arc<BrowserSurfaceShared>, page_thread: usize) {
    let target_id = shared.target_id();
    shared.dispose();
    remove_render_target(target_id);
    RenderStatistics::canvas_released();
    if thread_proxy::current_thread() == page_thread {
        unregister_canvas(target_id);
    } else {
        // When the call cannot be queued the script keeps the target: the
        // canvas of the closed view stays alive in the worker, unused.
        thread_proxy::run_on_thread(page_thread, Box::new(move || unregister_canvas(target_id)));
    }
}

/// The platform graphics of one canvas, whose render target may not exist
/// yet when the compositor is created.
///
/// Upstream's object keeps the render target and the size of the canvas in
/// fields of its own. This one is created by the thread of the user
/// interface and asked by the thread that renders, so it holds only what
/// the two share: whether it is ready and whether the target renders with a
/// context are read from there, and the context is the one of the render
/// target of the calling thread ([`get_render_target`]).
struct BrowserPlatformGraphics {
    shared: Arc<BrowserSurfaceShared>,
}

impl BrowserPlatformGraphics {
    fn uses_contexts(&self) -> bool {
        self.shared.uses_contexts().expect("the render target exists")
    }
}

impl IPlatformGraphics for BrowserPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        self.uses_contexts()
    }

    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Specified method is not supported.");
    }

    /// Called by the thread that renders.
    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        let target = get_render_target(self.shared.target_id()).expect("the render target exists");
        match target.platform_graphics_context() {
            Some(context) => context,
            None => panic!(
                "This platform graphics instance represents software rendering mode and cant create contexts, you are supposed to query IPlatformGraphicsReadyStateFeature to know this"
            ),
        }
    }

    fn as_feature_provider(&self) -> Option<&dyn IOptionalFeatureProvider> {
        Some(self)
    }
}

impl IOptionalFeatureProvider for BrowserPlatformGraphics {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IPlatformGraphicsReadyStateFeature>() {
            // Upstream answers with the object itself. A second object over
            // the same shared state is the same answer, and the feature then
            // holds no handle of the platform graphics.
            let this: Arc<dyn IPlatformGraphicsReadyStateFeature> = Arc::new(Self { shared: self.shared.clone() });
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsReadyStateFeature for BrowserPlatformGraphics {
    fn is_ready(&self) -> bool {
        self.shared.is_ready()
    }

    fn uses_contexts(&self) -> bool {
        BrowserPlatformGraphics::uses_contexts(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::canvas_helper::{RENDER_TARGET_KIND_SOFTWARE, RENDER_TARGET_KIND_WEB_GL};
    use crate::interop::JsObject;
    use crate::rendering::web_render_target::{
        set_script_render_targets_for_unit_tests, set_script_unregister_for_unit_tests, BrowserRenderTarget,
    };
    use std::cell::RefCell;
    use crate::rendering::BrowserSoftwareRenderTarget;

    fn software_script(_id: i32) -> Option<BrowserRenderTarget> {
        Some(BrowserRenderTarget::Software(BrowserSoftwareRenderTarget::new(JsObject::NULL)))
    }

    fn ready_state(graphics: &BrowserPlatformGraphics) -> Arc<dyn IPlatformGraphicsReadyStateFeature> {
        let features: &dyn IOptionalFeatureProvider = graphics;
        features.try_get_shared::<dyn IPlatformGraphicsReadyStateFeature>().expect("the graphics has the feature")
    }

    #[test]
    fn the_graphics_is_ready_when_a_thread_published_the_target_and_the_canvas_has_a_size() {
        let shared = BrowserSurfaceShared::new();
        let graphics = BrowserPlatformGraphics { shared: shared.clone() };
        let feature = ready_state(&graphics);
        assert!(!feature.is_ready());

        shared.on_size_changed(300.0, 180.0, 1.5);
        assert!(!feature.is_ready());

        // Published by another thread, as the render thread does.
        std::thread::spawn({
            let shared = shared.clone();
            move || shared.set_target_kind(RENDER_TARGET_KIND_WEB_GL)
        })
        .join()
        .unwrap();
        assert!(feature.is_ready());
        assert!(feature.uses_contexts());
        assert!(graphics.uses_shared_context());

        shared.on_size_changed(0.0, 0.0, 1.5);
        assert!(!feature.is_ready());
    }

    #[test]
    fn a_software_target_uses_no_context() {
        let shared = BrowserSurfaceShared::new();
        shared.set_target_kind(RENDER_TARGET_KIND_SOFTWARE);
        let graphics = BrowserPlatformGraphics { shared };

        assert!(!ready_state(&graphics).uses_contexts());
        assert!(!graphics.uses_shared_context());
        let features: &dyn IOptionalFeatureProvider = &graphics;
        assert!(features.try_get_feature(TypeId::of::<dyn IPlatformGraphics>()).is_none());
    }

    #[test]
    #[should_panic(expected = "the render target exists")]
    fn whether_contexts_are_used_is_not_known_before_the_target_exists() {
        let graphics = BrowserPlatformGraphics { shared: BrowserSurfaceShared::new() };

        graphics.uses_shared_context();
    }

    #[test]
    #[should_panic(expected = "represents software rendering mode")]
    fn a_software_target_has_no_shared_context() {
        set_script_render_targets_for_unit_tests(software_script);
        let shared = BrowserSurfaceShared::new();
        shared.set_target_id(11);
        shared.set_target_kind(RENDER_TARGET_KIND_SOFTWARE);
        let graphics = BrowserPlatformGraphics { shared };

        graphics.get_shared_context();
    }

    thread_local! {
        /// The canvases the thread of a test told its script to let go of.
        static UNREGISTERED: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
    }

    fn record_unregistered(id: i32) {
        UNREGISTERED.with(|unregistered| unregistered.borrow_mut().push(id));
    }

    #[test]
    fn a_released_canvas_is_disposed_leaves_the_table_of_its_thread_and_is_unregistered() {
        set_script_render_targets_for_unit_tests(software_script);
        set_script_unregister_for_unit_tests(record_unregistered);
        let shared = BrowserSurfaceShared::new();
        shared.register(9701);
        shared.set_target_kind(RENDER_TARGET_KIND_SOFTWARE);
        shared.on_size_changed(300.0, 180.0, 1.0);
        let graphics = BrowserPlatformGraphics { shared: shared.clone() };
        let feature = ready_state(&graphics);
        let surface = BrowserRenderSurface::new(shared.clone());
        assert!(get_render_target(9701).is_some());
        assert!(feature.is_ready() && surface.is_ready());
        assert!(surface.as_framebuffer_surface().is_some());

        // The one thread of a build without threads is the thread of the
        // page: its script is told directly.
        release_canvas_objects(&shared, thread_proxy::current_thread());

        assert!(shared.is_disposed());
        assert!(!feature.is_ready() && !surface.is_ready());
        // The surface is of no kind any more, and nothing wraps the target a
        // second time.
        assert!(surface.as_framebuffer_surface().is_none());
        assert!(!remove_render_target(9701));
        assert_eq!(vec![9701], UNREGISTERED.with(|unregistered| unregistered.borrow().clone()));
    }

    #[test]
    fn a_thread_that_did_not_create_the_canvas_does_not_tell_its_own_script() {
        set_script_render_targets_for_unit_tests(software_script);
        set_script_unregister_for_unit_tests(record_unregistered);
        let shared = BrowserSurfaceShared::new();
        shared.register(9702);
        shared.set_target_kind(RENDER_TARGET_KIND_SOFTWARE);

        // Created by a thread with another id: the call is queued for it
        // (and dropped here, where there are no threads to queue for).
        release_canvas_objects(&shared, thread_proxy::current_thread() + 1);

        assert!(shared.is_disposed());
        assert!(UNREGISTERED.with(|unregistered| unregistered.borrow().is_empty()));
    }

    #[test]
    #[should_panic(expected = "Specified method is not supported")]
    fn the_graphics_creates_no_context() {
        let graphics = BrowserPlatformGraphics { shared: BrowserSurfaceShared::new() };

        graphics.create_context();
    }
}
