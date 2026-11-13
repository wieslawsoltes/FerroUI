use super::{ISkiaGpu, ISkiaGpuRenderTarget, ISkiaGrContext, SkiaGpuBackend};
use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::skia_options::SkiaOptions;
use crate::skia_platform::SkiaPlatform;
use crate::text_rendering_settings::{TextRenderingSettings, TextRenderingSource};
use ferroui_base::platform::{
    IDrawingContextImpl, IRenderTarget, PlatformRenderTargetState, RenderTargetDrawingContextProperties,
    RenderTargetError, RenderTargetProperties, RenderTargetSceneInfo,
};
use skia_safe::{AlphaType, ImageInfo};
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
    /// The Skia backend of the context the frames of the target are drawn with, known from the
    /// first of them on (`clips_regions_per_draw`).
    backend: Cell<Option<SkiaGpuBackend>>,
}

impl SkiaGpuRenderTarget {
    /// Draws `paint` onto an offscreen surface of `gr_context` (the context
    /// of the frames), or of the recorder of a prewarm of its own when the
    /// context has one (Graphite), and compiles what was drawn: on a thread
    /// of its own for a prewarm recorder, by the flush of the context
    /// otherwise.
    fn prewarm_with(
        &self,
        gr_context: &Rc<dyn ISkiaGrContext>,
        scene_info: &RenderTargetSceneInfo,
        paint: &mut dyn FnMut(&mut dyn IDrawingContextImpl),
    ) -> bool {
        let (prewarm_context, finish): (Rc<dyn ISkiaGrContext>, Box<dyn FnOnce()>) = match gr_context.begin_prewarm() {
            Some(prewarm) => (prewarm.gr_context, prewarm.finish),
            None => {
                let flushed = gr_context.clone();
                let finish: Box<dyn FnOnce()> = Box::new(move || flushed.flush());
                (gr_context.clone(), finish)
            }
        };
        // The surfaces the scene of a frame is drawn on: the layers of the
        // composition target (`SurfaceRenderTarget`), of the native colour
        // type, premultiplied, without a colour space. The pipelines depend
        // on the format of what they draw onto.
        let text_rendering = self.frame_text_rendering.get();
        let info = ImageInfo::new(
            (scene_info.size.width.max(1), scene_info.size.height.max(1)),
            crate::helpers::pixel_format_helper::resolve_color_type(None),
            AlphaType::Premul,
            None,
        );
        let Some(surface) = prewarm_context.create_surface(&info, &text_rendering.surface_props()) else {
            return false;
        };
        {
            let nfo = CreateInfo {
                gr_context: Some(prewarm_context.clone()),
                surface: Some(surface),
                dpi: SkiaPlatform::default_dpi() * scene_info.scaling,
                scale_drawing_to_dpi: false,
                gpu: Some(self.skia_gpu.clone()),
                text_rendering,
                ..CreateInfo::default()
            };
            let mut context = DrawingContextImpl::new(nfo, Vec::new());
            paint(&mut context);
            context.dispose();
            // The context and its surface are released here: the surface
            // hands its draws to the recorder.
        }
        finish();
        true
    }

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
        Self {
            skia_gpu,
            render_target,
            text_rendering,
            frame_text_rendering: Rc::default(),
            backend: Cell::new(None),
        }
    }
}

/// Whether a context of `backend` applies a clip of several rectangles to every draw under it on
/// its own. Ganesh does: a region is the path of its boundary to it, which is no rectangle and
/// not convex, so each draw under it gets a mask of its own size, rasterised on the processor and
/// uploaded as a texture, or drawn into the stencil buffer (`ClipStack::apply`). A rectangle is
/// its scissor. Graphite draws the clip with the shader of the draw, and the raster backend has
/// regions of its own. A backend that is not known yet (before the first frame, which draws the
/// whole surface) is asked again with the next frame.
pub(crate) fn clips_regions_per_draw(backend: Option<SkiaGpuBackend>) -> bool {
    backend == Some(SkiaGpuBackend::Ganesh)
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
        self.backend.set(Some(gr_context.backend()));
        let mut context = DrawingContextImpl::new(nfo, vec![Box::new(move || session.dispose())]);
        // The programs of the runtime effects are made with the second frame
        // of a GPU context (the first paint is not delayed), not with the
        // frame that first needs them.
        if prewarm_frame(&gr_context) {
            context.prewarm_runtime_effects();
            // Box shadows use Skia's mask-filter path, not the registered
            // effect graphs. Compile its pipelines on a separate recorder,
            // before the first hover or connected animation needs them.
            if gr_context.backend() == SkiaGpuBackend::Graphite && prewarm_effect_pipelines() {
                let prewarm_scene = RenderTargetSceneInfo {
                    size: ferroui_base::PixelSize::new(512, 512),
                    ..scene_info.clone()
                };
                self.prewarm_with(&gr_context, &prewarm_scene, &mut prewarm_animation_pipelines);
            }
        }

        Ok((Box::new(context), RenderTargetDrawingContextProperties::default()))
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        self.render_target.state()
    }

    /// The frames of a Ganesh context draw one rectangle (desktop-performance.md, section 9.56).
    fn clips_regions_per_draw(&self) -> bool {
        clips_regions_per_draw(self.backend.get())
    }

    /// The layers compare their text rendering with the one of the frame
    /// (`SurfaceRenderTarget::is_corrupted`), which the drawing context of
    /// the target sets otherwise; nothing else of a rendering session is
    /// what a layer draws with. Which layers are drawn ahead is theirs to
    /// say: those that pipeline their frames.
    fn prepare_frame_ahead(&self, scene_info: &RenderTargetSceneInfo) -> bool {
        let text_rendering = self.text_rendering.settings(scene_info.scaling, scene_info.transparency_level);
        self.frame_text_rendering.set(text_rendering);
        true
    }

    /// Draws `paint` offscreen with the drawing context of the frames, so
    /// that the GPU makes its pipelines before a frame needs them (#437):
    /// on Graphite onto a recorder of its own, whose pipelines compile on a
    /// thread of its own; on Ganesh with the context of the frames, whose
    /// flush compiles the programs on this thread. Not when
    /// [`SkiaOptions::prewarm_effect_pipelines`] is off or the context is
    /// lost.
    fn prewarm(&self, scene_info: &RenderTargetSceneInfo, paint: &mut dyn FnMut(&mut dyn IDrawingContextImpl)) -> bool {
        // The Metal compiler of the iOS simulator rejects the destination
        // reads of the blend modes the effects use (gpu-rendering.md, "The
        // iOS simulator: no destination reads in shaders").
        if cfg!(all(target_os = "ios", target_abi = "sim")) || !prewarm_effect_pipelines() {
            return false;
        }
        let Some(scoped) = self.skia_gpu.try_get_gr_context() else { return false };
        let gr_context = scoped.value().clone();
        let prewarmed = !gr_context.is_lost() && self.prewarm_with(&gr_context, scene_info, paint);
        scoped.dispose();
        prewarmed
    }

    fn dispose(&self) {
        self.render_target.dispose();
    }
}

/// A connected flight scales its shadow nonuniformly when the source and
/// destination have different aspect ratios. SkCanvas::canAttemptBlurredRRectDraw
/// rejects that transform: the mask filter uses image-filter blur passes
/// instead, whose kernel-size buckets select different GPU programs.
pub(super) fn prewarm_animation_pipelines(context: &mut dyn IDrawingContextImpl) {
    use ferroui_base::media::imaging::BitmapInterpolationMode;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush};
    use ferroui_base::media::RenderOptions;
    use ferroui_base::media::{BoxShadow, BoxShadows, Color};
    use ferroui_base::platform::{
        AlphaFormat, IGeometryContext, IStreamGeometryBuilder, PixelFormat,
    };
    use ferroui_base::{Matrix, PixelSize, Point, Rect, RoundedRect, Vector};

    // A window containing paths uses Graphite's internal MSAA. The same
    // shaders on a single-sample target don't warm its Metal pipeline states.
    let mut path = crate::StreamGeometryBuilder::new(None);
    path.begin_figure(Point::new(8.0, 8.0), true);
    path.line_to(Point::new(500.0, 8.0), true);
    path.cubic_bezier_to(
        Point::new(200.0, 100.0),
        Point::new(200.0, 400.0),
        Point::new(500.0, 500.0),
        true,
    );
    path.line_to(Point::new(8.0, 500.0), true);
    path.end_figure(true);
    let geometry = path.to_geometry();
    let gradient = ImmutableLinearGradientBrush::from_stops(&[
        ImmutableGradientStop::new(0.0, Color::from_rgb(46, 134, 222)),
        ImmutableGradientStop::new(1.0, Color::from_rgb(63, 158, 90)),
    ]);
    context.push_opacity(0.5, None);
    context.draw_geometry(Some(&gradient), None, &*geometry);
    context.pop_opacity();

    for (x, y) in [(1.0, 1.0), (1.5, 1.25)] {
        // Fractional bounds keep the AA geometry in the alpha-mask pass;
        // integer-aligned rectangles can take a different, unantialiased path.
        context.set_transform(Matrix::create_translation(0.25, 0.5) * Matrix::create_scale(x, y));
        for blur in [0.1, 0.5, 1.0, 2.0, 4.0, 8.0, 12.0, 16.0, 24.0] {
            let shadows = BoxShadows::new(BoxShadow {
                blur,
                color: Color::from_argb(102, 0, 0, 0),
                offset_y: 4.0,
                ..BoxShadow::default()
            });
            context.draw_rectangle(
                None,
                None,
                // Skia emits an additional unclamped interior blur shader
                // only when its safe interior exceeds 128 * 128 pixels.
                RoundedRect::from_rect(Rect::new(48.0, 48.0, 240.0, 160.0)),
                &shadows,
            );
        }
    }
    // The frozen source and the live destination cross-fade at fractional
    // offsets/scales. An opaque, unscaled image doesn't warm these programs.
    let Ok(bitmap) = crate::ImmutableBitmap::from_pixels(
        PixelSize::new(4, 4),
        Vector::new(96.0, 96.0),
        16,
        PixelFormat::BGRA8888,
        AlphaFormat::Premul,
        &[128; 64],
    ) else {
        return;
    };
    context.set_transform(Matrix::create_translation(0.25, 0.5) * Matrix::create_scale(1.5, 1.25));
    let fill = ImmutableSolidColorBrush::new(Color::from_rgb(46, 134, 222));
    for opacity in [1.0, 0.5] {
        context.push_opacity(opacity, None);
        context.draw_rectangle(
            Some(&fill),
            None,
            RoundedRect::from_radius(Rect::new(24.0, 24.0, 64.0, 48.0), 12.0),
            &BoxShadows::default(),
        );
        context.pop_opacity();
    }
    for interpolation in [
        BitmapInterpolationMode::None,
        BitmapInterpolationMode::LowQuality,
    ] {
        context.push_render_options(RenderOptions {
            bitmap_interpolation_mode: interpolation,
            ..RenderOptions::default()
        });
        // Integer-aligned draws use a non-AA render step, distinct from
        // the fractional cross-fade draws, including at a flight's endpoint.
        for transform in [
            Matrix::IDENTITY,
            Matrix::create_translation(0.25, 0.5) * Matrix::create_scale(1.5, 1.25),
        ] {
            context.set_transform(transform);
            for opacity in [1.0, 0.5] {
                for source in [Rect::new(0.0, 0.0, 4.0, 4.0), Rect::new(1.0, 1.0, 2.0, 2.0)] {
                    context.draw_bitmap(
                        &bitmap,
                        opacity,
                        source,
                        Rect::new(24.0, 24.0, 64.0, 48.0),
                    );
                }
            }
        }
        context.pop_render_options();
    }
}

/// [`SkiaOptions::prewarm_effect_pipelines`] of the options in force on
/// this thread ([`SkiaOptions::current`]); on by default.
fn prewarm_effect_pipelines() -> bool {
    SkiaOptions::current().prewarm_effect_pipelines
}

ferroui_base::ferro_thread_local! {
    /// The GPU contexts of this thread and how many frames each drew.
    static FRAMES: std::cell::RefCell<Vec<(std::rc::Weak<dyn super::ISkiaGrContext>, usize)>> =
        std::cell::RefCell::new(Vec::new());
}

/// Whether the frame of `gr_context` that is beginning is the one that
/// prewarms the runtime effects: its second frame. The first paint of the
/// application is not delayed, and an application draws its next frames
/// while it starts (what it loads, its animations), before a user scrolls.
/// Rendered frames are the only time a render target has: a frame drawn
/// only to prewarm would need the compositor to ask for it.
pub(crate) fn prewarm_frame(gr_context: &Rc<dyn super::ISkiaGrContext>) -> bool {
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
        frame == 1
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
    fn the_prewarm_runs_with_the_second_frame_of_a_context() {
        let context: Rc<dyn ISkiaGrContext> = Rc::new(NoContext);
        let other: Rc<dyn ISkiaGrContext> = Rc::new(NoContext);
        let frames: Vec<_> = (0..5).map(|_| prewarm_frame(&context)).collect();
        assert_eq!(vec![false, true, false, false, false], frames, "not the first paint, once");
        // Each context prewarms its own programs.
        assert!(!prewarm_frame(&other));
        assert!(prewarm_frame(&other));
    }

    /// Section 9.56: only a Ganesh context applies a clip of several rectangles to every draw on
    /// its own, so only its frames are drawn as one rectangle. Graphite keeps the rectangles of
    /// the frame, and so does a target whose context is not known yet.
    #[test]
    fn only_a_ganesh_context_clips_regions_per_draw() {
        assert!(clips_regions_per_draw(Some(SkiaGpuBackend::Ganesh)));
        assert!(!clips_regions_per_draw(Some(SkiaGpuBackend::Graphite)));
        assert!(!clips_regions_per_draw(None));
    }
}
