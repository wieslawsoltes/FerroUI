//! The effects of the drawing context: a blur and a drop shadow of
//! everything that is drawn between a push and its pop, and the layers with
//! an effect (colour matrices, the multiply layers of lit visuals, the pixel
//! operations, the matrix convolution, the luminance mask).
//!
//! The Skia backend draws an effect as a layer whose paint has an image
//! filter. Here an effect is a filter layer of the scene where the renderer
//! of the scene has filters and nothing clips the scene; otherwise what is
//! drawn inside the effect is recorded into a scene of its own, which is
//! drawn, with the filter, into a picture that is composed into the scene of
//! the context when the effect is popped: by the hybrid renderer on the
//! device of the scene in a GPU mode (a texture of the pool of
//! `gpu/effect_passes.rs`), by the CPU renderer otherwise:
//!
//! - a rendering mode without filters ([`VelloSceneFilterCapabilities`])
//!   draws the picture;
//! - under a clip the filter layer of `vello_cpu` 0.3 is not usable: a clip
//!   that is open when the layer is pushed cuts the content of the layer
//!   before it is blurred and does not cut the blurred layer, where the
//!   canvas of Skia does the opposite. The picture is composed under the
//!   clips.
//!
//! vello's renderers have no colour matrix, no multiply that keeps the
//! alpha below, no pixel operations and no convolution. In the hybrid and
//! GPU modes such a layer is drawn on the device (gpu-rendering.md, "Vello",
//! step 2): its content is recorded into a scene of the device, rendered
//! into a texture of the pool, and a pass of `gpu/effect_passes.rs` reads it
//! into another texture of the pool, which the scene the layer was pushed
//! in paints with. In the CPU mode, and inside a scene of the CPU renderer,
//! the content is rendered by the CPU renderer and changed on the processor
//! (the reference).
//!
//! [`VelloSceneFilterCapabilities`]: crate::scene::VelloSceneFilterCapabilities

use super::box_shadows::blur_radius_to_sigma;
use super::{DrawingContextImpl, SavedKind};
use crate::helpers::pixel_format_helper::to_image;
use crate::scene::{IVelloSceneSink, VelloCpuSceneSink, VelloSceneFilter};
use crate::vello_extensions::rect_path;
use ferroui_base::media::effects::IEffect;
use ferroui_base::media::{ColorMatrix, ColorMatrixAlphaMode, ConvolveMatrix};
use ferroui_base::platform::{
    IDrawingContextImpl, IDrawingContextImplWithEffects, LightingParameters, MultiplyLayerFactors, PixelOperation,
};
use ferroui_base::{PixelSize, Rect};
use kurbo::{Affine, Shape, Vec2};
use peniko::color::AlphaColor;
use peniko::BlendMode;
use std::cell::Cell;

ferroui_base::ferro_thread_local! {
    static CPU_LAYERS: Cell<u64> = Cell::new(0);
}

/// How many layers with an effect this thread drew on the processor: their
/// content rendered by the CPU renderer, read and changed there and drawn as
/// an image (the colour matrix, multiply and convolution layers, the
/// luminance mask). The layers of the hybrid and GPU modes take the route
/// of the device and count nothing.
#[allow(dead_code)]
pub(crate) fn cpu_layer_count() -> u64 {
    CPU_LAYERS.with(Cell::get)
}

fn note_cpu_layer() {
    CPU_LAYERS.with(|count| count.set(count.get() + 1));
}

/// The colour matrix of the luminance mask (`ImmutableLuminanceToAlphaEffect`)
/// applied to the premultiplied colours as they are: the luminance of the
/// colour as the alpha, black (the weights of the CPU path of `end_effect`).
#[cfg(feature = "hybrid")]
fn luminance_mask_matrix() -> ColorMatrix {
    let (r, g, b) = (0.2126, 0.7152, 0.0722);
    ColorMatrix::new([[0.0, 0.0, 0.0, r], [0.0, 0.0, 0.0, g], [0.0, 0.0, 0.0, b], [0.0; 4], [0.0; 4]])
}

/// A layer whose content is recorded into a scene of the device, rendered
/// into a texture of the pool and passed through a program of
/// `gpu/effect_passes.rs` into another texture of the pool, which the scene
/// the layer was pushed in paints with.
#[cfg(feature = "hybrid")]
#[derive(Clone)]
pub(super) struct DeviceLayer {
    device: std::sync::Arc<crate::gpu::VelloWgpuDevice>,
    /// The top left pixel of the area of the layer in the scene it was
    /// pushed in.
    x: f64,
    y: f64,
    /// The size of the area in pixels.
    width: u32,
    height: u32,
    /// The size of the scene of the content: a size of the pool, at least
    /// the area.
    scene_width: u16,
    scene_height: u16,
    /// The bounds of the layer in the coordinates of the context when it
    /// was pushed, with the transform from them to the pixels of the scene
    /// it was pushed in; `None` for the whole area.
    bounds: Option<(Rect, Affine)>,
}

/// No layer is drawn on a device in a build without the hybrid renderer.
#[cfg(not(feature = "hybrid"))]
pub(super) enum DeviceLayer {}

/// The result of a layer drawn on the device: a texture of the pool.
#[cfg(feature = "hybrid")]
type LayerTexture = std::sync::Arc<crate::gpu::VelloDeviceTexture>;
#[cfg(not(feature = "hybrid"))]
type LayerTexture = std::convert::Infallible;

/// An effect that is pushed.
pub(super) struct EffectFrame {
    /// Whether the content of the effect is cut to the clip rectangle of
    /// the push, by a clip that ends before the effect does.
    cropped: bool,
    /// The scene the effect was pushed in, when what is drawn inside the
    /// effect is recorded into a scene of its own.
    outer: Option<OuterScene>,
    /// The content is turned into a luminance mask when the effect ends
    /// (`ImmutableLuminanceToAlphaEffect`): on the device by the colour
    /// matrix pass when `device` is set, on the processor otherwise.
    luminance_to_alpha: bool,
    device: Option<DeviceLayer>,
}

/// The scene of the context while an effect is recorded into another.
pub(super) struct OuterScene {
    sink: Box<dyn IVelloSceneSink>,
    /// The pixel of the target that is the top left pixel of this scene.
    origin: Vec2,
    /// The clips that are open in this scene.
    open_clips: usize,
    /// The size in pixels of the area of this scene the scene that replaced
    /// it is composed into: the size of that scene, or less for a scene of
    /// the device (whose size is a size of the pool).
    area: (u16, u16),
}

impl DrawingContextImpl {
    /// The filter of an effect, or `None` for an effect that changes
    /// nothing.
    fn create_effect(&self, effect: &dyn IEffect) -> Option<VelloSceneFilter> {
        if let Some(blur) = effect.as_blur_effect() {
            if blur.radius() <= 0.0 {
                return None;
            }
            return Some(VelloSceneFilter::Blur { std_deviation: blur_radius_to_sigma(blur.radius()) });
        }

        if let Some(drop) = effect.as_drop_shadow_effect() {
            let std_deviation = if drop.blur_radius() > 0.0 { blur_radius_to_sigma(drop.blur_radius()) } else { 0.0 };
            let drop_color = drop.color();
            let mut alpha = drop_color.a as f64 * drop.opacity();
            if !self.use_opacity_save_layer {
                alpha *= self.current_opacity;
            }
            let color =
                AlphaColor::from_rgba8(drop_color.r, drop_color.g, drop_color.b, alpha.clamp(0.0, 255.0) as u8);

            return Some(VelloSceneFilter::DropShadow {
                dx: drop.offset_x() as f32,
                dy: drop.offset_y() as f32,
                std_deviation,
                color,
            });
        }

        // A custom effect (#455) has no factory in this backend yet: its
        // content is drawn unchanged, in a layer without a filter. A runtime
        // shader effect (E4) too, until its WGSL program is drawn (E5).
        if let Some(custom) = effect.as_custom_effect() {
            custom.report_unhandled("Vello");
        }

        None
    }

    /// Whether an effect that is pushed now can be a filter layer of the
    /// scene of the context.
    fn draws_effects_in_the_scene(&mut self) -> bool {
        self.open_clips == 0 && self.sink().filter_capabilities().filter_layers
    }

    /// Replaces the scene of the context by a new one that covers
    /// `clip_rect` and what `filter` spreads of it beyond the rectangle (the
    /// whole scene without one), and returns the scene that was replaced.
    ///
    /// The clip rectangle cuts the content of the effect, not what the
    /// filter makes of it: a blur of content that is cut spreads beyond the
    /// cut, as far as a blur reaches, and a shadow lies where its offset
    /// puts it.
    fn begin_effect_scene(&mut self, clip_rect: Option<Rect>, filter: &VelloSceneFilter, force_cpu: bool) -> OuterScene {
        let (width, height) = (self.sink().width(), self.sink().height());
        let whole = kurbo::Rect::new(0.0, 0.0, width as f64, height as f64);

        let bounds = match clip_rect {
            Some(clip_rect) => {
                let transform = self.device_transform();
                let scale = super::box_shadows::largest_scale(transform);
                let (std_deviation, offset) = match *filter {
                    VelloSceneFilter::Blur { std_deviation } => (f64::from(std_deviation), 0.0),
                    VelloSceneFilter::DropShadow { dx, dy, std_deviation, .. } => {
                        (f64::from(std_deviation), f64::from(dx.abs().max(dy.abs())))
                    }
                };
                let reach = ((super::box_shadows::BLUR_REACH * std_deviation + offset) * scale).ceil() + 1.0;
                let bounds = (transform * rect_path(clip_rect))
                    .bounding_box()
                    .inflate(reach, reach)
                    .expand()
                    .intersect(whole);
                // A scene has at least a pixel.
                let (x0, y0) = (bounds.x0.clamp(0.0, whole.x1 - 1.0), bounds.y0.clamp(0.0, whole.y1 - 1.0));
                kurbo::Rect::new(x0, y0, bounds.x1.clamp(x0 + 1.0, whole.x1), bounds.y1.clamp(y0 + 1.0, whole.y1))
            }
            None => whole,
        };
        let area = (bounds.width() as u16, bounds.height() as u16);

        // A scene whose pixels are read back on the processor (the luminance
        // mask outside the GPU modes) is drawn there.
        let scene: Box<dyn IVelloSceneSink> = if force_cpu {
            Box::new(VelloCpuSceneSink::new(area.0, area.1))
        } else {
            self.create_filter_scene(area.0, area.1)
        };
        let sink = match self.sink.replace(scene) {
            Some(sink) => sink,
            None => panic!("The drawing context has been disposed"),
        };

        let outer = OuterScene { sink, origin: self.sink_origin, open_clips: self.open_clips, area };
        self.sink_origin += Vec2::new(bounds.x0, bounds.y0);
        self.open_clips = 0;
        outer
    }

    /// Puts the scene `outer` back as the scene of the context, and returns
    /// the scene that replaced it.
    fn leave_scene(&mut self, outer: OuterScene) -> Box<dyn IVelloSceneSink> {
        let scene = match self.sink.replace(outer.sink) {
            Some(scene) => scene,
            None => panic!("The drawing context has been disposed"),
        };
        self.sink_origin = outer.origin;
        self.open_clips = outer.open_clips;
        scene
    }

    /// Ends the innermost effect: its layer, and when it was recorded into
    /// a scene of its own, that scene, which is rendered and composed into
    /// the scene the effect was pushed in.
    pub(super) fn end_effect(&mut self) {
        let frame = self.effect_stack.pop().unwrap_or_else(|| panic!("The effect stack is empty"));

        self.sink().pop_layer();

        let Some(outer) = frame.outer else {
            return;
        };

        let position = self.sink_origin - outer.origin;
        let area = outer.area;
        let mut scene = self.leave_scene(outer);

        if frame.luminance_to_alpha {
            if let Some(layer) = frame.device {
                #[cfg(feature = "hybrid")]
                {
                    let matrix = luminance_mask_matrix();
                    let operation = LayerOperation::Matrix { matrix, alpha_mode: ColorMatrixAlphaMode::Straight };
                    self.end_device_layer(scene, layer, &operation);
                    return;
                }
                #[cfg(not(feature = "hybrid"))]
                match layer {}
            }
            note_cpu_layer();
            let (width, height) = (scene.width(), scene.height());
            crate::perf::count(crate::perf::Phase::EffectAsImage, u64::from(width) * u64::from(height) * 4);
            let mut rgba = vec![0u8; width as usize * height as usize * 4];
            scene.render_to_pixels(&mut rgba);
            for pixel in rgba.chunks_exact_mut(4) {
                // The pixels are premultiplied: multiplying by alpha again would
                // darken a translucent luminance mask.
                let alpha = (0.2126 * f64::from(pixel[0]) + 0.7152 * f64::from(pixel[1]) + 0.0722 * f64::from(pixel[2]))
                    .round()
                    .clamp(0.0, 255.0) as u8;
                pixel.copy_from_slice(&[0, 0, 0, alpha]);
            }
            self.draw_image_at_pixels(to_image(rgba, PixelSize::new(width as i32, height as i32)), position.x, position.y);
        } else {
            self.draw_filter_scene(&mut *scene, position.x, position.y, area.0, area.1, crate::perf::Phase::EffectAsImage);
        }
    }

    /// A scene of its own for something that is drawn through a filter and
    /// composed into the scene of the context as a picture of `width` by
    /// `height` pixels: on the device of the scene of the context by the
    /// hybrid renderer, which has filter layers, when the scene is drawn on
    /// a device (the hybrid mode, and the GPU mode, whose renderer has no
    /// filters, in a build with both), of the size of a texture of the pool
    /// (at least the picture); by the CPU renderer otherwise, of the size of
    /// the picture.
    pub(super) fn create_filter_scene(&mut self, width: u16, height: u16) -> Box<dyn IVelloSceneSink> {
        #[cfg(feature = "hybrid")]
        if let Some(device) = self.filter_scene_device() {
            let (scene_width, scene_height) =
                crate::gpu::effect_passes::pooled_size(&device, u32::from(width), u32::from(height));
            let limit = u32::from(u16::MAX);
            let (scene_width, scene_height) = (scene_width.min(limit) as u16, scene_height.min(limit) as u16);
            return Box::new(crate::scene::VelloHybridSceneSink::new(device, scene_width, scene_height));
        }

        Box::new(VelloCpuSceneSink::single_threaded(width, height))
    }

    /// The device a filter scene is drawn on (see
    /// [`create_filter_scene`](Self::create_filter_scene)).
    #[cfg(feature = "hybrid")]
    fn filter_scene_device(&mut self) -> Option<std::sync::Arc<crate::gpu::VelloWgpuDevice>> {
        let scene = self.scene();
        let on_device = match scene.rendering_mode() {
            crate::VelloRenderingMode::Hybrid => scene.filter_capabilities().filter_layers,
            crate::VelloRenderingMode::Gpu => scene.capabilities().device_textures,
            crate::VelloRenderingMode::Cpu => false,
        };
        on_device.then(|| scene.device().cloned()).flatten()
    }

    /// Whether [`create_filter_scene`](Self::create_filter_scene) makes a
    /// scene of the device.
    pub(super) fn filter_scene_on_device(&mut self) -> bool {
        #[cfg(feature = "hybrid")]
        return self.filter_scene_device().is_some();
        #[cfg(not(feature = "hybrid"))]
        false
    }

    /// Renders a scene of the CPU renderer that
    /// [`create_filter_scene`](Self::create_filter_scene) made into a
    /// picture.
    pub(super) fn render_filter_scene_to_image(
        &mut self,
        scene: &mut dyn IVelloSceneSink,
        phase: crate::perf::Phase,
    ) -> peniko::ImageData {
        let (width, height) = (scene.width(), scene.height());
        crate::perf::count(phase, u64::from(width) * u64::from(height) * 4);
        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        scene.render_to_pixels(&mut rgba);
        to_image(rgba, PixelSize::new(width as i32, height as i32))
    }

    /// Renders a scene of [`create_filter_scene`](Self::create_filter_scene)
    /// and draws its picture of `width` by `height` pixels (the top left of
    /// the scene) into the scene of the context, its top left pixel at
    /// (`x`, `y`) of that scene: as a texture of the pool of the device when
    /// the scene was drawn there, as an image otherwise.
    pub(super) fn draw_filter_scene(
        &mut self,
        scene: &mut dyn IVelloSceneSink,
        x: f64,
        y: f64,
        width: u16,
        height: u16,
        phase: crate::perf::Phase,
    ) {
        crate::perf::count(phase, u64::from(width) * u64::from(height) * 4);

        #[cfg(any(feature = "hybrid", feature = "gpu"))]
        if let Some(device) = scene.device().cloned() {
            if self.draws_on_device(&device) {
                // A texture of the pool, of the size of the scene: the
                // scene of the frame holds it until it is rendered.
                let texture = crate::gpu::effect_passes::pooled_texture(
                    &device,
                    u32::from(scene.width()),
                    u32::from(scene.height()),
                );
                match scene.render_to_texture(&crate::gpu::VelloGpuTexture { texture: texture.texture() }) {
                    Ok(()) => {
                        let brush = crate::scene::VelloSceneBrush::Texture(crate::scene::VelloSceneTexture {
                            texture,
                            x_extend: peniko::Extend::Pad,
                            y_extend: peniko::Extend::Pad,
                            quality: peniko::ImageQuality::Low,
                            alpha: 1.0,
                        });
                        self.draw_brush_at_pixels(brush, x, y, f64::from(width), f64::from(height));
                    }
                    Err(error) => crate::gpu::log_render_failure("hybrid", &error),
                }
                return;
            }
        }

        let (scene_width, scene_height) = (scene.width(), scene.height());
        let mut rgba = vec![0u8; scene_width as usize * scene_height as usize * 4];
        scene.render_to_pixels(&mut rgba);
        // The picture is the top left of the scene.
        let (width, height) = (width.min(scene_width), height.min(scene_height));
        if (width, height) != (scene_width, scene_height) {
            let row = width as usize * 4;
            let mut cropped = Vec::with_capacity(row * height as usize);
            for line in rgba.chunks_exact(scene_width as usize * 4).take(height as usize) {
                cropped.extend_from_slice(&line[..row]);
            }
            rgba = cropped;
        }
        let image = to_image(rgba, PixelSize::new(width as i32, height as i32));
        self.draw_image_at_pixels(image, x, y);
    }
}

impl IDrawingContextImplWithEffects for DrawingContextImpl {
    fn push_effect(&mut self, clip_rect: Option<Rect>, effect: &dyn IEffect) {
        // The luminance mask is computed from the pixels of the content: on
        // the device by the colour matrix pass in a GPU mode, in a scene of
        // the processor otherwise.
        if effect.as_any().is::<ferroui_base::media::ImmutableLuminanceToAlphaEffect>() {
            #[cfg(feature = "hybrid")]
            if let Some(layer) = self.device_layer(clip_rect) {
                let outer = self.enter_device_scene(&layer);
                self.sink().push_layer(BlendMode::default(), 1.0);
                self.effect_stack.push(EffectFrame {
                    cropped: clip_rect.is_some(),
                    outer: Some(outer),
                    luminance_to_alpha: true,
                    device: Some(layer),
                });
                self.save(SavedKind::Effect);
                if let Some(rect) = clip_rect {
                    self.push_clip(rect);
                }
                return;
            }
            let outer = self.begin_effect_scene(clip_rect, &VelloSceneFilter::Blur { std_deviation: 0.0 }, true);
            self.sink().push_layer(BlendMode::default(), 1.0);
            self.effect_stack.push(EffectFrame {
                cropped: clip_rect.is_some(),
                outer: Some(outer),
                luminance_to_alpha: true,
                device: None,
            });
            self.save(SavedKind::Effect);
            if let Some(rect) = clip_rect {
                self.push_clip(rect);
            }
            return;
        }
        let Some(filter) = self.create_effect(effect) else {
            // An effect that changes nothing is a layer.
            self.sink().push_layer(BlendMode::default(), 1.0);
            self.effect_stack.push(EffectFrame { cropped: false, outer: None, luminance_to_alpha: false, device: None });
            self.save(SavedKind::Effect);
            return;
        };

        let outer =
            if self.draws_effects_in_the_scene() { None } else { Some(self.begin_effect_scene(clip_rect, &filter, false)) };

        let transform = self.device_transform();
        self.sink().push_filter_layer(&filter, transform);
        self.effect_stack.push(EffectFrame { cropped: clip_rect.is_some(), outer, luminance_to_alpha: false, device: None });
        self.save(SavedKind::Effect);

        // The clip rectangle bounds the layer of the effect: Skia draws no
        // more of the content of a filtered layer than its bounds hold, so
        // the content is cut to them before it is filtered.
        if let Some(clip_rect) = clip_rect {
            self.push_clip(clip_rect);
        }
    }

    fn pop_effect(&mut self) {
        let cropped = self.effect_stack.last().unwrap_or_else(|| panic!("The effect stack is empty")).cropped;

        if cropped {
            self.restore();
        }
        self.restore();
    }
}

/// A layer with an effect that is pushed: what is drawn inside it is
/// recorded into a scene of its own, which is rendered and changed by the
/// operation when it is popped, and composed into the scene the layer was
/// pushed in: on the device when `device` is set, on the processor
/// otherwise.
pub(super) struct ColorMatrixFrame {
    outer: OuterScene,
    operation: LayerOperation,
    device: Option<DeviceLayer>,
}

/// What a layer does to its content.
enum LayerOperation {
    /// A colour matrix layer.
    Matrix { matrix: ColorMatrix, alpha_mode: ColorMatrixAlphaMode },
    /// A multiply layer of factors (on the processor): the factors
    /// stretched over `bounds` (in the coordinates of the context);
    /// `scene_to_local` maps the pixels of the scene of the layer to those
    /// coordinates.
    Multiply { factors: MultiplyLayerFactors, bounds: Rect, scene_to_local: Affine },
    /// A multiply layer of the lights evaluated at each pixel (on the
    /// device); `scene_to_local` as for `Multiply`.
    #[cfg_attr(not(feature = "hybrid"), allow(dead_code))]
    LightingMultiply { lighting: LightingParameters, scene_to_local: Affine },
    /// A matrix convolution.
    Convolve(ConvolveMatrix),
}

/// A pixel operation that is pushed (on the device): each source is
/// recorded into a scene of the device and rendered into a texture of the
/// pool, which the pass of the operation combines into what the sources
/// before it made.
#[cfg_attr(not(feature = "hybrid"), allow(dead_code))]
pub(super) struct PixelOperationFrame {
    operation: PixelOperation,
    layer: DeviceLayer,
    /// Maps the pixels of the scenes of the sources to the coordinates of
    /// the context.
    scene_to_local: Affine,
    /// The scene the operation was pushed in, while a source is recorded.
    outer: Option<OuterScene>,
    /// What the sources so far made.
    result: Option<LayerTexture>,
    /// The opacity in effect when the operation was pushed, restored when
    /// it is popped.
    inherited_opacity: f64,
}

impl DrawingContextImpl {
    /// Replaces the scene of the context by one of the CPU renderer that
    /// covers `bounds` (in the coordinates of the context), cut to the
    /// target, and returns the scene that was replaced.
    fn begin_cpu_scene(&mut self, bounds: Rect) -> OuterScene {
        let (width, height) = (self.sink().width(), self.sink().height());
        let whole = kurbo::Rect::new(0.0, 0.0, width as f64, height as f64);
        let transform = self.device_transform();
        let device = (transform * rect_path(bounds)).bounding_box().expand().intersect(whole);
        let (x0, y0) = (device.x0.clamp(0.0, whole.x1 - 1.0), device.y0.clamp(0.0, whole.y1 - 1.0));
        let device = kurbo::Rect::new(x0, y0, device.x1.clamp(x0 + 1.0, whole.x1), device.y1.clamp(y0 + 1.0, whole.y1));
        let area = (device.width() as u16, device.height() as u16);

        let scene: Box<dyn IVelloSceneSink> = Box::new(VelloCpuSceneSink::single_threaded(area.0, area.1));
        let sink = match self.sink.replace(scene) {
            Some(sink) => sink,
            None => panic!("The drawing context has been disposed"),
        };
        let outer = OuterScene { sink, origin: self.sink_origin, open_clips: self.open_clips, area };
        self.sink_origin += Vec2::new(device.x0, device.y0);
        self.open_clips = 0;
        outer
    }

    /// Pushes a layer with `operation` over `bounds`: on the device in a GPU
    /// mode (but a multiply layer of factors), on the processor otherwise.
    fn push_operation_layer(&mut self, bounds: Rect, operation: LayerOperation) {
        #[cfg(feature = "hybrid")]
        if !matches!(operation, LayerOperation::Multiply { .. }) {
            if let Some(layer) = self.device_layer(Some(bounds)) {
                let outer = self.enter_device_scene(&layer);
                self.color_matrix_stack.push(ColorMatrixFrame { outer, operation, device: Some(layer) });
                self.save(SavedKind::ColorMatrix);
                // Nothing outside the bounds is kept.
                self.push_clip(bounds);
                return;
            }
        }
        let outer = self.begin_cpu_scene(bounds);
        self.color_matrix_stack.push(ColorMatrixFrame { outer, operation, device: None });
        self.save(SavedKind::ColorMatrix);
        // Nothing outside the bounds is kept.
        self.push_clip(bounds);
    }

    /// Ends the innermost layer with an effect: its scene is rendered,
    /// changed by its operation and drawn into the scene it was pushed in.
    pub(super) fn end_color_matrix(&mut self) {
        let frame = self.color_matrix_stack.pop().unwrap_or_else(|| panic!("The colour matrix stack is empty"));
        let position = self.sink_origin - frame.outer.origin;
        let mut scene = self.leave_scene(frame.outer);

        if let Some(layer) = frame.device {
            #[cfg(feature = "hybrid")]
            {
                self.end_device_layer(scene, layer, &frame.operation);
                return;
            }
            #[cfg(not(feature = "hybrid"))]
            match layer {}
        }

        note_cpu_layer();
        let (width, height) = (scene.width(), scene.height());
        crate::perf::count(crate::perf::Phase::EffectAsImage, u64::from(width) * u64::from(height) * 4);
        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        scene.render_to_pixels(&mut rgba);
        match &frame.operation {
            LayerOperation::Matrix { matrix, alpha_mode } => matrix.apply_to_rgba8(&mut rgba, *alpha_mode),
            LayerOperation::Multiply { factors, bounds, scene_to_local } => {
                let (bounds, scene_to_local) = (*bounds, *scene_to_local);
                factors.apply_to_premultiplied_rgba8(&mut rgba, width as usize, |x, y| {
                    let local = scene_to_local * kurbo::Point::new(x, y);
                    let (u, v) = ((local.x - bounds.x) / bounds.width, (local.y - bounds.y) / bounds.height);
                    ((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)).then_some((u, v))
                });
            }
            // Pushed on the device only.
            LayerOperation::LightingMultiply { .. } => {}
            LayerOperation::Convolve(convolution) => {
                convolution.apply_to_premultiplied_rgba8(&mut rgba, width as usize, height as usize)
            }
        }
        let image = to_image(rgba, PixelSize::new(width as i32, height as i32));
        self.draw_image_at_pixels(image, position.x, position.y);
    }

    /// Ends the source of a pixel operation that is being recorded.
    pub(super) fn end_pixel_source(&mut self) {
        #[cfg(feature = "hybrid")]
        self.end_device_pixel_source();
    }
}

/// The route of the device (gpu-rendering.md, "Vello", step 2).
#[cfg(feature = "hybrid")]
impl DrawingContextImpl {
    /// The device a layer with an effect is drawn on: the device of the
    /// scene of the context, when the scene is drawn there and paints with
    /// its textures (the hybrid and GPU modes); `None` in the CPU mode and
    /// inside a scene of the CPU renderer.
    fn layer_device(&mut self) -> Option<std::sync::Arc<crate::gpu::VelloWgpuDevice>> {
        let scene = self.scene();
        if !scene.capabilities().device_textures {
            return None;
        }
        scene.device().cloned()
    }

    /// A layer of the device over `bounds` (in the coordinates of the
    /// context; the whole scene for `None`), cut to the scene, when the
    /// scene of the context is drawn on a device.
    fn device_layer(&mut self, bounds: Option<Rect>) -> Option<DeviceLayer> {
        let device = self.layer_device()?;
        // The scene to draw into: what the target held is under the layer.
        let (width, height) = (f64::from(self.sink().width()), f64::from(self.sink().height()));
        let whole = kurbo::Rect::new(0.0, 0.0, width, height);
        let transform = self.device_transform();
        let area = match bounds {
            Some(bounds) => (transform * rect_path(bounds)).bounding_box().expand().intersect(whole),
            None => whole,
        };
        // An area has at least a pixel.
        let (x0, y0) = (area.x0.clamp(0.0, whole.x1 - 1.0), area.y0.clamp(0.0, whole.y1 - 1.0));
        let area = kurbo::Rect::new(x0, y0, area.x1.clamp(x0 + 1.0, whole.x1), area.y1.clamp(y0 + 1.0, whole.y1));
        let (area_width, area_height) = (area.width() as u32, area.height() as u32);
        let (scene_width, scene_height) = crate::gpu::effect_passes::pooled_size(&device, area_width, area_height);
        let limit = u32::from(u16::MAX);
        Some(DeviceLayer {
            device,
            x: area.x0,
            y: area.y0,
            width: area_width,
            height: area_height,
            scene_width: scene_width.min(limit) as u16,
            scene_height: scene_height.min(limit) as u16,
            bounds: bounds.map(|bounds| (bounds, transform)),
        })
    }

    /// Replaces the scene of the context by a scene of the device for the
    /// content of `layer`, and returns the scene that was replaced.
    fn enter_device_scene(&mut self, layer: &DeviceLayer) -> OuterScene {
        let scene: Box<dyn IVelloSceneSink> =
            Box::new(crate::scene::VelloHybridSceneSink::new(layer.device.clone(), layer.scene_width, layer.scene_height));
        let sink = match self.sink.replace(scene) {
            Some(sink) => sink,
            None => panic!("The drawing context has been disposed"),
        };
        let area = (layer.width.min(u32::from(u16::MAX)) as u16, layer.height.min(u32::from(u16::MAX)) as u16);
        let outer = OuterScene { sink, origin: self.sink_origin, open_clips: self.open_clips, area };
        self.sink_origin += Vec2::new(layer.x, layer.y);
        self.open_clips = 0;
        outer
    }

    /// Renders the scene of the content of `layer` into a texture of the
    /// pool, and drops the scene: what it painted with is released once its
    /// render is submitted.
    fn render_device_scene(mut scene: Box<dyn IVelloSceneSink>, layer: &DeviceLayer) -> Option<LayerTexture> {
        let texture = crate::gpu::effect_passes::pooled_texture(
            &layer.device,
            u32::from(scene.width()),
            u32::from(scene.height()),
        );
        let rendered = scene.render_to_texture(&crate::gpu::VelloGpuTexture { texture: texture.texture() });
        drop(scene);
        match rendered {
            Ok(()) => Some(texture),
            Err(error) => {
                crate::gpu::log_render_failure("hybrid", &error);
                None
            }
        }
    }

    /// Paints the result of a layer into the scene it was pushed in, one
    /// texel per pixel of its area: over its bounds, or the whole area.
    fn paint_device_layer(&mut self, layer: &DeviceLayer, texture: LayerTexture, alpha: f32, blend_mode: BlendMode) {
        use crate::scene::{VelloSceneBrush, VelloScenePaint, VelloSceneTexture};
        let brush = VelloSceneBrush::Texture(VelloSceneTexture {
            texture,
            x_extend: peniko::Extend::Pad,
            y_extend: peniko::Extend::Pad,
            quality: peniko::ImageQuality::Low,
            alpha,
        });
        match layer.bounds {
            Some((bounds, transform)) => {
                let paint = VelloScenePaint { brush, transform: transform.inverse() * Affine::translate((layer.x, layer.y)) };
                // The edges of the clip of the content: not anti-aliased.
                let anti_alias = self.edge_anti_alias(false);
                self.sink().fill(&rect_path(bounds), peniko::Fill::NonZero, transform, &paint, blend_mode, anti_alias);
            }
            None => {
                let path = rect_path(Rect::new(layer.x, layer.y, f64::from(layer.width), f64::from(layer.height)));
                let paint = VelloScenePaint { brush, transform: Affine::translate((layer.x, layer.y)) };
                let anti_alias = self.edge_anti_alias(false);
                self.sink().fill(&path, peniko::Fill::NonZero, Affine::IDENTITY, &paint, blend_mode, anti_alias);
            }
        }
    }

    /// Ends a layer drawn on the device: the content rendered into a texture
    /// of the pool, the pass of the operation, and its result painted into
    /// the scene the layer was pushed in (the order of the queue).
    fn end_device_layer(&mut self, scene: Box<dyn IVelloSceneSink>, layer: DeviceLayer, operation: &LayerOperation) {
        use crate::gpu::effect_passes::{self, LayerArea};
        let Some(content) = Self::render_device_scene(scene, &layer) else {
            return;
        };
        let area = LayerArea { width: layer.width, height: layer.height };
        let device = &layer.device;
        let result = match operation {
            LayerOperation::Matrix { matrix, alpha_mode } => {
                effect_passes::color_matrix_texture(device, &content, area, matrix, *alpha_mode)
            }
            LayerOperation::LightingMultiply { lighting, scene_to_local } => {
                effect_passes::light_multiply_texture(device, &content, area, lighting, *scene_to_local)
            }
            LayerOperation::Convolve(convolution) => {
                effect_passes::convolution_texture(device, &content, area, convolution)
            }
            // A multiply layer of factors is drawn on the processor.
            LayerOperation::Multiply { .. } => None,
        };
        // The content is the pool's again: the pass that reads it is
        // submitted.
        drop(content);
        if let Some(result) = result {
            self.paint_device_layer(&layer, result, 1.0, BlendMode::default());
        }
    }

    /// Ends the source of a pixel operation that is recorded: its scene is
    /// rendered into a texture of the pool, which is the result of the
    /// operation when it is the first source, and is combined into the
    /// result by the pass of the operation otherwise.
    fn end_device_pixel_source(&mut self) {
        use crate::gpu::effect_passes::{self, LayerArea};
        let Some(mut frame) = self.pixel_operation_stack.pop() else {
            panic!("The pixel operation stack is empty");
        };
        let Some(outer) = frame.outer.take() else {
            panic!("No source of the pixel operation is open");
        };
        let scene = self.leave_scene(outer);
        let source = Self::render_device_scene(scene, &frame.layer);
        let area = LayerArea { width: frame.layer.width, height: frame.layer.height };
        frame.result = match (frame.result.take(), source) {
            (Some(destination), Some(source)) => {
                let combined = effect_passes::pixel_operation_texture(
                    &frame.layer.device,
                    &destination,
                    &source,
                    area,
                    &frame.operation,
                    frame.scene_to_local,
                );
                // What the sources before made, should the pass fail.
                Some(combined.unwrap_or(destination))
            }
            (destination, source) => destination.or(source),
        };
        self.pixel_operation_stack.push(frame);
    }
}

impl ferroui_base::platform::IDrawingContextImplWithLayerEffects for DrawingContextImpl {
    fn push_blend_layer(&mut self, _bounds: Rect, mode: ferroui_base::media::imaging::BitmapBlendingMode) {
        let blend =
            if self.sink().capabilities().blend_layers { crate::vello_extensions::to_blend_mode(mode) } else { BlendMode::default() };
        self.sink().push_layer(blend, 1.0);
        self.save(SavedKind::Layer);
    }

    fn pop_blend_layer(&mut self) {
        self.restore();
    }

    /// On the device in a GPU mode (the colour matrix pass of
    /// `gpu/effect_passes.rs`), on the processor otherwise: vello 0.3 has
    /// no colour matrix filter (`vello_common` leaves `ColorMatrix`
    /// unimplemented).
    fn push_color_matrix_layer(&mut self, bounds: Rect, matrix: &ColorMatrix, alpha_mode: ColorMatrixAlphaMode) {
        self.push_operation_layer(bounds, LayerOperation::Matrix { matrix: *matrix, alpha_mode });
    }

    fn pop_color_matrix_layer(&mut self) {
        // The clip, then the layer.
        self.restore();
        self.restore();
    }

    /// The factors are applied to the pixels of a scene of the CPU renderer:
    /// they are an image of the processor (the light map of the compositor,
    /// for more lights than a drawing context evaluates), and vello has no
    /// blend mode that multiplies the colour and keeps the alpha of what is
    /// under it. The lit visuals of a GPU mode take
    /// [`push_lighting_multiply_layer`](Self::push_lighting_multiply_layer).
    fn push_multiply_layer(&mut self, bounds: Rect, factors: &MultiplyLayerFactors) {
        let outer = self.begin_cpu_scene(bounds);
        let scene_to_local = self.device_transform().inverse();
        self.color_matrix_stack.push(ColorMatrixFrame {
            outer,
            operation: LayerOperation::Multiply { factors: factors.clone(), bounds, scene_to_local },
            device: None,
        });
        self.save(SavedKind::ColorMatrix);
        self.push_clip(bounds);
    }

    fn pop_multiply_layer(&mut self) {
        self.restore();
        self.restore();
    }

    /// On the device in a GPU mode (the convolution pass of
    /// `gpu/effect_passes.rs`), by the CPU reference
    /// (`ConvolveMatrix::apply_to_premultiplied_rgba8`) otherwise.
    fn push_convolve_matrix_layer(&mut self, bounds: Rect, convolution: &ConvolveMatrix) {
        self.push_operation_layer(bounds, LayerOperation::Convolve(*convolution));
    }

    fn pop_convolve_matrix_layer(&mut self) {
        self.restore();
        self.restore();
    }

    /// In a GPU mode: the content is recorded into a scene of the device,
    /// and the light multiply pass of `gpu/effect_passes.rs` multiplies it
    /// by the lights it evaluates at each pixel. The CPU mode, WebGL2 and a
    /// scene of the CPU renderer leave it to the compositor's light map
    /// (`push_multiply_layer`).
    #[cfg(feature = "hybrid")]
    fn push_lighting_multiply_layer(&mut self, bounds: Rect, lighting: &LightingParameters) -> bool {
        if lighting.lights.len() > LightingParameters::MAX_LIGHTS {
            return false;
        }
        let Some(layer) = self.device_layer(Some(bounds)) else {
            return false;
        };
        let outer = self.enter_device_scene(&layer);
        let scene_to_local = self.device_transform().inverse();
        self.color_matrix_stack.push(ColorMatrixFrame {
            outer,
            operation: LayerOperation::LightingMultiply { lighting: lighting.clone(), scene_to_local },
            device: Some(layer),
        });
        self.save(SavedKind::ColorMatrix);
        self.push_clip(bounds);
        true
    }

    /// In a GPU mode: each source is recorded into a scene of the device
    /// and rendered into a texture of the pool, and the pixel operation
    /// pass of `gpu/effect_passes.rs` combines it into what the sources
    /// before it made; the result is composed when the operation is popped.
    /// The CPU mode, WebGL2 and a scene of the CPU renderer leave it to the
    /// compositor's pixels.
    #[cfg(feature = "hybrid")]
    fn push_pixel_operation(&mut self, bounds: Rect, operation: &PixelOperation) -> bool {
        if let PixelOperation::Blend { noise_grid, .. } = operation {
            if noise_grid.width <= 0.0 || noise_grid.height <= 0.0 {
                return false;
            }
        }
        let Some(layer) = self.device_layer(Some(bounds)) else {
            return false;
        };
        let outer = self.enter_device_scene(&layer);
        let scene_to_local = self.device_transform().inverse();
        // The sources are drawn with full opacity; the result is composed
        // with the opacity in effect.
        let inherited_opacity = self.current_opacity;
        self.current_opacity = 1.0;
        self.pixel_operation_stack.push(PixelOperationFrame {
            operation: *operation,
            layer,
            scene_to_local,
            outer: Some(outer),
            result: None,
            inherited_opacity,
        });
        self.save(SavedKind::PixelSource);
        // Nothing outside the bounds is kept (an offset of the arithmetic
        // composite would show everywhere else).
        self.push_clip(bounds);
        true
    }

    #[cfg(feature = "hybrid")]
    fn next_pixel_operation_source(&mut self) {
        // The clip and the source before.
        self.restore();
        self.restore();
        let Some(frame) = self.pixel_operation_stack.last() else {
            panic!("The pixel operation stack is empty");
        };
        let layer = frame.layer.clone();
        let outer = self.enter_device_scene(&layer);
        if let Some(frame) = self.pixel_operation_stack.last_mut() {
            frame.outer = Some(outer);
        }
        self.save(SavedKind::PixelSource);
        if let Some((bounds, _)) = layer.bounds {
            self.push_clip(bounds);
        }
    }

    #[cfg(feature = "hybrid")]
    fn pop_pixel_operation(&mut self) {
        // The clip and the last source.
        self.restore();
        self.restore();
        let Some(frame) = self.pixel_operation_stack.pop() else {
            panic!("The pixel operation stack is empty");
        };
        self.current_opacity = frame.inherited_opacity;
        if let Some(result) = frame.result {
            // As a bitmap of the result would be drawn: with the opacity and
            // the bitmap blending mode in effect.
            let alpha = ((255.0 * self.current_opacity) as u8) as f32 / 255.0;
            let blend_mode = crate::vello_extensions::to_blend_mode(self.render_options.bitmap_blending_mode);
            self.paint_device_layer(&frame.layer, result, alpha, blend_mode);
        }
    }

    /// In a GPU mode, the lights are evaluated per pixel by a render pass of
    /// the device into a texture the scene paints the area with
    /// (`gpu/effect_passes.rs`); the CPU mode, WebGL2 and a scene of the CPU
    /// renderer (inside a layer drawn on the processor) leave it to the
    /// compositor's light map.
    #[cfg(any(feature = "hybrid", feature = "gpu"))]
    fn draw_lighting(&mut self, rect: Rect, lighting: &ferroui_base::platform::LightingParameters) -> bool {
        use crate::scene::{VelloSceneBrush, VelloScenePaint, VelloSceneTexture};
        let device = {
            let scene = self.scene();
            if !scene.capabilities().device_textures {
                return false;
            }
            match scene.device() {
                Some(device) => device.clone(),
                None => return false,
            }
        };
        if lighting.lights.len() > ferroui_base::platform::LightingParameters::MAX_LIGHTS {
            return false;
        }
        let transform = self.device_transform();
        let (width, height) = (f64::from(self.scene().width()), f64::from(self.scene().height()));
        let area = (transform * rect_path(rect)).bounding_box().intersect(kurbo::Rect::new(0.0, 0.0, width, height));
        let (x0, y0) = (area.x0.floor().max(0.0), area.y0.floor().max(0.0));
        let (x1, y1) = (area.x1.ceil().min(width), area.y1.ceil().min(height));
        if x1 <= x0 || y1 <= y0 {
            // Nothing of the area is on the scene.
            return true;
        }
        let Some(texture) = crate::gpu::effect_passes::lighting_texture(
            &device,
            (x1 - x0) as u32,
            (y1 - y0) as u32,
            (x0 as u32, y0 as u32),
            transform.inverse(),
            lighting,
            crate::gpu::effect_passes::LightingOutput::Light,
        ) else {
            return false;
        };
        // As a bitmap of the light map would be drawn: with the opacity and
        // the bitmap blending mode in effect, one texel per device pixel.
        let alpha = ((255.0 * self.current_opacity) as u8) as f32 / 255.0;
        let brush = VelloSceneBrush::Texture(VelloSceneTexture {
            texture,
            x_extend: peniko::Extend::Pad,
            y_extend: peniko::Extend::Pad,
            quality: peniko::ImageQuality::Low,
            alpha,
        });
        let paint = VelloScenePaint { brush, transform: transform.inverse() * Affine::translate((x0, y0)) };
        let blend_mode = crate::vello_extensions::to_blend_mode(self.render_options.bitmap_blending_mode);
        let anti_alias = self.anti_alias();
        self.sink().fill(&rect_path(rect), peniko::Fill::NonZero, transform, &paint, blend_mode, anti_alias);
        true
    }
}
