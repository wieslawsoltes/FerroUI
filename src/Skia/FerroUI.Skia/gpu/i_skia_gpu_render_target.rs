use super::ISkiaGpuRenderSession;
use ferroui_base::platform::{
    PlatformRenderTargetState, RenderTargetError, RenderTargetProperties, RenderTargetSceneInfo,
};
use skia_safe::SurfaceProps;
use std::rc::Rc;

/// Custom Skia render target.
pub trait ISkiaGpuRenderTarget {
    /// The properties of the target (upstream 9b3e9c881b): the rotation a GL surface mounted in
    /// another orientation needs, reported by its platform surface. The default has none.
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties::default()
    }

    /// Starts rendering to this render target. The Skia surface of the
    /// session is created with `surface_props` (the text rendering of the
    /// frame, docs/porting/text-rendering.md).
    fn begin_rendering_session(
        &self,
        scene_info: &RenderTargetSceneInfo,
        surface_props: &SurfaceProps,
    ) -> Rc<dyn ISkiaGpuRenderSession>;

    /// Starts rendering to this render target, or says that the target is
    /// not ready or is corrupted (the two exceptions the compositor of the
    /// reference catches). The render target of the backend calls this
    /// member. A target that cannot report either keeps the default.
    fn try_begin_rendering_session(
        &self,
        scene_info: &RenderTargetSceneInfo,
        surface_props: &SurfaceProps,
    ) -> Result<Rc<dyn ISkiaGpuRenderSession>, RenderTargetError> {
        Ok(self.begin_rendering_session(scene_info, surface_props))
    }

    /// The state of the platform render target.
    fn state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::READY
    }

    /// Releases the render target.
    fn dispose(&self);
}
