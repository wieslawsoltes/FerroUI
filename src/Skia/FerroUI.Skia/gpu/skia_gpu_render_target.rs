use super::{ISkiaGpu, ISkiaGpuRenderTarget};
use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::skia_platform::SkiaPlatform;
use ferroui_base::platform::{
    IDrawingContextImpl, IRenderTarget, PlatformRenderTargetState, RenderTargetDrawingContextProperties,
    RenderTargetError, RenderTargetProperties, RenderTargetSceneInfo,
};
use std::rc::Rc;

/// Adapts a GPU render target to the platform render target contract.
pub struct SkiaGpuRenderTarget {
    skia_gpu: Rc<dyn ISkiaGpu>,
    render_target: Rc<dyn ISkiaGpuRenderTarget>,
}

impl SkiaGpuRenderTarget {
    /// Creates a render target that renders with `skia_gpu` to
    /// `render_target`.
    pub fn new(skia_gpu: Rc<dyn ISkiaGpu>, render_target: Rc<dyn ISkiaGpuRenderTarget>) -> Self {
        Self { skia_gpu, render_target }
    }
}

impl IRenderTarget for SkiaGpuRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties::default()
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        match self.try_create_drawing_context(scene_info) {
            Ok(created) => created,
            Err(error) => panic!("{error}"),
        }
    }

    fn try_create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<(Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties), RenderTargetError> {
        let session = self.render_target.try_begin_rendering_session(scene_info)?;

        let nfo = CreateInfo {
            gr_context: Some(session.gr_context()),
            surface: Some(session.sk_surface()),
            dpi: SkiaPlatform::default_dpi() * session.scale_factor(),
            scale_drawing_to_dpi: false,
            gpu: Some(self.skia_gpu.clone()),
            current_session: Some(session.clone()),
            ..CreateInfo::default()
        };

        let context = DrawingContextImpl::new(nfo, vec![Box::new(move || session.dispose())]);

        Ok((Box::new(context), RenderTargetDrawingContextProperties::default()))
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        self.render_target.state()
    }

    fn dispose(&self) {
        self.render_target.dispose();
    }
}
