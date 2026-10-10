//! The surface of a window in the DXGI swap chain mode: an OpenGL surface
//! whose render target presents through a swap chain of the window.

use super::dxgi_connection::DxgiConnection;
use super::dxgi_render_target::DxgiRenderTarget;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::IPlatformGraphicsContext;
use ferroui_opengl::egl::{EglContext, EglGlPlatformSurfaceBase, IEglWindowGlPlatformSurfaceInfo};
use ferroui_opengl::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget};
use ferroui_opengl::IGlContext;
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::Arc;

/// The surface. It is shared between the threads; all it holds is the
/// connection and the window, so the view a thread asks for is a surface
/// of its own over the same two. The render target it creates belongs to
/// the thread that renders, with the context.
pub(crate) struct DxgiSwapchainWindow {
    connection: Arc<DxgiConnection>,
    window: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
}

impl DxgiSwapchainWindow {
    pub fn new(connection: Arc<DxgiConnection>, window: Arc<dyn IEglWindowGlPlatformSurfaceInfo>) -> Arc<DxgiSwapchainWindow> {
        Arc::new(DxgiSwapchainWindow { connection, window })
    }
}

impl IPlatformRenderSurface for DxgiSwapchainWindow {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IGlPlatformSurface>() {
            let this: Rc<dyn IGlPlatformSurface> =
                Rc::new(DxgiSwapchainWindow { connection: self.connection.clone(), window: self.window.clone() });
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EglGlPlatformSurfaceBase for DxgiSwapchainWindow {}

impl IGlPlatformSurface for DxgiSwapchainWindow {
    /// # Panics
    /// Panics when `context` is not an EGL context (the failing cast of
    /// the reference) and when the swap chain cannot be created (the
    /// exception of the reference).
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        let Some(egl_context) = context.as_any().downcast_ref::<EglContext>() else {
            panic!("Unable to cast the context to type 'EglContext'.");
        };
        let current = egl_context.ensure_current();
        let target = DxgiRenderTarget::new(self.window.clone(), egl_context, self.connection.clone());
        current.dispose();
        match target {
            Ok(target) => Rc::new(target),
            Err(error) => panic!("{error}"),
        }
    }
}
