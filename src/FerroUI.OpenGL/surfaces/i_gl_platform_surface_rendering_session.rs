use crate::IGlContext;
use ferroui_base::PixelSize;
use std::rc::Rc;

/// One frame being rendered to an OpenGL surface. Disposing the session
/// presents the frame.
pub trait IGlPlatformSurfaceRenderingSession {
    /// The context the frame is rendered with.
    fn context(&self) -> Rc<dyn IGlContext>;

    /// The size of the framebuffer in device pixels.
    fn size(&self) -> PixelSize;

    /// The scaling from logical units to device pixels.
    fn scaling(&self) -> f64;

    /// Whether the first row of the framebuffer is its top row.
    fn is_y_flipped(&self) -> bool;

    /// Finishes and presents the frame.
    fn dispose(&self);
}
