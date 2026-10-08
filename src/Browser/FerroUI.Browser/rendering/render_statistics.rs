use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU64, AtomicUsize, Ordering};

/// What the page can read about the frames that reached its canvases: how
/// many, which thread drew the last one and to what, and what the ticks of
/// a render thread cost the thread of the page.
///
/// Not from upstream. With a render worker nothing on the thread of the
/// page sees a frame, so a page (a test, a diagnostic overlay of the host)
/// that wants to know where rendering happens reads it here. The numbers
/// are counted for all canvases of the module.
///
/// The fields are read one after the other from counters that the thread
/// that renders goes on writing: two of them may belong to different
/// frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct RenderStatistics {
    /// The frames that were drawn to a canvas since the module started.
    pub frames: u64,
    /// The thread that drew the last frame, as
    /// [`current_thread`](crate::interop::thread_proxy::current_thread)
    /// names it there; 0 before the first frame, and always in a build
    /// without threads.
    pub frame_thread: usize,
    /// The kind of the render target of the last frame:
    /// [`RENDER_TARGET_KIND_WEB_GL`](crate::interop::canvas_helper::RENDER_TARGET_KIND_WEB_GL)
    /// or
    /// [`RENDER_TARGET_KIND_SOFTWARE`](crate::interop::canvas_helper::RENDER_TARGET_KIND_SOFTWARE);
    /// 0 before the first frame.
    pub frame_kind: i32,
    /// The major version of OpenGL ES the last frame was drawn with (2 for
    /// WebGL 1, 3 for WebGL 2); 0 for a frame drawn in memory.
    pub frame_gl_major_version: i32,
    /// The width of the last frame in device pixels.
    pub frame_width: i32,
    /// The height of the last frame in device pixels.
    pub frame_height: i32,
    /// The ticks of the frame loop of a render thread since the module
    /// started. A tick renders a frame only when something changed. Ticks
    /// of the frame loop of the page are not counted.
    pub ticks: u64,
    /// The calls the runtime carried from the render thread to the main
    /// thread of the page during those ticks (a call of the C library that
    /// only the main thread can serve: a line on the console, a file);
    /// `None` when the script of the thread does not count them. While such
    /// a call is served the render thread waits for the thread of the page.
    pub tick_proxied_calls: Option<u64>,
    /// The index, in the table of the script of the module, of the function
    /// of the last such call; -1 when there was none.
    pub last_proxied_function: i64,
}

impl RenderStatistics {
    /// The statistics as they are now. Any thread may ask.
    pub fn current() -> Self {
        COUNTERS.read()
    }

    /// A frame was drawn to a canvas by the calling thread, `thread` as
    /// [`current_thread`](crate::interop::thread_proxy::current_thread)
    /// names it.
    pub(crate) fn frame_presented(thread: usize, kind: i32, gl_major_version: i32, width: i32, height: i32) {
        COUNTERS.frame_presented(thread, kind, gl_major_version, width, height);
    }

    /// A tick of the frame loop of a render thread ended; see
    /// [`tick_proxied_calls`](Self::tick_proxied_calls) and
    /// [`last_proxied_function`](Self::last_proxied_function) for the two
    /// arguments.
    pub(crate) fn tick_ended(proxied_calls: Option<u64>, last_proxied_function: Option<i64>) {
        COUNTERS.tick_ended(proxied_calls, last_proxied_function);
    }
}

struct Counters {
    frames: AtomicU64,
    frame_thread: AtomicUsize,
    frame_kind: AtomicI32,
    frame_gl_major_version: AtomicI32,
    frame_width: AtomicI32,
    frame_height: AtomicI32,
    ticks: AtomicU64,
    tick_proxied_calls: AtomicU64,
    /// Whether every tick so far came with a count of its proxied calls.
    proxied_calls_counted: AtomicBool,
    last_proxied_function: AtomicI64,
}

static COUNTERS: Counters = Counters::new();

impl Counters {
    const fn new() -> Self {
        Self {
            frames: AtomicU64::new(0),
            frame_thread: AtomicUsize::new(0),
            frame_kind: AtomicI32::new(0),
            frame_gl_major_version: AtomicI32::new(0),
            frame_width: AtomicI32::new(0),
            frame_height: AtomicI32::new(0),
            ticks: AtomicU64::new(0),
            tick_proxied_calls: AtomicU64::new(0),
            proxied_calls_counted: AtomicBool::new(true),
            last_proxied_function: AtomicI64::new(-1),
        }
    }

    fn frame_presented(&self, thread: usize, kind: i32, gl_major_version: i32, width: i32, height: i32) {
        self.frame_thread.store(thread, Ordering::SeqCst);
        self.frame_kind.store(kind, Ordering::SeqCst);
        self.frame_gl_major_version.store(gl_major_version, Ordering::SeqCst);
        self.frame_width.store(width, Ordering::SeqCst);
        self.frame_height.store(height, Ordering::SeqCst);
        // Last: a reader that sees the new count sees the frame it counts.
        self.frames.fetch_add(1, Ordering::SeqCst);
    }

    fn tick_ended(&self, proxied_calls: Option<u64>, last_proxied_function: Option<i64>) {
        match proxied_calls {
            Some(proxied_calls) => {
                self.tick_proxied_calls.fetch_add(proxied_calls, Ordering::SeqCst);
            }
            None => self.proxied_calls_counted.store(false, Ordering::SeqCst),
        }
        if let Some(function) = last_proxied_function {
            self.last_proxied_function.store(function, Ordering::SeqCst);
        }
        self.ticks.fetch_add(1, Ordering::SeqCst);
    }

    fn read(&self) -> RenderStatistics {
        let ticks = self.ticks.load(Ordering::SeqCst);
        let counted = ticks > 0 && self.proxied_calls_counted.load(Ordering::SeqCst);
        RenderStatistics {
            frames: self.frames.load(Ordering::SeqCst),
            frame_thread: self.frame_thread.load(Ordering::SeqCst),
            frame_kind: self.frame_kind.load(Ordering::SeqCst),
            frame_gl_major_version: self.frame_gl_major_version.load(Ordering::SeqCst),
            frame_width: self.frame_width.load(Ordering::SeqCst),
            frame_height: self.frame_height.load(Ordering::SeqCst),
            ticks,
            tick_proxied_calls: counted.then(|| self.tick_proxied_calls.load(Ordering::SeqCst)),
            last_proxied_function: self.last_proxied_function.load(Ordering::SeqCst),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::canvas_helper::{RENDER_TARGET_KIND_SOFTWARE, RENDER_TARGET_KIND_WEB_GL};

    #[test]
    fn nothing_is_known_before_the_first_frame() {
        let counters = Counters::new();

        assert_eq!(RenderStatistics { last_proxied_function: -1, ..Default::default() }, counters.read());
    }

    #[test]
    fn the_last_frame_is_described_and_every_frame_is_counted() {
        let counters = Counters::new();
        counters.frame_presented(4096, RENDER_TARGET_KIND_WEB_GL, 3, 460, 520);
        // Drawn by another thread, as the render thread does.
        std::thread::scope(|scope| {
            scope.spawn(|| counters.frame_presented(8192, RENDER_TARGET_KIND_SOFTWARE, 0, 300, 180));
        });

        let statistics = counters.read();
        assert_eq!(2, statistics.frames);
        assert_eq!(8192, statistics.frame_thread);
        assert_eq!(RENDER_TARGET_KIND_SOFTWARE, statistics.frame_kind);
        assert_eq!(0, statistics.frame_gl_major_version);
        assert_eq!((300, 180), (statistics.frame_width, statistics.frame_height));
        assert_eq!(0, statistics.ticks);
        assert_eq!(None, statistics.tick_proxied_calls);
    }

    #[test]
    fn the_proxied_calls_of_the_ticks_are_added_up() {
        let counters = Counters::new();
        counters.tick_ended(Some(0), None);
        counters.tick_ended(Some(2), Some(17));
        counters.tick_ended(Some(0), None);

        let statistics = counters.read();
        assert_eq!(3, statistics.ticks);
        assert_eq!(Some(2), statistics.tick_proxied_calls);
        assert_eq!(17, statistics.last_proxied_function);
    }

    #[test]
    fn a_tick_that_was_not_counted_makes_the_sum_unknown() {
        let counters = Counters::new();
        counters.tick_ended(Some(1), Some(3));
        counters.tick_ended(None, None);
        counters.tick_ended(Some(1), None);

        let statistics = counters.read();
        assert_eq!(3, statistics.ticks);
        assert_eq!(None, statistics.tick_proxied_calls);
        assert_eq!(3, statistics.last_proxied_function);
    }
}
