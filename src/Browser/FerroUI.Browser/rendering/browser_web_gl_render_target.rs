use super::web_render_target::update_size;
use super::{BrowserSurfaceShared, RenderStatistics};
use crate::interop::canvas_helper::{GlInfo, RENDER_TARGET_KIND_WEB_GL};
use crate::interop::{thread_proxy, JsObject};
use ferroui_base::platform::surfaces::IPlatformRenderSurfaceRenderTarget;
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphicsContext, RenderTargetSceneInfo};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::PixelSize;
use ferroui_opengl::gl_consts;
use ferroui_opengl::surfaces::{IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
use ferroui_opengl::{GetProcAddress, GlInterface, GlProfileType, GlVersion, IGlContext, OpenGlException};
use ferroui_skia::gpu::open_gl::IGlSkiaSpecificOptionsFeature;
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::ffi::c_void;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::thread::{self, ThreadId};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// The WebGL render target of the script side.
    type JsWebGlRenderTarget;

    #[wasm_bindgen(method, getter, js_name = contextHandle)]
    fn context_handle(this: &JsWebGlRenderTarget) -> i32;

    #[wasm_bindgen(method, getter, js_name = fboId)]
    fn fbo_id(this: &JsWebGlRenderTarget) -> u32;

    #[wasm_bindgen(method, getter)]
    fn stencil(this: &JsWebGlRenderTarget) -> i32;

    #[wasm_bindgen(method, getter)]
    fn sample(this: &JsWebGlRenderTarget) -> i32;

    #[wasm_bindgen(method, getter)]
    fn depth(this: &JsWebGlRenderTarget) -> i32;

    #[wasm_bindgen(method, getter)]
    fn attrs(this: &JsWebGlRenderTarget) -> JsWebGlAttributes;

    /// The attributes a WebGL context was created with.
    type JsWebGlAttributes;

    #[wasm_bindgen(method, getter, js_name = majorVersion)]
    fn major_version(this: &JsWebGlAttributes) -> i32;

    #[wasm_bindgen(js_namespace = WebGlRenderTarget, js_name = getCurrentContext)]
    fn get_current_context() -> i32;

    #[wasm_bindgen(js_namespace = WebGlRenderTarget, js_name = makeContextCurrent)]
    fn make_context_current(context: i32) -> bool;
}

/// A canvas that is rendered to with WebGL, as the thread that draws to it
/// holds it: the object of the script side of that thread, what the script
/// knows about the WebGL context, and the context itself.
///
/// Upstream's class is also the render surface of the canvas. Here the
/// surface is [`BrowserRenderSurface`](super::BrowserRenderSurface), which
/// both threads hold and which holds nothing of a thread; this object stays
/// in the table of the thread that renders
/// ([`get_render_target`](super::get_render_target)) and is what the surface
/// resolves there.
pub struct BrowserWebGlRenderTarget {
    js: JsObject,
    gl_info: GlInfo,
    gl_context: Rc<WebGlContext>,
}

impl BrowserWebGlRenderTarget {
    /// Wraps the WebGL render target of the script side.
    ///
    /// The thread that calls this is the one the target belongs to.
    pub fn new(js: JsObject) -> Rc<Self> {
        let target = JsCast::unchecked_ref::<JsWebGlRenderTarget>(&js);
        let gl_info = GlInfo {
            context_id: target.context_handle(),
            fbo_id: target.fbo_id(),
            stencils: target.stencil(),
            samples: target.sample(),
            depth: target.depth(),
        };
        let context_id = target.context_handle();
        let version = target.attrs().major_version();
        let gl_context = WebGlContext::new(
            context_id,
            GlVersion::new(GlProfileType::OpenGLES, if version > 1 { 3 } else { 2 }, 0),
            gl_info.samples,
            gl_info.stencils,
        );
        Rc::new(Self { js, gl_info, gl_context })
    }

    /// The WebGL context of the canvas.
    pub fn gl_context(&self) -> Rc<WebGlContext> {
        self.gl_context.clone()
    }

    /// Creates the OpenGL render target of the canvas. `shared` is where
    /// each frame reads the size the canvas has to have.
    pub fn create_gl_render_target(
        self: &Rc<Self>,
        shared: Arc<BrowserSurfaceShared>,
    ) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        Rc::new(GlSurface { target: self.clone(), shared })
    }

    fn update_size(&self, size: PixelSize) {
        update_size(&self.js, size);
    }
}

struct GlSession {
    restore_context: Cell<Option<Rc<dyn IDisposable>>>,
    context: Rc<dyn IGlContext>,
    size: PixelSize,
    scaling: f64,
    shared: Arc<BrowserSurfaceShared>,
}

impl IGlPlatformSurfaceRenderingSession for GlSession {
    fn context(&self) -> Rc<dyn IGlContext> {
        self.context.clone()
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    // This should technically be delivered via the scaling of the composition target anyway, why do we
    // still have this property
    fn scaling(&self) -> f64 {
        self.scaling
    }

    fn is_y_flipped(&self) -> bool {
        false
    }

    fn dispose(&self) {
        if let Some(restore_context) = self.restore_context.take() {
            // The frame is drawn: the backend has flushed it before it ends
            // the session. Not from upstream: where frames are drawn is
            // otherwise invisible to the thread of the page.
            RenderStatistics::frame_presented(
                thread_proxy::current_thread(),
                RENDER_TARGET_KIND_WEB_GL,
                self.context.version().major(),
                self.size.width,
                self.size.height,
            );
            restore_context.dispose();
            // Whoever waits for the first frame of this canvas is told.
            self.shared.frame_presented();
        }
    }
}

struct GlSurface {
    target: Rc<BrowserWebGlRenderTarget>,
    shared: Arc<BrowserSurfaceShared>,
}

impl IPlatformRenderSurfaceRenderTarget for GlSurface {}

impl IGlPlatformSurfaceRenderTarget for GlSurface {
    fn begin_draw(&self, _scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        // The size as the thread of the user interface last wrote it. The
        // canvas is given that size here, by the thread that draws to it:
        // for a canvas that was transferred to a worker no other can.
        let (size, scaling) = self.shared.size();
        self.target.update_size(size);
        let restore_context = self.target.gl_context.ensure_current();
        self.target
            .gl_context
            .gl_interface()
            .bind_framebuffer(gl_consts::GL_FRAMEBUFFER, self.target.gl_info.fbo_id as i32);
        Rc::new(GlSession {
            restore_context: Cell::new(Some(restore_context)),
            context: self.target.gl_context.clone(),
            size,
            scaling,
            shared: self.shared.clone(),
        })
    }

    fn dispose(&self) {
        // No-op
    }
}

#[cfg(target_os = "emscripten")]
mod native {
    use std::ffi::{c_char, c_void, CString};

    extern "C" {
        fn emscripten_GetProcAddress(name: *const c_char) -> *const c_void;
    }

    /// The address of the entry point of the WebGL emulation of the module,
    /// or null.
    pub(super) fn get_proc_address(name: &str) -> *const c_void {
        let Ok(name) = CString::new(name) else { return std::ptr::null() };
        // SAFETY: the function reads the NUL-terminated name, which outlives
        // the call, and returns the address of an entry point or null.
        unsafe { emscripten_GetProcAddress(name.as_ptr()) }
    }
}

#[cfg(not(target_os = "emscripten"))]
mod native {
    use std::ffi::c_void;

    /// There is no WebGL emulation outside the module: nothing resolves.
    pub(super) fn get_proc_address(_name: &str) -> *const c_void {
        std::ptr::null()
    }
}

/// A WebGL context registered with the module.
pub struct WebGlContext {
    this: Weak<WebGlContext>,
    context_id: i32,
    thread: ThreadId,
    version: GlVersion,
    gl_interface: Rc<GlInterface>,
    sample_count: i32,
    stencil_size: i32,
}

struct RestoreContext {
    context_id: Cell<Option<i32>>,
}

impl IDisposable for RestoreContext {
    fn dispose(&self) {
        if let Some(context_id) = self.context_id.take() {
            make_context_current(context_id);
        }
    }
}

impl WebGlContext {
    /// Wraps the context with the given id and resolves its entry points.
    pub fn new(context_id: i32, version: GlVersion, sample_count: i32, stencil_size: i32) -> Rc<Self> {
        let old = get_current_context();
        if !make_context_current(context_id) {
            panic!("{}", OpenGlException::new("Unable to make the context current"));
        }
        let get_proc_address: GetProcAddress = Rc::new(|name: &str| -> *const c_void { native::get_proc_address(name) });
        // SAFETY: in the WebAssembly module the loader resolves names in the
        // WebGL emulation the module is linked with, whose entry points have
        // the signatures of OpenGL ES under the calling convention of the
        // target; anywhere else it returns null for every name, which the
        // contract allows (and construction then fails on the first required
        // entry point). The context was just made current on this thread,
        // which is the one the context is used on from here on
        // (`verify_access`). The previous context is restored below: every
        // later use of the interface happens inside `ensure_current`, which
        // `begin_draw` and the Skia GPU take before they call into it.
        let gl_interface = Rc::new(unsafe { GlInterface::new(version, get_proc_address) });
        make_context_current(old);

        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            context_id,
            thread: thread::current().id(),
            version,
            gl_interface,
            sample_count,
            stencil_size,
        })
    }

    fn verify_access(&self) {
        if self.thread != thread::current().id() {
            panic!("Call from invalid thread");
        }
    }
}

impl IOptionalFeatureProvider for WebGlContext {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IGlContext>() {
            let this: Rc<dyn IGlContext> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }
        if feature_type == TypeId::of::<dyn IGlSkiaSpecificOptionsFeature>() {
            let this: Rc<dyn IGlSkiaSpecificOptionsFeature> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsContext for WebGlContext {
    // TODO: Implement
    fn is_lost(&self) -> bool {
        false
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.verify_access();
        if get_current_context() == self.context_id {
            return Disposable::empty();
        }
        self.make_current()
    }

    fn dispose(&self) {
        // No-op, destroyed with the render target
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IGlContext for WebGlContext {
    fn version(&self) -> GlVersion {
        self.version
    }

    fn gl_interface(&self) -> Rc<GlInterface> {
        self.gl_interface.clone()
    }

    fn sample_count(&self) -> i32 {
        self.sample_count
    }

    fn stencil_size(&self) -> i32 {
        self.stencil_size
    }

    fn make_current(&self) -> Rc<dyn IDisposable> {
        self.verify_access();
        let old = get_current_context();
        if !make_context_current(self.context_id) {
            panic!("{}", OpenGlException::new("Unable to make the context current"));
        }
        Rc::new(RestoreContext { context_id: Cell::new(Some(old)) })
    }

    fn is_shared_with(&self, _context: &dyn IGlContext) -> bool {
        false
    }

    fn can_create_shared_context(&self) -> bool {
        false
    }

    fn create_shared_context(&self, _preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>> {
        panic!("Specified method is not supported.");
    }
}

impl IGlSkiaSpecificOptionsFeature for WebGlContext {
    fn use_native_skia_gr_gl_interface(&self) -> bool {
        true
    }
}
