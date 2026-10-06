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

use super::support::*;
use crate::pages::{ButtonsPage, TableViewPage};
use ferroui_base::input::raw::{RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{IInputDevice, IInputRoot, MouseDevice, Pointer, PointerType, RawInputModifiers};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
    IPlatformRenderSurfaceRenderTarget,
};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormats, RenderTargetSceneInfo, RetainedFramebuffer};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, PixelSize, Point, Ref, Vector, Visual};
use ferroui_controls::testing::{CompositorTestServices, MockWindowImpl, MockWindowingPlatform};
use ferroui_controls::{Control, ScrollViewer, TableView, Window};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

const WIDTH: f64 = 1280.0;
const HEIGHT: f64 = 800.0;

/// A window surface rendered to in memory, as the software surface of the
/// browser is.
struct RasterSurface {
    frames: Rc<Cell<u32>>,
}

impl RasterSurface {
    fn new() -> Rc<Self> {
        Rc::new(Self { frames: Rc::new(Cell::new(0)) })
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

struct RasterTarget {
    framebuffer: RefCell<Option<Rc<RetainedFramebuffer>>>,
    frames: Rc<Cell<u32>>,
}

impl IPlatformRenderSurfaceRenderTarget for RasterTarget {}

impl IFramebufferRenderTarget for RasterTarget {
    fn lock(&self, _scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        let size = PixelSize::new(WIDTH as i32, HEIGHT as i32);
        let mut framebuffer = self.framebuffer.borrow_mut();
        let framebuffer =
            framebuffer.get_or_insert_with(|| RetainedFramebuffer::new(size, PixelFormats::RGBA8888, AlphaFormat::Premul));
        let frames = self.frames.clone();
        let locked = framebuffer.lock(Vector::new(96.0, 96.0), move |_| frames.set(frames.get() + 1));
        (locked, FramebufferLockProperties::default())
    }

    fn dispose(&self) {
        if let Some(framebuffer) = self.framebuffer.borrow_mut().take() {
            framebuffer.dispose();
        }
    }
}

/// A shown window that renders through the compositor into a raster
/// surface, with the input of a mouse.
struct Bench {
    services: CompositorTestServices,
    window_impl: Rc<MockWindowImpl>,
    window: Ref<Window>,
    surface: Rc<RasterSurface>,
    mouse: Rc<MouseDevice>,
    timestamp: Cell<u64>,
}

impl Bench {
    fn start(content: impl FnOnce() -> Ref<Control>) -> Bench {
        let services = start_catalog_compositor_application();
        let window_impl = MockWindowingPlatform::create_window_mock_with_size(WIDTH, HEIGHT);
        services.setup(&window_impl);
        let surface = RasterSurface::new();
        window_impl.setup_surfaces(vec![surface.clone() as Rc<dyn IPlatformRenderSurface>]);
        let window = Window::with_impl(window_impl.clone());
        window.set_width(WIDTH);
        window.set_height(HEIGHT);
        window.set_content(Some(Control::boxed(&content())));
        window.show();
        let bench = Bench {
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
    fn frame(&self) -> (Duration, Duration) {
        let start = Instant::now();
        Dispatcher::ui_thread().run_jobs(None);
        let jobs = start.elapsed();
        let start = Instant::now();
        self.services.render_loop().tick();
        Dispatcher::ui_thread().run_jobs(None);
        (jobs, start.elapsed())
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
        let rendered = self.surface.frames.get();
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
            self.surface.frames.get() - rendered,
            stats.median,
            stats.mean,
            stats.p95,
            stats.max,
            Stats::of(&jobs).median,
            Stats::of(&renders).median,
        );
        stats
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        self.window.close();
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
    let rendered = bench.surface.frames.get();
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
    assert_eq!(40, bench.surface.frames.get() - rendered);
    assert!(rows.len() <= realized + 2, "{} rows for {realized} realized", rows.len());
}
