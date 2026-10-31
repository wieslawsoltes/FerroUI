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
        // The programs of the runtime effects are made with the first frame
        // of a GPU context, not with the frame that first needs them.
        if first_frame_of(&gr_context) {
            context.prewarm_runtime_effects();
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
    /// The GPU contexts whose runtime effects were prewarmed.
    static PREWARMED: std::cell::RefCell<Vec<std::rc::Weak<dyn super::ISkiaGrContext>>> =
        std::cell::RefCell::new(Vec::new());
}

/// Whether this is the first frame of `gr_context` on this thread: true
/// once per context.
pub(crate) fn first_frame_of(gr_context: &Rc<dyn super::ISkiaGrContext>) -> bool {
    PREWARMED.with(|prewarmed| {
        let mut prewarmed = prewarmed.borrow_mut();
        prewarmed.retain(|context| context.strong_count() > 0);
        if prewarmed.iter().any(|context| std::ptr::addr_eq(context.as_ptr(), Rc::as_ptr(gr_context))) {
            return false;
        }
        prewarmed.push(Rc::downgrade(gr_context));
        true
    })
}
