//! One frame from a worker, without the compositor: step B2.1 of
//! `docs/porting/browser-render-worker.md`
//! (`scripts/build-browser.sh render_worker_clear --threads`).
//!
//! The page starts a thread (`renderWorkerClearStart`). The thread makes its
//! worker take transferred canvases (the worker handler of the render target
//! registry of the script side), publishes its id and returns to the event
//! loop of its worker without ending. The page then creates a canvas surface
//! with that id (`renderWorkerClearCreateSurface`): the script transfers the
//! control of the canvas to the worker, the worker creates the WebGL render
//! target and reports it, and the thread wraps the target as the browser
//! backend does, makes its context current, binds its framebuffer and clears
//! it to one colour.
//!
//! The main thread never waits: the host page asks for the state from a
//! timer (`renderWorkerClearState`) and writes it into the element `result`,
//! where the behaviour test
//! (`scripts/browser/tests/render_worker_clear.test.mjs`) reads it. The
//! picture itself is checked by the test in a capture of the page.
//!
//! In a build without threads the example compiles and reports that the
//! thread could not start.

#![cfg_attr(target_os = "emscripten", no_main)]

use ferroui_base::platform::RenderTargetSceneInfo;
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::PixelSize;
use ferroui_browser::interop::canvas_helper::{add_render_target_registered, CanvasSurface, RENDER_TARGET_KIND_WEB_GL};
use ferroui_browser::rendering::{get_render_target, initialize_worker, BrowserRenderTarget, CanvasSize};
use ferroui_browser::BrowserRenderingMode;
use ferroui_opengl::gl_consts;
use ferroui_opengl::surfaces::try_get_gl_surface;
use ferroui_opengl::IGlContext;
use std::any::TypeId;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use wasm_bindgen::prelude::*;

/// The colour the canvas is cleared to, as the test reads it in the capture.
const COLOR: [u8; 3] = [32, 96, 192];

// The stages of the example, in the order they are reached.
const IDLE: i32 = 0;
const STARTING: i32 = 1;
const WORKER_READY: i32 = 2;
const SURFACE_CREATED: i32 = 3;
const DONE: i32 = 4;
const FAILED: i32 = 5;

// What the two threads share. Everything is an atomic or is written once: the
// main thread only reads it from a timer and never waits for the other one.
static STAGE: AtomicI32 = AtomicI32::new(IDLE);
/// The thread that called `renderWorkerClearStart`.
static MAIN_THREAD: AtomicUsize = AtomicUsize::new(0);
/// The thread that takes the canvas; 0 until it has installed its handler.
static WORKER_THREAD: AtomicUsize = AtomicUsize::new(0);
/// The size of the canvas in device pixels, given by the page.
static CANVAS_WIDTH: AtomicI32 = AtomicI32::new(0);
static CANVAS_HEIGHT: AtomicI32 = AtomicI32::new(0);
/// The thread the clear ran on.
static CLEARED_ON: AtomicUsize = AtomicUsize::new(0);
static TARGET_ID: AtomicI32 = AtomicI32::new(0);
static TARGET_KIND: AtomicI32 = AtomicI32::new(0);
static GL_MAJOR_VERSION: AtomicI32 = AtomicI32::new(0);
static GL_ERROR: AtomicI32 = AtomicI32::new(0);
static FAILURE: OnceLock<String> = OnceLock::new();

/// What the worker thread keeps for as long as it lives: the render target
/// and its context belong to it.
struct Frame {
    _target: Arc<dyn BrowserRenderTarget>,
    _context: Rc<dyn IGlContext>,
}

thread_local! {
    /// The canvas surface, on the thread of the page.
    static SURFACE: RefCell<Option<CanvasSurface>> = const { RefCell::new(None) };
    /// The frames of the worker thread.
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
}

#[cfg(target_os = "emscripten")]
mod native {
    extern "C" {
        fn pthread_self() -> usize;
        fn emscripten_runtime_keepalive_push();
    }

    /// The id of the calling thread: the key of its worker in the table of
    /// threads of the module.
    pub fn current_thread() -> usize {
        // SAFETY: the function takes no argument and returns the address of
        // the descriptor of the calling thread.
        unsafe { pthread_self() }
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
    pub fn current_thread() -> usize {
        0
    }

    pub fn keep_thread_alive() {}
}

fn fail(message: String) {
    // The first failure is the one that is reported.
    let _ = FAILURE.set(message);
    STAGE.store(FAILED, Ordering::SeqCst);
}

/// Starts the thread that will take the canvas. `width` and `height` are the
/// size of the canvas in device pixels. The outcome is read with
/// [`render_worker_clear_state`].
#[wasm_bindgen(js_name = renderWorkerClearStart)]
pub fn render_worker_clear_start(width: i32, height: i32) {
    if STAGE.compare_exchange(IDLE, STARTING, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return;
    }
    MAIN_THREAD.store(native::current_thread(), Ordering::SeqCst);
    CANVAS_WIDTH.store(width, Ordering::SeqCst);
    CANVAS_HEIGHT.store(height, Ordering::SeqCst);
    let spawned = std::thread::Builder::new().name("render_worker_clear".to_string()).spawn(|| {
        // The subscription lasts as long as the thread; its token is not kept.
        add_render_target_registered(Rc::new(clear_render_target));
        initialize_worker();
        // The start function returns below, and the thread has to go on: the
        // canvas arrives as a message to its worker.
        native::keep_thread_alive();
        WORKER_THREAD.store(native::current_thread(), Ordering::SeqCst);
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
    let modes = [BrowserRenderingMode::WebGL2 as i32, BrowserRenderingMode::WebGL1 as i32];
    let thread = WORKER_THREAD.load(Ordering::SeqCst) as i32;
    // No top-level: the size changes the script reports find none and are dropped.
    let surface = CanvasSurface::create_render_target_surface(&container, &modes, 0, thread);
    let target_id = surface.target_id();
    SURFACE.with(|kept| *kept.borrow_mut() = Some(surface));
    target_id
}

/// Runs on the worker thread when its worker has created the render target
/// of the transferred canvas: one frame, which is a clear.
fn clear_render_target(target_id: i32, kind: i32) {
    TARGET_ID.store(target_id, Ordering::SeqCst);
    TARGET_KIND.store(kind, Ordering::SeqCst);
    if kind != RENDER_TARGET_KIND_WEB_GL {
        fail("the worker created a software render target, not a WebGL one".to_string());
        return;
    }
    let size_getter: CanvasSize =
        Rc::new(|| (PixelSize::new(CANVAS_WIDTH.load(Ordering::SeqCst), CANVAS_HEIGHT.load(Ordering::SeqCst)), 1.0));
    let Some(target) = get_render_target(target_id, size_getter.clone()) else {
        fail(format!("the registry of the worker has no render target {target_id}"));
        return;
    };
    let context = target
        .platform_graphics_context()
        .and_then(|context| context.try_get_feature(TypeId::of::<dyn IGlContext>()))
        .and_then(|feature| feature.downcast_ref::<Rc<dyn IGlContext>>().cloned());
    let Some(context) = context else {
        fail("the render target has no OpenGL context".to_string());
        return;
    };
    let Some(surface) = try_get_gl_surface(&*target.as_render_surface()) else {
        fail("the render target is not an OpenGL surface".to_string());
        return;
    };

    // The frame, as a render backend draws one: the session sets the size of
    // the canvas, makes the context current and binds the framebuffer.
    let (size, scaling) = size_getter();
    let render_target = surface.create_gl_render_target(&context);
    let session =
        render_target.begin_draw(&RenderTargetSceneInfo::new(size, scaling, CompositionTransparencyLevel::None));
    let gl = session.context().gl_interface();
    gl.viewport(0, 0, size.width, size.height);
    gl.clear_color(f32::from(COLOR[0]) / 255.0, f32::from(COLOR[1]) / 255.0, f32::from(COLOR[2]) / 255.0, 1.0);
    gl.clear(gl_consts::GL_COLOR_BUFFER_BIT);
    let error = gl.get_error();
    gl.flush();
    session.dispose();
    // The browser presents the canvas when the worker returns to its event loop.

    GL_MAJOR_VERSION.store(context.version().major(), Ordering::SeqCst);
    GL_ERROR.store(error, Ordering::SeqCst);
    CLEARED_ON.store(native::current_thread(), Ordering::SeqCst);
    FRAMES.with(|frames| frames.borrow_mut().push(Frame { _target: target, _context: context }));
    let _ = STAGE.compare_exchange(SURFACE_CREATED, DONE, Ordering::SeqCst, Ordering::SeqCst);
}

/// The state as a line of `name=value` pairs: `atomics` (whether the module
/// was built with threads), `state` (`idle`, `starting`, `worker_ready`,
/// `surface_created`, `done` or `failed`), from `worker_ready` on the
/// `thread` that takes the canvas, and with `done` the `target`, its `kind`,
/// the major version of OpenGL ES (`gl`), the error of the clear
/// (`gl_error`), the `size`, the `color` and whether the clear ran on a
/// thread other than the one of the page (`other_thread`).
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
            let cleared_on = CLEARED_ON.load(Ordering::SeqCst);
            let other_thread = cleared_on != 0 && cleared_on != MAIN_THREAD.load(Ordering::SeqCst);
            let kind =
                if TARGET_KIND.load(Ordering::SeqCst) == RENDER_TARGET_KIND_WEB_GL { "webgl" } else { "software" };
            format!(
                "atomics={atomics} state=done thread={thread} other_thread={other_thread} target={} kind={kind} gl={} gl_error={} size={}x{} color={},{},{}",
                TARGET_ID.load(Ordering::SeqCst),
                GL_MAJOR_VERSION.load(Ordering::SeqCst),
                GL_ERROR.load(Ordering::SeqCst),
                CANVAS_WIDTH.load(Ordering::SeqCst),
                CANVAS_HEIGHT.load(Ordering::SeqCst),
                COLOR[0],
                COLOR[1],
                COLOR[2],
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
