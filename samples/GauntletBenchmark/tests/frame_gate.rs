//! The frame gate: the gauntlet (tier 16) and the still wall (tier 20) in a headless window of
//! 1280 x 820 points at a scaling of 2, as the desktop program opens them, rendered through the
//! compositor into a raster Skia framebuffer of 2560 x 1640 pixels, with no pointer device.
//! It is the frame benchmark of the local performance gate (`scripts/perf/gate.py`); run it in an
//! optimised build, alone:
//!
//! ```sh
//! cargo test -p gauntlet-benchmark --release --test frame_gate -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `GATE_FRAMES` (240) and `GATE_WARMUP` (60) set the frames measured and the frames before them;
//! `PERF_FONTS` the folder of `Roboto-Regular.ttf` and `Roboto-Bold.ttf` (the Material theme's
//! assets of this checkout by default).
//!
//! The compositor is in the render-thread mode of the desktop (`Compositor::with_render_thread`,
//! synchronous commits on the UI thread): a thread of the test ticks the render loop whenever the
//! compositor wakes it, as the copy of the harness of `samples/ControlCatalog/tests/frame_benchmark.rs`
//! below does. The frames are the screens' own: each callback of the top level's animation frame
//! advances the frame index once and requests the next one, and the media context runs the next
//! render pass once the render thread has accepted the batch before it. Nothing follows wall
//! time: the global clock never ticks, and the screens read only the frame index. No frame waits
//! for a display, so the times are the cost of a frame.
//!
//! Each scenario prints a line for the gate,
//!
//! ```text
//! GATE {"bench":"gauntlet/16","frames":240,"warmup":60,"ui_ms_per_frame":12.3456,"render_ms_per_frame":7.8901,"wall_ms_per_frame":14.0000,"rendered":241}
//! ```
//!
//! with, per frame of the frame index: the time the UI thread spent running its jobs (the
//! animation frame callback, layout, the recording of the render data, the commit and the jobs
//! the frames post back; not the time it waited), the time the render thread spent in its ticks,
//! and the wall time of the run. `rendered` is the frames the surface received. The gate compares
//! the three times for a frame the surface received (`frames / rendered` of them), and a scenario
//! fails here when the surface receives fewer than nine in ten of its frames: every frame of the
//! scenarios changes the picture, and a frame of the frame index that commits nothing is a second
//! animation frame and a second layout for one picture (docs/porting/desktop-performance.md,
//! section 9.46).

use ferroui_base::animation::{IClock, IGlobalClock, PlayState, SignedDuration};
use ferroui_base::media::{FontManager, MediaContext};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
    IPlatformRenderSurfaceRenderTarget,
};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormats, RenderTargetSceneInfo, RetainedFramebuffer};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::{IRenderLoop, IRenderLoopTask};
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{IInputDevice, InputManager, MouseDevice, RawInputModifiers};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, Point, Ref, Vector};
use ferroui_controls::testing::{CompositorTestServices, MockWindowingPlatform, TestServices};
use ferroui_controls::{Application, Control, Window};
use ferroui_themes_fluent::FluentTheme;
use gauntlet_benchmark::{Clock, DeviceRoboto, GauntletView, WallView};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

/// The window of the desktop program (`app.rs`), in points, and the scaling of a display of high
/// density.
const WIDTH: f64 = 1280.0;
const HEIGHT: f64 = 820.0;
const SCALING: f64 = 2.0;

/// The interval of the ticks the server compositor asks for without a commit (60 Hz).
const TICK_INTERVAL: Duration = Duration::from_micros(16_667);

/// How long the frame index may stand still before the gate gives up.
const FRAME_TIMEOUT: Duration = Duration::from_secs(30);

fn variable(name: &str, fallback: i32) -> i32 {
    std::env::var(name).ok().and_then(|value| value.trim().parse().ok()).filter(|value| *value >= 0).unwrap_or(fallback)
}

// --- the surface ---------------------------------------------------------------------------------

/// A window surface rendered to in memory: a raster framebuffer of the window's pixels.
struct RasterSurface {
    frames: Arc<AtomicU32>,
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

/// The render target of a [`RasterSurface`]: an object of the server side, used on either thread
/// under the compositor lock; what it shares with the test is the atomic count of frames.
struct RasterTarget {
    framebuffer: RefCell<Option<Rc<RetainedFramebuffer>>>,
    frames: Arc<AtomicU32>,
}

impl IPlatformRenderSurfaceRenderTarget for RasterTarget {}

impl IFramebufferRenderTarget for RasterTarget {
    fn lock(&self, _scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        let size = PixelSize::new((WIDTH * SCALING) as i32, (HEIGHT * SCALING) as i32);
        let mut framebuffer = self.framebuffer.borrow_mut();
        let framebuffer =
            framebuffer.get_or_insert_with(|| RetainedFramebuffer::new(size, PixelFormats::RGBA8888, AlphaFormat::Premul));
        let frames = self.frames.clone();
        let locked = framebuffer.lock(Vector::new(96.0 * SCALING, 96.0 * SCALING), move |_| {
            frames.fetch_add(1, Ordering::SeqCst);
        });
        (locked, FramebufferLockProperties::default())
    }

    fn dispose(&self) {
        if let Some(framebuffer) = self.framebuffer.borrow_mut().take() {
            framebuffer.dispose();
        }
    }
}

// --- the render thread ---------------------------------------------------------------------------

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

/// A render loop ticked by a thread of its own when it is woken: the compositor wakes it for
/// every batch it commits. It reports that it runs in the background.
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

    /// The body of the render thread: ticks when the loop is woken, and after [`TICK_INTERVAL`]
    /// when the last tick asked for another.
    fn run(&self) {
        let mut state = self.state();
        loop {
            while !state.stopped && !state.requested {
                let (next, result) = self.changed.wait_timeout(state, TICK_INTERVAL).unwrap_or_else(|e| e.into_inner());
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

/// The global clock of the gate: animations subscribe to it, and it never ticks.
#[derive(Default)]
struct StillClock {
    subject: LightweightSubject<SignedDuration>,
    play_state: Cell<Option<PlayState>>,
}

impl IObservable<SignedDuration> for StillClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for StillClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get().unwrap_or(PlayState::Run)
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(Some(value))
    }
}

impl IGlobalClock for StillClock {}

// --- the gate ------------------------------------------------------------------------------------

/// A shown window with a screen, rendering on a thread of the gate.
struct Gate {
    input_impl: Rc<ferroui_controls::testing::MockWindowImpl>,
    pointer: Option<Point>,
    // Declared before the services: the compositor is dropped before the application scope.
    _compositor: Rc<Compositor>,
    render_loop: Arc<WokenRenderLoop>,
    render_thread: Option<thread::JoinHandle<()>>,
    _services: CompositorTestServices,
    window: Ref<Window>,
    frames: Arc<AtomicU32>,
}

/// What a run of frames measured.
struct Run {
    /// The frames the frame index advanced by.
    frames: u32,
    /// The frames the surface received.
    rendered: u32,
    ui: Duration,
    render: Duration,
    wall: Duration,
}

impl Gate {
    /// Starts an application with the services of the desktop program (Skia, HarfBuzz, the light
    /// Fluent theme, Roboto from files) and shows `screen` in a window; the first frame is
    /// rendered by the UI thread (the show is a synchronous commit), the ones after it by the
    /// render thread.
    /// Shows `screen`; with `pointer`, a mouse moves to that point of the window first, so that
    /// pointer-over and tooltips follow the scene as on a desktop with the pointer in the window.
    fn start(screen: impl FnOnce() -> Ref<Control>, pointer: Option<Point>) -> Gate {
        if std::env::var_os("PERF_FONTS").is_none() {
            std::env::set_var("PERF_FONTS", concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/FerroUI.Themes.Material/Assets"));
        }
        let services = CompositorTestServices::start(
            TestServices::styled_window()
                .with_render_interface(Rc::new(ferroui_skia::PlatformRenderInterface::new(None, None)))
                .with_font_manager_impl(Rc::new(ferroui_skia::FontManagerImpl::new()))
                .with_text_shaper_impl(Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new()))
                .with_global_clock(Rc::new(StillClock::default()))
                // As an application: input goes through the pre-processors (pointer-over), and
                // tooltips follow the pointer (`ToolTipService`, registered with the input manager).
                .with_input_manager(Rc::new(InputManager::new()))
                .with_theme(|| FluentTheme::new().as_style()),
        );
        // What the application of the program does when it initialises (`app.rs`).
        Application::current()
            .expect("the application of the gate")
            .set_requested_theme_variant(Some(ThemeVariant::light()));
        FontManager::current().add_font_collection(Rc::new(DeviceRoboto::new()));

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

        let window_impl = MockWindowingPlatform::create_window_mock_with_size(WIDTH, HEIGHT);
        window_impl.setup_compositor(Some(compositor.clone()));
        window_impl.render_scaling.set(SCALING);
        window_impl.desktop_scaling.set(SCALING);
        let frames = Arc::new(AtomicU32::new(0));
        let surface = Arc::new(RasterSurface { frames: frames.clone() });
        window_impl.setup_surfaces(vec![surface as Arc<dyn IPlatformRenderSurface>]);

        let input_impl = window_impl.clone();
        let window = Window::with_impl(window_impl);
        window.set_width(WIDTH);
        window.set_height(HEIGHT);
        window.set_can_resize(false);
        window.set_content(Some(Control::boxed(screen())));
        window.show();

        let thread_loop = render_loop.clone();
        let render_thread = thread::spawn(move || thread_loop.run());
        Gate {
            input_impl,
            pointer,
            _compositor: compositor,
            render_loop,
            render_thread: Some(render_thread),
            _services: services,
            window,
            frames,
        }
    }

    /// Moves a mouse to the gate's pointer position, once the screen is laid out and rendered.
    fn move_pointer(&self) {
        let Some(position) = self.pointer else { return };
        let input = ferroui_controls::platform::ITopLevelImpl::input(&*self.input_impl).expect("the window handles input");
        let device: Rc<dyn IInputDevice> = MouseDevice::new();
        input(Rc::new(RawPointerEventArgs::new(
            device,
            0,
            self.window.input_root(),
            RawPointerEventType::Move,
            position,
            RawInputModifiers::NONE,
        )));
    }

    /// Runs the jobs of the UI thread until the screen's animation frames have advanced the frame
    /// index by `frames`, and returns what that took.
    fn run(&self, frames: i32) -> Run {
        let first = Clock::frame();
        let target = first + frames;
        let rendered = self.frames.load(Ordering::SeqCst);
        let tick_time = self.render_loop.tick_time();
        let start = Instant::now();
        let mut ui = Duration::ZERO;
        let mut last = first;
        let mut moved_at = start;
        while Clock::frame() < target {
            let jobs = Instant::now();
            Dispatcher::ui_thread().run_jobs(None);
            ui += jobs.elapsed();
            let frame = Clock::frame();
            if frame != last {
                last = frame;
                moved_at = Instant::now();
                continue;
            }
            assert!(moved_at.elapsed() < FRAME_TIMEOUT, "the frame index stood at {frame} for {FRAME_TIMEOUT:?}");
            self.render_loop.wait_for_progress(Duration::from_millis(1));
        }
        Run {
            frames: (Clock::frame() - first) as u32,
            rendered: self.frames.load(Ordering::SeqCst) - rendered,
            ui,
            render: self.render_loop.tick_time().saturating_sub(tick_time),
            wall: start.elapsed(),
        }
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        // The render thread ends first: the window is closed by frames of this thread.
        self.render_loop.stop();
        if let Some(handle) = self.render_thread.take() {
            let _ = handle.join();
        }
        self.window.close();
    }
}

/// Shows `screen`, runs the warm-up frames and the measured ones, and prints the line of the gate
/// and a summary.
fn measure(bench: &str, pointer: Option<Point>, screen: impl FnOnce() -> Ref<Control>) {
    let frames = variable("GATE_FRAMES", 240).max(1);
    let warmup = variable("GATE_WARMUP", 60);
    let gate = Gate::start(screen, pointer);
    gate.run(warmup);
    gate.move_pointer();
    gate.run(warmup.min(10));
    let counters = ferroui_base::diagnostics::perf_counters::snapshot();
    let run = gate.run(frames);
    let counters = ferroui_base::diagnostics::perf_counters::snapshot().since(&counters);
    assert!(run.rendered > 0, "{bench}: the surface received no frame");
    // Every frame of the scenarios changes the picture, so every render pass has a batch to
    // commit: fewer frames received than frames run means passes that committed nothing.
    assert!(
        u64::from(run.rendered) * 10 >= u64::from(run.frames) * 9,
        "{bench}: the surface received {} frames of {}: render passes committed nothing",
        run.rendered,
        run.frames
    );

    let per_frame = |time: Duration| time.as_secs_f64() * 1000.0 / f64::from(run.frames);
    let (ui, render, wall) = (per_frame(run.ui), per_frame(run.render), per_frame(run.wall));
    println!(
        "GATE {{\"bench\":\"{bench}\",\"frames\":{},\"warmup\":{warmup},\"ui_ms_per_frame\":{ui:.4},\
         \"render_ms_per_frame\":{render:.4},\"wall_ms_per_frame\":{wall:.4},\"rendered\":{}}}",
        run.frames, run.rendered
    );
    println!(
        "{bench}: {} frames after {warmup} of warm-up ({} rendered), Skia raster {} x {}, render-thread mode: \
         UI thread {ui:.3} ms a frame, render thread {render:.3} ms a frame, {wall:.3} ms a frame of wall time ({:.1} fps)",
        run.frames,
        run.rendered,
        (WIDTH * SCALING) as i32,
        (HEIGHT * SCALING) as i32,
        1000.0 / wall
    );
    // With the feature `counters`: the work of the framework per frame (nothing without it).
    if cfg!(feature = "counters") {
        println!("{bench} counters:\n{}", counters.report_per(u64::from(run.frames), "frame"));
    }
}

#[test]
#[ignore = "the frame gate: run in an optimised build with --ignored --nocapture --test-threads=1"]
fn gauntlet_tier_16() {
    measure("gauntlet/16", None, || GauntletView::new(16, 0).upcast());
}

#[test]
#[ignore = "the frame gate: run in an optimised build with --ignored --nocapture --test-threads=1"]
fn still_wall_tier_20() {
    measure("wall/20", None, || WallView::new(20, 0).upcast());
}

#[test]
#[ignore = "the frame gate: run in an optimised build with --ignored --nocapture --test-threads=1"]
fn still_wall_tier_20_with_the_pointer_in_the_window() {
    measure("wall/20+pointer", Some(Point::new(WIDTH / 2.0, HEIGHT / 2.0)), || WallView::new(20, 0).upcast());
}
