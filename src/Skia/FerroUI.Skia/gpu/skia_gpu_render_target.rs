use super::{ISkiaGpu, ISkiaGpuRenderTarget};
use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::skia_platform::SkiaPlatform;
use crate::text_rendering_settings::{TextRenderingSettings, TextRenderingSource};
use ferroui_base::platform::{
    IDrawingContextImpl, IRenderTarget, PlatformRenderTargetState, RenderTargetDrawingContextProperties,
    RenderTargetError, RenderTargetProperties, RenderTargetSceneInfo,
};
use std::cell::Cell;
use std::rc::Rc;

/// Adapts a GPU render target to the platform render target contract.
pub struct SkiaGpuRenderTarget {
    skia_gpu: Rc<dyn ISkiaGpu>,
    render_target: Rc<dyn ISkiaGpuRenderTarget>,
    text_rendering: TextRenderingSource,
    /// The text rendering of the frame drawn last, shared with the layers created for it: the
    /// composition target keeps a layer between frames, whose surface properties are those of
    /// the frame it was created in, and creates it again once they differ.
    frame_text_rendering: Rc<Cell<TextRenderingSettings>>,
}

impl SkiaGpuRenderTarget {
    /// Creates a render target that renders with `skia_gpu` to
    /// `render_target`, with the default text rendering of the backend.
    pub fn new(skia_gpu: Rc<dyn ISkiaGpu>, render_target: Rc<dyn ISkiaGpuRenderTarget>) -> Self {
        Self::with_text_rendering(skia_gpu, render_target, TextRenderingSource::default())
    }

    /// Creates a render target that renders with `skia_gpu` to
    /// `render_target` and reads the text rendering parameters of the
    /// platform from `text_rendering` every frame.
    pub fn with_text_rendering(
        skia_gpu: Rc<dyn ISkiaGpu>,
        render_target: Rc<dyn ISkiaGpuRenderTarget>,
        text_rendering: TextRenderingSource,
    ) -> Self {
        Self { skia_gpu, render_target, text_rendering, frame_text_rendering: Rc::default() }
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
        let text_rendering = self.text_rendering.settings(scene_info.scaling, scene_info.transparency_level);
        self.frame_text_rendering.set(text_rendering);
        let session = self.render_target.try_begin_rendering_session(scene_info, &text_rendering.surface_props())?;

        let nfo = CreateInfo {
            gr_context: Some(session.gr_context()),
            surface: Some(session.sk_surface()),
            dpi: SkiaPlatform::default_dpi() * session.scale_factor(),
            scale_drawing_to_dpi: false,
            gpu: Some(self.skia_gpu.clone()),
            current_session: Some(session.clone()),
            text_rendering,
            frame_text_rendering: Some(self.frame_text_rendering.clone()),
            ..CreateInfo::default()
        };

        let gr_context = session.gr_context();
        let mut context = DrawingContextImpl::new(nfo, vec![Box::new(move || session.dispose())]);
        // The programs of the runtime effects are made a stage a frame from
        // the second frame of a GPU context on, not with the frame that
        // first needs them.
        if let Some(stage) = prewarm_stage(&gr_context) {
            context.prewarm_runtime_effects_stage(stage);
        }

        Ok((Box::new(context), RenderTargetDrawingContextProperties::default()))
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        self.render_target.state()
    }

    fn dispose(&self) {
        self.render_target.dispose();
    }
}

ferroui_base::ferro_thread_local! {
    /// The GPU contexts of this thread and how many frames each drew.
    static FRAMES: std::cell::RefCell<Vec<(std::rc::Weak<dyn super::ISkiaGrContext>, usize)>> =
        std::cell::RefCell::new(Vec::new());
}

/// The stage of the prewarm of the runtime effects the frame of
/// `gr_context` that is beginning runs: none on its first frame (the first
/// paint of the application is not delayed), stage `n - 1` on its frame
/// `n`, none once every stage ran.
pub(crate) fn prewarm_stage(gr_context: &Rc<dyn super::ISkiaGrContext>) -> Option<usize> {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        frames.retain(|(context, _)| context.strong_count() > 0);
        let index = match frames.iter().position(|(context, _)| std::ptr::addr_eq(context.as_ptr(), Rc::as_ptr(gr_context))) {
            Some(index) => index,
            None => {
                frames.push((Rc::downgrade(gr_context), 0));
                frames.len() - 1
            }
        };
        let frame = frames[index].1;
        frames[index].1 = frame.saturating_add(1);
        (1..=DrawingContextImpl::PREWARM_STAGES).contains(&frame).then(|| frame - 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::{ISkiaGrContext, SkiaGpuBackend};
    use skia_safe::{Image, ImageInfo, Surface, SurfaceProps};

    struct NoContext;

    impl ISkiaGrContext for NoContext {
        fn backend(&self) -> SkiaGpuBackend {
            SkiaGpuBackend::Graphite
        }
        fn create_surface(&self, _image_info: &ImageInfo, _surface_props: &SurfaceProps) -> Option<Surface> {
            None
        }
        fn max_render_target_size(&self) -> Option<i32> {
            None
        }
        fn flush(&self) {}
        fn reset_context(&self) {}
        fn set_resource_cache_limit(&self, _max_resource_bytes: i64) {}
        fn snapshot_to_raster(&self, _surface: &mut Surface) -> Option<Image> {
            None
        }
        fn is_lost(&self) -> bool {
            false
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn the_prewarm_runs_a_stage_a_frame_from_the_second_frame_of_a_context() {
        let context: Rc<dyn ISkiaGrContext> = Rc::new(NoContext);
        let other: Rc<dyn ISkiaGrContext> = Rc::new(NoContext);
        let stages: Vec<_> = (0..DrawingContextImpl::PREWARM_STAGES + 3).map(|_| prewarm_stage(&context)).collect();
        assert_eq!(None, stages[0], "the first paint is not delayed");
        for (stage, scheduled) in stages[1..=DrawingContextImpl::PREWARM_STAGES].iter().enumerate() {
            assert_eq!(Some(stage), *scheduled);
        }
        assert!(stages[DrawingContextImpl::PREWARM_STAGES + 1..].iter().all(Option::is_none));
        // Each context has stages of its own.
        assert_eq!(None, prewarm_stage(&other));
        assert_eq!(Some(0), prewarm_stage(&other));
    }
}
