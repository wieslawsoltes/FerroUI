use super::ISkiaGpuRenderSession;
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetSceneInfo};
use std::rc::Rc;

/// Custom Skia render target.
pub trait ISkiaGpuRenderTarget {
    /// Starts rendering to this render target.
    fn begin_rendering_session(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn ISkiaGpuRenderSession>;

    /// The state of the platform render target.
    fn state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::READY
    }

    /// Releases the render target.
    fn dispose(&self);
}
