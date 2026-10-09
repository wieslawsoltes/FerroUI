use crate::scene::{
    IVelloSceneSink, VelloSceneBrush, VelloSceneCapabilities, VelloSceneFilter, VelloSceneFilterCapabilities,
    VelloSceneGlyphRun, VelloScenePaint, VelloScenePixelRect,
};
use crate::vello_options::VelloRenderingMode;
use glifo::{FontEmbolden, Glyph};
use kurbo::{Affine, BezPath, Diagonal2, PathEl, Stroke, StrokeOpts};
use peniko::color::{AlphaColor, Srgb};
use peniko::{BlendMode, Fill, ImageBrush, ImageSampler};
use std::borrow::Cow;
use std::collections::HashMap;
use vello_cpu::filter_effects::{EdgeMode, Filter, FilterPrimitive};
use vello_cpu::{
    ImageSource, PaintType, PixmapMut, RasterizerSettings, RenderContext, RenderSettings, Resources, TargetInit,
};

/// The coverage from which a pixel of an edge without anti-aliasing is
/// painted: half of it.
pub(super) const ALIASING_THRESHOLD: u8 = 128;

/// The distance, in pixels of the target, a curve may be from the lines it
/// is drawn as.
///
/// The renderer flattens curves itself, to a quarter of a pixel
/// (`vello_common`, `flatten.rs`): the lines are chords, so a round shape
/// is drawn up to a quarter of a pixel thinner than it is, and its edge is
/// visibly lighter than the same edge drawn by Skia (design document,
/// section 1.3). Curves are therefore flattened here, five times finer, and
/// the renderer is given lines.
pub(super) const CURVE_TOLERANCE: f64 = 0.05;

/// A path in the pixels of the target as lines only, or `None` for a path
/// that has no curve: such a path is drawn as it is, with its transform.
fn flatten(path: &BezPath, transform: Affine) -> Option<BezPath> {
    if !path.elements().iter().any(|element| matches!(element, PathEl::QuadTo(..) | PathEl::CurveTo(..))) {
        return None;
    }

    let _perf = crate::perf::scope(crate::perf::Phase::Flatten, path.elements().len() as u64);
    let mut flattened = BezPath::new();
    kurbo::flatten(path.iter().map(|element| transform * element), CURVE_TOLERANCE, |element| flattened.push(element));
    Some(flattened)
}

/// The outline of a stroke in the space of its path, to the tolerance of
/// the sinks at `scale`, the scale of the transform the path is drawn with.
pub(super) fn stroke_outline(path: &BezPath, stroke: &Stroke, scale: f64) -> BezPath {
    let _perf = crate::perf::scope(crate::perf::Phase::StrokeOutline, path.elements().len() as u64);
    kurbo::stroke(path.iter(), stroke, &StrokeOpts::default(), CURVE_TOLERANCE / scale)
}

/// The path to hand to the renderer with the transform to draw it with.
pub(super) fn prepare(path: &BezPath, transform: Affine) -> (Cow<'_, BezPath>, Affine) {
    match flatten(path, transform) {
        Some(flattened) => (Cow::Owned(flattened), Affine::IDENTITY),
        None => (Cow::Borrowed(path), transform),
    }
}

/// The scene of the CPU mode: the render context of `vello_cpu`, the
/// sparse-strips renderer on the processor.
pub struct VelloCpuSceneSink {
    context: RenderContext,
    resources: Resources,
    /// The images of the frame in the form the renderer samples, by the
    /// identity of their pixels: an image that is drawn many times (a
    /// tile) is converted once.
    images: HashMap<u64, ImageSource>,
    /// The rectangles of the target that are made transparent before the
    /// scene is composed over what the target holds, when it is
    /// ([`IVelloSceneSink::retain_target`]).
    retained: Option<Vec<VelloScenePixelRect>>,
}

impl VelloCpuSceneSink {
    /// Creates the scene of a target of the given size.
    ///
    /// The scene is rendered on the thread that draws: the renderer's own
    /// thread pool is not compiled in (see the workspace manifest).
    pub fn new(width: u16, height: u16) -> Self {
        let settings = RenderSettings { num_threads: 0, ..RenderSettings::default() };

        Self {
            context: RenderContext::new_with(width, height, settings),
            resources: Resources::new(),
            images: HashMap::new(),
            retained: None,
        }
    }

    fn set_anti_alias(&mut self, anti_alias: bool) {
        self.context.set_aliasing_threshold(if anti_alias { None } else { Some(ALIASING_THRESHOLD) });
    }

    /// Sets the paint. `transform` is the transform of the shape as the
    /// contract has it and `path_transform` the one the renderer is given
    /// for the path, which is the identity for a path that was flattened:
    /// the paint stays where the contract puts it.
    fn set_paint(&mut self, paint: &VelloScenePaint, transform: Affine, path_transform: Affine) {
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
            // This sink draws on no device (its capabilities say so): the
            // one that has a texture reads it back and paints with an image.
            #[cfg(any(feature = "hybrid", feature = "gpu"))]
            VelloSceneBrush::Texture(_) => {
                panic!("The CPU rendering mode of the Vello backend does not paint with a texture of a device")
            }
        };

        // The renderer places a paint by the transform of the path times
        // the transform of the paint.
        let paint_transform =
            if path_transform == transform { paint.transform } else { transform * paint.transform };

        self.context.set_paint(paint_type);
        self.context.set_paint_transform(paint_transform);
    }
}

impl IVelloSceneSink for VelloCpuSceneSink {
    fn rendering_mode(&self) -> VelloRenderingMode {
        VelloRenderingMode::Cpu
    }

    fn capabilities(&self) -> VelloSceneCapabilities {
        VelloSceneCapabilities {
            blend_layers: true,
            aliased_edges: true,
            aliased_rectangles: true,
            image_paints: true,
            read_back: true,
            device_textures: false,
            retained_targets: true,
        }
    }

    fn retain_target(&mut self, cleared: &[VelloScenePixelRect]) {
        self.retained.get_or_insert_with(Vec::new).extend_from_slice(cleared);
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
        self.retained = None;
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
        let (path, path_transform) = prepare(path, transform);

        self.set_anti_alias(anti_alias);
        self.set_paint(paint, transform, path_transform);
        self.context.set_transform(path_transform);
        self.context.set_fill_rule(fill_rule);
        self.context.set_blend_mode(blend_mode);
        self.context.fill_path(&path);
        self.context.set_blend_mode(BlendMode::default());
    }

    fn stroke(&mut self, path: &BezPath, stroke: &Stroke, transform: Affine, paint: &VelloScenePaint, anti_alias: bool) {
        // A stroke is the fill of its outline, which the renderer makes
        // with kurbo as well, but to its own tolerance for curves: the
        // outline is made here, in the space of the path, to the tolerance
        // of this sink at the scale of the transform.
        let scale = transform.determinant().abs().sqrt();
        if !(scale.is_finite() && scale > 0.0) {
            return;
        }
        let outline = stroke_outline(path, stroke, scale);

        self.fill(&outline, Fill::NonZero, transform, paint, BlendMode::default(), anti_alias);
    }

    fn draw_glyph_run(
        &mut self,
        glyph_run: &VelloSceneGlyphRun<'_>,
        transform: Affine,
        paint: &VelloScenePaint,
        anti_alias: bool,
    ) {
        if glyph_run.glyphs.is_empty() || !(glyph_run.font_size.is_finite() && glyph_run.font_size > 0.0) {
            return;
        }

        let _perf = crate::perf::scope(crate::perf::Phase::GlyphRun, glyph_run.glyphs.len() as u64);
        self.set_anti_alias(anti_alias);
        self.set_paint(paint, transform, transform);
        self.context.set_transform(transform);
        self.context.set_fill_rule(Fill::NonZero);

        // The renderer widens the outline it keeps of a glyph, which is in
        // the units of the font unless the glyph is hinted, when it is in
        // pixels: an emboldened run is not hinted, so that the amount has
        // one unit.
        let emboldened = glyph_run.embolden > 0.0;

        let mut builder = self
            .context
            .glyph_run(&mut self.resources, glyph_run.font)
            .font_size(glyph_run.font_size)
            .normalized_coords(glyph_run.normalized_coords)
            .hint(glyph_run.hint && !emboldened);

        if emboldened {
            let amount = glyph_run.embolden * glyph_run.units_per_em.max(1) as f64 / glyph_run.font_size as f64;
            builder = builder.font_embolden(FontEmbolden::new(Diagonal2::new(amount, amount)));
        }

        if glyph_run.skew != 0.0 {
            builder = builder.glyph_transform(Affine::skew(glyph_run.skew, 0.0));
        }

        // A glyph the renderer has no representation of (a bitmap format it
        // does not decode) is left out by it and reported: the rest of the
        // run is drawn, as a font without that glyph would be.
        let _ = builder.fill_glyphs(glyph_run.glyphs.iter().map(|glyph| Glyph { id: glyph.id, x: glyph.x, y: glyph.y }));
    }

    fn push_clip(&mut self, path: &BezPath, fill_rule: Fill, transform: Affine, anti_alias: bool) {
        let (path, path_transform) = prepare(path, transform);

        self.set_anti_alias(anti_alias);
        self.context.set_transform(path_transform);
        self.context.set_fill_rule(fill_rule);
        self.context.push_clip_path(&path);
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
        let pixels_len = pixels.len() as u64;
        assert_eq!(
            pixels.len(),
            width as usize * height as usize * 4,
            "The pixels of the target do not have the size of the scene"
        );

        // A target that is kept: its cleared rectangles are made transparent
        // here, and the renderer composes the scene over the rest.
        let target_init = match &self.retained {
            Some(cleared) => {
                let row_bytes = width as usize * 4;
                for rect in cleared {
                    let (x0, x1) = (rect.x0.min(width) as usize * 4, rect.x1.min(width) as usize * 4);
                    for row in rect.y0.min(height) as usize..rect.y1.min(height) as usize {
                        pixels[row * row_bytes + x0..row * row_bytes + x1.max(x0)].fill(0);
                    }
                }
                TargetInit::SrcOver
            }
            None => TargetInit::Clear(AlphaColor::TRANSPARENT),
        };

        let target = PixmapMut::new(width, height, pixels)
            .unwrap_or_else(|| panic!("The pixels of the target do not have the size of the scene"));

        let settings = RasterizerSettings { target_init, ..RasterizerSettings::default() };

        let _perf = crate::perf::scope(crate::perf::Phase::CpuRender, pixels_len);
        self.context.flush();
        self.context.render_with(target, &mut self.resources, settings);
    }

    fn filter_capabilities(&self) -> VelloSceneFilterCapabilities {
        // The filters of the renderer panic with its thread pool, which is
        // not compiled in.
        VelloSceneFilterCapabilities { filter_layers: true, blurred_rounded_rects: true }
    }

    fn push_filter_layer(&mut self, filter: &VelloSceneFilter, transform: Affine) {
        // Beyond its content a layer is transparent to the blur, as it is
        // to an image filter of Skia.
        let edge_mode = EdgeMode::None;
        let primitive = match *filter {
            VelloSceneFilter::Blur { std_deviation } => FilterPrimitive::GaussianBlur { std_deviation, edge_mode },
            VelloSceneFilter::DropShadow { dx, dy, std_deviation, color } => {
                FilterPrimitive::DropShadow { dx, dy, std_deviation, color, edge_mode }
            }
        };

        // The renderer scales the lengths of a filter by the transform that
        // is current when its layer is pushed.
        self.context.set_transform(transform);
        self.context.push_layer(None, None, None, None, Some(Filter::from_primitive(primitive)));
    }

    fn fill_blurred_rounded_rect(
        &mut self,
        rect: kurbo::Rect,
        radius: f64,
        std_deviation: f64,
        invert: bool,
        transform: Affine,
        color: AlphaColor<Srgb>,
    ) {
        self.set_anti_alias(true);
        self.context.set_paint(PaintType::Solid(color));
        self.context.set_paint_transform(Affine::IDENTITY);
        self.context.set_transform(transform);
        self.context.set_fill_rule(Fill::NonZero);
        // The renderer evaluates `erf(distance / std_dev)` where the blur of
        // an edge by a Gaussian of deviation s is `erf(distance / (s √2))`:
        // what it calls the standard deviation is √2 times the Gaussian's
        // (measured against its own blur filter: with the factor the two
        // agree to 5 of 255 on a rectangle, without it they are 21 apart).
        let std_dev = std_deviation * std::f64::consts::SQRT_2;
        self.context.fill_blurred_rounded_rect(&rect, radius as f32, std_dev as f32, invert);
    }
}
