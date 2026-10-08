//! Frames from a worker, without the compositor: steps B2.1 and B2.3 of
//! `docs/porting/browser-render-worker.md`
//! (`scripts/build-browser.sh render_worker_clear --threads`).
//!
//! The page starts a thread (`renderWorkerClearStart`). The thread makes its
//! worker take transferred canvases (the worker handler of the render target
//! registry of the script side), publishes its id and returns to the event
//! loop of its worker without ending. The page then creates a canvas surface
//! with that id (`renderWorkerClearCreateSurface`): the script transfers the
//! control of the canvas to the worker, the worker creates the render target
//! and reports it, and the thread wraps the target as the browser backend
//! does and draws a frame: one colour, and a square of another colour at a
//! fixed place in device pixels, so that a capture tells a frame of the
//! right size from a stretched one.
//!
//! What the page chooses (the query parameters of the host page):
//!
//! - the rendering mode. With WebGL the frame is a clear of the framebuffer
//!   of the canvas through the GL interface of the port. With `Software2D`
//!   it is a retained framebuffer that the thread fills and the software
//!   render target puts to the 2D context of the canvas;
//! - a frame loop: the render timer of the browser backend is started on the
//!   thread and ticks with the animation frames of its worker; each frame
//!   has another colour and is counted in an atomic.
//!
//! What crosses between the two threads, each through the piece of the
//! browser backend that the compositor will use:
//!
//! - the size of the canvas, from the observer of the canvas element on the
//!   thread of the page to the thread that draws, through a
//!   `BrowserSurfaceShared`;
//! - a wake-up of the thread of the page after each frame, through the
//!   signal handle of the browser dispatcher. The page is told the frame
//!   counter from the dispatcher's signal, not from a timer;
//! - a frame out of turn, asked of the render timer by the thread of the
//!   page;
//! - and, as an experiment (`renderWorkerClearWaitForFrame`), the thread of
//!   the page waiting on a condition variable for the next frame, which on
//!   that thread is a spin of the runtime and not a sleep. It is bounded by
//!   a timeout, so the page never hangs.
//!
//! The state is read by the host page from a timer until the first frame
//! (`renderWorkerClearState`) and written into the element `result`, where
//! the behaviour test (`scripts/browser/tests/render_worker_clear.test.mjs`)
//! reads it. The picture itself is checked by the test in a capture of the
//! page.
//!
//! In a build without threads the example compiles and reports that the
//! thread could not start.

#![cfg_attr(target_os = "emscripten", no_main)]

use ferroui_base::platform::surfaces::IFramebufferRenderTarget;
use ferroui_base::platform::RenderTargetSceneInfo;
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::rendering::IRenderTimer;
use ferroui_base::threading::{IDispatcherImpl, IDispatcherSignal};
use ferroui_base::PixelSize;
use ferroui_browser::interop::canvas_helper::{
    add_render_target_registered, add_size_changed, CanvasSurface, RENDER_TARGET_KIND_WEB_GL,
};
use ferroui_browser::interop::thread_proxy::current_thread;
use ferroui_browser::rendering::{
    get_render_target, initialize_worker, BrowserRenderTarget, BrowserRenderTimer, BrowserSurfaceShared,
};
use ferroui_browser::{BrowserRenderingMode, BrowserSingleThreadedDispatcherImpl};
use ferroui_opengl::gl_consts;
use ferroui_opengl::surfaces::{try_get_gl_surface, IGlPlatformSurfaceRenderTarget};
use ferroui_opengl::IGlContext;
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};
use wasm_bindgen::prelude::*;

/// The colour of a frame without a frame loop, as the test reads it in the
/// capture. With a frame loop the first two channels stay and the third
/// changes with each frame ([`frame_color`]).
const COLOR: [u8; 3] = [32, 96, 192];
/// The lowest value of the third channel with a frame loop, and the number
/// of values it takes.
const ANIMATED_BLUE: u32 = 64;
const ANIMATED_BLUE_STEPS: u32 = 96;

/// The colour of the square every frame has, its distance from the left and
/// the top edge of the canvas and the length of its side, in device pixels.
const MARKER_COLOR: [u8; 3] = [224, 160, 32];
const MARKER_OFFSET: i32 = 20;
const MARKER_SIZE: i32 = 30;

/// The id the canvas is created with in place of the id of a top-level:
/// there is none, and no top-level ever has this id.
const NO_TOP_LEVEL: i32 = 0;

// The stages of the example, in the order they are reached.
const IDLE: i32 = 0;
const STARTING: i32 = 1;
const WORKER_READY: i32 = 2;
const SURFACE_CREATED: i32 = 3;
const DONE: i32 = 4;
const FAILED: i32 = 5;

// What the two threads share. Everything is an atomic, is written once, or
// is a piece of the browser backend made for two threads. The frame signal
// at the end is the one exception, and only the experiment waits on it.
static STAGE: AtomicI32 = AtomicI32::new(IDLE);
/// The thread that called `renderWorkerClearStart`.
static MAIN_THREAD: AtomicUsize = AtomicUsize::new(0);
/// The thread that takes the canvas; 0 until it has installed its handler.
static WORKER_THREAD: AtomicUsize = AtomicUsize::new(0);
/// The rendering mode the page asked for (a `BrowserRenderingMode`), or 0
/// for WebGL 2 with WebGL 1 as the second choice.
static REQUESTED_MODE: AtomicI32 = AtomicI32::new(0);
/// Whether the thread runs a frame loop.
static ANIMATED: AtomicBool = AtomicBool::new(false);
/// The size and the scaling of the canvas, and the kind of its target.
static SHARED: OnceLock<Arc<BrowserSurfaceShared>> = OnceLock::new();
/// The render timer, started on the thread that draws.
static TIMER: OnceLock<Arc<BrowserRenderTimer>> = OnceLock::new();
/// The wake-up of the dispatcher of the thread of the page.
static WAKE_UP: OnceLock<Arc<dyn IDispatcherSignal>> = OnceLock::new();
/// The frames drawn so far.
static FRAMES: AtomicU32 = AtomicU32::new(0);
/// The thread the last frame was drawn on.
static DRAWN_ON: AtomicUsize = AtomicUsize::new(0);
/// The size and the colour (as `0xRRGGBB`) of the last frame.
static DRAWN_WIDTH: AtomicI32 = AtomicI32::new(0);
static DRAWN_HEIGHT: AtomicI32 = AtomicI32::new(0);
static DRAWN_COLOR: AtomicU32 = AtomicU32::new(0);
/// Whether the last frame has the square.
static DRAWN_MARKER: AtomicBool = AtomicBool::new(false);
static TARGET_ID: AtomicI32 = AtomicI32::new(0);
static TARGET_KIND: AtomicI32 = AtomicI32::new(0);
static GL_MAJOR_VERSION: AtomicI32 = AtomicI32::new(0);
/// The first error OpenGL reported after a frame.
static GL_ERROR: AtomicI32 = AtomicI32::new(0);
static FAILURE: OnceLock<String> = OnceLock::new();
/// The frame signal of the experiment: the count of frames under a lock and
/// the condition that a frame was drawn.
static FRAME_COUNT: Mutex<u64> = Mutex::new(0);
static FRAME_DRAWN: Condvar = Condvar::new();

#[wasm_bindgen]
extern "C" {
    /// The object of the host page that is told the frame counter.
    type Reporter;

    /// `frames` frames were drawn when the dispatcher of the page was woken
    /// for the `wake_ups`-th time.
    #[wasm_bindgen(method)]
    fn frames(this: &Reporter, frames: u32, wake_ups: u32);
}

/// How the thread draws to the canvas.
enum Painter {
    /// With the WebGL context of the canvas.
    WebGl {
        _context: Rc<dyn IGlContext>,
        render_target: Rc<dyn IGlPlatformSurfaceRenderTarget>,
        /// A framebuffer of the context that holds the square; `None` when
        /// the context cannot copy between framebuffers (WebGL 1), and the
        /// frames have no square then.
        marker: Option<i32>,
    },
    /// Into a framebuffer that the target retains and puts to the canvas.
    Software { render_target: Rc<dyn IFramebufferRenderTarget> },
}

/// What the worker thread keeps for as long as it lives: the render target
/// and what draws to it belong to that thread.
struct Canvas {
    _target: Arc<dyn BrowserRenderTarget>,
    painter: Painter,
}

thread_local! {
    /// The canvas surface, on the thread of the page.
    static SURFACE: RefCell<Option<CanvasSurface>> = const { RefCell::new(None) };
    /// The dispatcher backend of the thread of the page.
    static DISPATCHER: RefCell<Option<Rc<BrowserSingleThreadedDispatcherImpl>>> = const { RefCell::new(None) };
    /// The object of the page that is told the frame counter.
    static REPORTER: RefCell<Option<Reporter>> = const { RefCell::new(None) };
    /// How often the dispatcher of the thread of the page was woken.
    static WAKE_UPS: Cell<u32> = const { Cell::new(0) };
    /// The canvas of the worker thread.
    static CANVAS: RefCell<Option<Canvas>> = const { RefCell::new(None) };
}

#[cfg(target_os = "emscripten")]
mod native {
    extern "C" {
        fn emscripten_runtime_keepalive_push();
    }

    /// Keeps the calling thread when its start function returns: the thread
    /// does not exit, and its worker goes on receiving messages and calls.
    pub fn keep_thread_alive() {
        // SAFETY: the function takes no argument; it increments a counter of
        // the script of the calling thread.
        unsafe { emscripten_runtime_keepalive_push() }
    }
}

#[cfg(not(target_os = "emscripten"))]
mod native {
    /// There are no workers outside a web page.
    pub fn keep_thread_alive() {}
}

fn fail(message: String) {
    // The first failure is the one that is reported.
    let _ = FAILURE.set(message);
    STAGE.store(FAILED, Ordering::SeqCst);
}

fn shared() -> &'static Arc<BrowserSurfaceShared> {
    SHARED.get_or_init(BrowserSurfaceShared::new)
}

/// The render timer: one that ticks in the background, so setting its
/// callback does not start it; the thread that draws starts it on itself.
fn timer() -> &'static Arc<BrowserRenderTimer> {
    TIMER.get_or_init(|| BrowserRenderTimer::new(true))
}

/// The colour of the frame with the given number.
fn frame_color(frame: u32) -> [u8; 3] {
    if ANIMATED.load(Ordering::SeqCst) {
        [COLOR[0], COLOR[1], (ANIMATED_BLUE + frame % ANIMATED_BLUE_STEPS) as u8]
    } else {
        COLOR
    }
}

/// Starts the thread that will take the canvas. `width` and `height` are the
/// size of the canvas in device pixels and `scaling` the device pixel ratio,
/// until the observer of the canvas reports them. `mode` is a rendering mode
/// (1 `Software2D`, 2 `WebGL1`, 3 `WebGL2`) or 0 for WebGL 2 with WebGL 1 as
/// the second choice. With `animated` the thread runs a frame loop.
/// `reporter` is an object of the page whose `frames(frames, wakeUps)` is
/// called when the thread of the page is woken after a frame, or undefined.
/// The outcome is read with [`render_worker_clear_state`].
#[wasm_bindgen(js_name = renderWorkerClearStart)]
pub fn render_worker_clear_start(width: i32, height: i32, scaling: f64, mode: i32, animated: bool, reporter: JsValue) {
    if STAGE.compare_exchange(IDLE, STARTING, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return;
    }
    MAIN_THREAD.store(current_thread(), Ordering::SeqCst);
    REQUESTED_MODE.store(mode, Ordering::SeqCst);
    ANIMATED.store(animated, Ordering::SeqCst);

    // The size crosses to the other thread through the shared state: first
    // what the page measured, then what the observer of the canvas reports.
    // The subscription lasts as long as the page; its token is not kept.
    shared().set_size(PixelSize::new(width, height), scaling);
    add_size_changed(Rc::new(|top_level_id, pixel_width, pixel_height, dpr| {
        if top_level_id == NO_TOP_LEVEL {
            shared().on_size_changed(pixel_width, pixel_height, dpr);
        }
    }));

    // The dispatcher backend of this thread. Nothing dispatches through it
    // here: the example only listens to its signal, which the other thread
    // raises through the handle after each frame.
    let dispatcher = BrowserSingleThreadedDispatcherImpl::new();
    dispatcher.signaled().add(Rc::new(|()| report_frames()));
    let _ = WAKE_UP.set(dispatcher.signal_handle());
    DISPATCHER.with(|kept| *kept.borrow_mut() = Some(dispatcher));
    if !reporter.is_undefined() && !reporter.is_null() {
        REPORTER.with(|kept| *kept.borrow_mut() = Some(reporter.unchecked_into::<Reporter>()));
    }

    let spawned = std::thread::Builder::new().name("render_worker_clear".to_string()).spawn(move || {
        // The subscription lasts as long as the thread; its token is not kept.
        add_render_target_registered(Rc::new(on_render_target_registered));
        initialize_worker();
        // The start function returns below, and the thread has to go on: the
        // canvas arrives as a message to its worker, and so do its frames.
        native::keep_thread_alive();
        if animated {
            let timer = timer();
            timer.set_tick(Some(Arc::new(|_time: Duration| draw_next_frame())));
            timer.start_on_this_thread();
        }
        WORKER_THREAD.store(current_thread(), Ordering::SeqCst);
        let _ = STAGE.compare_exchange(STARTING, WORKER_READY, Ordering::SeqCst, Ordering::SeqCst);
    });
    // The handle is dropped: the thread is detached and nothing joins it.
    if let Err(error) = spawned {
        fail(format!("the thread could not start: {error}"));
    }
}

/// Creates the canvas in `container` and hands it to the worker thread.
/// Returns the id of the render target, or 0 when the thread is not ready.
#[wasm_bindgen(js_name = renderWorkerClearCreateSurface)]
pub fn render_worker_clear_create_surface(container: JsValue) -> i32 {
    // Before the canvas is created: the worker may report the target at once.
    if STAGE.compare_exchange(WORKER_READY, SURFACE_CREATED, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return 0;
    }
    let requested = REQUESTED_MODE.load(Ordering::SeqCst);
    let modes = if requested == 0 {
        vec![BrowserRenderingMode::WebGL2 as i32, BrowserRenderingMode::WebGL1 as i32]
    } else {
        vec![requested]
    };
    let thread = WORKER_THREAD.load(Ordering::SeqCst) as i32;
    // No top-level: the size changes the script reports reach the subscriber
    // of `renderWorkerClearStart`.
    let surface = CanvasSurface::create_render_target_surface(&container, &modes, NO_TOP_LEVEL, thread);
    let target_id = surface.target_id();
    shared().set_target_id(target_id);
    SURFACE.with(|kept| *kept.borrow_mut() = Some(surface));
    target_id
}

/// Runs on the worker thread when its worker has created the render target
/// of the transferred canvas: the thread wraps the target and draws the
/// first frame.
fn on_render_target_registered(target_id: i32, kind: i32) {
    TARGET_ID.store(target_id, Ordering::SeqCst);
    TARGET_KIND.store(kind, Ordering::SeqCst);
    shared().set_target_kind(kind);
    let software_asked = REQUESTED_MODE.load(Ordering::SeqCst) == BrowserRenderingMode::Software2D as i32;
    if kind != RENDER_TARGET_KIND_WEB_GL && !software_asked {
        fail("the worker created a software render target, not a WebGL one".to_string());
        return;
    }
    // The target asks for the size at the start of each frame and sets the
    // size of the canvas, which only this thread can do.
    let Some(target) = get_render_target(target_id, shared().size_getter()) else {
        fail(format!("the registry of the worker has no render target {target_id}"));
        return;
    };
    let surface = target.as_render_surface();
    let painter = if kind == RENDER_TARGET_KIND_WEB_GL {
        let context = target
            .platform_graphics_context()
            .and_then(|context| context.try_get_feature(TypeId::of::<dyn IGlContext>()))
            .and_then(|feature| feature.downcast_ref::<Rc<dyn IGlContext>>().cloned());
        let Some(context) = context else {
            fail("the render target has no OpenGL context".to_string());
            return;
        };
        let Some(gl_surface) = try_get_gl_surface(&*surface) else {
            fail("the render target is not an OpenGL surface".to_string());
            return;
        };
        GL_MAJOR_VERSION.store(context.version().major(), Ordering::SeqCst);
        let marker = create_marker(&context);
        Painter::WebGl { render_target: gl_surface.create_gl_render_target(&context), _context: context, marker }
    } else {
        let Some(framebuffer_surface) = surface.as_framebuffer_surface() else {
            fail("the render target is not a framebuffer surface".to_string());
            return;
        };
        Painter::Software { render_target: framebuffer_surface.create_framebuffer_render_target() }
    };
    CANVAS.with(|canvas| *canvas.borrow_mut() = Some(Canvas { _target: target, painter }));

    draw_next_frame();
    if FRAMES.load(Ordering::SeqCst) == 0 {
        fail("the first frame was not drawn: the canvas has no size".to_string());
        return;
    }
    let _ = STAGE.compare_exchange(SURFACE_CREATED, DONE, Ordering::SeqCst, Ordering::SeqCst);
}

/// Creates a framebuffer of the context that holds the square, for the
/// frames to copy from. `None` when the context cannot copy between
/// framebuffers or the framebuffer is not complete.
fn create_marker(context: &Rc<dyn IGlContext>) -> Option<i32> {
    let gl = context.gl_interface();
    if !gl.is_blit_framebuffer_available() {
        return None;
    }
    let restore = context.make_current();
    let framebuffer = gl.gen_framebuffer();
    let renderbuffer = gl.gen_renderbuffer();
    gl.bind_renderbuffer(gl_consts::GL_RENDERBUFFER, renderbuffer);
    gl.renderbuffer_storage(gl_consts::GL_RENDERBUFFER, gl_consts::GL_RGBA8, MARKER_SIZE, MARKER_SIZE);
    gl.bind_framebuffer(gl_consts::GL_FRAMEBUFFER, framebuffer);
    gl.framebuffer_renderbuffer(
        gl_consts::GL_FRAMEBUFFER,
        gl_consts::GL_COLOR_ATTACHMENT0,
        gl_consts::GL_RENDERBUFFER,
        renderbuffer,
    );
    let complete = gl.check_framebuffer_status(gl_consts::GL_FRAMEBUFFER) == gl_consts::GL_FRAMEBUFFER_COMPLETE;
    if complete {
        gl.viewport(0, 0, MARKER_SIZE, MARKER_SIZE);
        clear(&gl, MARKER_COLOR);
    }
    // The session of a frame binds the framebuffer of the canvas again.
    restore.dispose();
    complete.then_some(framebuffer)
}

fn clear(gl: &ferroui_opengl::GlInterface, color: [u8; 3]) {
    gl.clear_color(f32::from(color[0]) / 255.0, f32::from(color[1]) / 255.0, f32::from(color[2]) / 255.0, 1.0);
    gl.clear(gl_consts::GL_COLOR_BUFFER_BIT);
}

/// Draws one frame on the worker thread, when its canvas exists and has a
/// size, counts it, and tells the thread of the page.
fn draw_next_frame() {
    let drawn = CANVAS.with(|canvas| canvas.borrow().as_ref().is_some_and(draw));
    if !drawn {
        return;
    }
    FRAMES.fetch_add(1, Ordering::SeqCst);
    DRAWN_ON.store(current_thread(), Ordering::SeqCst);
    // For the thread that waits for a frame in the experiment.
    *FRAME_COUNT.lock().unwrap_or_else(PoisonError::into_inner) += 1;
    FRAME_DRAWN.notify_all();
    // The wake-up of the dispatcher of the thread of the page.
    if let Some(wake_up) = WAKE_UP.get() {
        wake_up.signal();
    }
}

/// One frame: the colour of its number from edge to edge and the square.
/// Returns whether it was drawn.
fn draw(canvas: &Canvas) -> bool {
    // The size as the thread of the page last wrote it. A canvas without a
    // size cannot be drawn to.
    let (size, scaling) = shared().size();
    if size.width <= 0 || size.height <= 0 {
        return false;
    }
    let color = frame_color(FRAMES.load(Ordering::SeqCst));
    let scene_info = RenderTargetSceneInfo::new(size, scaling, CompositionTransparencyLevel::None);
    let (drawn_size, has_marker) = match &canvas.painter {
        Painter::WebGl { render_target, marker, .. } => {
            // As a render backend draws a frame: the session sets the size
            // of the canvas, makes the context current and binds the
            // framebuffer of the canvas.
            let session = render_target.begin_draw(&scene_info);
            let size = session.size();
            let gl = session.context().gl_interface();
            gl.viewport(0, 0, size.width, size.height);
            clear(&gl, color);
            if let Some(marker) = marker {
                // The square is copied pixel for pixel; OpenGL counts rows
                // from the bottom.
                let top = size.height - MARKER_OFFSET;
                gl.bind_framebuffer(gl_consts::GL_READ_FRAMEBUFFER, *marker);
                gl.blit_framebuffer(
                    0,
                    0,
                    MARKER_SIZE,
                    MARKER_SIZE,
                    MARKER_OFFSET,
                    top - MARKER_SIZE,
                    MARKER_OFFSET + MARKER_SIZE,
                    top,
                    gl_consts::GL_COLOR_BUFFER_BIT,
                    gl_consts::GL_NEAREST,
                );
            }
            let error = gl.get_error();
            gl.flush();
            session.dispose();
            // The browser presents the canvas when the worker returns to its
            // event loop.
            if error != 0 {
                let _ = GL_ERROR.compare_exchange(0, error, Ordering::SeqCst, Ordering::SeqCst);
            }
            (size, marker.is_some())
        }
        Painter::Software { render_target } => {
            // The target sets the size of the canvas and hands out its
            // retained framebuffer, a new one when the size changed.
            let (framebuffer, _) = render_target.lock(&scene_info);
            let size = framebuffer.size();
            let row_bytes = framebuffer.row_bytes() as usize;
            let marker = MARKER_OFFSET..MARKER_OFFSET + MARKER_SIZE;
            framebuffer.with_data(&mut |data: &mut [u8]| {
                for (y, row) in data.chunks_exact_mut(row_bytes).enumerate() {
                    let in_marker_rows = marker.contains(&(y as i32));
                    for (x, pixel) in row.chunks_exact_mut(4).take(size.width as usize).enumerate() {
                        let color = if in_marker_rows && marker.contains(&(x as i32)) { MARKER_COLOR } else { color };
                        // Opaque, so premultiplied and straight are the same.
                        pixel.copy_from_slice(&[color[0], color[1], color[2], 255]);
                    }
                }
            });
            // Disposing the lock puts the pixels to the canvas.
            framebuffer.dispose();
            (size, true)
        }
    };
    DRAWN_WIDTH.store(drawn_size.width, Ordering::SeqCst);
    DRAWN_HEIGHT.store(drawn_size.height, Ordering::SeqCst);
    DRAWN_COLOR.store((u32::from(color[0]) << 16) | (u32::from(color[1]) << 8) | u32::from(color[2]), Ordering::SeqCst);
    DRAWN_MARKER.store(has_marker, Ordering::SeqCst);
    true
}

/// Runs on the thread of the page when its dispatcher is signalled: the
/// other thread drew a frame and woke it.
fn report_frames() {
    let wake_ups = WAKE_UPS.with(|wake_ups| {
        wake_ups.set(wake_ups.get() + 1);
        wake_ups.get()
    });
    REPORTER.with(|reporter| {
        if let Some(reporter) = reporter.borrow().as_ref() {
            reporter.frames(FRAMES.load(Ordering::SeqCst), wake_ups);
        }
    });
}

/// The frames drawn so far, read from the atomic the other thread counts
/// in: the count moves while the thread of the page is busy.
#[wasm_bindgen(js_name = renderWorkerClearFrames)]
pub fn render_worker_clear_frames() -> u32 {
    FRAMES.load(Ordering::SeqCst)
}

/// The experiment: the calling thread, which is the thread of the page,
/// waits on a condition variable until the other thread has drawn a frame,
/// for at most `timeout_ms` milliseconds. With `out_of_turn` it first asks
/// the render timer for a frame out of turn, as the compositor does before
/// a synchronous wait; without it the wait ends with the next animation
/// frame of the worker, if a worker has animation frames while the thread
/// of its page does not return to the browser.
///
/// Returns a line of `name=value` pairs: `ended` (whether a frame was drawn
/// before the timeout), `duration_ms`, `out_of_turn`, and the frame counter
/// before and after (`frames_before`, `frames_after`).
#[wasm_bindgen(js_name = renderWorkerClearWaitForFrame)]
pub fn render_worker_clear_wait_for_frame(out_of_turn: bool, timeout_ms: u32) -> String {
    let frames_before = FRAMES.load(Ordering::SeqCst);
    let start = Instant::now();
    let count = FRAME_COUNT.lock().unwrap_or_else(PoisonError::into_inner);
    let seen = *count;
    if out_of_turn {
        timer().request_tick_out_of_turn();
    }
    let (count, outcome) = FRAME_DRAWN
        .wait_timeout_while(count, Duration::from_millis(u64::from(timeout_ms)), |count| *count == seen)
        .unwrap_or_else(PoisonError::into_inner);
    drop(count);
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    format!(
        "ended={} duration_ms={duration_ms:.3} out_of_turn={out_of_turn} frames_before={frames_before} frames_after={}",
        !outcome.timed_out(),
        FRAMES.load(Ordering::SeqCst),
    )
}

/// The state as a line of `name=value` pairs: `atomics` (whether the module
/// was built with threads), `state` (`idle`, `starting`, `worker_ready`,
/// `surface_created`, `done` or `failed`), from `worker_ready` on the
/// `thread` that takes the canvas, and with `done` the `target`, its `kind`,
/// the major version of OpenGL ES (`gl`, 0 for a software target), the
/// first error of OpenGL after a frame (`gl_error`), the `size` and the
/// `color` of the last frame, whether it ran on a thread other than the one
/// of the page (`other_thread`), the number of `frames` and whether the
/// last one has the square (`marker`).
#[wasm_bindgen(js_name = renderWorkerClearState)]
pub fn render_worker_clear_state() -> String {
    let atomics = cfg!(target_feature = "atomics");
    let thread = WORKER_THREAD.load(Ordering::SeqCst);
    match STAGE.load(Ordering::SeqCst) {
        IDLE => format!("atomics={atomics} state=idle"),
        STARTING => format!("atomics={atomics} state=starting"),
        WORKER_READY => format!("atomics={atomics} state=worker_ready thread={thread}"),
        SURFACE_CREATED => format!("atomics={atomics} state=surface_created thread={thread}"),
        DONE => {
            let drawn_on = DRAWN_ON.load(Ordering::SeqCst);
            let other_thread = drawn_on != 0 && drawn_on != MAIN_THREAD.load(Ordering::SeqCst);
            let kind =
                if TARGET_KIND.load(Ordering::SeqCst) == RENDER_TARGET_KIND_WEB_GL { "webgl" } else { "software" };
            let color = DRAWN_COLOR.load(Ordering::SeqCst);
            format!(
                "atomics={atomics} state=done thread={thread} other_thread={other_thread} target={} kind={kind} gl={} gl_error={} size={}x{} color={},{},{} frames={} marker={}",
                TARGET_ID.load(Ordering::SeqCst),
                GL_MAJOR_VERSION.load(Ordering::SeqCst),
                GL_ERROR.load(Ordering::SeqCst),
                DRAWN_WIDTH.load(Ordering::SeqCst),
                DRAWN_HEIGHT.load(Ordering::SeqCst),
                (color >> 16) & 0xff,
                (color >> 8) & 0xff,
                color & 0xff,
                FRAMES.load(Ordering::SeqCst),
                DRAWN_MARKER.load(Ordering::SeqCst),
            )
        }
        _ => format!(
            "atomics={atomics} state=failed error={}",
            FAILURE.get().map_or("unknown", |message| message.as_str())
        ),
    }
}

/// Outside a web page there is no canvas and no worker to hand it to.
#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!(
        "render_worker_clear runs in a browser: build it with scripts/build-browser.sh render_worker_clear --threads"
    );
}
