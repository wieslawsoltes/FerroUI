use crate::gpu::{log_render_failure, premultiply_pixel, VelloGpuTexture, VelloWgpuDevice};
use crate::scene::vello_cpu_scene_sink::{prepare, stroke_outline};
use crate::scene::{IVelloSceneSink, VelloSceneBrush, VelloSceneCapabilities, VelloSceneGlyphRun, VelloScenePaint};
use crate::vello_options::VelloRenderingMode;
use kurbo::{Affine, BezPath, Diagonal2, PathEl, Point, Rect, Stroke, Vec2};
use peniko::color::{palette, AlphaColor};
use peniko::{BlendMode, Brush, Compose, Fill, GradientKind, ImageBrush, ImageSampler, LinearGradientPosition, Mix};
use std::collections::HashMap;
use std::sync::Arc;
use vello::{AaConfig, AaSupport, FontEmbolden, Glyph, RenderParams, Renderer, RendererOptions, Scene};

/// How the GPU mode anti-aliases edges.
///
/// The compute renderer has no mode without anti-aliasing. Measured against
/// the CPU mode on the scenes of the comparison harness (design document,
/// section 8), area coverage is the nearest of the three and the cheapest:
/// it is what the sparse-strips renderers compute too.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum VelloGpuAntiAliasing {
    /// Exact area coverage of each pixel.
    #[default]
    Area,
    /// Eight samples a pixel.
    Msaa8,
    /// Sixteen samples a pixel.
    Msaa16,
}

impl VelloGpuAntiAliasing {
    fn config(self) -> AaConfig {
        match self {
            Self::Area => AaConfig::Area,
            Self::Msaa8 => AaConfig::Msaa8,
            Self::Msaa16 => AaConfig::Msaa16,
        }
    }
}

/// What the GPU mode keeps on a device: a renderer for each anti-aliasing
/// method that was used (its pipelines are compiled for the method) and the
/// texture a scene is rendered into on its way to a texture the compute
/// shaders cannot write.
struct GpuRendererState {
    renderers: HashMap<VelloGpuAntiAliasing, Renderer>,
    /// The texture a scene for a window is rendered into: the renderer
    /// writes with a compute shader, which the texture of a drawable does
    /// not allow.
    intermediate: Option<wgpu::Texture>,
    /// The images that stand for what scenes paint with, by their size and
    /// alpha form: the n-th image or texture of a size in a scene is always
    /// the n-th of these (see [`ImagePlace`]).
    places: HashMap<ImagePlace, Vec<peniko::ImageData>>,
    /// The textures of the images of the last renders, by the identity of
    /// their pixels, each with the render it was last drawn in.
    image_textures: HashMap<u64, (wgpu::Texture, u64)>,
    render_count: u64,
}

/// The size and the alpha form of an image a scene paints with.
///
/// The renderer keeps the images of its scenes in an atlas, at places it
/// hands out by the identity of the pixels of an image, and its shader
/// samples an image at the place plus the position in the image, in single
/// precision: the same image at another place is sampled with weights that
/// differ in their last bits, and a pixel in a few hundred comes out a
/// digit of a color apart (a pixel on the border between two texels of an
/// image that is sampled by the nearest pixel, as another texel). A bitmap
/// that is created again for every frame has other pixels by identity each
/// time, so the same scene drawn twice was not drawn the same way twice.
///
/// The scene therefore paints with images of the renderer's state that
/// stand for what it paints with (an image without pixels for each size,
/// alpha form and count in a scene), and the pixels reach the atlas from a
/// texture of the device (`Renderer::override_image`): the same scene has
/// the same places every time it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ImagePlace {
    width: u32,
    height: u32,
    premultiplied: bool,
}

/// What an image of a scene stands for.
enum ImageSource {
    /// A texture of the device.
    Texture(Arc<crate::gpu::VelloDeviceTexture>),
    /// Pixels, which become a texture of the device when the scene is
    /// rendered.
    Pixels(peniko::ImageData),
}

/// The number of renders the texture of an image that is no longer drawn is
/// kept for, as in the hybrid mode.
const IMAGE_TEXTURE_LIFETIME: u64 = 16;

const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<GpuRendererState>();
};

/// What is open in the scene.
enum Open {
    /// A layer. `reopened` clips of the layer below were closed before it
    /// and are open again inside it: they are the entries above this one.
    Layer { reopened: usize },
    /// A clip, as it was pushed.
    Clip(OpenClip),
}

#[derive(Clone)]
struct OpenClip {
    path: BezPath,
    fill_rule: Fill,
    transform: Affine,
}

/// The scene of the GPU mode: the scene of `vello`, the renderer of the
/// Vello project that does everything from the flattening of curves to the
/// composition in compute shaders.
///
/// Its scene is another interface than the scenes of the sparse-strips
/// renderers (every call takes its transform, brush and style) and lacks
/// things they have. What this sink does about each (design document,
/// section 1.3):
///
/// * **A shape has no blend mode, only a layer has.** A shape that is not
///   composed source-over is drawn in a layer of its mode.
/// * **A clip is a layer too, and what is blended inside it does not see
///   what is below the clip** (the scene's own words, issue 1198 of the
///   project). A blended shape or layer inside clips is therefore drawn
///   outside them: the clips are closed, the layer of the blend is opened
///   over the whole target, the clips are opened again inside it, and
///   afterwards the other way round.
/// * **A layer composed `Copy` replaces what is below it by the coverage
///   of its clip, not in proportion to it.** A shape composed `Copy` is
///   drawn in two steps that are exact on its edges: what is below is
///   taken out by the coverage of the shape (`DestOut`), and the paint is
///   added by it (`Plus`). `Clear` is the first step alone.
/// * **Curves are flattened on the GPU to a tolerance of a quarter of a
///   pixel**, as the sparse-strips renderers do on the processor; like in
///   the other sinks they are flattened here, five times finer, and
///   strokes are expanded to their outline here (measured: the shapes of
///   the tests of this mode differ from the CPU mode in 0.86 % of the
///   pixels when the renderer flattens and strokes, and in a tenth of that
///   when the sink does).
/// * **A gradient is evaluated at the corner of a pixel**, not at its
///   center as an image is and as the other renderers do: gradients are
///   moved by half a pixel.
/// * **A linear gradient under a transform that does not keep angles is
///   drawn between its transformed points**, askew: linear gradients are
///   given in the pixels of the target.
/// * **No edges without anti-aliasing.** A rectangle whose transform keeps
///   its sides on the axes is snapped to the pixels its aliased edges
///   would cover, which is the same pixels; any other shape is drawn
///   anti-aliased ([`VelloSceneCapabilities::aliased_edges`] is `false`).
/// * **Its output is not premultiplied.** The pixels read back are
///   premultiplied here, and the copy into the texture of a window
///   premultiplies.
///
/// Not closed: a shape composed `SrcIn`, `DestIn`, `SrcOut` or `DestAtop`
/// (the modes that also change what lies outside the source) inside a clip
/// takes out what is below it in the part of the shape the clip hides;
/// without a clip around it, it is exact but for the anti-aliasing of its
/// edge.
pub struct VelloGpuSceneSink {
    device: Arc<VelloWgpuDevice>,
    scene: Scene,
    width: u16,
    height: u16,
    anti_aliasing: VelloGpuAntiAliasing,
    open: Vec<Open>,
    /// What the scene paints with, each with the image that stands for it
    /// in the scene ([`ImagePlace`]): the renderer copies the texture of it
    /// into the place of that image in its atlas when the scene is rendered
    /// (`Renderer::override_image`), on the device.
    images: Vec<(peniko::ImageData, ImageSource)>,
    /// The image that stands for each image or texture of the scene, by
    /// the identity of its pixels or of the texture.
    stand_ins: HashMap<u64, peniko::ImageData>,
    /// How many images of each size the scene paints with.
    place_counts: HashMap<ImagePlace, usize>,
}

impl VelloGpuSceneSink {
    /// Creates the scene of a target of the given size that is drawn with
    /// `device`.
    pub fn new(device: Arc<VelloWgpuDevice>, width: u16, height: u16) -> Self {
        Self::with_anti_aliasing(device, width, height, VelloGpuAntiAliasing::default())
    }

    /// Creates the scene with an anti-aliasing method of its own: the
    /// measure of the comparison harness.
    pub fn with_anti_aliasing(
        device: Arc<VelloWgpuDevice>,
        width: u16,
        height: u16,
        anti_aliasing: VelloGpuAntiAliasing,
    ) -> Self {
        Self {
            device,
            scene: Scene::new(),
            width,
            height,
            anti_aliasing,
            open: Vec::new(),
            images: Vec::new(),
            stand_ins: HashMap::new(),
            place_counts: HashMap::new(),
        }
    }

    /// The device the scene is drawn with.
    pub fn device(&self) -> &Arc<VelloWgpuDevice> {
        &self.device
    }

    fn target_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, f64::from(self.width), f64::from(self.height))
    }

    /// The image that stands in the scene for an image or a texture of
    /// the given identity ([`ImagePlace`]).
    fn stand_in(&mut self, id: u64, place: ImagePlace, source: impl FnOnce() -> ImageSource) -> peniko::ImageData {
        if let Some(image) = self.stand_ins.get(&id) {
            return image.clone();
        }

        let index = {
            let count = self.place_counts.entry(place).or_insert(0);
            *count += 1;
            *count - 1
        };
        let image = self.device.with_renderer_state(Self::new_state, |_, state| {
            let images = state.places.entry(place).or_default();
            while images.len() <= index {
                // An image without pixels: the renderer never reads it, it
                // reads a texture in its place.
                let no_pixels: Arc<[u8; 0]> = Arc::new([]);
                images.push(peniko::ImageData {
                    data: peniko::Blob::new(no_pixels),
                    format: peniko::ImageFormat::Rgba8,
                    alpha_type: match place.premultiplied {
                        true => peniko::ImageAlphaType::AlphaPremultiplied,
                        false => peniko::ImageAlphaType::Alpha,
                    },
                    width: place.width,
                    height: place.height,
                });
            }
            images[index].clone()
        });

        self.stand_ins.insert(id, image.clone());
        self.images.push((image.clone(), source()));
        image
    }

    /// The brush of a paint.
    fn brush(&mut self, paint: &VelloScenePaint) -> Brush {
        match &paint.brush {
            VelloSceneBrush::Solid(color) => Brush::Solid(*color),
            VelloSceneBrush::Gradient(gradient) => Brush::Gradient(gradient.clone()),
            VelloSceneBrush::Image(image) => {
                let place = ImagePlace {
                    width: image.image.width,
                    height: image.image.height,
                    premultiplied: image.image.alpha_type == peniko::ImageAlphaType::AlphaPremultiplied,
                };
                let pixels = image.image.clone();
                let stand_in = self.stand_in(image.image.data.id(), place, || ImageSource::Pixels(pixels));
                Brush::Image(ImageBrush {
                    image: stand_in,
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
                    Arc::ptr_eq(texture.texture.device(), &self.device),
                    "A texture is painted with on the device it belongs to"
                );
                let place = ImagePlace {
                    width: texture.texture.width(),
                    height: texture.texture.height(),
                    premultiplied: texture.texture.alpha() == crate::gpu::VelloTextureAlpha::Premultiplied,
                };
                let source = texture.texture.clone();
                let stand_in = self.stand_in(texture.texture.id(), place, || ImageSource::Texture(source));
                Brush::Image(ImageBrush {
                    image: stand_in,
                    sampler: ImageSampler {
                        x_extend: texture.x_extend,
                        y_extend: texture.y_extend,
                        quality: texture.quality,
                        alpha: texture.alpha,
                    },
                })
            }
        }
    }

    /// The clips that were opened since the innermost layer, outermost
    /// first.
    fn local_clips(&self) -> Vec<OpenClip> {
        let start = self.open.iter().rposition(|open| matches!(open, Open::Layer { .. })).map_or(0, |index| index + 1);

        self.open[start..]
            .iter()
            .filter_map(|open| match open {
                Open::Clip(clip) => Some(clip.clone()),
                Open::Layer { .. } => None,
            })
            .collect()
    }

    fn close_clips(&mut self, clips: &[OpenClip]) {
        for _ in clips {
            self.scene.pop_layer();
        }
    }

    fn open_clips(&mut self, clips: &[OpenClip]) {
        for clip in clips {
            self.scene.push_clip_layer(clip.fill_rule, clip.transform, &clip.path);
        }
    }

    /// Draws a shape inside the given clips in a layer over the whole
    /// target that is composed with `blend_mode`.
    fn blended_pass(
        &mut self,
        clips: &[OpenClip],
        blend_mode: BlendMode,
        path: &BezPath,
        fill_rule: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
    ) {
        let target = self.target_rect();

        self.scene.push_layer(Fill::NonZero, blend_mode, 1.0, Affine::IDENTITY, &target);
        self.open_clips(clips);
        self.scene.fill(fill_rule, transform, brush, Some(brush_transform), path);
        self.close_clips(clips);
        self.scene.pop_layer();
    }

    /// Renders the scene into a view of a texture of the device that the
    /// compute shaders can write (`Rgba8Unorm`, `STORAGE_BINDING`),
    /// replacing what it held.
    #[allow(clippy::too_many_arguments)]
    fn render(
        scene: &Scene,
        images: &[(peniko::ImageData, ImageSource)],
        anti_aliasing: VelloGpuAntiAliasing,
        device: &VelloWgpuDevice,
        state: &mut GpuRendererState,
        view: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        if !state.renderers.contains_key(&anti_aliasing) {
            let options = RendererOptions {
                antialiasing_support: AaSupport::from_iter([anti_aliasing.config()]),
                ..RendererOptions::default()
            };
            let renderer = Renderer::new(device.device(), options).map_err(|error| error.to_string())?;
            state.renderers.insert(anti_aliasing, renderer);
        }
        let renderer = state.renderers.get_mut(&anti_aliasing).unwrap_or_else(|| panic!("The renderer was just created"));

        let params = RenderParams {
            base_color: palette::css::TRANSPARENT,
            width,
            height,
            antialiasing_method: anti_aliasing.config(),
        };

        // What the scene paints with takes the places of the images that
        // stand for it, for this render: the textures of the device as they
        // are, the pixels of an image as a texture that is made when the
        // image is first drawn and kept while it is drawn.
        state.render_count += 1;
        let render_count = state.render_count;
        for (image, source) in images {
            let texture = match source {
                ImageSource::Texture(texture) => texture.texture().clone(),
                ImageSource::Pixels(pixels) => {
                    let entry = state.image_textures.entry(pixels.data.id()).or_insert_with(|| {
                        (device.create_image_texture(pixels.width, pixels.height, &rgba(pixels)), 0)
                    });
                    entry.1 = render_count;
                    entry.0.clone()
                }
            };
            renderer.override_image(
                image,
                Some(wgpu::TexelCopyTextureInfoBase {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                }),
            );
        }
        state.image_textures.retain(|_, (_, last_used)| render_count - *last_used < IMAGE_TEXTURE_LIFETIME);

        let result = {
            let _perf = crate::perf::scope(crate::perf::Phase::Render, u64::from(width) * u64::from(height) * 4);
            renderer
                .render_to_texture(device.device(), device.queue(), scene, view, &params)
                .map_err(|error| error.to_string())
        };

        for (image, _) in images {
            renderer.override_image(image, None);
        }

        result
    }

    fn new_state(_device: &VelloWgpuDevice) -> GpuRendererState {
        GpuRendererState {
            renderers: HashMap::new(),
            intermediate: None,
            places: HashMap::new(),
            image_textures: HashMap::new(),
            render_count: 0,
        }
    }

    /// What the compute shaders write into.
    const TARGET_USAGE: wgpu::TextureUsages =
        wgpu::TextureUsages::STORAGE_BINDING.union(wgpu::TextureUsages::TEXTURE_BINDING);
}

/// The pixels of an image in the order of the channels of a texture, as
/// they are otherwise: the renderer is told whether they are premultiplied.
fn rgba(image: &peniko::ImageData) -> std::borrow::Cow<'_, [u8]> {
    let data = image.data.data();
    if !matches!(image.format, peniko::ImageFormat::Bgra8) {
        return std::borrow::Cow::Borrowed(data);
    }

    let mut rgba = data.to_vec();
    rgba.chunks_exact_mut(4).for_each(|pixel| pixel.swap(0, 2));
    std::borrow::Cow::Owned(rgba)
}

/// A linear gradient with its transform applied to its points, and the
/// identity as its transform; any other brush as it is.
///
/// The renderer transforms the two points of a linear gradient and draws
/// the gradient between the transformed points, which is the gradient of
/// the transform only when the transform keeps angles: under a scale that
/// differs in the two directions or a skew the lines of one color are no
/// longer perpendicular to the line between the points, and the renderer
/// draws them so. The gradient is therefore given in the pixels of the
/// target: it starts where the start point lands, and ends on the line of
/// the color of the end point, opposite the start.
fn linear_gradient_in_target(brush: Brush, brush_in_target: Affine) -> (Brush, Affine) {
    let Brush::Gradient(mut gradient) = brush else {
        return (brush, brush_in_target);
    };
    let GradientKind::Linear(position) = gradient.kind else {
        return (Brush::Gradient(gradient), brush_in_target);
    };

    let direction = position.end - position.start;
    let length_squared = direction.hypot2();
    let determinant = brush_in_target.determinant();
    if !(length_squared > 0.0 && determinant != 0.0 && determinant.is_finite()) {
        return (Brush::Gradient(gradient), brush_in_target);
    }

    // The offset of a point x of the target along the gradient is
    // g . (x - start), with g the direction over its squared length carried
    // by the inverse transpose of the transform.
    let [a, b, c, d, _, _] = brush_in_target.as_coeffs();
    let scaled = direction / length_squared;
    let normal = Vec2::new(d * scaled.x - b * scaled.y, -c * scaled.x + a * scaled.y) / determinant;

    let start = brush_in_target * position.start;
    let end = start + normal / normal.hypot2();
    gradient.kind = GradientKind::Linear(LinearGradientPosition::new(start, end));

    (Brush::Gradient(gradient), Affine::IDENTITY)
}

/// The rectangle a path is, when it is one with its sides on the axes.
fn as_rect(path: &BezPath) -> Option<Rect> {
    let mut points: Vec<Point> = Vec::with_capacity(5);

    for (index, element) in path.elements().iter().enumerate() {
        match (index, element) {
            (0, PathEl::MoveTo(point)) => points.push(*point),
            (1..=4, PathEl::LineTo(point)) => points.push(*point),
            (4 | 5, PathEl::ClosePath) if index == path.elements().len() - 1 => {}
            _ => return None,
        }
    }

    if points.len() == 5 {
        if points[4] != points[0] {
            return None;
        }
        points.pop();
    }
    if points.len() != 4 {
        return None;
    }

    let rect = Rect::from_points(points[0], points[2]);
    let on_the_axes = (0..4).all(|index| {
        let (a, b) = (points[index], points[(index + 1) % 4]);
        a.x == b.x || a.y == b.y
    });
    let corners = points[1] != points[3] && points.iter().all(|point| {
        (point.x == rect.x0 || point.x == rect.x1) && (point.y == rect.y0 || point.y == rect.y1)
    });

    (on_the_axes && corners).then_some(rect)
}

/// A rectangle in the pixels of the target as the pixels its edges without
/// anti-aliasing cover (a pixel is covered when its center is inside), or
/// `None` for a path that is not a rectangle or a transform that does not
/// keep its sides on the axes.
fn snap_to_pixels(path: &BezPath, transform: Affine) -> Option<BezPath> {
    let [a, b, c, d, _, _] = transform.as_coeffs();
    if b != 0.0 || c != 0.0 || a == 0.0 || d == 0.0 {
        return None;
    }

    let rect = transform.transform_rect_bbox(as_rect(path)?);
    let snap = |edge: f64| (edge - 0.5).ceil();
    let snapped = Rect::new(snap(rect.x0), snap(rect.y0), snap(rect.x1), snap(rect.y1));

    Some(kurbo::Shape::to_path(&snapped, 0.1))
}

impl IVelloSceneSink for VelloGpuSceneSink {
    fn rendering_mode(&self) -> VelloRenderingMode {
        VelloRenderingMode::Gpu
    }

    fn capabilities(&self) -> VelloSceneCapabilities {
        // Aliased edges: rectangles on the axes only (see the type).
        VelloSceneCapabilities {
            blend_layers: true,
            aliased_edges: false,
            aliased_rectangles: true,
            image_paints: true,
            read_back: true,
            device_textures: true,
            retained_targets: false,
        }
    }

    fn device(&self) -> Option<&Arc<VelloWgpuDevice>> {
        Some(&self.device)
    }

    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn reset(&mut self) {
        self.scene.reset();
        self.open.clear();
        self.images.clear();
        self.stand_ins.clear();
        self.place_counts.clear();
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
        let brush = self.brush(paint);
        // The brush in the pixels of the target. The renderer evaluates a
        // gradient at the corner of a pixel, not at its center (its fine
        // shader; images it samples at the center): a gradient is moved
        // by half a pixel, so that the corner gets the color of the center.
        let brush_in_target = match &paint.brush {
            VelloSceneBrush::Gradient(_) => Affine::translate((-0.5, -0.5)) * transform * paint.transform,
            _ => transform * paint.transform,
        };
        let (brush, brush_in_target) = linear_gradient_in_target(brush, brush_in_target);

        let snapped = if anti_alias { None } else { snap_to_pixels(path, transform) };
        let flattened;
        let (path, transform) = match &snapped {
            Some(snapped) => (snapped, Affine::IDENTITY),
            None => {
                // Curves as lines, to the tolerance of the other modes: the
                // renderer flattens to its own, coarser one (see the type).
                let (path, path_transform) = prepare(path, transform);
                flattened = path;
                (&*flattened, path_transform)
            }
        };

        // The scene places a brush by the transform of the shape times the
        // transform of the brush.
        let brush_transform = match transform == Affine::IDENTITY {
            true => brush_in_target,
            false if transform.determinant() != 0.0 => transform.inverse() * brush_in_target,
            // A shape that has no area draws nothing.
            false => return,
        };

        if blend_mode == BlendMode::default() {
            self.scene.fill(fill_rule, transform, &brush, Some(brush_transform), path);
            return;
        }

        let clips = self.local_clips();
        self.close_clips(&clips);

        match blend_mode.compose {
            Compose::Clear | Compose::Copy => {
                let take_out = BlendMode::new(Mix::Normal, Compose::DestOut);
                let opaque = Brush::Solid(AlphaColor::BLACK);
                self.blended_pass(&clips, take_out, path, fill_rule, transform, &opaque, Affine::IDENTITY);

                if blend_mode.compose == Compose::Copy {
                    let add = BlendMode::new(Mix::Normal, Compose::Plus);
                    self.blended_pass(&clips, add, path, fill_rule, transform, &brush, brush_transform);
                }
            }
            Compose::SrcIn | Compose::DestIn | Compose::SrcOut | Compose::DestAtop => {
                let target = self.target_rect();
                self.scene.push_layer(fill_rule, blend_mode, 1.0, transform, path);
                self.open_clips(&clips);
                self.scene.fill(Fill::NonZero, Affine::IDENTITY, &brush, Some(transform * brush_transform), &target);
                self.close_clips(&clips);
                self.scene.pop_layer();
            }
            _ => self.blended_pass(&clips, blend_mode, path, fill_rule, transform, &brush, brush_transform),
        }

        self.open_clips(&clips);
    }

    fn stroke(&mut self, path: &BezPath, stroke: &Stroke, transform: Affine, paint: &VelloScenePaint, _anti_alias: bool) {
        // As in the other modes: the outline of the stroke, made here to
        // the tolerance of the sinks at the scale of the transform.
        let scale = transform.determinant().abs().sqrt();
        if !(scale.is_finite() && scale > 0.0) {
            return;
        }
        let outline = stroke_outline(path, stroke, scale);

        self.fill(&outline, Fill::NonZero, transform, paint, BlendMode::default(), true);
    }

    fn draw_glyph_run(
        &mut self,
        glyph_run: &VelloSceneGlyphRun<'_>,
        transform: Affine,
        paint: &VelloScenePaint,
        _anti_alias: bool,
    ) {
        if glyph_run.glyphs.is_empty() || !(glyph_run.font_size.is_finite() && glyph_run.font_size > 0.0) {
            return;
        }
        if transform.determinant() == 0.0 {
            return;
        }
        let _perf = crate::perf::scope(crate::perf::Phase::GlyphRun, glyph_run.glyphs.len() as u64);

        // The brush as a shape gets it (see `fill`): a gradient moved by
        // half a pixel, a linear one in the pixels of the target. The scene
        // places the brush of a run by the transform of the run times the
        // transform of the brush.
        let brush_in_target = match &paint.brush {
            VelloSceneBrush::Gradient(_) => Affine::translate((-0.5, -0.5)) * transform * paint.transform,
            _ => transform * paint.transform,
        };
        let brush = self.brush(paint);
        let (brush, brush_in_target) = linear_gradient_in_target(brush, brush_in_target);
        let brush_transform = transform.inverse() * brush_in_target;

        // The renderer widens the outline of a glyph at the size of the
        // font, which is the unit of the amount of the run (the glyph
        // renderer of the other modes widens it in the units of the font).
        // An emboldened run is not hinted, as in the other modes.
        let emboldened = glyph_run.embolden > 0.0;

        let mut builder = self
            .scene
            .draw_glyphs(glyph_run.font)
            .transform(transform)
            .font_size(glyph_run.font_size)
            .normalized_coords(glyph_run.normalized_coords)
            .hint(glyph_run.hint && !emboldened)
            .brush(&brush)
            .brush_transform(Some(brush_transform));

        if emboldened {
            builder =
                builder.font_embolden(FontEmbolden::new(Diagonal2::new(glyph_run.embolden, glyph_run.embolden)));
        }

        if glyph_run.skew != 0.0 {
            // The shear of the run is in a space of which y points down;
            // the renderer applies the transform of a glyph to its outline
            // before it turns the outline over, where y points up.
            let shear = Affine::FLIP_Y * Affine::skew(glyph_run.skew, 0.0) * Affine::FLIP_Y;
            builder = builder.glyph_transform(Some(shear));
        }

        builder.draw(Fill::NonZero, glyph_run.glyphs.iter().map(|glyph| Glyph { id: glyph.id, x: glyph.x, y: glyph.y }));
    }

    fn push_clip(&mut self, path: &BezPath, fill_rule: Fill, transform: Affine, anti_alias: bool) {
        let snapped = if anti_alias { None } else { snap_to_pixels(path, transform) };
        let clip = match snapped {
            Some(snapped) => OpenClip { path: snapped, fill_rule: Fill::NonZero, transform: Affine::IDENTITY },
            None => {
                let (path, transform) = prepare(path, transform);
                OpenClip { path: path.into_owned(), fill_rule, transform }
            }
        };

        self.scene.push_clip_layer(clip.fill_rule, clip.transform, &clip.path);
        self.open.push(Open::Clip(clip));
    }

    fn pop_clip(&mut self) {
        if let Some(Open::Clip(_)) = self.open.last() {
            self.open.pop();
            self.scene.pop_layer();
        }
    }

    fn push_layer(&mut self, blend_mode: BlendMode, opacity: f32) {
        let target = self.target_rect();

        if blend_mode == BlendMode::default() {
            self.scene.push_layer(Fill::NonZero, blend_mode, opacity, Affine::IDENTITY, &target);
            self.open.push(Open::Layer { reopened: 0 });
            return;
        }

        // A layer that is blended does not see what is below the clips
        // around it: it is opened outside them, and they inside it.
        let clips = self.local_clips();
        self.close_clips(&clips);
        self.scene.push_layer(Fill::NonZero, blend_mode, opacity, Affine::IDENTITY, &target);
        self.open_clips(&clips);

        self.open.push(Open::Layer { reopened: clips.len() });
        self.open.extend(clips.into_iter().map(Open::Clip));
    }

    fn pop_layer(&mut self) {
        let Some(index) = self.open.iter().rposition(|open| matches!(open, Open::Layer { .. })) else {
            return;
        };
        let Open::Layer { reopened } = self.open[index] else {
            return;
        };

        // Whatever is open above the layer ends with it: the clips that
        // were opened again inside it, and clips its caller left open.
        for _ in index + 1..self.open.len() {
            self.scene.pop_layer();
        }
        self.open.truncate(index);
        self.scene.pop_layer();

        if reopened > 0 {
            let clips = self.local_clips();
            self.open_clips(&clips);
        }
    }

    fn render_to_pixels(&mut self, pixels: &mut [u8]) {
        let (width, height) = (u32::from(self.width), u32::from(self.height));
        let texture = self.device.create_rgba_texture(width, height, Self::TARGET_USAGE);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (scene, images, anti_aliasing) = (&self.scene, &self.images, self.anti_aliasing);

        let rendered = self.device.with_renderer_state(Self::new_state, |device, state| {
            Self::render(scene, images, anti_aliasing, device, state, &view, width, height)
        });

        match rendered {
            Ok(()) => {
                if self.device.read_texture(&texture, pixels) {
                    pixels.chunks_exact_mut(4).for_each(premultiply_pixel);
                }
            }
            Err(error) => log_render_failure("GPU", &error),
        }
    }

    fn render_to_texture(&mut self, target: &VelloGpuTexture<'_>) -> Result<(), String> {
        let (width, height) = (u32::from(self.width), u32::from(self.height));
        let (scene, images, anti_aliasing) = (&self.scene, &self.images, self.anti_aliasing);
        let format = target.texture.format();
        let target_view = target.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // A texture the compute shaders can write (the texture of a layer)
        // is rendered into as it is: it then holds colors that are not
        // premultiplied, which is what this renderer samples from it.
        if format == wgpu::TextureFormat::Rgba8Unorm
            && target.texture.usage().contains(wgpu::TextureUsages::STORAGE_BINDING)
        {
            return self.device.with_renderer_state(Self::new_state, |device, state| {
                Self::render(scene, images, anti_aliasing, device, state, &target_view, width, height)
            });
        }

        let intermediate = self.device.with_renderer_state(Self::new_state, |device, state| {
            let reusable =
                state.intermediate.as_ref().is_some_and(|texture| texture.width() == width && texture.height() == height);
            if !reusable {
                state.intermediate = Some(device.create_rgba_texture(width, height, Self::TARGET_USAGE));
            }
            let intermediate = state.intermediate.clone().unwrap_or_else(|| panic!("The texture was just created"));
            let view = intermediate.create_view(&wgpu::TextureViewDescriptor::default());

            Self::render(scene, images, anti_aliasing, device, state, &view, width, height)?;
            Ok::<_, String>(view)
        })?;

        // The renderer's output is not premultiplied; the texture of a
        // window holds premultiplied pixels.
        self.device.copy_texture_to_texture(&intermediate, target.texture, true);
        Ok(())
    }
}
