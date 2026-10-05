use super::IGlPlatformSurfaceRenderingSession;
use ferroui_base::platform::surfaces::IPlatformRenderSurfaceRenderTarget;
use ferroui_base::platform::RenderTargetSceneInfo;
use std::rc::Rc;

/// The render target of an OpenGL surface.
pub trait IGlPlatformSurfaceRenderTarget: IPlatformRenderSurfaceRenderTarget {
    /// Starts rendering a frame.
    fn begin_draw(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession>;

    /// Releases the render target.
    fn dispose(&self);
}
