use super::ISkiaGpuRenderSession;
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetError, RenderTargetSceneInfo};
use std::rc::Rc;

/// Custom Skia render target.
pub trait ISkiaGpuRenderTarget {
    /// Starts rendering to this render target.
    fn begin_rendering_session(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn ISkiaGpuRenderSession>;

    /// Starts rendering to this render target, or says that the target is
    /// not ready or is corrupted (the two exceptions the compositor of the
    /// reference catches). The render target of the backend calls this
    /// member. A target that cannot report either keeps the default.
    fn try_begin_rendering_session(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn ISkiaGpuRenderSession>, RenderTargetError> {
        Ok(self.begin_rendering_session(scene_info))
    }

    /// The state of the platform render target.
    fn state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::READY
    }

    /// Releases the render target.
    fn dispose(&self);
}
