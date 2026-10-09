use crate::scene::{IVelloSceneSink, VelloSceneBrush, VelloSceneCapabilities, VelloScenePaint};
use crate::vello_options::VelloRenderingMode;
use kurbo::{Affine, BezPath, Stroke};
use peniko::color::AlphaColor;
use peniko::{BlendMode, Fill, ImageBrush, ImageSampler};
use std::collections::HashMap;
use vello_cpu::{
    ImageSource, PaintType, PixmapMut, RasterizerSettings, RenderContext, RenderSettings, Resources, TargetInit,
};

/// The coverage from which a pixel of an edge without anti-aliasing is
/// painted: half of it.
const ALIASING_THRESHOLD: u8 = 128;

/// The scene of the CPU mode: the render context of `vello_cpu`, the
/// sparse-strips renderer on the processor.
pub struct VelloCpuSceneSink {
    context: RenderContext,
    resources: Resources,
    /// The images of the frame in the form the renderer samples, by the
    /// identity of their pixels: an image that is drawn many times (a
    /// tile) is converted once.
    images: HashMap<u64, ImageSource>,
}

impl VelloCpuSceneSink {
    /// Creates the scene of a target of the given size.
    ///
    /// The scene is rendered on the thread that draws: the renderer's own
    /// thread pool is not compiled in (see the workspace manifest).
    pub fn new(width: u16, height: u16) -> Self {
        let settings = RenderSettings { num_threads: 0, ..RenderSettings::default() };

        Self { context: RenderContext::new_with(width, height, settings), resources: Resources::new(), images: HashMap::new() }
    }

    fn set_anti_alias(&mut self, anti_alias: bool) {
        self.context.set_aliasing_threshold(if anti_alias { None } else { Some(ALIASING_THRESHOLD) });
    }

    fn set_paint(&mut self, paint: &VelloScenePaint) {
        let paint_type: PaintType = match &paint.brush {
            VelloSceneBrush::Solid(color) => PaintType::Solid(*color),
            VelloSceneBrush::Gradient(gradient) => PaintType::Gradient(gradient.clone()),
            VelloSceneBrush::Image(image) => {
                let source = self
                    .images
                    .entry(image.image.data.id())
                    .or_insert_with(|| ImageSource::from_peniko_image_data(&image.image))
                    .clone();

                PaintType::Image(ImageBrush {
                    image: source,
                    sampler: ImageSampler {
                        x_extend: image.x_extend,
                        y_extend: image.y_extend,
                        quality: image.quality,
                        alpha: image.alpha,
                    },
                })
            }
        };

        self.context.set_paint(paint_type);
        self.context.set_paint_transform(paint.transform);
    }
}

impl IVelloSceneSink for VelloCpuSceneSink {
    fn rendering_mode(&self) -> VelloRenderingMode {
        VelloRenderingMode::Cpu
    }

    fn capabilities(&self) -> VelloSceneCapabilities {
        VelloSceneCapabilities { blend_layers: true, aliased_edges: true, image_paints: true, read_back: true }
    }

    fn width(&self) -> u16 {
        self.context.width()
    }

    fn height(&self) -> u16 {
        self.context.height()
    }

    fn reset(&mut self) {
        self.context.reset();
        self.images.clear();
    }

    fn fill(
        &mut self,
        path: &BezPath,
        fill_rule: Fill,
        transform: Affine,
        paint: &VelloScenePaint,
        blend_mode: BlendMode,
        anti_alias: bool,
    ) {
        self.set_anti_alias(anti_alias);
        self.set_paint(paint);
        self.context.set_transform(transform);
        self.context.set_fill_rule(fill_rule);
        self.context.set_blend_mode(blend_mode);
        self.context.fill_path(path);
        self.context.set_blend_mode(BlendMode::default());
    }

    fn stroke(&mut self, path: &BezPath, stroke: &Stroke, transform: Affine, paint: &VelloScenePaint, anti_alias: bool) {
        self.set_anti_alias(anti_alias);
        self.set_paint(paint);
        self.context.set_transform(transform);
        self.context.set_stroke(stroke.clone());
        self.context.stroke_path(path);
    }

    fn push_clip(&mut self, path: &BezPath, fill_rule: Fill, transform: Affine, anti_alias: bool) {
        self.set_anti_alias(anti_alias);
        self.context.set_transform(transform);
        self.context.set_fill_rule(fill_rule);
        self.context.push_clip_path(path);
    }

    fn pop_clip(&mut self) {
        self.context.pop_clip();
    }

    fn push_layer(&mut self, blend_mode: BlendMode, opacity: f32) {
        self.context.push_layer(None, Some(blend_mode), Some(opacity), None, None);
    }

    fn pop_layer(&mut self) {
        self.context.pop_layer();
    }

    fn render_to_pixels(&mut self, pixels: &mut [u8]) {
        let (width, height) = (self.context.width(), self.context.height());
        let target = PixmapMut::new(width, height, pixels)
            .unwrap_or_else(|| panic!("The pixels of the target do not have the size of the scene"));

        let settings =
            RasterizerSettings { target_init: TargetInit::Clear(AlphaColor::TRANSPARENT), ..RasterizerSettings::default() };

        self.context.flush();
        self.context.render_with(target, &mut self.resources, settings);
    }
}
