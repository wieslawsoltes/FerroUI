use super::web_render_target::{worker_started, PENDING_RENDER_THREAD};
use super::{BrowserRenderTimer, BrowserSurfaceShared};
use crate::interop::thread_proxy;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// The render thread of the page: a thread of the module whose worker takes
/// the canvases of the views and draws to them.
///
/// Upstream starts a worker of its runtime, waits for it and only then goes
/// on with the start of the application. A start function of this port
/// cannot wait (the thread of the page may not block on a thread that needs
/// the page to start), so [`start`](Self::start) returns at once and the
/// thread reports itself when it runs:
///
/// - the thread installs the handler of the registry of its script
///   ([`initialize_worker`](super::initialize_worker)), follows the reports
///   of that registry ([`on_render_target_registered`](Self::on_render_target_registered)),
///   keeps itself alive past the return of its start function, starts the
///   frame loop it was given, and publishes its id;
/// - a canvas that is created before that
///   ([`canvas_thread_id`](Self::canvas_thread_id) answers
///   [`PENDING_RENDER_THREAD`]) is transferred by the script at once and
///   posted to the worker when the thread of the page hears of the thread;
///   its surface is simply not ready until the worker has reported the
///   target.
///
/// There is one render thread for the life of the page; it never ends.
///
/// The platform starts it when it is registered, for the frame loop of the
/// page ([`BrowserSharedRenderLoop::start_render_thread`](super::BrowserSharedRenderLoop::start_render_thread)).
/// A module built without threads cannot start it at all.
pub struct RenderWorker;

/// What the threads of the module know about the render thread.
struct WorkerState {
    /// Whether the thread was asked for and could be created.
    started: AtomicBool,
    /// The thread that asked for it and creates the canvases, as
    /// [`thread_proxy::current_thread`] names it there.
    page_thread: AtomicUsize,
    /// The render thread; 0 until it has installed its handler.
    thread: AtomicUsize,
    /// Whether the script of the thread of the page has been told.
    announced: AtomicBool,
}

static STATE: WorkerState = WorkerState::new();

impl WorkerState {
    const fn new() -> Self {
        Self {
            started: AtomicBool::new(false),
            page_thread: AtomicUsize::new(0),
            thread: AtomicUsize::new(0),
            announced: AtomicBool::new(false),
        }
    }

    /// Records that `page_thread` asks for the render thread. Returns
    /// whether it is the first to ask.
    fn begin(&self, page_thread: usize) -> bool {
        if self.started.swap(true, Ordering::SeqCst) {
            return false;
        }
        self.page_thread.store(page_thread, Ordering::SeqCst);
        true
    }

    /// The thread could not be created: there is no render thread, and it
    /// may be asked for again.
    fn abandon(&self) {
        self.started.store(false, Ordering::SeqCst);
    }

    /// Called by the render thread when its worker takes canvases.
    fn running(&self, thread: usize) {
        self.thread.store(thread, Ordering::SeqCst);
    }

    fn exists(&self) -> bool {
        self.started.load(Ordering::SeqCst)
    }

    fn thread(&self) -> usize {
        self.thread.load(Ordering::SeqCst)
    }

    fn page_thread(&self) -> usize {
        self.page_thread.load(Ordering::SeqCst)
    }

    fn canvas_thread_id(&self) -> i32 {
        if !self.exists() {
            return 0;
        }
        match self.thread() {
            0 => PENDING_RENDER_THREAD,
            // The id is an address in a memory of at most 4 GB; the script
            // reads the same bits as the key of the thread.
            thread => thread as i32,
        }
    }

    /// The id to tell the script of the thread of the page, once: `Some`
    /// for the first call that is made on that thread after the render
    /// thread reported itself.
    fn take_announcement(&self, current_thread: usize) -> Option<i32> {
        let thread = self.thread();
        if thread == 0 || current_thread != self.page_thread() {
            return None;
        }
        if self.announced.swap(true, Ordering::SeqCst) {
            return None;
        }
        Some(thread as i32)
    }
}

impl RenderWorker {
    /// Starts the render thread and returns at once; `Ok(false)` when it
    /// was started before. Called by the thread that creates the canvases
    /// (the thread of the page).
    ///
    /// `timer` is the render timer whose frame loop the thread runs: it is
    /// started on the thread ([`BrowserRenderTimer::start_on_this_thread`]).
    /// `on_thread` runs on the thread before that, when its worker already
    /// takes canvases; a caller subscribes there to what arrives on that
    /// thread.
    ///
    /// # Errors
    /// An error when the thread could not be created; in a module built
    /// without threads always.
    pub fn start(
        timer: Option<Arc<BrowserRenderTimer>>,
        on_thread: Option<Box<dyn FnOnce() + Send>>,
    ) -> std::io::Result<bool> {
        if !STATE.begin(thread_proxy::current_thread()) {
            return Ok(false);
        }
        let spawned = native::spawn(move || {
            if let Some(on_thread) = on_thread {
                on_thread();
            }
            if let Some(timer) = timer {
                timer.start_on_this_thread();
            }
            // Last: from here on a canvas is posted to this thread at once.
            STATE.running(thread_proxy::current_thread());
            // The thread of the page hears of it from its event loop and
            // posts the canvases it held back. When the call cannot be
            // queued, the next canvas that is created tells the script
            // (`canvas_thread_id`).
            thread_proxy::run_on_thread(STATE.page_thread(), Box::new(Self::announce));
        });
        match spawned {
            Ok(()) => Ok(true),
            Err(error) => {
                STATE.abandon();
                Err(error)
            }
        }
    }

    /// Whether a render thread exists: it was started, though it may not
    /// have reported itself yet. Upstream's condition for where the frame
    /// loop runs (`IsThreadingEnabled`) is a property of the build; here it
    /// is whether the thread was started, so that a module built with
    /// threads can still run on one.
    pub fn exists() -> bool {
        STATE.exists()
    }

    /// The id of the render thread, as [`thread_proxy::current_thread`]
    /// names it there (what `pthread_self` returns); 0 until the thread has
    /// reported itself, and always without one.
    pub fn thread_id() -> usize {
        STATE.thread()
    }

    /// The thread a canvas is created for
    /// ([`create_render_target_surface`](crate::interop::canvas_helper::CanvasSurface::create_render_target_surface)):
    /// 0 without a render thread, the id of the render thread once it has
    /// reported itself, and [`PENDING_RENDER_THREAD`] in between. Called by
    /// the thread that creates the canvases.
    pub fn canvas_thread_id() -> i32 {
        Self::announce();
        STATE.canvas_thread_id()
    }

    /// Queues `work` for the render thread, which runs it from the event
    /// loop of its worker. Returns whether it was queued: `false` while the
    /// thread has not reported itself, and without one.
    pub fn post(work: impl FnOnce() + Send + 'static) -> bool {
        thread_proxy::run_on_thread(STATE.thread(), Box::new(work))
    }

    /// The worker of the calling thread created the render target
    /// `target_id` of a canvas that was transferred to it, and the target
    /// is of `kind`: publishes the kind in the canvas registered under the
    /// id ([`BrowserSurfaceShared::report_target`]), from where both
    /// threads read that the surface is ready. The render thread subscribes
    /// this to the reports of its registry
    /// ([`add_render_target_registered`](crate::interop::canvas_helper::add_render_target_registered)).
    ///
    /// The render loop is not asked for a frame: its timer ticks with every
    /// animation frame of the worker, and a composition target without a
    /// render target asks its surface again on each.
    pub fn on_render_target_registered(target_id: i32, kind: i32) {
        BrowserSurfaceShared::report_target(target_id, kind);
    }

    /// Tells the script of the thread of the page that the render thread
    /// takes canvases, once. Does nothing on another thread and before the
    /// render thread has reported itself.
    fn announce() {
        if let Some(thread_id) = STATE.take_announcement(thread_proxy::current_thread()) {
            worker_started(thread_id);
        }
    }
}

#[cfg(all(target_os = "emscripten", target_feature = "atomics"))]
mod native {
    use super::RenderWorker;
    use crate::interop::canvas_helper::add_render_target_registered;
    use crate::rendering::initialize_worker;
    use std::rc::Rc;

    extern "C" {
        fn emscripten_runtime_keepalive_push();
    }

    /// Creates the render thread. The thread makes its worker take the
    /// canvases that are transferred to it, keeps itself alive, runs
    /// `on_thread` and returns to the event loop of its worker, where the
    /// canvases, the animation frames and the calls queued for it arrive.
    pub(super) fn spawn(on_thread: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
        let spawned = std::thread::Builder::new().name("ferroui-render".to_string()).spawn(move || {
            // The subscription lasts as long as the thread; its token is not
            // kept.
            add_render_target_registered(Rc::new(RenderWorker::on_render_target_registered));
            initialize_worker();
            // The start function returns below, and the thread has to go on.
            // SAFETY: the function takes no argument; it increments a counter
            // of the script of the calling thread, which keeps the thread
            // from exiting when its start function returns.
            unsafe { emscripten_runtime_keepalive_push() };
            on_thread();
        });
        // The handle is dropped: the thread is detached and nothing joins it.
        spawned.map(drop)
    }
}

#[cfg(not(all(target_os = "emscripten", target_feature = "atomics")))]
mod native {
    /// A module without threads has one thread, and it is the one of the
    /// page.
    pub(super) fn spawn(_on_thread: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "the module was built without threads"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::canvas_helper::{RENDER_TARGET_KIND_SOFTWARE, RENDER_TARGET_KIND_WEB_GL};

    #[test]
    fn a_canvas_is_created_for_no_thread_a_pending_thread_or_the_render_thread() {
        let state = WorkerState::new();
        assert!(!state.exists());
        assert_eq!(0, state.canvas_thread_id());

        // Started by the thread 5, not running yet.
        assert!(state.begin(5));
        assert!(!state.begin(6));
        assert!(state.exists());
        assert_eq!(5, state.page_thread());
        assert_eq!(0, state.thread());
        assert_eq!(PENDING_RENDER_THREAD, state.canvas_thread_id());

        state.running(4096);
        assert_eq!(4096, state.thread());
        assert_eq!(4096, state.canvas_thread_id());
    }

    #[test]
    fn the_script_of_the_page_is_told_once_on_its_thread_when_the_thread_runs() {
        let state = WorkerState::new();
        assert!(state.begin(5));
        // Not running: nothing to tell.
        assert_eq!(None, state.take_announcement(5));

        state.running(4096);
        // The render thread itself, and any other, does not tell.
        assert_eq!(None, state.take_announcement(4096));
        assert_eq!(Some(4096), state.take_announcement(5));
        assert_eq!(None, state.take_announcement(5));
    }

    #[test]
    fn a_thread_that_could_not_be_created_can_be_asked_for_again() {
        let state = WorkerState::new();
        assert!(state.begin(5));
        state.abandon();

        assert!(!state.exists());
        assert_eq!(0, state.canvas_thread_id());
        assert!(state.begin(7));
        assert_eq!(7, state.page_thread());
    }

    #[test]
    fn without_threads_there_is_no_render_thread() {
        let started = RenderWorker::start(None, Some(Box::new(|| panic!("there is no thread to run this"))));

        assert_eq!(std::io::ErrorKind::Unsupported, started.expect_err("the build has no threads").kind());
        assert!(!RenderWorker::exists());
        assert_eq!(0, RenderWorker::thread_id());
        assert_eq!(0, RenderWorker::canvas_thread_id());
        assert!(!RenderWorker::post(|| panic!("there is no thread to run this")));
    }

    #[test]
    fn a_report_of_the_worker_publishes_the_kind_of_the_target() {
        let shared = BrowserSurfaceShared::new();
        shared.register(9601);
        // The report arrives on the render thread.
        std::thread::spawn(|| RenderWorker::on_render_target_registered(9601, RENDER_TARGET_KIND_WEB_GL))
            .join()
            .unwrap();
        assert_eq!(Some(true), shared.uses_contexts());

        // A report that is faster than the thread that created the canvas.
        RenderWorker::on_render_target_registered(9602, RENDER_TARGET_KIND_SOFTWARE);
        let late = BrowserSurfaceShared::new();
        late.register(9602);
        assert_eq!(Some(false), late.uses_contexts());
    }
}
