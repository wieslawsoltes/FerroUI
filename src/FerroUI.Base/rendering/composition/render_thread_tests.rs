//! The render-thread mode of the compositor: the thread that ticks the
//! render loop renders, under the compositor lock, and the thread of the
//! compositor renders too where a platform asks for it. Not from upstream,
//! where the two threads are a property of the platform set-up and not of
//! a test.
//!
//! The last part is the third shape: a compositor that is confined to its
//! render thread, whose own thread never renders.

use super::{CompositionTarget, CompositionVisual, Compositor, RenderSurfaces};
use crate::media::MediaContext;
use crate::platform::surfaces::IPlatformRenderSurface;
use crate::platform::{
    IDrawingContextImpl, IDrawingContextLayerImpl, IOptionalFeatureProvider, IPlatformGraphics,
    IPlatformGraphicsContext, IPlatformRenderInterfaceContext, IRenderTarget, RenderTargetDrawingContextProperties,
    RenderTargetProperties, RenderTargetSceneInfo,
};
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::testing::{
    DrawingLog, ManualRenderLoop, MockDrawingContextLayerImpl, MockPlatformRenderInterface, MockRenderTarget,
};
use crate::threading::Dispatcher;
use crate::{PixelSize, Size, Vector};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

fn render_thread_compositor(
    render_loop: &Arc<ManualRenderLoop>,
    use_ui_thread_for_synchronous_commits: bool,
) -> Rc<Compositor> {
    Compositor::with_render_thread(
        render_loop.clone(),
        None,
        use_ui_thread_for_synchronous_commits,
        &MediaContext::instance().scheduler(),
        Dispatcher::ui_thread(),
        None,
        None,
    )
}

/// A render thread that ticks the loop until it is stopped.
struct RenderThread {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl RenderThread {
    fn start(render_loop: &Arc<ManualRenderLoop>) -> RenderThread {
        let stop = Arc::new(AtomicBool::new(false));
        let (thread_stop, thread_loop) = (stop.clone(), render_loop.clone());
        let handle = thread::spawn(move || {
            while !thread_stop.load(Ordering::SeqCst) {
                thread_loop.tick();
                thread::yield_now();
            }
        });
        RenderThread { stop, handle: Some(handle) }
    }
}

impl Drop for RenderThread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[test]
fn the_thread_that_ticks_the_loop_renders() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
    let render_loop = ManualRenderLoop::new();
    let compositor = render_thread_compositor(&render_loop, false);
    assert!(compositor.renders_on_render_thread());
    let ui_thread = thread::current().id();

    // Objects are created and changed on this thread; their server objects
    // are created by the batch, by the thread that renders.
    let parent = compositor.create_container_visual();
    let child = compositor.create_solid_color_visual();
    parent.children().add((*child).clone());

    let task = compositor
        .invoke_server_job_async(move |server| Ok((thread::current().id(), server.object_count())), false);
    let continued = Rc::new(Cell::new(false));
    let flag = continued.clone();
    task.on_completed(move || flag.set(true));

    // The media context commits the batch on this thread.
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!task.is_completed());

    let tick_loop = render_loop.clone();
    let render_thread = thread::spawn(move || {
        tick_loop.tick();
        thread::current().id()
    })
    .join()
    .expect("the render thread ran");
    assert_ne!(ui_thread, render_thread);

    // The job ran on the render thread and its result is here.
    assert!(task.is_completed_successfully());
    let (job_thread, object_count) = task.take_result().expect("completed").expect("the job succeeded");
    assert_eq!(render_thread, job_thread);
    assert!(object_count >= 2, "the server objects of the two visuals exist: {object_count}");

    // The continuation belongs to this thread: it runs from the dispatcher.
    assert!(!continued.get());
    Dispatcher::ui_thread().run_jobs(None);
    assert!(continued.get());
}

#[test]
fn a_synchronous_commit_waits_for_the_render_thread() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
    let render_loop = ManualRenderLoop::background();
    let compositor = render_thread_compositor(&render_loop, false);
    let ui_thread = thread::current().id();
    let _render_thread = RenderThread::start(&render_loop);

    let ran_on = Arc::new(std::sync::Mutex::new(None));
    let slot = ran_on.clone();
    compositor.post_server_job(move |_| *slot.lock().unwrap() = Some(thread::current().id()), false);

    // Returns once the render thread has applied the batch and rendered.
    MediaContext::instance().immediate_render_requested(&compositor);
    let ran_on = ran_on.lock().unwrap().expect("the job ran before the commit returned");
    assert_ne!(ui_thread, ran_on);
}

#[test]
fn a_synchronous_commit_renders_on_the_ui_thread_where_the_platform_asks_for_it() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
    let render_loop = ManualRenderLoop::background();
    // `UseUiThreadForSynchronousCommits`: what the native platform asks for.
    let compositor = render_thread_compositor(&render_loop, true);
    let ui_thread = thread::current().id();

    let ran_on = Arc::new(std::sync::Mutex::new(None));
    let slot = ran_on.clone();
    compositor.post_server_job(move |_| *slot.lock().unwrap() = Some(thread::current().id()), false);

    // No render thread is running: this thread renders the frame itself.
    MediaContext::instance().immediate_render_requested(&compositor);
    assert_eq!(Some(ui_thread), *ran_on.lock().unwrap());
}

#[test]
fn both_threads_render_one_after_the_other() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
    let render_loop = ManualRenderLoop::background();
    let compositor = render_thread_compositor(&render_loop, true);
    let _render_thread = RenderThread::start(&render_loop);

    // Every job checks that no other job is inside the server at the same
    // time: the frames of the two threads exclude each other.
    let inside = Arc::new(AtomicUsize::new(0));
    let overlaps = Arc::new(AtomicUsize::new(0));
    let ran = Arc::new(AtomicUsize::new(0));
    const ROUNDS: usize = 200;
    for _ in 0..ROUNDS {
        let (inside, overlaps, ran) = (inside.clone(), overlaps.clone(), ran.clone());
        compositor.post_server_job(
            move |_| {
                if inside.fetch_add(1, Ordering::SeqCst) != 0 {
                    overlaps.fetch_add(1, Ordering::SeqCst);
                }
                thread::yield_now();
                inside.fetch_sub(1, Ordering::SeqCst);
                ran.fetch_add(1, Ordering::SeqCst);
            },
            false,
        );
        // Commits and renders on this thread while the render thread ticks.
        MediaContext::instance().immediate_render_requested(&compositor);
    }

    assert_eq!(ROUNDS, ran.load(Ordering::SeqCst));
    assert_eq!(0, overlaps.load(Ordering::SeqCst));
}

// --- a compositor confined to its render thread ---------------------------------
//
// `UseUiThreadForSynchronousCommits` false on a loop that runs in the
// background: the thread of the compositor never renders. The doubles below
// stand for what belongs to the thread that draws (a graphics context that
// only exists there, a render target on a canvas only that thread may draw
// to); they share a log that names the thread of their first use and
// records every use or drop by another thread.

/// What the doubles of a confined compositor saw.
#[derive(Default)]
struct ConfinementLog {
    owner: Mutex<Option<ThreadId>>,
    events: Mutex<Vec<&'static str>>,
    violations: Mutex<Vec<String>>,
}

impl ConfinementLog {
    /// Records a use or a drop of a double by the calling thread.
    fn touch(&self, what: &'static str) {
        let current = thread::current().id();
        let owner = *self.owner.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(current);
        if owner == current {
            self.events.lock().unwrap_or_else(|e| e.into_inner()).push(what);
        } else {
            self.violations
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(format!("{what} on {current:?}, which is not {owner:?}"));
        }
    }

    /// The thread that first used a double.
    fn owner(&self) -> Option<ThreadId> {
        *self.owner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Whether the owner did `what`.
    fn saw(&self, what: &str) -> bool {
        self.events.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|event| *event == what)
    }

    fn violations(&self) -> Vec<String> {
        self.violations.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// The surface of a window. The object is made by the thread of the window
/// and handed to the compositor whole; from then on it is used, and
/// dropped, by the thread that renders.
struct ConfinedSurface {
    log: Arc<ConfinementLog>,
}

impl IPlatformRenderSurface for ConfinedSurface {
    fn is_ready(&self) -> bool {
        self.log.touch("the surface is asked whether it is ready");
        true
    }

    fn as_any(&self) -> &dyn Any {
        self.log.touch("the surface is asked for its type");
        self
    }
}

impl Drop for ConfinedSurface {
    fn drop(&mut self) {
        self.log.touch("the surface is dropped");
    }
}

/// The platform graphics: an object of the thread of the compositor, which
/// the thread that renders asks for its context.
struct ConfinedGraphics {
    log: Arc<ConfinementLog>,
}

impl IPlatformGraphics for ConfinedGraphics {
    fn uses_shared_context(&self) -> bool {
        self.log.touch("the graphics are asked whether the context is shared");
        false
    }

    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.log.touch("the graphics context is created");
        Rc::new(ConfinedGraphicsContext { log: self.log.clone() })
    }

    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.create_context()
    }
}

struct ConfinedGraphicsContext {
    log: Arc<ConfinementLog>,
}

impl IOptionalFeatureProvider for ConfinedGraphicsContext {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        self.log.touch("the graphics context is asked for a feature");
        None
    }
}

impl IPlatformGraphicsContext for ConfinedGraphicsContext {
    fn is_lost(&self) -> bool {
        self.log.touch("the graphics context is asked whether it is lost");
        false
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.log.touch("the graphics context is made current");
        let log = self.log.clone();
        Disposable::create(move || log.touch("the graphics context is restored"))
    }

    fn dispose(&self) {
        self.log.touch("the graphics context is disposed");
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for ConfinedGraphicsContext {
    fn drop(&mut self) {
        self.log.touch("the graphics context is dropped");
    }
}

/// A feature the backend context hands to callers.
struct ConfinedFeature;

/// The backend context: creates the render targets and holds the graphics
/// context.
struct ConfinedBackendContext {
    log: Arc<ConfinementLog>,
    drawing_log: DrawingLog,
    _graphics_context: Option<Rc<dyn IPlatformGraphicsContext>>,
}

impl IOptionalFeatureProvider for ConfinedBackendContext {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        self.log.touch("the backend context is asked for a feature");
        None
    }
}

impl IPlatformRenderInterfaceContext for ConfinedBackendContext {
    fn create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        self.log.touch("the render target is created");
        for surface in surfaces {
            assert!(surface.as_any().is::<ConfinedSurface>());
        }
        Rc::new(ConfinedRenderTarget { log: self.log.clone(), inner: MockRenderTarget::new(self.drawing_log.clone()) })
    }

    fn create_offscreen_render_target(
        &self,
        pixel_size: PixelSize,
        _scaling: Vector,
        _enable_text_antialiasing: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        self.log.touch("an offscreen render target is created");
        Rc::new(MockDrawingContextLayerImpl::new(self.drawing_log.clone(), pixel_size))
    }

    fn is_lost(&self) -> bool {
        self.log.touch("the backend context is asked whether it is lost");
        false
    }

    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        None
    }

    fn public_features(&self) -> HashMap<TypeId, Rc<dyn Any>> {
        self.log.touch("the backend context is asked for its public features");
        let feature: Rc<dyn Any> = Rc::new(ConfinedFeature);
        HashMap::from([(TypeId::of::<ConfinedFeature>(), feature)])
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
        self.log.touch("the backend context is asked whether it can create a render target");
        surfaces.iter().all(|surface| surface.is_ready())
    }

    fn dispose(&self) {
        self.log.touch("the backend context is disposed");
    }
}

impl Drop for ConfinedBackendContext {
    fn drop(&mut self) {
        self.log.touch("the backend context is dropped");
    }
}

struct ConfinedRenderTarget {
    log: Arc<ConfinementLog>,
    inner: Rc<MockRenderTarget>,
}

impl IRenderTarget for ConfinedRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        self.log.touch("the render target is asked for its properties");
        self.inner.properties()
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        self.log.touch("the render target is drawn to");
        self.inner.create_drawing_context(scene_info)
    }

    fn dispose(&self) {
        self.log.touch("the render target is disposed");
        self.inner.dispose();
    }
}

impl Drop for ConfinedRenderTarget {
    fn drop(&mut self) {
        self.log.touch("the render target is dropped");
    }
}

/// A compositor that is confined to the thread that ticks `render_loop`,
/// over the doubles above.
struct ConfinedCompositor {
    log: Arc<ConfinementLog>,
    render_loop: Arc<ManualRenderLoop>,
    compositor: Rc<Compositor>,
}

impl ConfinedCompositor {
    fn new(render_interface: &MockPlatformRenderInterface) -> ConfinedCompositor {
        let log = Arc::new(ConfinementLog::default());
        let context_log = log.clone();
        render_interface.set_backend_context_factory(move |drawing_log, graphics_context| {
            context_log.touch("the backend context is created");
            let context: Rc<dyn IPlatformRenderInterfaceContext> = Rc::new(ConfinedBackendContext {
                log: context_log.clone(),
                drawing_log: drawing_log.clone(),
                _graphics_context: graphics_context,
            });
            context
        });
        let render_loop = ManualRenderLoop::background();
        let graphics: Rc<dyn IPlatformGraphics> = Rc::new(ConfinedGraphics { log: log.clone() });
        let compositor = Compositor::with_render_thread(
            render_loop.clone(),
            Some(graphics),
            false,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );
        assert!(compositor.is_confined_to_render_thread());
        ConfinedCompositor { log, render_loop, compositor }
    }

    /// Creates a target over a surface that nothing else holds, with a root
    /// visual, and shows it: a synchronous commit, as a top level does.
    fn show(&self) -> (Rc<CompositionTarget>, Rc<CompositionVisual>) {
        let surface: Arc<dyn IPlatformRenderSurface> = Arc::new(ConfinedSurface { log: self.log.clone() });
        let surfaces: RenderSurfaces = Arc::new(move || vec![surface.clone()]);
        let target = self.compositor.create_composition_target(surfaces);
        let root = self.compositor.create_container_visual();
        root.set_size(Vector::new(100.0, 80.0));
        target.set_root(Some(root.clone()));
        target.set_scaling(1.0);
        target.set_size(Size::new(100.0, 80.0));
        target.set_is_enabled(true);
        MediaContext::instance().immediate_render_requested(&self.compositor);
        (target, root)
    }

    /// Disposes the target as a renderer does, and lets go of it and of its
    /// root.
    fn dispose(&self, target: Rc<CompositionTarget>, root: Rc<CompositionVisual>) {
        target.set_root(None);
        target.mark_disposed_out_of_band();
        MediaContext::instance().sync_dispose_composition_target(&self.compositor, target.server());
        drop((target, root));
    }

    /// Drops the compositor, once the media context has committed what was
    /// pending and no longer holds it. Returns the loop and the log.
    fn drop_compositor(self) -> (Arc<ManualRenderLoop>, Arc<ConfinementLog>) {
        let ConfinedCompositor { log, render_loop, compositor } = self;
        wait_until("nothing else holds the compositor", || Rc::strong_count(&compositor) == 1);
        drop(compositor);
        (render_loop, log)
    }
}

/// Runs the jobs of the dispatcher until `condition` holds.
fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        Dispatcher::ui_thread().run_jobs(None);
        if condition() {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn a_confined_compositor_keeps_what_renders_on_the_render_thread() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let (_locator_scope, render_interface) = MockPlatformRenderInterface::install();
    let confined = ConfinedCompositor::new(&render_interface);
    let test_thread = thread::current().id();

    // Nothing has rendered yet, so no feature is known: the answer is
    // pending, and the backend context is not created here to give it.
    assert!(confined.compositor.try_get_render_interface_feature(TypeId::of::<ConfinedFeature>()).is_none());
    assert_eq!(None, confined.log.owner());

    let render_thread = RenderThread::start(&confined.render_loop);

    // Show: the first frame creates the graphics context, the backend
    // context and the render target, and draws.
    let (target, root) = confined.show();
    assert!(confined.log.saw("the graphics context is created"));
    assert!(confined.log.saw("the backend context is created"));
    assert!(confined.log.saw("the render target is created"));
    assert!(confined.log.saw("the render target is drawn to"));
    assert!(confined.log.saw("the surface is asked for its type"));

    // Resize, as a top level reports it: another synchronous commit.
    root.set_size(Vector::new(200.0, 150.0));
    target.set_size(Size::new(200.0, 150.0));
    MediaContext::instance().immediate_render_requested(&confined.compositor);
    // Each wait asked the loop for a frame out of turn first.
    assert_eq!(2, confined.render_loop.frames_requested_out_of_turn());

    // The job of the render thread has filled the cache by now.
    let feature = confined
        .compositor
        .try_get_render_interface_feature(TypeId::of::<ConfinedFeature>())
        .expect("the render thread has told which features there are");
    assert!(feature.is::<ConfinedFeature>());
    drop(feature);
    // A feature the context does not have is absent, not pending.
    assert!(confined.compositor.try_get_render_interface_feature(TypeId::of::<ConfinementLog>()).is_none());

    // A snapshot of the root is rendered offscreen by a job.
    let snapshot = confined.compositor.create_composition_visual_snapshot(&root, 1.0);
    MediaContext::instance().immediate_render_requested(&confined.compositor);
    assert!(snapshot.is_completed_successfully());
    assert!(confined.log.saw("an offscreen render target is created"));
    assert!(confined.log.saw("the graphics context is made current"));
    drop(snapshot);

    // The disposal of the target waits for the render thread too.
    confined.dispose(target, root);
    assert!(confined.log.saw("the render target is disposed"));
    assert!(confined.log.saw("the render target is dropped"));

    // The compositor only asks for the release: the render thread drops the
    // graph with its next tick, then the task leaves the loop.
    let (render_loop, log) = confined.drop_compositor();
    wait_until("the task of the compositor has left the loop", || render_loop.task_count() == 0);
    assert!(log.saw("the backend context is dropped"));
    assert!(log.saw("the graphics context is dropped"));
    drop(render_thread);

    assert_eq!(Vec::<String>::new(), log.violations());
    assert!(log.owner().is_some_and(|owner| owner != test_thread));
}

#[test]
fn a_confined_compositor_whose_loop_has_stopped_is_leaked_and_not_dropped_by_another_thread() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let (_locator_scope, render_interface) = MockPlatformRenderInterface::install();
    let confined = ConfinedCompositor::new(&render_interface);
    let test_thread = thread::current().id();
    let render_thread = RenderThread::start(&confined.render_loop);
    let (target, root) = confined.show();
    assert!(confined.log.saw("the render target is created"));
    confined.dispose(target, root);

    // The render thread ends with the graph alive: nothing will run the
    // release the compositor asks for.
    drop(render_thread);
    let (render_loop, log) = confined.drop_compositor();
    assert_eq!(1, render_loop.task_count());

    // The loop holds the last handle to the graph. Dropping it here must
    // not drop what belongs to the render thread.
    drop(render_loop);
    assert!(!log.saw("the backend context is dropped"));
    assert!(!log.saw("the graphics context is dropped"));
    assert_eq!(Vec::<String>::new(), log.violations());
    assert!(log.owner().is_some_and(|owner| owner != test_thread));
}
