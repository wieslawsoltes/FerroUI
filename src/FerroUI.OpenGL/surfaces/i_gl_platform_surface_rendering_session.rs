use crate::IGlContext;
use ferroui_base::platform::IPartialPresentationFeature;
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

    /// The partial presentation of the frame, when the render target presents partially
    /// (upstream `IGlPlatformSurfaceRenderingSessionWithPartialPresentation`, 070c3be990). A
    /// renderer that takes it declares the dirty rectangles and says when the drawing
    /// succeeded; a frame whose feature is not taken is presented whole. `None` by default.
    fn partial_presentation(&self) -> Option<Rc<dyn IPartialPresentationFeature>> {
        None
    }

    /// Finishes and presents the frame.
    fn dispose(&self);
}
