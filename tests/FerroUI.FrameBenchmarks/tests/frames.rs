//! The frame benchmarks of the local performance gate: every scene of `scenes.rs` in a headless
//! window of 1280 x 820 points at a scaling of 2, rendered through the compositor into a raster Skia
//! framebuffer of 2560 x 1640 pixels, in the render-thread mode of the desktop. Run them in an
//! optimised build, alone:
//!
//! ```sh
//! cargo test -p ferroui-frame-benchmarks --release --test frames -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `GATE_FRAMES` (240) and `GATE_WARMUP` (60) set the frames measured and the frames before them;
//! `PERF_FONTS` the folder of `Roboto-Regular.ttf` and `Roboto-Bold.ttf`.
//!
//! The render loop, the surface and the clock are those of the gauntlet's frame gate
//! (`samples/GauntletBenchmark/tests/frame_gate.rs`): a thread of the test ticks the render loop
//! whenever the compositor wakes it, the frames are the scene's own (each animation frame advances
//! the frame index once), and nothing follows wall time.
//!
//! Each scene prints a line for the gate,
//!
//! ```text
//! GATE {"bench":"frames/text","frames":240,"warmup":60,"ui_ms_per_frame":1.2,"render_ms_per_frame":2.3,"wall_ms_per_frame":2.5,"rendered":241,"build_ms":12.3,"show_ms":45.6,"startup_ms":57.9}
//! ```
//!
//! with, per frame: the time the UI thread spent running its jobs, the time the render thread spent
//! in its ticks and the wall time; and, once: the time the scene took to build and the window to
//! show it (its first layout, recording and commit).
//!
//! The scenes of the start (`STARTUP_SCENES`) are shown once and measured to their first frame,
//!
//! ```text
//! GATE {"bench":"startup/tiles","services_ms":30.1,"build_ms":210.4,"content_ms":0.1,"show_ms":650.2,"first_ui_ms":310.8,"first_render_ms":420.5,"first_frame_ms":735.0,"startup_ms":1595.7,"build_mb":1230.0,"show_mb":860.0,"first_frame_mb":1190.0,"startup_mb":3280.0}
//! ```
//!
//! with the time the services of the application took to start (the theme among them), the scene
//! to build, the window to take it as its content and to show it (the attachment to the tree, the
//! styles and the first layout), and what follows the show until the surface has its first frame:
//! the jobs of the UI thread (the recording and the commit), the ticks of the render thread (the
//! batch and the drawing) and the wall time of the two. `startup_ms` is the sum of the build, the
//! content, the show and the wall time to the first frame. The `_mb` fields are the resident
//! megabytes the build, the show and the first frame added to the process, and their sum.
//!
//! With the `counters` feature a start also prints, for each of its phases, the allocations it
//! made and the blocks it left alive by size class (`heap`), and the counters of the framework.
//!
//! `sizes_of_an_element` prints the sizes of the objects an element in the tree is made of,
//!
//! ```text
//! SIZES {"FerroObject":344,"Border":2160,"CompositionVisual":1032,"ServerCompositionVisual":1080}
//! ```
//!
//! and the gate takes any growth of one of them as a regression.

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
use ferroui_frame_benchmarks::{Clock, FrameHost, Roboto, Scene, SCENES, STARTUP_SCENES, WARM_UP};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Once};
use std::thread;
use std::time::{Duration, Instant};

mod heap;

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

    /// Waits until the tick that is running, if one is, has ended and its time was counted.
    fn wait_for_tick_end(&self) {
        let mut state = self.state();
        while state.ticking {
            state = self.changed.wait_timeout(state, TICK_INTERVAL).unwrap_or_else(|e| e.into_inner()).0;
        }
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
    /// The time the scene took to build, and the window to show it: its first layout, the
    /// recording of its first frame and the commit of it, on the UI thread.
    build: Duration,
    show: Duration,
    /// The time the services of the application took to start, and, of `show`, the time the
    /// window took to take the scene as its content.
    services: Duration,
    content: Duration,
    /// The memory the process held, in megabytes: before the scene was built, when it was
    /// built, and when the window had shown it.
    memory: [f64; 3],
    /// The heap at the same moments, and when the window had taken the scene as its content:
    /// zero without the feature `counters`.
    heap: [heap::Heap; 4],
}

/// The memory the process holds, in megabytes: the resident set `ps` reports. Zero where
/// there is no `ps`. It is read between the phases of a start, outside of what they time.
fn resident_megabytes() -> f64 {
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|kilobytes| kilobytes.trim().parse::<f64>().ok())
        .map_or(0.0, |kilobytes| kilobytes / 1024.0)
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
    fn start(scene: &Scene) -> Gate {
        let pointer = scene.pointer.then_some(Point::new(WIDTH / 2.0, HEIGHT / 2.0));
        Clock::reset();
        let started = Instant::now();
        // `FERROUI_GATE_VISUALS_ON_DEMAND=0` gives every element a composition visual, as before
        // section 9.41: the two are compared with it.
        if std::env::var("FERROUI_GATE_VISUALS_ON_DEMAND").is_ok_and(|value| value.trim() == "0") {
            ferroui_base::Visual::set_composition_visuals_on_demand(false);
        }
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
        FontManager::current().add_font_collection(Rc::new(Roboto::new()));
        let services_time = started.elapsed();

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
        let (memory_before, heap_before) = (resident_megabytes(), heap::snapshot());
        let built = Instant::now();
        let screen = FrameHost::new((scene.build)());
        let build = built.elapsed();
        let (memory_built, heap_built) = (resident_megabytes(), heap::snapshot());
        let shown = Instant::now();
        window.set_content(Some(Control::boxed(&screen)));
        let content = shown.elapsed();
        let heap_content = heap::snapshot();
        window.show();
        let show = shown.elapsed();
        let (memory_shown, heap_shown) = (resident_megabytes(), heap::snapshot());

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
            build,
            show,
            services: services_time,
            content,
            memory: [memory_before, memory_built, memory_shown],
            heap: [heap_before, heap_built, heap_content, heap_shown],
        }
    }

    /// Runs the jobs of the UI thread until the surface has received its first frame, and returns
    /// what that took: the jobs of the UI thread, the ticks of the render thread, the wall time.
    fn first_frame(&self) -> Run {
        let tick_time = self.render_loop.tick_time();
        let start = Instant::now();
        let mut ui = Duration::ZERO;
        while self.frames.load(Ordering::SeqCst) == 0 {
            let jobs = Instant::now();
            Dispatcher::ui_thread().run_jobs(None);
            ui += jobs.elapsed();
            if self.frames.load(Ordering::SeqCst) != 0 {
                break;
            }
            assert!(start.elapsed() < FRAME_TIMEOUT, "the surface received no frame in {FRAME_TIMEOUT:?}");
            self.render_loop.wait_for_progress(Duration::from_millis(1));
        }
        let wall = start.elapsed();
        // The frame is counted when it is presented, inside the tick that drew it.
        self.render_loop.wait_for_tick_end();
        Run {
            frames: 1,
            rendered: self.frames.load(Ordering::SeqCst),
            ui,
            render: self.render_loop.tick_time().saturating_sub(tick_time),
            wall,
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

/// The frames the warm-up of the process runs.
const PROCESS_WARM_UP_FRAMES: i32 = 30;

/// Shows the scene of the warm-up and runs a few frames of it, once in the process, before the
/// first scene that is measured. Without it the first scene carries what a process does once: the
/// scene `lines`, the first by its name, was built and shown in 3.0 and 5 to 6.6 ms in a process
/// that started cold and in 0.9 and 1.4 ms in one that did not.
fn warm_up_the_process() {
    static WARMED: Once = Once::new();
    WARMED.call_once(|| {
        ferroui_frame_benchmarks::page_in_the_program();
        let gate = Gate::start(&WARM_UP);
        gate.run(PROCESS_WARM_UP_FRAMES);
    });
}

/// Shows the scene, runs the warm-up frames and the measured ones, and prints the line of the gate
/// and a summary.
fn measure(scene: &Scene) {
    let bench = format!("frames/{}", scene.name);
    let frames = variable("GATE_FRAMES", 240).max(1);
    let warmup = variable("GATE_WARMUP", 60);
    warm_up_the_process();
    let gate = Gate::start(scene);
    gate.run(warmup);
    gate.move_pointer();
    gate.run(warmup.min(10));
    let counters = ferroui_base::diagnostics::perf_counters::snapshot();
    let run = gate.run(frames);
    let counters = ferroui_base::diagnostics::perf_counters::snapshot().since(&counters);
    assert!(run.rendered > 0, "{bench}: the surface received no frame");

    let per_frame = |time: Duration| time.as_secs_f64() * 1000.0 / f64::from(run.frames);
    let (ui, render, wall) = (per_frame(run.ui), per_frame(run.render), per_frame(run.wall));
    let milliseconds = |time: Duration| time.as_secs_f64() * 1000.0;
    let (build, show) = (milliseconds(gate.build), milliseconds(gate.show));
    println!(
        "GATE {{\"bench\":\"{bench}\",\"frames\":{},\"warmup\":{warmup},\"ui_ms_per_frame\":{ui:.4},\
         \"render_ms_per_frame\":{render:.4},\"wall_ms_per_frame\":{wall:.4},\"rendered\":{},\
         \"build_ms\":{build:.3},\"show_ms\":{show:.3},\"startup_ms\":{:.3}}}",
        run.frames,
        run.rendered,
        build + show
    );
    println!(
        "{bench}: built in {build:.1} ms, shown in {show:.1} ms; {} frames after {warmup} of warm-up ({} rendered): \
         UI thread {ui:.3} ms a frame, render thread {render:.3} ms a frame, {wall:.3} ms of wall time ({:.1} fps). \
         Guards {}.",
        run.frames,
        run.rendered,
        1000.0 / wall,
        scene.guards
    );
    if cfg!(feature = "counters") {
        println!("{bench} counters:\n{}", counters.report_per(u64::from(run.frames), "frame"));
    }
}

/// Shows a scene of the start, waits for its first frame, and prints the line of the gate and a
/// summary.
fn measure_start(scene: &Scene) {
    let bench = format!("startup/{}", scene.name);
    // A profiler that attaches to the process by its name needs the start to wait for it.
    if let Some(wait) = std::env::var("FERROUI_GATE_START_DELAY_MS").ok().and_then(|wait| wait.parse().ok()) {
        std::thread::sleep(Duration::from_millis(wait));
    }
    // A start is of a process that has built nothing yet, with its program in memory: the pages
    // of the program the start runs through are not read from the disk inside it.
    ferroui_frame_benchmarks::page_in_the_program();
    let counters = ferroui_base::diagnostics::perf_counters::snapshot();
    let gate = Gate::start(scene);
    let first = gate.first_frame();
    let counters = ferroui_base::diagnostics::perf_counters::snapshot().since(&counters);
    assert!(first.rendered > 0, "{bench}: the surface received no frame");
    let (memory_first, heap_first) = (resident_megabytes(), heap::snapshot());
    // A tool that reads the memory of the process needs it to stay as the first frame left it.
    if let Some(wait) = std::env::var("FERROUI_GATE_END_DELAY_MS").ok().and_then(|wait| wait.parse().ok()) {
        std::thread::sleep(Duration::from_millis(wait));
    }

    let milliseconds = |time: Duration| time.as_secs_f64() * 1000.0;
    let services = milliseconds(gate.services);
    let build = milliseconds(gate.build);
    let content = milliseconds(gate.content);
    let show = milliseconds(gate.show.saturating_sub(gate.content));
    let (first_ui, first_render, first_frame) =
        (milliseconds(first.ui), milliseconds(first.render), milliseconds(first.wall));
    let startup = build + content + show + first_frame;
    // What the scene added to the memory of the process: by its build, by the show of the
    // window, by the first frame.
    let [before, built, shown] = gate.memory;
    let (build_mb, show_mb, first_frame_mb) = (built - before, shown - built, memory_first - shown);
    let startup_mb = memory_first - before;
    println!(
        "GATE {{\"bench\":\"{bench}\",\"services_ms\":{services:.3},\"build_ms\":{build:.3},\
         \"content_ms\":{content:.3},\"show_ms\":{show:.3},\"first_ui_ms\":{first_ui:.3},\
         \"first_render_ms\":{first_render:.3},\"first_frame_ms\":{first_frame:.3},\"startup_ms\":{startup:.3},\
         \"build_mb\":{build_mb:.1},\"show_mb\":{show_mb:.1},\"first_frame_mb\":{first_frame_mb:.1},\
         \"startup_mb\":{startup_mb:.1}}}"
    );
    println!(
        "{bench}: services in {services:.1} ms; built in {build:.1} ms, taken as the content in {content:.1} ms, \
         shown in {show:.1} ms; the first frame {first_frame:.1} ms after the show (UI thread {first_ui:.1} ms, \
         render thread {first_render:.1} ms): {startup:.1} ms from the build to the first frame. Memory: \
         {build_mb:.0} MB built, {show_mb:.0} MB more shown, {first_frame_mb:.0} MB more at the first frame \
         ({startup_mb:.0} MB). Guards {}.",
        scene.guards
    );
    if cfg!(feature = "counters") {
        let [before, built, content, shown] = gate.heap;
        println!("{bench} heap:");
        println!("  the build: {}", built.since(&before).report());
        println!("  the content: {}", content.since(&built).report());
        println!("  the show: {}", shown.since(&content).report());
        println!("  the first frame: {}", heap_first.since(&shown).report());
        println!("{bench} sizes:\n{}", sizes_report());
        println!("{bench} counters:\n{}", counters.report());
        let mut census = Census::default();
        census.count(&gate.window, true);
        println!("{bench} census:\n{}", census.report());
    }
}

/// What the elements of a shown scene ask of the compositor: how many draw, and how many set
/// something that only a composition visual of their own can carry.
#[derive(Default)]
struct Census {
    elements: usize,
    leaves: usize,
    /// Record a drawing.
    drawing: usize,
    /// Set an input of the visual (opacity, a clip, a transform, an effect, a mask, a cache mode,
    /// hidden, mirrored), whether they draw or not.
    with_inputs: usize,
    /// Clip to their bounds and set no other input.
    only_clip_to_bounds: usize,
    /// Draw nothing and set no input: nothing of them reaches a frame but their position.
    bare: usize,
    /// Draw, set no input.
    drawing_without_inputs: usize,
    /// Draw nothing, set an input.
    inputs_without_drawing: usize,
    /// Bare elements whose parent is bare too.
    bare_under_bare: usize,
    /// The longest chain of bare elements.
    bare_chain: usize,
    /// Have no composition visual (section 9.41).
    without_visual: usize,
    /// Draw, and have no composition visual: what they draw is an item of the composition
    /// visual of the nearest element above that has one (section 9.42).
    items: usize,
    /// Of those that draw nothing and set an input: how many set each one. An element that sets
    /// two is counted for both.
    inputs_set: [usize; INPUT_NAMES.len()],
    /// Of those that draw nothing and set an input: how many set exactly one, by the one.
    one_input_set: [usize; INPUT_NAMES.len()],
    /// Have a composition visual, draw nothing and set no input: something else asked for the
    /// visual (it was handed out, a transition, a custom hit test, options, a child that needs
    /// the parent to have one).
    visual_for_another_reason: usize,
    /// Have a composition visual and are below an element that is not visible: nothing of them
    /// reaches a frame.
    visuals_below_hidden: usize,
    /// Are below an element that is not visible.
    below_hidden: usize,
}

/// The inputs of a composition visual an element sets, in the order of `Census::inputs_set`.
const INPUT_NAMES: [&str; 9] = [
    "clip to bounds",
    "not visible",
    "opacity",
    "a clip",
    "a render transform",
    "an effect",
    "an opacity mask",
    "a cache mode",
    "mirrored",
];

impl Census {
    fn count(&mut self, visual: &ferroui_base::Visual, root: bool) {
        self.walk(visual, root, 0, false);
    }

    fn walk(&mut self, visual: &ferroui_base::Visual, root: bool, bare_above: usize, hidden_above: bool) {
        let children = visual.visual_children().to_vec();
        self.elements += 1;
        if children.is_empty() {
            self.leaves += 1;
        }
        // Asking an element without a composition visual for it would give it one.
        let has_visual = visual.has_composition_visual();
        if !has_visual {
            self.without_visual += 1;
        }
        let is_item = visual.has_composition_item();
        if is_item {
            self.items += 1;
        }
        let draws = is_item
            || has_visual && visual.composition_visual().is_some_and(|composition| composition.draw_list().is_some());
        let other_inputs = visual.opacity() != 1.0
            || visual.clip().is_some()
            || visual.cache_mode().is_some()
            || visual.effect().is_some()
            || visual.opacity_mask().is_some()
            || visual.render_transform().is_some()
            || !visual.is_visible()
            || visual.has_mirror_transform();
        let inputs = other_inputs || visual.clip_to_bounds();
        if hidden_above {
            self.below_hidden += 1;
            if has_visual {
                self.visuals_below_hidden += 1;
            }
        }
        if !draws && inputs {
            let set = [
                visual.clip_to_bounds(),
                !visual.is_visible(),
                visual.opacity() != 1.0,
                visual.clip().is_some(),
                visual.render_transform().is_some(),
                visual.effect().is_some(),
                visual.opacity_mask().is_some(),
                visual.cache_mode().is_some(),
                visual.has_mirror_transform(),
            ];
            let one = set.iter().filter(|set| **set).count() == 1;
            for (index, _) in set.iter().enumerate().filter(|(_, set)| **set) {
                self.inputs_set[index] += 1;
                if one {
                    self.one_input_set[index] += 1;
                }
            }
        }
        if has_visual && !draws && !inputs && !root {
            self.visual_for_another_reason += 1;
        }
        if draws {
            self.drawing += 1;
        }
        if inputs {
            self.with_inputs += 1;
        }
        if visual.clip_to_bounds() && !other_inputs {
            self.only_clip_to_bounds += 1;
        }
        let bare = !draws && !inputs && !root;
        match (draws, inputs) {
            (true, false) => self.drawing_without_inputs += 1,
            (false, true) => self.inputs_without_drawing += 1,
            _ => {}
        }
        let chain = if bare { bare_above + 1 } else { 0 };
        if bare {
            self.bare += 1;
            if bare_above > 0 {
                self.bare_under_bare += 1;
            }
            self.bare_chain = self.bare_chain.max(chain);
        }
        let hidden = hidden_above || !visual.is_visible();
        for child in &children {
            self.walk(child, false, chain, hidden);
        }
    }

    fn report(&self) -> String {
        let share = |count: usize| 100.0 * count as f64 / self.elements.max(1) as f64;
        let line = |name: &str, count: usize| format!("  {name}: {count} ({:.1} %)", share(count));
        [
            format!("  elements: {}", self.elements),
            line("leaves", self.leaves),
            line("record a drawing", self.drawing),
            line("set an input of the visual", self.with_inputs),
            line("clip to bounds and nothing else", self.only_clip_to_bounds),
            line("draw nothing, set nothing", self.bare),
            line("draw, set nothing", self.drawing_without_inputs),
            line("draw nothing, set an input", self.inputs_without_drawing),
            line("draw nothing, set nothing, under one that does neither", self.bare_under_bare),
            format!("  longest chain that draws nothing and sets nothing: {}", self.bare_chain),
            line("have no composition visual", self.without_visual),
            line("draw as an item of a composition visual above them", self.items),
            line("have a composition visual, draw nothing, set nothing", self.visual_for_another_reason),
            line("are below one that is not visible", self.below_hidden),
            line("have a composition visual below one that is not visible", self.visuals_below_hidden),
            "  draw nothing, set an input, by the input (and with no other input):".to_string(),
        ]
        .into_iter()
        .chain(INPUT_NAMES.iter().enumerate().map(|(index, name)| {
            format!(
                "    {name}: {} ({:.1} %), alone {}",
                self.inputs_set[index],
                share(self.inputs_set[index]),
                self.one_input_set[index]
            )
        }))
        .collect::<Vec<_>>()
        .join("\n")
    }
}

/// The sizes, in bytes, of the objects an element in the tree is made of: the element (each class
/// holds the fields of its base class and its own), its composition visual and the visual the
/// render thread keeps for it. An element pays every one of them, and a start touches each.
fn sizes() -> Vec<(&'static str, usize)> {
    use ferroui_base::rendering::composition::server::{
        ServerCompositionDrawListVisual, ServerCompositionVisual, ServerCompositionVisualCollection, ServerObject,
        VisualReadback,
    };
    use ferroui_base::rendering::composition::{CompositionObject, CompositionVisual, CompositionVisualCollection};
    use std::mem::size_of;
    vec![
        ("FerroObject", size_of::<ferroui_base::FerroObject>()),
        ("Animatable", size_of::<ferroui_base::animation::Animatable>()),
        ("StyledElement", size_of::<ferroui_base::StyledElement>()),
        ("Visual", size_of::<ferroui_base::Visual>()),
        ("Layoutable", size_of::<ferroui_base::layout::Layoutable>()),
        ("Interactive", size_of::<ferroui_base::interactivity::Interactive>()),
        ("InputElement", size_of::<ferroui_base::input::InputElement>()),
        ("Control", size_of::<Control>()),
        ("Decorator", size_of::<ferroui_controls::Decorator>()),
        ("Border", size_of::<ferroui_controls::Border>()),
        ("Panel", size_of::<ferroui_controls::Panel>()),
        ("StackPanel", size_of::<ferroui_controls::StackPanel>()),
        ("TextBlock", size_of::<ferroui_controls::TextBlock>()),
        ("CompositionObject", size_of::<CompositionObject>()),
        ("CompositionVisual", size_of::<CompositionVisual>()),
        ("CompositionVisualCollection", size_of::<CompositionVisualCollection>()),
        ("VisualReadback", size_of::<VisualReadback>()),
        ("ServerObject", size_of::<ServerObject>()),
        ("ServerCompositionVisual", size_of::<ServerCompositionVisual>()),
        ("ServerCompositionVisualCollection", size_of::<ServerCompositionVisualCollection>()),
        ("ServerCompositionDrawListVisual", size_of::<ServerCompositionDrawListVisual>()),
    ]
}

fn sizes_report() -> String {
    sizes().iter().map(|(name, size)| format!("  {name}: {size} bytes")).collect::<Vec<_>>().join("\n")
}

/// Prints the sizes of the objects of an element; `scripts/perf/gate.py` compares them.
#[test]
#[ignore = "a part of the frame benchmarks: run with --ignored --nocapture"]
fn sizes_of_an_element() {
    let fields = sizes().iter().map(|(name, size)| format!("\"{name}\":{size}")).collect::<Vec<_>>().join(",");
    println!("SIZES {{{fields}}}");
    println!("{}", sizes_report());
}

fn scene(name: &str) -> &'static Scene {
    SCENES.iter().find(|scene| scene.name == name).expect("a scene of the frame benchmarks")
}

macro_rules! scenes {
    ($($test:ident => $name:literal),* $(,)?) => {$(
        #[test]
        #[ignore = "a frame benchmark: run in an optimised build with --ignored --nocapture --test-threads=1"]
        fn $test() {
            measure(scene($name));
        }
    )*};
}

scenes! {
    text => "text",
    paragraph => "paragraph",
    list => "list",
    transforms => "transforms",
    width => "width",
    wall => "wall",
    wall_with_the_pointer => "wall+pointer",
    moves => "moves",
    lines => "lines",
    items => "items",
    recolor => "recolor",
    cards => "cards",
    reorder => "reorder",
    deep => "deep",
    rows => "rows",
    pile => "pile",
}

macro_rules! starts {
    ($($test:ident => $name:literal),* $(,)?) => {$(
        #[test]
        #[ignore = "a start benchmark: run in an optimised build with --ignored --nocapture --test-threads=1"]
        fn $test() {
            let scene = STARTUP_SCENES.iter().find(|scene| scene.name == $name).expect("a scene of the start");
            measure_start(scene);
        }
    )*};
}

starts! {
    start_of_tiles => "tiles",
    start_of_controls => "controls",
}

#[test]
fn every_scene_has_a_test() {
    let tested = [
        "text", "paragraph", "list", "transforms", "width", "wall", "wall+pointer", "moves", "lines", "items",
        "recolor", "cards", "reorder", "deep", "rows", "pile",
    ];
    for scene in SCENES {
        assert!(tested.contains(&scene.name), "the scene {} has no test", scene.name);
    }
    let started = ["tiles", "controls"];
    for scene in STARTUP_SCENES {
        assert!(started.contains(&scene.name), "the scene of the start {} has no test", scene.name);
    }
}
