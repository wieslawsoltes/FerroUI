use super::EglDisplay;
use crate::OpenGlException;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
use std::rc::Rc;

/// The platform graphics of an EGL display: every context is a new context of the display.
pub struct EglPlatformGraphics {
    display: Rc<EglDisplay>,
}

impl EglPlatformGraphics {
    pub fn new(display: Rc<EglDisplay>) -> Self {
        Self { display }
    }

    pub fn display(&self) -> &Rc<EglDisplay> {
        &self.display
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
    /// Panics when the context cannot be created (the exception of the original).
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        match self.display.create_context(None) {
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
