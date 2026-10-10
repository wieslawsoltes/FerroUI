//! The worker as the UI thread reaches it (the port of
//! `WaylandWorkerClient.cs`).
//!
//! The reference creates the compositor of the framework in the constructor
//! of the worker. The compositor of the port is an object of the UI thread,
//! so the client creates it, with the render loop and the platform graphics
//! of the worker, and gives the worker the handle of its server side.
//!
//! `CreatePopupHandle` belongs to stage 2 of
//! `docs/porting/wayland-platform.md`.

use super::persistent::i_w_surface_event_sink::WXdgTopLevelEventSinkProxy;
use super::persistent::i_w_xdg_top_level::WXdgTopLevelProxy;
use super::persistent::i_wayland_cursor::WaylandCursorProxy;
use super::persistent::w_surface::{WSurfaceId, WXdgTopLevel};
use super::persistent::wayland_bitmap_cursor::WaylandBitmapCursor;
use super::persistent::wayland_cursor::{WaylandCursorId, WaylandStandardCursor};
use super::transient::rendering::i_wayland_framebuffer_surface::WaylandRenderSurfaceTarget;
use super::transient::rendering::wayland_egl_wsi_surface::WaylandEglWsiSurface;
use super::transient::rendering::wayland_framebuffer::WaylandFramebuffer;
use super::wayland_dispatch_priority::WaylandDispatchPriority;
use super::wayland_worker::{with_worker_thread, WaylandWorker, WaylandWorkerThread, WorkerJob, WorkerMarshaller};
use crate::wayland_surface_create_result::WaylandSurfaceCreateResult;
use ferroui_base::input::StandardCursorType;
use ferroui_base::media::MediaContext;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::IPlatformGraphics;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::IRenderLoop;
use ferroui_base::threading::Dispatcher;
use ferroui_base::PixelSize;
use ferroui_x11::raw_event_grouping::{AutomaticRawEventGrouperDispatchQueue, IRawEventGrouperDispatchQueue};
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

/// UI-thread-safe interface to the wayland worker.
/// UI thread code should use this instead of accessing the worker directly.
pub struct WaylandWorkerClient {
    worker: Arc<WaylandWorker>,
    compositor: Rc<Compositor>,
    input_dispatch_queue: Rc<dyn IRawEventGrouperDispatchQueue>,
    marshaller: WorkerMarshaller,
    _after_commit: Rc<dyn IDisposable>,
}

impl WaylandWorkerClient {
    /// The client of a worker, on the UI thread, with the compositor of the framework.
    pub fn new(worker: Arc<WaylandWorker>) -> Rc<Self> {
        let render_loop: Arc<dyn IRenderLoop> = worker.render_loop().clone();
        let graphics: Arc<dyn IPlatformGraphics> = worker.platform_graphics().clone();
        // `new Compositor(_renderLoop, PlatformGraphics)`: a loop that runs in the background
        // and no synchronous commits on the UI thread, so the thread that ticks the loop, the
        // worker, renders.
        let compositor = Compositor::with_render_thread(
            render_loop,
            Some(graphics),
            false,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );
        worker.set_server_compositor(compositor.locked_server());

        let after_commit = {
            let worker = worker.clone();
            compositor.after_commit(move || worker.on_after_commit())
        };

        let marshaller: WorkerMarshaller = {
            let worker = worker.clone();
            let compositor = Rc::downgrade(&compositor);
            Rc::new(move |job: WorkerJob, priority: WaylandDispatchPriority| {
                if priority == WaylandDispatchPriority::Oob {
                    worker.post_oob(job);
                } else if let Some(compositor) = compositor.upgrade() {
                    Self::post_with_commit_to(&worker, &compositor, job);
                }
            })
        };

        Rc::new(Self {
            worker,
            compositor,
            input_dispatch_queue: AutomaticRawEventGrouperDispatchQueue::new(None),
            marshaller,
            _after_commit: after_commit,
        })
    }

    fn post_with_commit_to(worker: &Arc<WaylandWorker>, compositor: &Rc<Compositor>, job: WorkerJob) {
        worker.note_pending_server_job();
        // The job runs on the thread that renders, which is the worker.
        compositor.post_server_job(
            move |_| {
                with_worker_thread(job);
            },
            false,
        );
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    /// Shared platform-wide input-dispatch queue.
    pub fn input_dispatch_queue(&self) -> &Rc<dyn IRawEventGrouperDispatchQueue> {
        &self.input_dispatch_queue
    }

    /// Wakes the wayland worker's render loop. Safe to call from any thread.
    pub fn any_thread_wakeup_render_loop(&self) {
        self.worker.any_thread_wakeup_render_loop();
    }

    /// Marshaller for UI→worker cross-thread proxies. Bound to this worker instance —
    /// no global state.
    pub fn marshaller(&self) -> &WorkerMarshaller {
        &self.marshaller
    }

    /// Posts a rare out-of-band callback directly to the wayland thread queue.
    pub fn post_oob(&self, job: impl FnOnce(&mut WaylandWorkerThread) + Send + 'static) {
        self.worker.post_oob(Box::new(job));
    }

    /// Posts a callback to the wayland thread batched with the current compositor commit.
    pub fn post_with_commit(&self, job: impl FnOnce(&mut WaylandWorkerThread) + Send + 'static) {
        Self::post_with_commit_to(&self.worker, &self.compositor, Box::new(job));
    }

    /// Posts a callback to the wayland thread queue out-of-band (bypassing the compositor batch)
    /// and returns the receiver of its result.
    pub fn invoke_oob<T: Send + 'static>(
        &self,
        callback: impl FnOnce(&mut WaylandWorkerThread) -> T + Send + 'static,
    ) -> Receiver<T> {
        self.worker.invoke_oob(callback)
    }

    /// Creates a UI-thread wrapper for a standard themed cursor. The wrapper must be released via
    /// `destroy` of the proxy (e.g. by disposing the cursor of the platform).
    pub fn create_standard_cursor(&self, cursor_type: StandardCursorType) -> WaylandCursorProxy {
        let id = WaylandCursorId::new();
        self.post_oob(move |worker| worker.register_standard_cursor(id, WaylandStandardCursor::new(cursor_type)));
        WaylandCursorProxy::new(id, self.marshaller.clone())
    }

    /// Creates a persistent worker-side custom bitmap cursor and returns a UI-thread wrapper for it.
    /// `pixels` must be tightly-packed Bgra8888 premultiplied data
    /// (stride == `size.width * 4`). The returned wrapper must be released via
    /// `destroy` of the proxy.
    pub fn create_bitmap_cursor(&self, pixels: Vec<u8>, size: PixelSize, hotspot_x: i32, hotspot_y: i32) -> WaylandCursorProxy {
        let id = WaylandCursorId::new();
        self.post_oob(move |worker| {
            worker.register_bitmap_cursor(id, WaylandBitmapCursor::new(pixels, size, hotspot_x, hotspot_y));
        });
        WaylandCursorProxy::new(id, self.marshaller.clone())
    }

    /// Creates a new xdg_toplevel surface and returns the bundle of UI-thread
    /// accessors needed to drive it. The underlying top-level
    /// is worker-thread state and is intentionally not exposed.
    pub fn create_top_level_handle(&self, sink: WXdgTopLevelEventSinkProxy) -> WaylandSurfaceCreateResult<WXdgTopLevelProxy> {
        let id = WSurfaceId::new();
        let (basic_init_sender, basic_init_completed) = channel();
        let worker_handle = self.worker.clone();
        // The constructor of a surface of the reference posts its registration out of band.
        self.post_oob(move |worker| {
            worker.register_top_level(WXdgTopLevel::new(id, worker_handle, sink, basic_init_sender));
        });

        // Render surfaces matching the platform graphics: the surface of EGL, which the
        // renderer takes when the worker has a GPU backend, and the framebuffer surface,
        // always included as a software fallback. The reference asks the worker's surface
        // for the list on the thread that renders; the surfaces of the port are handles
        // that name the worker's surface, so the list is made here.
        let render_surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![
            Arc::new(WaylandEglWsiSurface::new(id)),
            Arc::new(WaylandFramebuffer::new(WaylandRenderSurfaceTarget::Surface(id))),
        ];

        WaylandSurfaceCreateResult {
            proxy: WXdgTopLevelProxy::new(id, self.marshaller.clone()),
            render_surfaces,
            basic_init_completed,
        }
    }
}
