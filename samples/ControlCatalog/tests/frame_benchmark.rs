//! Frame benchmarks of catalog pages: the pages render through the
//! compositing renderer into a raster Skia framebuffer, frame by frame, as
//! the platform backends drive them (input, layout, the commit of the
//! compositor and the rendering of the frame).
//!
//! Not ports: the upstream sample has no tests. The benchmarks are ignored
//! by default (the test that is not checks what they rely on); run them in
//! an optimised build:
//!
//! ```sh
//! cargo test -p control-catalog --release --lib frame_benchmark -- --ignored --nocapture --test-threads=1
//! ```
//!
//! # Both modes of the compositor
//!
//! `frame_benchmark_table_view_scrolling_render_thread` runs one scenario
//! (the table scrolled by 20 pixels a frame) twice, each time in an
//! application of its own: in the dispatcher-thread mode, where the UI
//! thread renders every frame, and in the render-thread mode
//! (`Compositor::with_render_thread`, synchronous commits on the UI thread,
//! as the macOS platform creates its compositor), where a thread of the
//! benchmark renders under the compositor lock. Alone:
//!
//! ```sh
//! cargo test -p control-catalog --release --lib frame_benchmark_table_view_scrolling_render_thread -- --ignored --nocapture --test-threads=1
//! ```
//!
//! The pacing is the one of the other benchmarks: no frame waits for a
//! display. A frame starts when the one before it is complete, and the
//! render thread ticks as soon as the compositor wakes its loop (a batch
//! was committed), not at a fixed cadence; only the ticks the server
//! compositor asks for without a commit come at 60 a second. The times are
//! therefore the cost of a frame, not what a display shows: at a fixed
//! cadence the completion of a frame would also hold the wait for the next
//! tick, which says nothing about either mode.
//!
//! What is printed per mode, in milliseconds per frame:
//!
//! - **UI thread**: the time the UI thread is busy from the input (the
//!   offset is set) until it has nothing left to do for the frame: the
//!   input, layout, the recording of the render data, the commit and the
//!   jobs the frame posts back. In the dispatcher-thread mode it holds the
//!   rendering of the frame; in the render-thread mode it does not, and the
//!   time the UI thread waits for the render thread without work is not
//!   counted. It is the time during which the UI thread cannot take the
//!   next input.
//! - **frame completion**: from the input until the batch that holds its
//!   changes has been rendered (the `rendered` completion of the batch,
//!   stamped by the thread that renders). It is the latency of the input.
//!   In the render-thread mode it holds the hand-over to the other thread.
//! - **render thread** (render-thread mode only): the time inside the ticks
//!   of the frame.
//!
//! The last line compares the medians. The render-thread mode is not
//! expected to make a frame cheaper (the UI thread and the render thread
//! together do the work the UI thread does alone in the other mode); what
//! it is for is that the UI thread is free again sooner. Compare runs taken
//! on a quiet machine, in the same build, and take the comparison rather
//! than the absolute numbers: the surface is a raster framebuffer, not the
//! GPU surface of a window.

use super::support::*;
use crate::pages::{ButtonsPage, TableViewPage};
use ferroui_base::input::raw::{RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{IInputDevice, IInputRoot, MouseDevice, Pointer, PointerType, RawInputModifiers};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
    IPlatformRenderSurfaceRenderTarget,
};
use ferroui_base::media::MediaContext;
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormats, RenderTargetSceneInfo, RetainedFramebuffer};
use ferroui_base::rendering::composition::transport::CompositionBatch;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::{IRenderLoop, IRenderLoopTask};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, PixelSize, Point, Ref, Vector, Visual};
use ferroui_controls::testing::{CompositorTestServices, MockWindowImpl, MockWindowingPlatform};
use ferroui_controls::{Control, ScrollViewer, TableView, Window};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

const WIDTH: f64 = 1280.0;
const HEIGHT: f64 = 800.0;

/// A window surface rendered to in memory, as the software surface of the
/// browser is.
struct RasterSurface {
    frames: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl RasterSurface {
    fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self { frames: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)) })
    }
}

impl IPlatformRenderSurface for RasterSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for RasterSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        Rc::new(RasterTarget { framebuffer: RefCell::new(None), frames: self.frames.clone() })
    }
}

/// The render target of a [`RasterSurface`]. It is an object of the server
/// side: created by the frame that first renders the window and used by the
/// frames after it, on either thread, always under the compositor lock. The
/// cell and the framebuffer are bound to that lock, not to a thread; what
/// the target shares with the bench is the atomic count of frames.
struct RasterTarget {
    framebuffer: RefCell<Option<Rc<RetainedFramebuffer>>>,
    frames: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl IPlatformRenderSurfaceRenderTarget for RasterTarget {}

impl IFramebufferRenderTarget for RasterTarget {
    fn lock(&self, _scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        let size = PixelSize::new(WIDTH as i32, HEIGHT as i32);
        let mut framebuffer = self.framebuffer.borrow_mut();
        let framebuffer =
            framebuffer.get_or_insert_with(|| RetainedFramebuffer::new(size, PixelFormats::RGBA8888, AlphaFormat::Premul));
        let frames = self.frames.clone();
        let locked = framebuffer.lock(Vector::new(96.0, 96.0), move |_| {
            frames.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        (locked, FramebufferLockProperties::default())
    }

    fn dispose(&self) {
        if let Some(framebuffer) = self.framebuffer.borrow_mut().take() {
            framebuffer.dispose();
        }
    }
}

/// Which thread renders the frames of a bench.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RenderMode {
    /// The UI thread renders: the compositor of the test services.
    DispatcherThread,
    /// A thread of the bench renders, under the compositor lock.
    RenderThread,
}

impl RenderMode {
    fn label(self) -> &'static str {
        match self {
            RenderMode::DispatcherThread => "dispatcher-thread mode",
            RenderMode::RenderThread => "render-thread mode",
        }
    }
}

/// The interval of the ticks the server compositor asks for without a
/// commit (a tick of a display at 60 Hz).
const TICK_INTERVAL: Duration = Duration::from_micros(16_667);

/// How long a frame may take before the bench gives up on it.
const FRAME_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Default)]
struct RenderLoopState {
    /// The loop was woken and has not ticked since.
    requested: bool,
    /// A tick is running.
    ticking: bool,
    /// A task of the last tick asked for the next one.
    wants_next_tick: bool,
    stopped: bool,
    /// The time spent inside the ticks so far.
    tick_time: Duration,
}

/// A render loop ticked by a thread of its own, when it is woken: the
/// compositor wakes it for every batch it commits. It reports that it runs
/// in the background.
#[derive(Default)]
struct WokenRenderLoop {
    tasks: Mutex<Vec<Arc<dyn IRenderLoopTask>>>,
    state: Mutex<RenderLoopState>,
    changed: Condvar,
}

impl WokenRenderLoop {
    fn state(&self) -> MutexGuard<'_, RenderLoopState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The body of the render thread: ticks when the loop is woken, and
    /// after [`TICK_INTERVAL`] when the last tick asked for another.
    fn run(&self) {
        let mut state = self.state();
        loop {
            while !state.stopped && !state.requested {
                let (next, result) =
                    self.changed.wait_timeout(state, TICK_INTERVAL).unwrap_or_else(|e| e.into_inner());
                state = next;
                if result.timed_out() && state.wants_next_tick {
                    break;
                }
            }
            if state.stopped {
                return;
            }
            state.requested = false;
            state.ticking = true;
            drop(state);

            let start = Instant::now();
            let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let mut wants_next_tick = false;
            for task in tasks {
                wants_next_tick |= task.render();
            }
            let elapsed = start.elapsed();

            state = self.state();
            state.ticking = false;
            state.wants_next_tick = wants_next_tick;
            state.tick_time += elapsed;
            self.changed.notify_all();
        }
    }

    /// Whether no tick is running and none was asked for by a commit.
    fn is_idle(&self) -> bool {
        let state = self.state();
        !state.requested && !state.ticking
    }

    fn tick_time(&self) -> Duration {
        self.state().tick_time
    }

    /// Waits until a tick ends, for `timeout` at most.
    fn wait_for_progress(&self, timeout: Duration) {
        let state = self.state();
        if state.requested || state.ticking {
            let _ = self.changed.wait_timeout(state, timeout);
        } else {
            drop(state);
            thread::yield_now();
        }
    }

    fn stop(&self) {
        self.state().stopped = true;
        self.changed.notify_all();
    }
}

impl IRenderLoop for WokenRenderLoop {
    fn add(&self, i: Arc<dyn IRenderLoopTask>) {
        self.tasks.lock().unwrap_or_else(|e| e.into_inner()).push(i);
    }

    fn remove(&self, i: &Arc<dyn IRenderLoopTask>) {
        self.tasks.lock().unwrap_or_else(|e| e.into_inner()).retain(|t| !Arc::ptr_eq(t, i));
    }

    fn runs_in_background(&self) -> bool {
        true
    }

    fn wakeup(&self) {
        self.state().requested = true;
        self.changed.notify_all();
    }
}

/// The compositor of a bench in the render-thread mode, and the thread that
/// ticks its render loop.
struct RenderThread {
    compositor: Rc<Compositor>,
    render_loop: Arc<WokenRenderLoop>,
    handle: RefCell<Option<thread::JoinHandle<()>>>,
}

impl RenderThread {
    /// Creates the compositor as the macOS platform does in this mode: the
    /// thread that ticks renders, and the UI thread renders the synchronous
    /// commits itself. The thread is not running yet.
    fn new() -> RenderThread {
        let render_loop = Arc::new(WokenRenderLoop::default());
        let compositor = Compositor::with_render_thread(
            render_loop.clone(),
            None,
            true,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );
        assert!(compositor.renders_on_render_thread());
        RenderThread { compositor, render_loop, handle: RefCell::new(None) }
    }

    fn start(&self) {
        let render_loop = self.render_loop.clone();
        *self.handle.borrow_mut() = Some(thread::spawn(move || render_loop.run()));
    }

    fn stop(&self) {
        self.render_loop.stop();
        if let Some(handle) = self.handle.borrow_mut().take() {
            let _ = handle.join();
        }
    }
}

/// A shown window that renders through the compositor into a raster
/// surface, with the input of a mouse.
struct Bench {
    // Declared before the services: its compositor is dropped before the
    // application scope, as the compositor of the services is.
    render_thread: Option<RenderThread>,
    services: CompositorTestServices,
    window_impl: Rc<MockWindowImpl>,
    window: Ref<Window>,
    surface: std::sync::Arc<RasterSurface>,
    mouse: Rc<MouseDevice>,
    timestamp: Cell<u64>,
}

impl Bench {
    fn start(content: impl FnOnce() -> Ref<Control>) -> Bench {
        Bench::start_in(RenderMode::DispatcherThread, content)
    }

    fn start_in(mode: RenderMode, content: impl FnOnce() -> Ref<Control>) -> Bench {
        let services = start_catalog_compositor_application();
        let window_impl = MockWindowingPlatform::create_window_mock_with_size(WIDTH, HEIGHT);
        let render_thread = match mode {
            RenderMode::DispatcherThread => {
                services.setup(&window_impl);
                None
            }
            RenderMode::RenderThread => {
                // The window renders through a compositor of the bench
                // instead of the one of the services, which stays unused.
                let render_thread = RenderThread::new();
                window_impl.setup_compositor(Some(render_thread.compositor.clone()));
                Some(render_thread)
            }
        };
        let surface = RasterSurface::new();
        window_impl.setup_surfaces(vec![surface.clone() as std::sync::Arc<dyn IPlatformRenderSurface>]);
        let window = Window::with_impl(window_impl.clone());
        window.set_width(WIDTH);
        window.set_height(HEIGHT);
        window.set_content(Some(Control::boxed(&content())));
        window.show();
        // The first frame is a frame of the UI thread in either mode (the
        // show is a synchronous commit); the render thread renders the ones
        // after it.
        if let Some(render_thread) = &render_thread {
            render_thread.start();
        }
        let bench = Bench {
            render_thread,
            services,
            window_impl,
            window,
            surface,
            mouse: MouseDevice::with_pointer(Pointer::new(0, PointerType::Mouse, true)),
            timestamp: Cell::new(0),
        };
        for _ in 0..3 {
            bench.frame();
        }
        bench
    }

    /// Runs the jobs of the dispatcher, renders a frame and runs the jobs it
    /// posted. Returns the time of the jobs and of the frame.
    ///
    /// In the render-thread mode the frame is rendered by the render thread:
    /// this thread waits until that thread has nothing left to render.
    fn frame(&self) -> (Duration, Duration) {
        let start = Instant::now();
        Dispatcher::ui_thread().run_jobs(None);
        let jobs = start.elapsed();
        let start = Instant::now();
        match &self.render_thread {
            None => {
                self.services.render_loop().tick();
                Dispatcher::ui_thread().run_jobs(None);
            }
            Some(render_thread) => {
                self.pump(render_thread, None);
            }
        }
        (jobs, start.elapsed())
    }

    /// The compositor the window renders through.
    fn compositor(&self) -> &Rc<Compositor> {
        match &self.render_thread {
            None => self.services.compositor(),
            Some(render_thread) => &render_thread.compositor,
        }
    }

    /// Runs the jobs of the dispatcher until `batch` has been rendered (when
    /// there is one), the render thread has nothing left to render and the
    /// jobs its frames posted have run. Returns the time spent running jobs:
    /// the time this thread waits without work is not part of it.
    fn pump(&self, render_thread: &RenderThread, batch: Option<&CompositionBatch>) -> Duration {
        let deadline = Instant::now() + FRAME_TIMEOUT;
        let mut busy = Duration::ZERO;
        let mut settled = false;
        loop {
            let start = Instant::now();
            Dispatcher::ui_thread().run_jobs(None);
            busy += start.elapsed();
            let rendered = batch.map_or(true, |batch| batch.rendered().is_completed());
            if rendered && render_thread.render_loop.is_idle() {
                if settled {
                    return busy;
                }
                // The frame may have posted jobs, and they may commit again:
                // run them and look once more.
                settled = true;
                continue;
            }
            settled = false;
            assert!(Instant::now() < deadline, "the render thread did not complete the frame");
            render_thread.render_loop.wait_for_progress(Duration::from_millis(1));
        }
    }

    fn root(&self) -> Rc<dyn IInputRoot> {
        self.window_impl.input_root().expect("the input root of the window")
    }

    fn next_timestamp(&self) -> u64 {
        self.timestamp.set(self.timestamp.get() + 16);
        self.timestamp.get()
    }

    fn device(&self) -> Rc<dyn IInputDevice> {
        self.mouse.clone()
    }

    fn input(&self, args: Rc<dyn ferroui_base::input::raw::IRawInputEventArgs>) {
        let input = ferroui_controls::platform::ITopLevelImpl::input(&*self.window_impl).expect("the window handles input");
        input(args);
    }

    fn pointer_move(&self, position: Point) {
        self.input(Rc::new(RawPointerEventArgs::new(
            self.device(),
            self.next_timestamp(),
            self.root(),
            RawPointerEventType::Move,
            position,
            RawInputModifiers::NONE,
        )));
    }

    fn wheel(&self, position: Point, delta: Vector) {
        self.input(Rc::new(RawMouseWheelEventArgs::new(
            self.device(),
            self.next_timestamp(),
            self.root(),
            position,
            delta,
            RawInputModifiers::NONE,
        )));
    }

    /// Runs `frames` frames, each after `before_frame(frame)`, and prints the
    /// times.
    fn measure(&self, name: &str, frames: usize, before_frame: impl Fn(usize)) -> Stats {
        let rendered = self.surface.frames.load(std::sync::atomic::Ordering::SeqCst);
        let mut totals = Vec::with_capacity(frames);
        let mut jobs = Vec::with_capacity(frames);
        let mut renders = Vec::with_capacity(frames);
        for frame in 0..frames {
            let start = Instant::now();
            before_frame(frame);
            let input = start.elapsed();
            let (job, render) = self.frame();
            totals.push(input + job + render);
            jobs.push(input + job);
            renders.push(render);
        }
        let stats = Stats::of(&totals);
        println!(
            "{name}: {frames} frames, {} rendered; ms per frame: median {:.3}, mean {:.3}, p95 {:.3}, max {:.3} \
             (input and layout median {:.3}, render median {:.3})",
            self.surface.frames.load(std::sync::atomic::Ordering::SeqCst) - rendered,
            stats.median,
            stats.mean,
            stats.p95,
            stats.max,
            Stats::of(&jobs).median,
            Stats::of(&renders).median,
        );
        stats
    }

    /// Runs `frames` frames, each after `before_frame(frame)`, in the mode
    /// of the bench, and returns the times of each (see the header of the
    /// file for what they are). A frame starts when the one before it is
    /// complete.
    fn run_frames(&self, frames: usize, before_frame: impl Fn(usize)) -> FrameTimes {
        let count = || self.surface.frames.load(std::sync::atomic::Ordering::SeqCst);
        let rendered_before = count();
        let mut times = FrameTimes {
            mode: if self.render_thread.is_some() { RenderMode::RenderThread } else { RenderMode::DispatcherThread },
            frames,
            rendered: 0,
            frames_drawn: 0,
            ui: Vec::with_capacity(frames),
            commits: Vec::with_capacity(frames),
            completions: Vec::with_capacity(frames),
            renders: Vec::with_capacity(frames),
        };
        for frame in 0..frames {
            let drawn_before = count();
            let tick_time_before = self.render_thread.as_ref().map(|render_thread| render_thread.render_loop.tick_time());

            let start = Instant::now();
            before_frame(frame);
            let input = start.elapsed();

            // The batch the changes of the input are committed in. Its
            // completion is stamped by the thread that renders it.
            let batch = self.compositor().request_commit_async();
            let rendered_at = Arc::new(Mutex::new(None::<Instant>));
            let slot = rendered_at.clone();
            batch.rendered().on_completed(move || {
                *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
            });

            match &self.render_thread {
                None => {
                    let (mut jobs, mut render) = self.frame();
                    // A commit that was requested while the batch before it was
                    // still pending is made one pass of the dispatcher later: the
                    // frame is pumped until the batch of the input is rendered.
                    for _ in 0..8 {
                        if rendered_at.lock().unwrap_or_else(|e| e.into_inner()).is_some() {
                            break;
                        }
                        let (more_jobs, more_render) = self.frame();
                        jobs += more_jobs;
                        render += more_render;
                    }
                    times.ui.push(input + jobs + render);
                    times.commits.push(input + jobs);
                    times.renders.push(render);
                }
                Some(render_thread) => {
                    // Layout, the recording of the render data and the
                    // commit; then whatever the frame posts back.
                    let commit = Instant::now();
                    Dispatcher::ui_thread().run_jobs(None);
                    let jobs = commit.elapsed();
                    let rest = self.pump(render_thread, Some(&*batch));
                    times.ui.push(input + jobs + rest);
                    times.commits.push(input + jobs);
                    let tick_time = render_thread.render_loop.tick_time();
                    times.renders.push(tick_time.saturating_sub(tick_time_before.unwrap_or(tick_time)));
                }
            }

            let rendered_at: Option<Instant> = *rendered_at.lock().unwrap_or_else(|e| e.into_inner());
            let Some(rendered_at) = rendered_at else { panic!("the batch of frame {frame} was not rendered") };
            times.completions.push(rendered_at.saturating_duration_since(start));
            if count() > drawn_before {
                times.frames_drawn += 1;
            }
        }
        times.rendered = count() - rendered_before;
        times
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        // The render thread ends first: the window is closed by frames of
        // this thread.
        if let Some(render_thread) = &self.render_thread {
            render_thread.stop();
        }
        self.window.close();
    }
}

/// The times of the frames of [`Bench::run_frames`].
struct FrameTimes {
    mode: RenderMode,
    frames: usize,
    /// The frames the surface received.
    rendered: u32,
    /// The frames of the scenario during which the surface received a
    /// frame.
    frames_drawn: usize,
    /// The time the UI thread was busy, per frame.
    ui: Vec<Duration>,
    /// The part of it up to the commit: the input, layout and the commit.
    commits: Vec<Duration>,
    /// From the input until its batch was rendered, per frame.
    completions: Vec<Duration>,
    /// The time of the rendering: in the dispatcher-thread mode the tick on
    /// the UI thread with the jobs it posted (a part of `ui`); in the
    /// render-thread mode the time inside the ticks of the render thread.
    renders: Vec<Duration>,
}

impl FrameTimes {
    /// Prints the times and returns the statistics of the UI thread.
    fn print(&self, name: &str) -> Stats {
        let mode = self.mode.label();
        let line = |stats: &Stats| {
            format!(
                "ms per frame: median {:.3}, mean {:.3}, p95 {:.3}, max {:.3}",
                stats.median, stats.mean, stats.p95, stats.max
            )
        };
        let ui = Stats::of(&self.ui);
        println!("{name} [{mode}]: {} frames, {} rendered", self.frames, self.rendered);
        match self.mode {
            RenderMode::DispatcherThread => println!(
                "{name} [{mode}] UI thread: {} (input, layout and commit median {:.3}, render median {:.3})",
                line(&ui),
                Stats::of(&self.commits).median,
                Stats::of(&self.renders).median,
            ),
            RenderMode::RenderThread => println!(
                "{name} [{mode}] UI thread: {} (input, layout and commit median {:.3})",
                line(&ui),
                Stats::of(&self.commits).median,
            ),
        }
        println!("{name} [{mode}] frame completion: {}", line(&Stats::of(&self.completions)));
        if self.mode == RenderMode::RenderThread {
            println!("{name} [{mode}] render thread: {}", line(&Stats::of(&self.renders)));
        }
        ui
    }
}

struct Stats {
    median: f64,
    mean: f64,
    p95: f64,
    max: f64,
}

impl Stats {
    fn of(times: &[Duration]) -> Stats {
        let mut ms: Vec<f64> = times.iter().map(|time| time.as_secs_f64() * 1000.0).collect();
        ms.sort_by(f64::total_cmp);
        let at = |q: f64| ms[((ms.len() - 1) as f64 * q).round() as usize];
        Stats { median: at(0.5), mean: ms.iter().sum::<f64>() / ms.len() as f64, p95: at(0.95), max: at(1.0) }
    }
}

fn descendants<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter_map(|visual| visual.cast::<T>()).collect()
}

/// The scroll viewer of a control and the point in the middle of its
/// viewport, in the coordinates of the window.
fn scroll_viewer_of(bench: &Bench, root: &Visual) -> (Ref<ScrollViewer>, Point) {
    let scroll_viewer = descendants::<ScrollViewer>(root).into_iter().next().expect("a scroll viewer");
    let bounds = scroll_viewer.bounds();
    let origin = scroll_viewer.translate_point(Point::new(0.0, 0.0), &bench.window).expect("in the window");
    (scroll_viewer, Point::new(origin.x + bounds.width / 2.0, origin.y + bounds.height / 2.0))
}

#[test]
#[ignore = "benchmark: run in an optimised build with --ignored --nocapture"]
fn frame_benchmark_table_view_scrolling() {
    let bench = Bench::start(|| TableViewPage::new().upcast());
    let table_view = descendants::<TableView>(&bench.window).into_iter().next().expect("the table view");
    let (scroll_viewer, center) = scroll_viewer_of(&bench, &table_view);
    bench.pointer_move(center);
    bench.frame();

    // Wheel scrolling by 20 pixels a frame, down then up.
    bench.measure("table view, wheel scrolling 20 px per frame", 300, |frame| {
        let delta = if (frame / 100) % 2 == 0 { -0.4 } else { 0.4 };
        bench.wheel(center, Vector::new(0.0, delta));
    });
    assert!(scroll_viewer.offset().y >= 0.0);

    // Scrolling by setting the offset, as a scroll bar drag does.
    bench.measure("table view, offset scrolling 20 px per frame", 300, |frame| {
        let y = (frame % 100) as f64 * 20.0;
        scroll_viewer.set_offset(Vector::new(0.0, y));
    });

    bench.measure("table view, idle", 100, |_| {});
}

/// The frames of the scenario of both modes, and the frames of one sweep of
/// the table down or up.
const SCROLL_FRAMES: usize = 300;
const SCROLL_SWEEP: usize = 40;

/// The offset of the table at a frame: 20 pixels further each frame, down
/// for a sweep and up for the next, so that every frame changes the offset.
fn scroll_offset_at(frame: usize) -> f64 {
    let position = (frame + 1) % (2 * SCROLL_SWEEP);
    let step = if position <= SCROLL_SWEEP { position } else { 2 * SCROLL_SWEEP - position };
    step as f64 * 20.0
}

/// Scrolls the table of the table view page by setting the offset, as a
/// scroll bar drag does, in an application of its own, and prints the times.
fn table_view_offset_scrolling(mode: RenderMode) -> (FrameTimes, Stats) {
    let bench = Bench::start_in(mode, || TableViewPage::new().upcast());
    let table_view = descendants::<TableView>(&bench.window).into_iter().next().expect("the table view");
    let (scroll_viewer, center) = scroll_viewer_of(&bench, &table_view);
    bench.pointer_move(center);
    bench.frame();

    let scroll = |frame: usize| {
        let y = scroll_offset_at(frame);
        assert_ne!(y, scroll_viewer.offset().y, "frame {frame} scrolls");
        scroll_viewer.set_offset(Vector::new(0.0, y));
    };
    // A sweep down and up that is not measured: the rows are realized and
    // the caches of both threads are filled. It ends where it started.
    bench.run_frames(2 * SCROLL_SWEEP, &scroll);
    assert_eq!(0.0, scroll_viewer.offset().y);

    let times = bench.run_frames(SCROLL_FRAMES, &scroll);
    assert_eq!(scroll_offset_at(SCROLL_FRAMES - 1), scroll_viewer.offset().y);
    let ui = times.print("table view, offset scrolling 20 px per frame");
    (times, ui)
}

#[test]
#[ignore = "benchmark: run in an optimised build with --ignored --nocapture"]
fn frame_benchmark_table_view_scrolling_render_thread() {
    // The dispatcher-thread mode: each frame of the scenario is rendered by
    // a tick of the frame (a step whose commit waits for the batch before
    // it takes a second tick, which draws as well).
    let (dispatcher_thread, dispatcher_thread_ui) = table_view_offset_scrolling(RenderMode::DispatcherThread);
    assert!(dispatcher_thread.rendered >= SCROLL_FRAMES as u32);
    assert_eq!(SCROLL_FRAMES, dispatcher_thread.frames_drawn);

    // The render-thread mode: every scroll step reached the surface before
    // the next one was made, so no frame is lost. The render thread may
    // render more frames than there are steps (what a frame posts back to
    // the UI thread is committed, and rendered, on its own), never fewer.
    let (render_thread, render_thread_ui) = table_view_offset_scrolling(RenderMode::RenderThread);
    assert_eq!(SCROLL_FRAMES, render_thread.frames_drawn);
    assert!(
        render_thread.rendered >= SCROLL_FRAMES as u32,
        "{} frames rendered for {SCROLL_FRAMES} scroll steps",
        render_thread.rendered
    );

    println!(
        "table view, offset scrolling 20 px per frame: UI thread median {:.3} ms in the {}, {:.3} ms in the {} \
         ({:.0} % of it; the render thread takes {:.3} ms); frame completion median {:.3} ms and {:.3} ms",
        dispatcher_thread_ui.median,
        dispatcher_thread.mode.label(),
        render_thread_ui.median,
        render_thread.mode.label(),
        render_thread_ui.median / dispatcher_thread_ui.median * 100.0,
        Stats::of(&render_thread.renders).median,
        Stats::of(&dispatcher_thread.completions).median,
        Stats::of(&render_thread.completions).median,
    );
}

#[test]
#[ignore = "benchmark: run in an optimised build with --ignored --nocapture"]
fn frame_benchmark_buttons_page() {
    let bench = Bench::start(|| ButtonsPage::new().upcast());
    bench.measure("buttons page, idle", 100, |_| {});

    // The pointer moves over the buttons of the page, 10 pixels a frame.
    let buttons = descendants::<ferroui_controls::Button>(&bench.window);
    let points: Vec<Point> = buttons
        .iter()
        .filter_map(|button| {
            let bounds = button.bounds();
            let origin = button.translate_point(Point::new(0.0, 0.0), &bench.window)?;
            (origin.y > 0.0 && origin.y < HEIGHT).then(|| Point::new(origin.x + bounds.width / 2.0, origin.y + bounds.height / 2.0))
        })
        .collect();
    assert!(!points.is_empty());
    bench.measure("buttons page, pointer moving across buttons", 300, |frame| {
        bench.pointer_move(points[frame % points.len()]);
    });
    bench.measure("buttons page, pointer moving 1 px inside a button", 100, |frame| {
        let point = points[0];
        bench.pointer_move(Point::new(point.x + (frame % 2) as f64, point.y));
    });

    let (_, center) = scroll_viewer_of(&bench, &bench.window);
    bench.pointer_move(center);
    bench.frame();
    bench.measure("buttons page, wheel scrolling 20 px per frame", 300, |frame| {
        let delta = if (frame / 50) % 2 == 0 { -0.4 } else { 0.4 };
        bench.wheel(center, Vector::new(0.0, delta));
    });
}

#[test]
fn table_view_wheel_scrolling_renders_every_frame_and_reuses_its_rows() {
    let bench = Bench::start(|| TableViewPage::new().upcast());
    let table_view = descendants::<TableView>(&bench.window).into_iter().next().expect("the table view");
    let (scroll_viewer, center) = scroll_viewer_of(&bench, &table_view);
    bench.pointer_move(center);
    bench.frame();
    assert!(table_view.is_pointer_over());

    let realized = table_view.get_realized_containers().len();
    assert!(realized > 0);
    let mut rows: Vec<*const Control> = Vec::new();
    let rendered = bench.surface.frames.load(std::sync::atomic::Ordering::SeqCst);
    for _ in 0..40 {
        bench.wheel(center, Vector::new(0.0, -0.4));
        bench.frame();
        for row in table_view.get_realized_containers() {
            let row: *const Control = &*row;
            if !rows.contains(&row) {
                rows.push(row);
            }
        }
    }

    // 40 frames of 20 pixels: the table scrolled by 800 pixels, each frame
    // was rendered, and the rows that left the viewport were recycled into
    // the rows that entered it instead of new rows being created.
    assert_eq!(800.0, scroll_viewer.offset().y);
    assert_eq!(40, bench.surface.frames.load(std::sync::atomic::Ordering::SeqCst) - rendered);
    assert!(rows.len() <= realized + 2, "{} rows for {realized} realized", rows.len());
}

/// The rounds of the test below.
const UNPACED_ROUNDS: usize = 400;

/// The render-thread mode under changes the UI thread does not pace: the
/// benchmarks above wait for each frame before they make the next change,
/// which a user does not. Here the UI thread moves the pointer, scrolls,
/// resizes the window and replaces its content while the render thread is
/// rendering, commits without waiting, and renders a frame itself now and
/// then (what a resize or a show does on the desktop), so that the two
/// threads meet at the compositor lock in every order.
#[test]
fn the_render_thread_mode_takes_unpaced_changes_of_the_ui_thread() {
    let bench = Bench::start_in(RenderMode::RenderThread, || TableViewPage::new().upcast());
    let count = || bench.surface.frames.load(std::sync::atomic::Ordering::SeqCst);
    let rendered = count();
    let center = Point::new(WIDTH / 2.0, HEIGHT / 2.0);

    let mut batches = Vec::new();
    for round in 0..UNPACED_ROUNDS {
        bench.pointer_move(Point::new((round * 37 % WIDTH as usize) as f64, (round * 23 % HEIGHT as usize) as f64));
        let delta = if (round / 20) % 2 == 0 { -0.4 } else { 0.4 };
        bench.wheel(center, Vector::new(0.0, delta));
        if round % 10 == 5 {
            bench.window.set_width(WIDTH - (round % 7) as f64 * 20.0);
            bench.window.set_height(HEIGHT - (round % 5) as f64 * 20.0);
        }
        if round % 25 == 24 {
            let page: Ref<Control> =
                if (round / 25) % 2 == 0 { ButtonsPage::new().upcast::<Control>() } else { TableViewPage::new().upcast::<Control>() };
            bench.window.set_content(Some(Control::boxed(&page)));
        }
        if round % 3 == 0 {
            batches.push(bench.compositor().request_commit_async());
        }
        // Layout, the recording of the render data and the commit; the frame
        // is not waited for.
        Dispatcher::ui_thread().run_jobs(None);
        if round % 7 == 3 {
            bench.compositor().render_on_this_thread();
        }
    }

    // Everything that was committed is rendered once the threads settle.
    bench.frame();
    for (index, batch) in batches.iter().enumerate() {
        assert!(batch.rendered().is_completed(), "batch {index} was not rendered");
    }
    assert!(count() > rendered);

    // And the window still renders what changes after that.
    let before = count();
    bench.window.set_content(Some(Control::boxed(&ButtonsPage::new().upcast::<Control>())));
    bench.frame();
    assert!(count() > before);
}
