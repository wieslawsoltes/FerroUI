use super::EglDisplay;
use crate::OpenGlException;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
use ferroui_base::utilities::ThreadBound;
use std::rc::Rc;

/// The platform graphics of an EGL display: every context is a new context of the display.
///
/// The platform graphics are shared by threads (`IPlatformGraphics: Send + Sync`), and the
/// display is not: it is an `Rc` whose contexts each hold a handle to it, and the EGL
/// objects behind it were not read for use by two threads. So the display stays bound to
/// the thread that created the platform graphics: that thread creates the contexts, and a
/// call from another thread panics instead of touching the display. A compositor that
/// renders on another thread needs a display that may be shared before it can use these
/// graphics; no platform creates them yet.
pub struct EglPlatformGraphics {
    display: ThreadBound<Rc<EglDisplay>>,
}

impl EglPlatformGraphics {
    pub fn new(display: Rc<EglDisplay>) -> Self {
        Self { display: ThreadBound::new(display) }
    }

    /// # Panics
    /// Panics on a thread other than the one that created the platform graphics.
    pub fn display(&self) -> &Rc<EglDisplay> {
        self.display.get()
    }

    /// Creates the platform graphics of the display `display_factory` creates; a failure of
    /// the factory is logged and gives `None`.
    ///
    /// The overload of the original without arguments creates the default display of the
    /// EGL library of the system, which it loads by name: that constructor of the interface
    /// is not ported, a platform passes a factory that creates the display with its
    /// interface.
    pub fn try_create(
        display_factory: impl FnOnce() -> Result<Rc<EglDisplay>, OpenGlException>,
    ) -> Option<EglPlatformGraphics> {
        match display_factory() {
            Ok(display) => Some(EglPlatformGraphics::new(display)),
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    logger.log_with_values(None, "Unable to initialize EGL-based rendering: {0}", &[&e]);
                }
                None
            }
        }
    }
}

impl IPlatformGraphics for EglPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        false
    }

    /// # Panics
    /// Panics when the context cannot be created (the exception of the original), and on a
    /// thread other than the one that created the platform graphics.
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        match self.display().create_context(None) {
            Ok(context) => context,
            Err(error) => panic!("{error}"),
        }
    }

    /// # Panics
    /// Always: the graphics have no shared context (the `NotSupportedException` of the
    /// original).
    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Specified method is not supported.")
    }
}
