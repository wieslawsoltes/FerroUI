use crate::gpu::{log_render_failure, premultiplied_rgba, VelloGpuTexture, VelloWgpuDevice};
use crate::scene::vello_cpu_scene_sink::{prepare, stroke_outline, ALIASING_THRESHOLD};
use crate::scene::{
    IVelloSceneSink, VelloSceneBrush, VelloSceneCapabilities, VelloSceneFilter, VelloSceneFilterCapabilities,
    VelloSceneGlyphRun, VelloScenePaint, VelloScenePixelRect,
};
use vello_cpu::filter_effects::{EdgeMode, Filter, FilterPrimitive};
use glifo::{FontEmbolden, Glyph};
use crate::vello_options::VelloRenderingMode;
use kurbo::{Affine, BezPath, Diagonal2, Rect, Shape, Stroke};
use peniko::color::AlphaColor;
use peniko::{BlendMode, Fill, ImageBrush, ImageData, ImageSampler};
use std::collections::HashMap;
use std::sync::Arc;
use vello_gpu::{
    ClearSettings, ImageSource, PaintType, RectU16, RenderSize, RenderTargetConfig, Renderer, Resources, Scene,
    TargetInit, TextureBindings, TextureId,
};

/// The number of renders the texture of an image that is no longer drawn is
/// kept for: an image that is drawn every frame is uploaded once, and one
/// that left the scene is released soon after.
const IMAGE_TEXTURE_LIFETIME: u64 = 16;

/// A renderer of the hybrid mode for targets of one format, with what it
/// keeps between frames.
struct FormatRenderer {
    renderer: Renderer,
    resources: Resources,
}

/// The texture of an image and the render it was last drawn in.
struct ImageTexture {
    view: wgpu::TextureView,
    last_used: u64,
}

/// What the hybrid mode keeps on a device: a renderer for each format of a
/// target (its pipelines are compiled for the format), and the textures of
/// the images of the last frames.
struct HybridRendererState {
    renderers: HashMap<wgpu::TextureFormat, FormatRenderer>,
    images: HashMap<u64, ImageTexture>,
    render_count: u64,
}

const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<HybridRendererState>();
};

/// The scene of the hybrid mode: the scene of `vello_gpu` (which was
/// `vello_hybrid`), the sparse-strips renderer that prepares the strips on
/// the processor and rasterizes and composes them on the GPU, with vertex
/// and fragment shaders only.
///
/// The scene has the methods of the scene of the CPU mode, name for name,
/// and shares its code for paths (`vello_common`): what this sink does is
/// what [`VelloCpuSceneSink`](crate::scene::VelloCpuSceneSink) does, but
/// for three things the renderer does not do itself (design document,
/// section 1.3):
///
/// * a fill that is composed destructively (`Copy`, `Clear`, `SrcIn`,
///   `DestIn`, `SrcOut`, `DestAtop`) is refused by the scene unless it is
///   isolated: it is drawn as a layer that is clipped to the shape and
///   composed with that mode;
/// * an image is not taken as pixels with the scene: it is a texture of
///   the device, created when the scene is rendered and kept for the next
///   frames;
/// * mask layers are refused: the drawing context draws an opacity mask as
///   a layer composed with `DestIn`, which this sink draws like any other.
pub struct VelloHybridSceneSink {
    device: Arc<VelloWgpuDevice>,
    /// The format of the target the scene is rendered into. A renderer is
    /// made for a format, and the glyphs of a scene are prepared with what
    /// that renderer keeps (its glyph caches): the scene belongs to one.
    format: wgpu::TextureFormat,
    scene: Scene,
    /// The images of the scene by the identity of their pixels.
    images: HashMap<u64, ImageData>,
    /// The textures of the device the scene paints with, by their
    /// identity.
    textures: HashMap<u64, wgpu::TextureView>,
    /// The rectangles of the target that are made transparent before the
    /// scene is composed over what the target holds, when it is
    /// ([`IVelloSceneSink::retain_target`]).
    retained: Option<Vec<RectU16>>,
}

impl VelloHybridSceneSink {
    /// Creates the scene of a target of the given size that is drawn with
    /// `device` into memory.
    pub fn new(device: Arc<VelloWgpuDevice>, width: u16, height: u16) -> Self {
        Self::for_format(device, width, height, wgpu::TextureFormat::Rgba8Unorm)
    }

    /// Creates the scene of a target of the given size and format: the
    /// texture of a window.
    pub fn for_format(device: Arc<VelloWgpuDevice>, width: u16, height: u16, format: wgpu::TextureFormat) -> Self {
        Self {
            device,
            format,
            scene: Scene::new(width, height),
            images: HashMap::new(),
            textures: HashMap::new(),
            retained: None,
        }
    }

    fn new_state(_device: &VelloWgpuDevice) -> HybridRendererState {
        HybridRendererState { renderers: HashMap::new(), images: HashMap::new(), render_count: 0 }
    }

    fn format_renderer<'a>(
        state: &'a mut HybridRendererState,
        device: &VelloWgpuDevice,
        format: wgpu::TextureFormat,
        width: u16,
        height: u16,
    ) -> &'a mut FormatRenderer {
        state.renderers.entry(format).or_insert_with(|| {
            let (renderer, resources) = Renderer::new(device.device(), &RenderTargetConfig { format, width, height });
            FormatRenderer { renderer, resources }
        })
    }

    /// The device the scene is drawn with.
    pub fn device(&self) -> &Arc<VelloWgpuDevice> {
        &self.device
    }

    fn set_anti_alias(&mut self, anti_alias: bool) {
        self.scene.set_aliasing_threshold(if anti_alias { None } else { Some(ALIASING_THRESHOLD) });
    }

    /// The paint in the form of the renderer, and its transform from the
    /// space of the paint to the pixels of the target.
    fn paint(&mut self, paint: &VelloScenePaint, transform: Affine) -> (PaintType, Affine) {
        let paint_type = match &paint.brush {
            VelloSceneBrush::Solid(color) => PaintType::Solid(*color),
            VelloSceneBrush::Gradient(gradient) => PaintType::Gradient(gradient.clone()),
            VelloSceneBrush::Image(image) => {
                let id = image.image.data.id();
                self.images.entry(id).or_insert_with(|| image.image.clone());

                PaintType::Image(ImageBrush {
                    image: ImageSource::ExternalTexture {
                        id: TextureId(id),
                        source_region: RectU16 {
                            x0: 0,
                            y0: 0,
                            x1: image.image.width.min(u32::from(u16::MAX)) as u16,
                            y1: image.image.height.min(u32::from(u16::MAX)) as u16,
                        },
                        may_have_transparency: true,
                    },
                    sampler: ImageSampler {
                        x_extend: image.x_extend,
                        y_extend: image.y_extend,
                        quality: image.quality,
                        alpha: image.alpha,
                    },
                })
            }
            VelloSceneBrush::Texture(texture) => {
                assert!(
                    Arc::ptr_eq(texture.texture.device(), &self.device)
                        && texture.texture.alpha() == crate::gpu::VelloTextureAlpha::Premultiplied,
                    "The hybrid mode paints with a texture of its device that holds premultiplied colors"
                );
                let id = texture.texture.id();
                self.textures.entry(id).or_insert_with(|| texture.texture.view().clone());

                PaintType::Image(ImageBrush {
                    image: ImageSource::ExternalTexture {
                        id: TextureId(id),
                        source_region: RectU16 {
                            x0: 0,
                            y0: 0,
                            x1: texture.texture.width().min(u32::from(u16::MAX)) as u16,
                            y1: texture.texture.height().min(u32::from(u16::MAX)) as u16,
                        },
                        may_have_transparency: true,
                    },
                    sampler: ImageSampler {
                        x_extend: texture.x_extend,
                        y_extend: texture.y_extend,
                        quality: texture.quality,
                        alpha: texture.alpha,
                    },
                })
            }
        };

        (paint_type, transform * paint.transform)
    }

    /// The rectangle of the whole target.
    fn target_path(&self) -> BezPath {
        Rect::new(0.0, 0.0, f64::from(self.scene.width()), f64::from(self.scene.height())).to_path(0.1)
    }

    /// Renders the scene into a view of a texture of the device, replacing
    /// what the texture held.
    fn render(&mut self, view: &wgpu::TextureView, format: wgpu::TextureFormat) -> Result<(), String> {
        if format != self.format {
            return Err(format!("The scene was made for a target of the format {:?}, not {format:?}", self.format));
        }
        let (width, height) = (self.scene.width(), self.scene.height());
        let (scene, images, textures) = (&self.scene, &self.images, &self.textures);
        let retained = self.retained.as_deref();

        self.device.with_renderer_state(
            Self::new_state,
            |device, state| {
                state.render_count += 1;
                let render_count = state.render_count;

                let mut bindings = TextureBindings::new();
                for (id, image) in images {
                    let texture = state.images.entry(*id).or_insert_with(|| {
                        let texture = device.create_image_texture(image.width, image.height, &premultiplied_rgba(image));
                        ImageTexture { view: texture.create_view(&wgpu::TextureViewDescriptor::default()), last_used: 0 }
                    });
                    texture.last_used = render_count;
                    bindings.insert(TextureId(*id), texture.view.clone());
                }
                for (id, view) in textures {
                    bindings.insert(TextureId(*id), view.clone());
                }

                // A target that is kept: its cleared rectangles are made
                // transparent, and the scene is composed over the rest.
                let target_init = match retained {
                    Some([]) => TargetInit::SrcOver,
                    Some(rects) => TargetInit::Clear(ClearSettings::Rects { color: AlphaColor::TRANSPARENT, rects }),
                    None => TargetInit::Clear(ClearSettings::Viewport { color: AlphaColor::TRANSPARENT }),
                };

                let format_renderer = Self::format_renderer(state, device, format, width, height);

                let _perf =
                    crate::perf::scope(crate::perf::Phase::Render, u64::from(width) * u64::from(height) * 4);
                let mut encoder = device
                    .device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("FerroUI Vello hybrid") });

                let result = format_renderer.renderer.render(
                    scene,
                    &mut format_renderer.resources,
                    device.device(),
                    device.queue(),
                    &mut encoder,
                    &RenderSize { width, height },
                    view,
                    None,
                    &bindings,
                    target_init,
                );
                device.queue().submit([encoder.finish()]);

                state.images.retain(|_, texture| render_count - texture.last_used < IMAGE_TEXTURE_LIFETIME);

                result.map_err(|error| error.to_string())
            },
        )
    }
}

/// Whether a font has glyphs that are pictures of fixed sizes.
fn has_bitmap_strikes(font: &peniko::FontData) -> bool {
    use skrifa::MetadataProvider;

    skrifa::FontRef::from_index(font.data.as_ref(), font.index).is_ok_and(|font| !font.bitmap_strikes().is_empty())
}

impl IVelloSceneSink for VelloHybridSceneSink {
    fn rendering_mode(&self) -> VelloRenderingMode {
        VelloRenderingMode::Hybrid
    }

    fn capabilities(&self) -> VelloSceneCapabilities {
        VelloSceneCapabilities {
            blend_layers: true,
            aliased_edges: true,
            aliased_rectangles: true,
            image_paints: true,
            read_back: true,
            device_textures: true,
            retained_targets: true,
        }
    }

    fn device(&self) -> Option<&Arc<VelloWgpuDevice>> {
        Some(&self.device)
    }

    fn retain_target(&mut self, cleared: &[VelloScenePixelRect]) {
        let (width, height) = (self.scene.width(), self.scene.height());
        self.retained.get_or_insert_with(Vec::new).extend(cleared.iter().filter_map(|rect| {
            let rect = RectU16 { x0: rect.x0.min(width), y0: rect.y0.min(height), x1: rect.x1.min(width), y1: rect.y1.min(height) };
            (rect.x0 < rect.x1 && rect.y0 < rect.y1).then_some(rect)
        }));
    }

    fn width(&self) -> u16 {
        self.scene.width()
    }

    fn height(&self) -> u16 {
        self.scene.height()
    }

    fn reset(&mut self) {
        self.scene.reset();
        self.images.clear();
        self.textures.clear();
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
        let (paint_type, paint_transform) = self.paint(paint, transform);

        self.set_anti_alias(anti_alias);
        self.scene.set_fill_rule(fill_rule);
        self.scene.set_blend_mode(BlendMode::default());

        if blend_mode.is_destructive() {
            // The scene refuses a destructive composition of a shape that
            // is not isolated. A layer that is clipped to the shape and
            // holds the paint everywhere composes the same pixels: inside
            // the shape the paint by the mode, outside it nothing, and on
            // the edge each by the coverage of the edge.
            self.scene.set_transform(path_transform);
            self.scene.push_layer(Some(&path), Some(blend_mode), None, None, None);

            let target = self.target_path();
            self.scene.set_transform(Affine::IDENTITY);
            self.scene.set_fill_rule(Fill::NonZero);
            self.scene.set_paint(paint_type);
            self.scene.set_paint_transform(paint_transform);
            self.scene.fill_path(&target);
            self.scene.pop_layer();
            return;
        }

        // The scene places a paint by the transform of the path times the
        // transform of the paint.
        let relative_paint_transform = match path_transform == transform {
            true => paint.transform,
            false => paint_transform,
        };

        self.scene.set_transform(path_transform);
        self.scene.set_paint(paint_type);
        self.scene.set_paint_transform(relative_paint_transform);
        self.scene.set_blend_mode(blend_mode);
        self.scene.fill_path(&path);
        self.scene.set_blend_mode(BlendMode::default());
    }

    fn stroke(&mut self, path: &BezPath, stroke: &Stroke, transform: Affine, paint: &VelloScenePaint, anti_alias: bool) {
        // As in the CPU mode: the outline of the stroke, made here to the
        // tolerance of the sinks at the scale of the transform.
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

        // As the CPU mode draws a run, with the glyph renderer both scenes
        // share (`glifo`); see there for the bold simulation and hinting.
        let (paint_type, _) = self.paint(paint, transform);
        self.set_anti_alias(anti_alias);
        self.scene.set_blend_mode(BlendMode::default());
        self.scene.set_transform(transform);
        self.scene.set_fill_rule(Fill::NonZero);
        self.scene.set_paint(paint_type);
        self.scene.set_paint_transform(paint.transform);

        let emboldened = glyph_run.embolden > 0.0;
        let (scene, format) = (&mut self.scene, self.format);
        let (width, height) = (scene.width(), scene.height());

        // The glyphs are prepared with what the renderer of the target
        // keeps between frames, under the lock of the device.
        self.device.with_renderer_state(Self::new_state, |device, state| {
            let resources = &mut Self::format_renderer(state, device, format, width, height).resources;

            let mut builder = scene
                .glyph_run(resources, glyph_run.font)
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

            // The glyphs of a font with bitmap strikes (a colour font of
            // pictures) are images, which this renderer takes from an
            // atlas of the device only: the glyph atlas of the renderer is
            // on for such a font. Outlines are drawn as paths, as in the
            // CPU mode: the atlas is the renderer's own experiment and
            // draws them at other pixels.
            if has_bitmap_strikes(glyph_run.font) {
                builder = builder.atlas_cache(true);
            }

            // A glyph the renderer has no representation of is left out by
            // it and reported: the rest of the run is drawn.
            let _ = builder
                .fill_glyphs(glyph_run.glyphs.iter().map(|glyph| Glyph { id: glyph.id, x: glyph.x, y: glyph.y }));
        });
    }

    fn push_clip(&mut self, path: &BezPath, fill_rule: Fill, transform: Affine, anti_alias: bool) {
        let (path, path_transform) = prepare(path, transform);

        self.set_anti_alias(anti_alias);
        self.scene.set_transform(path_transform);
        self.scene.set_fill_rule(fill_rule);
        self.scene.push_clip_path(&path);
    }

    fn pop_clip(&mut self) {
        self.scene.pop_clip();
    }

    fn push_layer(&mut self, blend_mode: BlendMode, opacity: f32) {
        self.scene.push_layer(None, Some(blend_mode), Some(opacity), None, None);
    }

    fn pop_layer(&mut self) {
        self.scene.pop_layer();
    }

    fn render_to_pixels(&mut self, pixels: &mut [u8]) {
        let (width, height) = (u32::from(self.scene.width()), u32::from(self.scene.height()));
        let texture = self.device.create_rgba_texture(width, height, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // The texture is new and holds nothing: there is nothing to keep.
        self.retained = None;

        match self.render(&view, texture.format()) {
            Ok(()) => {
                self.device.read_texture(&texture, pixels);
            }
            // The scene could not be drawn (an intermediate texture could
            // not be had): the target is left as it was.
            Err(error) => log_render_failure("hybrid", &error),
        }
    }

    fn render_to_texture(&mut self, target: &VelloGpuTexture<'_>) -> Result<(), String> {
        let view = target.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.render(&view, target.texture.format())
    }

    fn filter_capabilities(&self) -> VelloSceneFilterCapabilities {
        // The filters of the CPU renderer, on the device: the two renderers
        // share the code that describes them.
        VelloSceneFilterCapabilities { filter_layers: true, blurred_rounded_rects: true }
    }

    fn push_filter_layer(&mut self, filter: &VelloSceneFilter, transform: Affine) {
        // As the CPU sink: beyond its content a layer is transparent to the
        // blur, and the lengths of the filter are scaled by the transform
        // that is current when the layer is pushed.
        let edge_mode = EdgeMode::None;
        let primitive = match *filter {
            VelloSceneFilter::Blur { std_deviation } => FilterPrimitive::GaussianBlur { std_deviation, edge_mode },
            VelloSceneFilter::DropShadow { dx, dy, std_deviation, color } => {
                FilterPrimitive::DropShadow { dx, dy, std_deviation, color, edge_mode }
            }
        };

        self.scene.set_blend_mode(BlendMode::default());
        self.scene.set_transform(transform);
        self.scene.push_layer(None, None, None, None, Some(Filter::from_primitive(primitive)));
    }

    fn fill_blurred_rounded_rect(
        &mut self,
        rect: kurbo::Rect,
        radius: f64,
        std_deviation: f64,
        invert: bool,
        transform: Affine,
        color: AlphaColor<peniko::color::Srgb>,
    ) {
        self.set_anti_alias(true);
        self.scene.set_blend_mode(BlendMode::default());
        self.scene.set_paint(PaintType::Solid(color));
        self.scene.set_paint_transform(Affine::IDENTITY);
        self.scene.set_transform(transform);
        self.scene.set_fill_rule(Fill::NonZero);
        // The factor of the CPU sink: what the renderers call the standard
        // deviation is the square root of two times the Gaussian's.
        let std_dev = std_deviation * std::f64::consts::SQRT_2;
        self.scene.fill_blurred_rounded_rect(&rect, radius as f32, std_dev as f32, invert);
    }
}
