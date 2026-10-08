use super::web_render_target::CanvasSize;
use crate::interop::canvas_helper::{RENDER_TARGET_KIND_SOFTWARE, RENDER_TARGET_KIND_WEB_GL};
use ferroui_base::PixelSize;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

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
/// - the kind of the render target, written by the render thread when its
///   worker has created the target;
/// - whether the view is disposed.
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
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<BrowserSurfaceShared>();
};

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

    /// The size as the function a render target asks at the start of each
    /// frame. The function belongs to the thread that calls this; the state
    /// behind it is shared.
    pub fn size_getter(self: &Arc<Self>) -> CanvasSize {
        let this = self.clone();
        Rc::new(move || this.size())
    }

    /// Records that the render target exists and its kind
    /// ([`RENDER_TARGET_KIND_WEB_GL`] or [`RENDER_TARGET_KIND_SOFTWARE`]).
    /// Called by the render thread when its worker reports the target.
    ///
    /// # Panics
    /// Panics when `kind` is neither.
    pub fn set_target_kind(&self, kind: i32) {
        assert!(
            kind == RENDER_TARGET_KIND_WEB_GL || kind == RENDER_TARGET_KIND_SOFTWARE,
            "{kind} is not a kind of render target"
        );
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!((PixelSize::new(300, 180), 1.5), (shared.size_getter())());
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
