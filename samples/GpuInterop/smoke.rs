//! The smoke run of the sample (`--smoke`; not a port): what can be checked of a demo without a
//! person in front of the window. The demo of the command line (the Vulkan demo by default,
//! `--d3d` on Windows, `--dmabuf` on Linux) draws a few frames into the drawing surface of the
//! compositor; the window is then drawn by the compositor into a snapshot (the composition
//! visual of the window at its render scaling, drawn by the render thread from the same server
//! visuals and through the same drawing code as the frames it presents), and the run checks that
//! the content the GPU API drew is in it, and that it changes with the sliders:
//!
//! 1. the demo initializes: the compositor has a GPU interop that takes the images of the API
//!    (the text the demo shows names the device it draws with);
//! 2. a frame with the values of the sliders at start (yaw, pitch and roll 5) is in the
//!    snapshot: at least 1 % of the area of the demo left of the sliders is saturated colour
//!    (the shaded teapot or cube; the window behind it is grey and white);
//! 3. a frame with another yaw is in the next snapshot, which differs from the first;
//! 4. with the disco slider at 1 the demo draws frame after frame: a snapshot taken then has the
//!    content too and differs again.
//!
//! Each check prints a line (`[ ok ]`, `[FAIL]`); the process exits with 0 only if every check
//! passed. With `FERROUI_SMOKE_SCREENSHOTS=<directory>` the snapshots are written there as PNG
//! files (the frames of the application, never the screen). On macOS
//! `FERROUI_SMOKE_RENDERING=metal|opengl` asks for that rendering mode of the compositor alone.

use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{Point, Ref, Visual};
use ferroui_controls::{AppBuilder, Application, Window};
use gpu_interop::task_support::{start, ServerJobTaskFuture};
use gpu_interop::{DemoType, DrawingSurfaceDemoBase, GpuDemo};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

static FAILED: AtomicBool = AtomicBool::new(false);
/// Whether the run reached its end (every check made).
static FINISHED: AtomicBool = AtomicBool::new(false);

/// How long the demo has to initialize.
const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(30);
/// How long the whole run may take before the watchdog ends the process.
const RUN_TIMEOUT: Duration = Duration::from_secs(120);
/// The width the sliders of the demo take on the right of the window (their column has a
/// minimal width of 300 and the grid a margin of 20), with some room.
const SLIDERS_WIDTH: f64 = 360.0;
/// The height the text of the demo takes at the top of the window.
const INFO_HEIGHT: f64 = 80.0;

fn report(ok: bool, what: &str) {
    println!("{} {what}", if ok { "[ ok ]" } else { "[FAIL]" });
    if !ok {
        FAILED.store(true, Ordering::SeqCst);
    }
}

fn info(what: &str) {
    println!("[info] {what}");
}

/// Adds the smoke run to the application builder.
pub fn configure(builder: AppBuilder, demo_type: DemoType) -> AppBuilder {
    std::thread::spawn(|| {
        std::thread::sleep(RUN_TIMEOUT);
        println!("[FAIL] the smoke run did not end within {} s", RUN_TIMEOUT.as_secs());
        std::process::exit(2);
    });
    rendering_options(builder).after_setup(move |_| {
        info(&format!("the {demo_type:?} demo"));
        drop(start(run(demo_type)));
    })
}

/// The exit code of the process.
pub fn exit_code(lifetime_exit_code: i32) -> std::process::ExitCode {
    if !FINISHED.load(Ordering::SeqCst) {
        report(false, &format!("the application ended before the smoke run did (exit code {lifetime_exit_code})"));
    }
    if FAILED.load(Ordering::SeqCst) {
        println!("Smoke run: FAILED");
        std::process::ExitCode::from(1)
    } else {
        println!("Smoke run: passed");
        std::process::ExitCode::SUCCESS
    }
}

/// `FERROUI_SMOKE_RENDERING=metal|opengl|software` on macOS: that rendering mode alone.
#[cfg(target_os = "macos")]
fn rendering_options(builder: AppBuilder) -> AppBuilder {
    use ferroui_native::{FerroNativePlatformOptions, FerroNativeRenderingMode};

    let mut options = FerroNativePlatformOptions::default();
    match std::env::var("FERROUI_SMOKE_RENDERING").ok().as_deref() {
        Some("opengl") => options.rendering_mode = vec![FerroNativeRenderingMode::OpenGl],
        Some("metal") => options.rendering_mode = vec![FerroNativeRenderingMode::Metal],
        Some("software") => options.rendering_mode = vec![FerroNativeRenderingMode::Software],
        _ => return builder,
    }
    info(&format!("rendering modes: {:?}", options.rendering_mode));
    builder.with(Rc::new(options)).after_setup(|_| {
        // Which graphics the platform applied: the OpenGL graphics are the ones with a shared
        // context, the Metal ones have none.
        use ferroui_base::platform::IPlatformGraphics;
        use ferroui_base::{FerroLocator, LocatorExtensions};
        let graphics = FerroLocator::current().get_service::<std::sync::Arc<dyn IPlatformGraphics>>();
        match graphics.as_deref() {
            Some(graphics) if graphics.uses_shared_context() => info("platform graphics: OpenGL (CGL)"),
            Some(_) => info("platform graphics: Metal"),
            None => info("platform graphics: none (software)"),
        }
    })
}

#[cfg(not(target_os = "macos"))]
fn rendering_options(builder: AppBuilder) -> AppBuilder {
    builder
}

/// A future that completes after `duration`, on the dispatcher.
struct Delay {
    duration: Duration,
    done: Rc<Cell<bool>>,
    waker: Rc<RefCell<Option<Waker>>>,
    started: bool,
}

fn delay(duration: Duration) -> Delay {
    Delay { duration, done: Rc::default(), waker: Rc::default(), started: false }
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.done.get() {
            return Poll::Ready(());
        }
        *self.waker.borrow_mut() = Some(cx.waker().clone());
        if !self.started {
            self.started = true;
            let (done, waker) = (self.done.clone(), self.waker.clone());
            let _timer = DispatcherTimer::run_once(
                move || {
                    done.set(true);
                    if let Some(waker) = waker.borrow_mut().take() {
                        waker.wake();
                    }
                },
                self.duration,
                DispatcherPriority::NORMAL,
            )
            .detach();
        }
        Poll::Pending
    }
}

fn main_window() -> Option<Ref<Window>> {
    let lifetime = Application::current().and_then(|application| application.application_lifetime());
    lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()).and_then(|l| l.main_window())
}

/// The pixels of a snapshot: four bytes each, the alpha last.
struct Frame {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl Frame {
    /// The pixels of `bitmap`. A snapshot of the compositor is an image of the GPU, which has no
    /// pixels to copy on the processor: it is encoded as PNG and decoded again.
    fn of(bitmap: &Bitmap) -> Option<Frame> {
        let mut encoded = Vec::new();
        bitmap.save(&mut encoded, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT)).ok()?;
        let mut decoder = png::Decoder::new(std::io::Cursor::new(encoded));
        decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
        let mut reader = decoder.read_info().ok()?;
        let mut buffer = vec![0u8; reader.output_buffer_size()?];
        let info = reader.next_frame(&mut buffer).ok()?;
        if info.color_type != png::ColorType::Rgba {
            return None;
        }
        buffer.truncate(info.buffer_size());
        Some(Frame { width: info.width as usize, height: info.height as usize, pixels: buffer })
    }

    fn pixel(&self, x: usize, y: usize) -> &[u8] {
        let i = (y * self.width + x) * 4;
        &self.pixels[i..i + 4]
    }

    /// The share of the pixels of `region` that are opaque and of a saturated colour.
    fn saturated_share(&self, region: Region) -> f64 {
        let mut saturated = 0usize;
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                let p = self.pixel(x, y);
                let (max, min) = (p[0].max(p[1]).max(p[2]), p[0].min(p[1]).min(p[2]));
                if p[3] > 200 && max - min > 48 {
                    saturated += 1;
                }
            }
        }
        saturated as f64 / region.area().max(1) as f64
    }

    /// The share of the pixels of `region` that differ from `other` by more than 16 in a channel.
    fn changed_share(&self, other: &Frame, region: Region) -> f64 {
        if (self.width, self.height) != (other.width, other.height) {
            return 1.0;
        }
        let mut changed = 0usize;
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                let (a, b) = (self.pixel(x, y), other.pixel(x, y));
                if (0..3).any(|c| a[c].abs_diff(b[c]) > 16) {
                    changed += 1;
                }
            }
        }
        changed as f64 / region.area().max(1) as f64
    }
}

/// A rectangle of pixels of a frame.
#[derive(Clone, Copy, Debug)]
struct Region {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl Region {
    fn area(&self) -> usize {
        (self.right - self.left) * (self.bottom - self.top)
    }
}

/// The part of the demo the sliders and the text leave free, in pixels of a frame of the window
/// at `scaling`.
fn demo_region(window: &Ref<Window>, demo: &Ref<DrawingSurfaceDemoBase>, frame: &Frame, scaling: f64) -> Region {
    let origin = demo.translate_point(Point::new(0.0, 0.0), window).unwrap_or(Point::new(0.0, 0.0));
    let bounds = demo.bounds();
    let clamp_x = |v: f64| ((v * scaling).max(0.0) as usize).min(frame.width);
    let clamp_y = |v: f64| ((v * scaling).max(0.0) as usize).min(frame.height);
    let left = clamp_x(origin.x);
    let top = clamp_y(origin.y + INFO_HEIGHT);
    let right = clamp_x(origin.x + bounds.width - SLIDERS_WIDTH).max(left);
    let bottom = clamp_y(origin.y + bounds.height).max(top);
    Region { left, top, right, bottom }
}

/// The window as its compositor draws it, written to the directory of the screenshots when
/// there is one.
async fn snapshot(window: &Ref<Window>, name: &str) -> Option<Frame> {
    let visual = ElementComposition::get_element_visual(&window.clone().upcast::<Visual>())?;
    let compositor = visual.compositor().clone();
    let task = compositor.create_composition_visual_snapshot(&visual, window.render_scaling());
    compositor.commit();
    if let Err(error) = ServerJobTaskFuture::new(task.clone()).await {
        report(false, &format!("the compositor made the snapshot {name}: {error}"));
        return None;
    }
    let bitmap = match task.take_result() {
        Some(Ok(bitmap)) => bitmap,
        _ => return None,
    };
    if let Some(directory) = std::env::var_os("FERROUI_SMOKE_SCREENSHOTS") {
        let directory = PathBuf::from(directory);
        let _ = std::fs::create_dir_all(&directory);
        let mode = std::env::var("FERROUI_SMOKE_RENDERING").unwrap_or_else(|_| "default".to_string());
        let path = directory.join(format!("{}-{mode}-{name}.png", std::env::consts::OS));
        match bitmap.save_to_file(&path.to_string_lossy(), &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT)) {
            Ok(()) => info(&format!("snapshot {name}: {}", path.display())),
            Err(error) => report(false, &format!("the snapshot {name} is written to {}: {error}", path.display())),
        }
    }
    let frame = Frame::of(&bitmap);
    if frame.is_none() {
        report(false, &format!("the pixels of the snapshot {name} are read"));
    }
    frame
}

fn find_demo(window: &Ref<Window>) -> Option<(Ref<GpuDemo>, Ref<DrawingSurfaceDemoBase>)> {
    let gpu_demo = window.find_descendant_of_type::<GpuDemo>(false)?;
    let demo = gpu_demo.demo()?.as_object()?.cast::<DrawingSurfaceDemoBase>()?;
    Some((gpu_demo, demo))
}

async fn run(demo_type: DemoType) {
    let started = Instant::now();
    // 1. The window, the demo in it, and its initialization.
    let (window, gpu_demo, demo) = loop {
        if let Some(window) = main_window() {
            if let Some((gpu_demo, demo)) = find_demo(&window) {
                if demo.is_initialized() {
                    break (window, gpu_demo, demo);
                }
                let text = demo.info();
                if !text.is_empty() {
                    report(false, &format!("the {demo_type:?} demo initializes: {text}"));
                    finish();
                    return;
                }
            }
        }
        if started.elapsed() > INITIALIZE_TIMEOUT {
            report(false, &format!("the {demo_type:?} demo initializes within {} s", INITIALIZE_TIMEOUT.as_secs()));
            finish();
            return;
        }
        delay(Duration::from_millis(100)).await;
    };
    report(true, &format!("the {demo_type:?} demo initializes: {}", demo.info().replace('\n', " / ")));
    report(gpu_demo.info() == demo.info(), "the demo control shows the text of the demo");

    let scaling = window.render_scaling();

    // 2. The first frames, with the values of the sliders at start.
    delay(Duration::from_millis(1500)).await;
    let Some(first) = snapshot(&window, "01-start").await else {
        report(false, "a snapshot of the window");
        finish();
        return;
    };
    let region = demo_region(&window, &demo, &first, scaling);
    info(&format!("the snapshot is {} by {} pixels; the region of the demo is {region:?}", first.width, first.height));
    let share = first.saturated_share(region);
    report(share >= 0.01, &format!("the frame the GPU drew is in the snapshot ({:.1} % of the demo is its colour)", share * 100.0));

    // 3. Another yaw: the next frame differs.
    gpu_demo.set_yaw(2.0);
    delay(Duration::from_millis(1000)).await;
    if let Some(second) = snapshot(&window, "02-yaw").await {
        let share = second.saturated_share(region);
        report(share >= 0.01, &format!("the frame with another yaw is in the snapshot ({:.1} %)", share * 100.0));
        let changed = second.changed_share(&first, region);
        report(changed >= 0.005, &format!("the frame changed with the yaw ({:.1} % of the demo changed)", changed * 100.0));

        // 4. Disco: frame after frame.
        gpu_demo.set_disco(1.0);
        delay(Duration::from_millis(1500)).await;
        if let Some(third) = snapshot(&window, "03-disco").await {
            let share = third.saturated_share(region);
            report(share >= 0.01, &format!("a frame of the disco mode is in the snapshot ({:.1} %)", share * 100.0));
            let changed = third.changed_share(&second, region);
            report(changed >= 0.005, &format!("the disco mode changed the frame ({:.1} % of the demo changed)", changed * 100.0));
        }
    }

    finish();
}

/// Marks the run as finished and closes the main window, which ends the application.
fn finish() {
    FINISHED.store(true, Ordering::SeqCst);
    if let Some(window) = main_window() {
        window.close();
    }
}
