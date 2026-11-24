use super::IGlPlatformSurfaceRenderingSession;
use ferroui_base::platform::surfaces::IPlatformRenderSurfaceRenderTarget;
use ferroui_base::platform::{RenderTargetError, RenderTargetProperties, RenderTargetSceneInfo};
use std::rc::Rc;

/// The render target of an OpenGL surface.
pub trait IGlPlatformSurfaceRenderTarget: IPlatformRenderSurfaceRenderTarget {
    /// Starts rendering a frame.
    fn begin_draw(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession>;

    /// Starts rendering a frame, or says that the target is not ready or is
    /// corrupted: what the reference reports by throwing
    /// `RenderTargetNotReadyException` or `RenderTargetCorruptedException`
    /// out of `BeginDraw`, and its compositor catches. A renderer calls
    /// this member. A target that cannot report either keeps the default.
    fn try_begin_draw(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, RenderTargetError> {
        Ok(self.begin_draw(scene_info))
    }

    /// The properties of the target the compositor needs (upstream 9b3e9c881b): mostly the
    /// rotation of a framebuffer mounted in another orientation than the scene, which the
    /// compositor applies when it draws its layer into the target. The default has none.
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties::default()
    }

    /// Releases the render target.
    fn dispose(&self);
}
