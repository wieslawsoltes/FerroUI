//! A window that is rendered to with a context of GLX (the port of
//! `Glx/GlxGlPlatformSurface.cs`).

use super::glx_context::GlxContext;
use crate::xlib;
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

/// A window that is rendered to with a context of GLX.
///
/// The surface is shared between the threads; all it holds is what the
/// window publishes of itself, so it is the same surface wherever it is
/// used. The render target it creates belongs to the thread that renders,
/// with the context.
pub struct GlxGlPlatformSurface {
    info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
}

impl GlxGlPlatformSurface {
    pub fn new(info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>) -> Arc<GlxGlPlatformSurface> {
        Arc::new(Self { info })
    }
}

impl IPlatformRenderSurface for GlxGlPlatformSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IGlPlatformSurface>() {
            // The view is a handle of the calling thread, the surface a handle shared between
            // the threads: the view is a surface of its own over the same window, which is all
            // the state of a surface.
            let this: Rc<dyn IGlPlatformSurface> = Rc::new(Self { info: self.info.clone() });
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IGlPlatformSurface for GlxGlPlatformSurface {
    /// # Panics
    /// Panics when `context` is not a context of GLX (the failing cast of
    /// the reference).
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        let Some(glx_context) = context.as_any().downcast_ref::<GlxContext>() else {
            panic!("Unable to cast the context to type 'GlxContext'.");
        };
        Rc::new(RenderTarget { context: glx_context.this(), info: self.info.clone(), last_size: Cell::new(None) })
    }
}

struct RenderTarget {
    context: Rc<GlxContext>,
    info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
    last_size: Cell<Option<PixelSize>>,
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
    fn begin_draw(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        let size = scene_info.size;
        let deferred_display = self.context.display().deferred_display();
        //if (expectedSize.HasValue)
        {
            xlib::x_configure_resize_window(deferred_display, self.info.handle() as _, size.width, size.height);
            xlib::x_flush(deferred_display);

            if self.last_size.get() != Some(size) {
                xlib::x_sync(deferred_display, true);
                self.last_size.set(Some(size));
            }
            self.context.glx().wait_x();
        }

        let old_context = match self.context.make_current_with(self.info.handle() as _) {
            Ok(old_context) => old_context,
            Err(error) => panic!("{error}"),
        };

        // Reset to default FBO first
        self.context.gl_interface().bind_framebuffer(GL_FRAMEBUFFER, 0);

        Rc::new(Session {
            context: self.context.clone(),
            info: self.info.clone(),
            size: Some(size),
            clear_context: old_context,
            disposed: Cell::new(false),
        })
    }

    fn dispose(&self) {
        // No-op
    }
}

struct Session {
    context: Rc<GlxContext>,
    info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
    size: Option<PixelSize>,
    clear_context: Rc<dyn IDisposable>,
    disposed: Cell<bool>,
}

impl IGlPlatformSurfaceRenderingSession for Session {
    fn context(&self) -> Rc<dyn IGlContext> {
        self.context.clone()
    }

    fn size(&self) -> PixelSize {
        self.size.unwrap_or_else(|| self.info.size())
    }

    fn scaling(&self) -> f64 {
        self.info.scaling()
    }

    fn is_y_flipped(&self) -> bool {
        false
    }

    fn dispose(&self) {
        // A frame is presented once: a second call would swap the buffers again.
        if self.disposed.replace(true) {
            return;
        }
        self.context.gl_interface().flush();
        self.context.glx().wait_gl();
        self.context.display().swap_buffers(self.info.handle() as _);
        self.context.glx().wait_x();
        self.clear_context.dispose();
    }
}
