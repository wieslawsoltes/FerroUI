use crate::interop::canvas_helper::{RENDER_TARGET_KIND_SOFTWARE, RENDER_TARGET_KIND_WEB_GL};
use crate::interop::thread_proxy;
use ferroui_base::PixelSize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, Weak};

/// What the thread of the user interface and the thread that renders both
/// know about one canvas.
///
/// Not from upstream, where the surface of a view is one object that both
/// threads read by convention. With a render worker the canvas itself exists
/// in the worker only, and its element only in the page; this object is what
/// is left between them (`docs/porting/browser-render-worker.md`, section
/// 4):
///
/// - the id of the render target, written once by the thread that created
///   the canvas;
/// - the size of the canvas in device pixels and its scaling, written by the
///   thread of the user interface when the element changes, read by the
///   render thread before each frame, which sets the size of the canvas it
///   draws to;
/// - the kind of the render target, written by the thread that owns the
///   target: the thread of the page for a canvas it kept, the render thread
///   when its worker has created the target of a canvas that was transferred
///   to it;
/// - whether the view is disposed;
/// - how many frames were drawn to the canvas, counted by the thread that
///   draws them, and the thread that is told of the first one
///   ([`on_first_frame`](Self::on_first_frame)).
///
/// A canvas is found by the id of its render target
/// ([`register`](Self::register), [`find`](Self::find),
/// [`report_target`](Self::report_target)): a worker reports a target by its
/// id, on a thread that has never seen the surface.
///
/// Everything is an atomic, so neither thread ever waits for the other here:
/// the thread of the page may not block, and a lock that the render thread
/// holds for a moment would make it spin. The size and the scaling are read
/// as one value (a frame never sees a new size with an old scaling) through
/// a version counter that is odd while they are being written.
pub struct BrowserSurfaceShared {
    target_id: AtomicI32,
    size_version: AtomicU32,
    width: AtomicI32,
    height: AtomicI32,
    scaling: AtomicU64,
    target_kind: AtomicI32,
    disposed: AtomicBool,
    frames: AtomicU64,
    /// The thread that asked for the first frame
    /// ([`on_first_frame`](Self::on_first_frame)), as
    /// [`thread_proxy::current_thread`] names it there.
    first_frame_thread: AtomicUsize,
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<BrowserSurfaceShared>();
};

/// The canvases of the module by the id of their render target, and the
/// targets that were reported before their canvas was registered.
///
/// One table for all threads, behind a lock, unlike the rest of this file:
/// it is entered when a canvas is created or closed and when a worker
/// reports a target, never during a frame, and for a look-up in a map.
#[derive(Default)]
struct SurfaceRegistry {
    surfaces: HashMap<i32, Weak<BrowserSurfaceShared>>,
    /// The kind of each target a worker reported under an id no canvas is
    /// registered with yet. The worker can be faster than the thread that
    /// created the canvas: the id is known to that thread only when the call
    /// that posted the canvas to the worker returns.
    reported: HashMap<i32, i32>,
}

static REGISTRY: OnceLock<Mutex<SurfaceRegistry>> = OnceLock::new();

fn registry() -> MutexGuard<'static, SurfaceRegistry> {
    REGISTRY.get_or_init(Mutex::default).lock().unwrap_or_else(PoisonError::into_inner)
}

/// What a thread runs when the first frame of a canvas has been drawn.
type FirstFrameHandler = Box<dyn FnOnce()>;

thread_local! {
    /// What the calling thread runs when the first frame of a canvas has
    /// been drawn, by the id of the render target of the canvas
    /// ([`BrowserSurfaceShared::on_first_frame`]). A handler holds objects
    /// of the page, so it stays with the thread that gave it: only the id
    /// crosses to the thread that draws and back.
    static FIRST_FRAME_HANDLERS: RefCell<HashMap<i32, FirstFrameHandler>> = RefCell::new(HashMap::new());
}

/// Runs what the calling thread wanted done at the first frame of the
/// canvas with the render target `target_id`, when it still waits.
fn first_frame_drawn(target_id: i32) {
    let handler = FIRST_FRAME_HANDLERS.with(|handlers| handlers.borrow_mut().remove(&target_id));
    if let Some(handler) = handler {
        handler();
    }
}

impl BrowserSurfaceShared {
    /// Creates the state of a canvas that has no render target, no id and no
    /// size yet.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            target_id: AtomicI32::new(0),
            size_version: AtomicU32::new(0),
            width: AtomicI32::new(0),
            height: AtomicI32::new(0),
            scaling: AtomicU64::new(0f64.to_bits()),
            target_kind: AtomicI32::new(0),
            disposed: AtomicBool::new(false),
            frames: AtomicU64::new(0),
            first_frame_thread: AtomicUsize::new(0),
        })
    }

    /// The id of the render target in the registry of the script side; 0
    /// while the canvas has not been created.
    pub fn target_id(&self) -> i32 {
        self.target_id.load(Ordering::SeqCst)
    }

    /// Records the id of the render target. Called by the thread that
    /// created the canvas, once it has.
    pub fn set_target_id(&self, target_id: i32) {
        self.target_id.store(target_id, Ordering::SeqCst);
    }

    /// Records the id of the render target and makes the canvas known under
    /// it to every thread ([`find`](Self::find)). Called by the thread that
    /// created the canvas, once it has. When a worker has already reported
    /// the target of that id, its kind is published here.
    ///
    /// The table holds the canvas weakly; a canvas that is registered again
    /// under an id takes the place of the one before.
    pub fn register(self: &Arc<Self>, target_id: i32) {
        self.set_target_id(target_id);
        let mut registry = registry();
        registry.surfaces.insert(target_id, Arc::downgrade(self));
        if let Some(kind) = registry.reported.remove(&target_id) {
            self.set_target_kind(kind);
        }
    }

    /// Takes the canvas out of the table of [`register`](Self::register): a
    /// target reported under its id from here on belongs to no canvas.
    /// Called when the view is closed. Does nothing for a canvas that is not
    /// registered, and leaves alone a canvas that took its id since.
    pub fn unregister(&self) {
        let target_id = self.target_id();
        let mut registry = registry();
        if registry.surfaces.get(&target_id).is_some_and(|surface| std::ptr::eq(surface.as_ptr(), self)) {
            registry.surfaces.remove(&target_id);
        }
    }

    /// The canvas that is registered under the id of a render target and
    /// still alive. Any thread may ask.
    pub fn find(target_id: i32) -> Option<Arc<Self>> {
        registry().surfaces.get(&target_id).and_then(Weak::upgrade)
    }

    /// The render target `target_id` exists and is of `kind`
    /// ([`RENDER_TARGET_KIND_WEB_GL`] or [`RENDER_TARGET_KIND_SOFTWARE`]):
    /// publishes the kind in the canvas registered under the id and returns
    /// the canvas. Called by the thread whose worker created the target.
    ///
    /// When no canvas is registered under the id yet, the kind is kept for
    /// the one that will be ([`register`](Self::register)) and the answer is
    /// `None`.
    ///
    /// # Panics
    /// Panics when `kind` is neither.
    pub fn report_target(target_id: i32, kind: i32) -> Option<Arc<Self>> {
        assert_target_kind(kind);
        let mut registry = registry();
        match registry.surfaces.get(&target_id).and_then(Weak::upgrade) {
            Some(surface) => {
                surface.set_target_kind(kind);
                Some(surface)
            }
            None => {
                registry.surfaces.remove(&target_id);
                registry.reported.insert(target_id, kind);
                None
            }
        }
    }

    /// The size of the canvas in device pixels and its scaling, as one
    /// value. Never waits: while a write is in progress it reads again.
    pub fn size(&self) -> (PixelSize, f64) {
        loop {
            let version = self.size_version.load(Ordering::SeqCst);
            if version & 1 == 0 {
                let width = self.width.load(Ordering::SeqCst);
                let height = self.height.load(Ordering::SeqCst);
                let scaling = f64::from_bits(self.scaling.load(Ordering::SeqCst));
                if self.size_version.load(Ordering::SeqCst) == version {
                    return (PixelSize::new(width, height), scaling);
                }
            }
            std::hint::spin_loop();
        }
    }

    /// Records the size of the canvas in device pixels and its scaling.
    /// Called by the thread of the user interface.
    pub fn set_size(&self, size: PixelSize, scaling: f64) {
        // The version is made odd for the duration of the write. One thread
        // writes in practice; a second writer waits for the first here, for
        // the three stores below.
        let mut version = self.size_version.load(Ordering::SeqCst);
        loop {
            if version & 1 == 1 {
                std::hint::spin_loop();
                version = self.size_version.load(Ordering::SeqCst);
                continue;
            }
            match self.size_version.compare_exchange_weak(
                version,
                version.wrapping_add(1),
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(current) => version = current,
            }
        }
        self.width.store(size.width, Ordering::SeqCst);
        self.height.store(size.height, Ordering::SeqCst);
        self.scaling.store(scaling.to_bits(), Ordering::SeqCst);
        self.size_version.store(version.wrapping_add(2), Ordering::SeqCst);
    }

    /// Records what the observer of the canvas element reports:
    /// `pixel_width` and `pixel_height` in device pixels and the device
    /// pixel ratio, as
    /// [`on_size_changed`](crate::interop::canvas_helper::on_size_changed)
    /// receives them.
    pub fn on_size_changed(&self, pixel_width: f64, pixel_height: f64, dpr: f64) {
        self.set_size(PixelSize::new(pixel_width as i32, pixel_height as i32), dpr);
    }

    /// Records that the render target exists and its kind
    /// ([`RENDER_TARGET_KIND_WEB_GL`] or [`RENDER_TARGET_KIND_SOFTWARE`]).
    /// Called by the thread that owns the target: the thread that created
    /// the canvas when it kept it, the render thread when its worker reports
    /// the target ([`report_target`](Self::report_target)).
    ///
    /// # Panics
    /// Panics when `kind` is neither.
    pub fn set_target_kind(&self, kind: i32) {
        assert_target_kind(kind);
        self.target_kind.store(kind, Ordering::SeqCst);
    }

    /// The kind of the render target; 0 while there is none.
    pub fn target_kind(&self) -> i32 {
        self.target_kind.load(Ordering::SeqCst)
    }

    /// Whether the render target exists.
    pub fn has_target(&self) -> bool {
        self.target_kind() != 0
    }

    /// Whether the target renders with a graphics context (WebGL); `None`
    /// while there is no target.
    pub fn uses_contexts(&self) -> Option<bool> {
        match self.target_kind() {
            0 => None,
            kind => Some(kind == RENDER_TARGET_KIND_WEB_GL),
        }
    }

    /// Marks the view as disposed: nothing renders to its canvas any more.
    pub fn dispose(&self) {
        self.disposed.store(true, Ordering::SeqCst);
    }

    /// Whether the view is disposed.
    pub fn is_disposed(&self) -> bool {
        self.disposed.load(Ordering::SeqCst)
    }

    /// Whether a frame can be rendered: the target exists, the canvas has a
    /// size, and the view is not disposed. Either thread may ask.
    pub fn is_ready(&self) -> bool {
        self.has_target() && !self.is_disposed() && self.size().0 != PixelSize::default()
    }

    /// The frames that were drawn to the canvas so far. Any thread may ask.
    pub fn frames(&self) -> u64 {
        self.frames.load(Ordering::SeqCst)
    }

    /// Runs `handler` once, on the calling thread, when the first frame has
    /// been drawn to the canvas; at once when it already has. Called by the
    /// thread that created the canvas, after the canvas has its id
    /// ([`register`](Self::register)); a second handler takes the place of
    /// one that still waits.
    ///
    /// Not from upstream, which has no such notification: its view closes
    /// the splash screen of the page at the first animation frame of the
    /// top-level, on the thread that also draws. That is before the frame
    /// is drawn, and with a render thread the frame is drawn by another
    /// thread, which may be seconds away
    /// (`docs/porting/browser-render-worker.md`, "B3 follow-ups").
    ///
    /// When this thread draws the canvas itself the handler runs inside the
    /// frame, right after it was drawn: it must not call into the view. When
    /// a render thread draws, that thread queues a call for this one
    /// ([`thread_proxy::run_on_thread`]) and the handler runs from the event
    /// loop of this thread; nothing is polled and nobody waits. A frame is
    /// only drawn while the page is visible and the canvas has a size, so
    /// the handler of a view that never draws never runs.
    pub fn on_first_frame(&self, handler: Box<dyn FnOnce()>) {
        let target_id = self.target_id();
        // Before the count is read: a thread that draws the first frame
        // after that reads the thread to tell.
        self.first_frame_thread.store(thread_proxy::current_thread(), Ordering::SeqCst);
        FIRST_FRAME_HANDLERS.with(|handlers| handlers.borrow_mut().insert(target_id, handler));
        if self.frames() > 0 {
            first_frame_drawn(target_id);
        }
    }

    /// Drops the handler of [`on_first_frame`](Self::on_first_frame) when it
    /// still waits. Called by the thread that gave it, when the view is
    /// closed.
    pub fn forget_first_frame(&self) {
        let target_id = self.target_id();
        let handler = FIRST_FRAME_HANDLERS.with(|handlers| handlers.borrow_mut().remove(&target_id));
        // Outside the table: what the handler holds may be dropped with it.
        drop(handler);
    }

    /// A frame was drawn to the canvas by the calling thread. The first one
    /// is made known to the thread that asked for it
    /// ([`on_first_frame`](Self::on_first_frame)): directly when that is
    /// this thread, through its event loop otherwise. When that call cannot
    /// be queued, the handler does not run.
    pub(crate) fn frame_presented(&self) {
        if self.frames.fetch_add(1, Ordering::SeqCst) != 0 {
            return;
        }
        let target_id = self.target_id();
        let thread = self.first_frame_thread.load(Ordering::SeqCst);
        if thread_proxy::current_thread() == thread {
            first_frame_drawn(target_id);
        } else {
            thread_proxy::run_on_thread(thread, Box::new(move || first_frame_drawn(target_id)));
        }
    }
}

fn assert_target_kind(kind: i32) {
    assert!(
        kind == RENDER_TARGET_KIND_WEB_GL || kind == RENDER_TARGET_KIND_SOFTWARE,
        "{kind} is not a kind of render target"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn a_new_surface_has_nothing_and_is_not_ready() {
        let shared = BrowserSurfaceShared::new();

        assert_eq!(0, shared.target_id());
        assert_eq!((PixelSize::default(), 0.0), shared.size());
        assert!(!shared.has_target());
        assert_eq!(None, shared.uses_contexts());
        assert!(!shared.is_disposed());
        assert!(!shared.is_ready());
    }

    #[test]
    fn it_is_ready_with_a_target_and_a_size_until_it_is_disposed() {
        let shared = BrowserSurfaceShared::new();
        shared.set_target_id(3);
        shared.set_target_kind(RENDER_TARGET_KIND_SOFTWARE);
        assert_eq!(Some(false), shared.uses_contexts());
        assert!(!shared.is_ready());

        shared.on_size_changed(300.0, 180.0, 1.5);
        assert_eq!(3, shared.target_id());
        assert_eq!((PixelSize::new(300, 180), 1.5), shared.size());
        assert!(shared.is_ready());

        shared.set_target_kind(RENDER_TARGET_KIND_WEB_GL);
        assert_eq!(Some(true), shared.uses_contexts());

        shared.dispose();
        assert!(shared.is_disposed());
        assert!(!shared.is_ready());
    }

    #[test]
    #[should_panic(expected = "is not a kind of render target")]
    fn an_unknown_kind_is_refused() {
        BrowserSurfaceShared::new().set_target_kind(7);
    }

    // The table of the canvases is one for the whole process, and the tests
    // run side by side: each test below has ids of its own.

    #[test]
    fn a_registered_surface_is_found_by_its_id_while_it_is_alive() {
        let shared = BrowserSurfaceShared::new();
        assert!(BrowserSurfaceShared::find(9101).is_none());

        shared.register(9101);
        assert_eq!(9101, shared.target_id());
        assert!(Arc::ptr_eq(&shared, &BrowserSurfaceShared::find(9101).expect("the surface is registered")));
        // Another thread finds the same object.
        let found = std::thread::spawn(|| BrowserSurfaceShared::find(9101)).join().unwrap();
        assert!(Arc::ptr_eq(&shared, &found.expect("the surface is registered")));

        // The table does not keep the surface alive.
        drop(shared);
        assert!(BrowserSurfaceShared::find(9101).is_none());
    }

    #[test]
    fn a_reported_target_publishes_its_kind_in_the_surface_of_its_id() {
        let shared = BrowserSurfaceShared::new();
        shared.register(9201);
        shared.on_size_changed(10.0, 10.0, 1.0);
        assert!(!shared.is_ready());

        // The report arrives on the thread of the worker.
        let reported =
            std::thread::spawn(|| BrowserSurfaceShared::report_target(9201, RENDER_TARGET_KIND_WEB_GL)).join().unwrap();

        assert!(Arc::ptr_eq(&shared, &reported.expect("the surface is registered")));
        assert_eq!(Some(true), shared.uses_contexts());
        assert!(shared.is_ready());
    }

    #[test]
    fn a_target_reported_before_its_surface_is_registered_is_kept_for_it() {
        // The worker was faster than the thread that created the canvas.
        assert!(BrowserSurfaceShared::report_target(9301, RENDER_TARGET_KIND_SOFTWARE).is_none());

        let shared = BrowserSurfaceShared::new();
        assert!(!shared.has_target());
        shared.register(9301);
        assert_eq!(Some(false), shared.uses_contexts());

        // The report was for one surface: the next one under the id starts
        // without a target.
        let next = BrowserSurfaceShared::new();
        next.register(9301);
        assert!(!next.has_target());
        assert!(Arc::ptr_eq(&next, &BrowserSurfaceShared::find(9301).expect("the surface is registered")));
    }

    #[test]
    fn an_unregistered_surface_is_not_found_and_takes_no_report() {
        let shared = BrowserSurfaceShared::new();
        shared.register(9401);
        shared.unregister();
        shared.unregister();

        assert!(BrowserSurfaceShared::find(9401).is_none());
        assert!(BrowserSurfaceShared::report_target(9401, RENDER_TARGET_KIND_WEB_GL).is_none());
        assert!(!shared.has_target());

        // A surface that took the id is not taken out by the one before.
        let next = BrowserSurfaceShared::new();
        next.register(9401);
        shared.unregister();
        assert!(Arc::ptr_eq(&next, &BrowserSurfaceShared::find(9401).expect("the surface is registered")));
    }

    #[test]
    fn the_first_frame_runs_the_handler_once() {
        let shared = BrowserSurfaceShared::new();
        shared.set_target_id(9801);
        let runs = Rc::new(Cell::new(0));
        shared.on_first_frame(Box::new({
            let runs = runs.clone();
            move || runs.set(runs.get() + 1)
        }));
        assert_eq!((0, 0), (shared.frames(), runs.get()));

        shared.frame_presented();
        assert_eq!((1, 1), (shared.frames(), runs.get()));
        shared.frame_presented();
        assert_eq!((2, 1), (shared.frames(), runs.get()));
    }

    #[test]
    fn a_handler_given_after_the_first_frame_runs_at_once() {
        let shared = BrowserSurfaceShared::new();
        shared.set_target_id(9802);
        shared.frame_presented();
        let ran = Rc::new(Cell::new(false));
        shared.on_first_frame(Box::new({
            let ran = ran.clone();
            move || ran.set(true)
        }));

        assert!(ran.get());
    }

    #[test]
    fn a_forgotten_handler_does_not_run() {
        let shared = BrowserSurfaceShared::new();
        shared.set_target_id(9803);
        shared.on_first_frame(Box::new(|| panic!("the view was closed before its first frame")));
        shared.forget_first_frame();

        shared.frame_presented();
        assert_eq!(1, shared.frames());
    }

    #[test]
    #[should_panic(expected = "is not a kind of render target")]
    fn a_report_of_an_unknown_kind_is_refused() {
        BrowserSurfaceShared::report_target(9501, 7);
    }

    #[test]
    fn a_reader_on_another_thread_never_sees_half_of_a_write() {
        let shared = BrowserSurfaceShared::new();
        let stop = Arc::new(AtomicBool::new(false));
        let reader = std::thread::spawn({
            let shared = shared.clone();
            let stop = stop.clone();
            move || {
                let mut reads = 0u32;
                loop {
                    let (size, scaling) = shared.size();
                    // Every write stores one number three times.
                    assert_eq!(size.width, size.height);
                    assert_eq!(f64::from(size.width), scaling);
                    reads += 1;
                    if stop.load(Ordering::SeqCst) {
                        return reads;
                    }
                }
            }
        });

        for value in 1..=20_000 {
            shared.set_size(PixelSize::new(value, value), f64::from(value));
        }
        stop.store(true, Ordering::SeqCst);

        assert!(reader.join().unwrap() > 0);
        assert_eq!((PixelSize::new(20_000, 20_000), 20_000.0), shared.size());
    }
}
