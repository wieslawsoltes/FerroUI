use super::android_framebuffer::{AndroidFramebuffer, DiscardedFramebuffer};
use super::invalidation_aware_surface_view::SurfaceShared;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::surfaces::{
    FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::platform::ILockedFramebuffer;
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

/// The software render surface of a top-level: frames are drawn into the
/// locked buffer of the native window.
///
/// The surface is shared with the thread that renders, so it holds what the
/// surface view publishes of itself, not the top-level.
pub(crate) struct FramebufferManager {
    surface: Arc<SurfaceShared>,
}

impl FramebufferManager {
    pub fn new(surface: Arc<SurfaceShared>) -> Self {
        Self { surface }
    }

    fn lock(surface: &SurfaceShared) -> Rc<dyn ILockedFramebuffer> {
        let scaling = surface.scaling();
        let framebuffer = surface.native_window().and_then(|window| AndroidFramebuffer::new(&window, scaling));
        match framebuffer {
            Some(framebuffer) => Rc::new(framebuffer),
            None => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::ANDROID_PLATFORM) {
                    logger.log(None, "Unable to obtain ANativeWindow: the frame is discarded");
                }
                Rc::new(DiscardedFramebuffer::new(scaling))
            }
        }
    }
}

impl IPlatformRenderSurface for FramebufferManager {
    fn is_ready(&self) -> bool {
        self.surface.has_native_window()
    }

    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for FramebufferManager {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let surface = self.surface.clone();
        Rc::new(FuncFramebufferRenderTarget::new(move || Self::lock(&surface)))
    }
}
