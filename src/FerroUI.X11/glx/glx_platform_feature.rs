//! The platform graphics of GLX (the port of `Glx/GlxPlatformFeature.cs`).

use super::glx_display::GlxDisplay;
use crate::x11_info::X11Info;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
use ferroui_opengl::GlVersion;
use std::rc::Rc;
use std::sync::Arc;

pub struct GlxPlatformGraphics {
    display: Arc<GlxDisplay>,
}

impl GlxPlatformGraphics {
    pub fn display(&self) -> &Arc<GlxDisplay> {
        &self.display
    }

    pub fn can_create_contexts(&self) -> bool {
        true
    }

    pub fn can_share_contexts(&self) -> bool {
        true
    }

    pub fn new(display: Arc<GlxDisplay>) -> Self {
        Self { display }
    }

    pub fn try_create(x11: &X11Info, gl_profiles: &[GlVersion]) -> Option<GlxPlatformGraphics> {
        match GlxDisplay::new(x11, gl_profiles) {
            Ok(disp) => Some(GlxPlatformGraphics::new(disp)),
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    logger.log_with_values(None, "Unable to initialize GLX-based rendering: {0}", &[&e]);
                }
                None
            }
        }
    }
}

impl IPlatformGraphics for GlxPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        false
    }

    /// # Panics
    /// Panics when the context cannot be created (the exception of the
    /// reference).
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        match self.display.create_context() {
            Ok(context) => context,
            Err(error) => panic!("{error}"),
        }
    }

    /// # Panics
    /// Always: the graphics have no shared context (the
    /// `NotSupportedException` of the reference).
    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Specified method is not supported.")
    }
}
