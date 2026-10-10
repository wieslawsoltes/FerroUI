//! A window that is rendered to with a context of WGL (the port of
//! `OpenGl/WglGlPlatformSurface.cs`).

use super::wgl_context::WglContext;
use super::wgl_gdi_resource_manager::WglGdiResourceManager;
use crate::interop::unmanaged_methods::swap_buffers;
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use ferroui_opengl::egl::IEglWindowGlPlatformSurfaceInfo;
use ferroui_opengl::gl_consts::GL_FRAMEBUFFER;
use ferroui_opengl::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
use ferroui_opengl::IGlContext;
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

/// A window that is rendered to with a context of WGL.
///
/// The surface is shared between the threads; all it holds is what the
/// window publishes of itself, so it is the same surface wherever it is
/// used. The render target it creates belongs to the thread that renders,
/// with the context.
pub struct WglGlPlatformSurface {
    info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
}

impl WglGlPlatformSurface {
    pub fn new(info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>) -> Arc<WglGlPlatformSurface> {
        Arc::new(Self { info })
    }
}

impl IPlatformRenderSurface for WglGlPlatformSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IGlPlatformSurface>() {
            // The view is a handle of the calling thread, the surface a
            // handle shared between the threads: the view is a surface of
            // its own over the same window, which is all the state of a
            // surface.
            let this: Rc<dyn IGlPlatformSurface> = Rc::new(Self { info: self.info.clone() });
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IGlPlatformSurface for WglGlPlatformSurface {
    /// # Panics
    /// Panics when `context` is not a context of WGL (the failing cast of
    /// the reference).
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        let Some(wgl_context) = context.as_any().downcast_ref::<WglContext>() else {
            panic!("Unable to cast the context to type 'WglContext'.");
        };
        let context = wgl_context.this();
        let hdc = context.create_configured_device_context(self.info.handle());
        Rc::new(RenderTarget { context, info: self.info.clone(), hdc: Cell::new(hdc) })
    }
}

struct RenderTarget {
    context: Rc<WglContext>,
    info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
    hdc: Cell<isize>,
}

impl IPlatformRenderSurfaceRenderTarget for RenderTarget {
    fn state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::READY
    }
}

impl IGlPlatformSurfaceRenderTarget for RenderTarget {
    /// # Panics
    /// Panics when the context cannot be made current with the window (the
    /// exception of the reference).
    fn begin_draw(&self, _scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        // TODO: use expectedPixelSize
        let old_context = match self.context.make_current_with(self.hdc.get()) {
            Ok(old_context) => old_context,
            Err(error) => panic!("{error}"),
        };

        // Reset to default FBO first
        self.context.gl_interface().bind_framebuffer(GL_FRAMEBUFFER, 0);

        Rc::new(Session {
            context: self.context.clone(),
            hdc: self.hdc.get(),
            info: self.info.clone(),
            clear_context: old_context,
            disposed: Cell::new(false),
        })
    }

    fn dispose(&self) {
        // The device context is given back once.
        let hdc = self.hdc.replace(0);
        if hdc != 0 {
            WglGdiResourceManager::release_dc(self.info.handle(), hdc);
        }
    }
}

struct Session {
    context: Rc<WglContext>,
    hdc: isize,
    info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
    clear_context: Rc<dyn IDisposable>,
    disposed: Cell<bool>,
}

impl IGlPlatformSurfaceRenderingSession for Session {
    fn context(&self) -> Rc<dyn IGlContext> {
        self.context.clone()
    }

    fn size(&self) -> PixelSize {
        self.info.size()
    }

    fn scaling(&self) -> f64 {
        self.info.scaling()
    }

    fn is_y_flipped(&self) -> bool {
        false
    }

    fn dispose(&self) {
        // A frame is presented once: a second call would swap the buffers
        // again.
        if self.disposed.replace(true) {
            return;
        }
        self.context.gl_interface().flush();
        swap_buffers(self.hdc);
        self.clear_context.dispose();
    }
}
