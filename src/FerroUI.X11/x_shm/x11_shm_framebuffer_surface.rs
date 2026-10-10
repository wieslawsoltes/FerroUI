//! The software render surface of a window over shared memory (the port
//! of `XShm/X11ShmFramebufferSurface.cs`).

use super::x11_shm_framebuffer_render_target::X11ShmFramebufferRenderTarget;
use crate::x11_deferred_display_dispatcher::X11DeferredDisplayDispatcher;
use crate::xlib::{VisualPointer, XDisplay, XID};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::surfaces::{IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface};
use std::any::Any;
use std::rc::Rc;

/// A window of the server as a target of software rendering through the
/// shared memory extension.
///
/// The surface is shared with the thread that renders, so it holds the
/// connection, the identifier of the window and its visual. The
/// dispatcher of the completion events, which the reference hands to the
/// surface, is the one of the thread that creates the render target.
pub struct X11ShmFramebufferSurface {
    deferred_display: XDisplay,
    window_handle: XID,
    visual: VisualPointer,
    depth: i32,
}

impl X11ShmFramebufferSurface {
    pub fn new(deferred_display: XDisplay, window_handle: XID, visual: VisualPointer, depth: i32) -> Self {
        Self { deferred_display, window_handle, visual, depth }
    }
}

impl IPlatformRenderSurface for X11ShmFramebufferSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for X11ShmFramebufferSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::X11_PLATFORM) {
            logger.log(None, "[X11ShmFramebufferSurface] CreateFramebufferRenderTarget");
        }

        Rc::new(X11ShmFramebufferRenderTarget::new(
            self.deferred_display,
            self.window_handle,
            self.visual,
            self.depth,
            X11DeferredDisplayDispatcher::for_current_thread(self.deferred_display),
        ))
    }
}
