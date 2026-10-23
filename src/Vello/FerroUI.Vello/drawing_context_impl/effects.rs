//! The effects of the drawing context: a blur and a drop shadow of
//! everything that is drawn between a push and its pop.
//!
//! The Skia backend draws an effect as a layer whose paint has an image
//! filter. Here an effect is a filter layer of the scene where the renderer
//! of the scene has filters and nothing clips the scene; otherwise what is
//! drawn inside the effect is recorded into a scene of its own, which the
//! CPU renderer draws, with the filter, into an image that is composed into
//! the scene of the context when the effect is popped:
//!
//! - a rendering mode without filters ([`VelloSceneFilterCapabilities`])
//!   draws the image;
//! - under a clip the filter layer of `vello_cpu` 0.3 is not usable: a clip
//!   that is open when the layer is pushed cuts the content of the layer
//!   before it is blurred and does not cut the blurred layer, where the
//!   canvas of Skia does the opposite. The image is composed under the
//!   clips.
//!
//! [`VelloSceneFilterCapabilities`]: crate::scene::VelloSceneFilterCapabilities

use super::box_shadows::blur_radius_to_sigma;
use super::{DrawingContextImpl, SavedKind};
use crate::helpers::pixel_format_helper::to_image;
use crate::scene::{IVelloSceneSink, VelloCpuSceneSink, VelloSceneFilter};
use crate::vello_extensions::rect_path;
use ferroui_base::media::effects::IEffect;
use ferroui_base::platform::{IDrawingContextImpl, IDrawingContextImplWithEffects};
use ferroui_base::{PixelSize, Rect};
use kurbo::{Shape, Vec2};
use peniko::color::AlphaColor;
use peniko::BlendMode;

/// An effect that is pushed.
pub(super) struct EffectFrame {
    /// Whether the content of the effect is cut to the clip rectangle of
    /// the push, by a clip that ends before the effect does.
    cropped: bool,
    /// The scene the effect was pushed in, when what is drawn inside the
    /// effect is recorded into a scene of its own.
    outer: Option<OuterScene>,
}

/// The scene of the context while an effect is recorded into another.
pub(super) struct OuterScene {
    sink: Box<dyn IVelloSceneSink>,
    /// The pixel of the target that is the top left pixel of this scene.
    origin: Vec2,
    /// The clips that are open in this scene.
    open_clips: usize,
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

        None
    }

    /// Whether an effect that is pushed now can be a filter layer of the
    /// scene of the context.
    fn draws_effects_in_the_scene(&mut self) -> bool {
        self.open_clips == 0 && self.sink().filter_capabilities().filter_layers
    }

    /// Replaces the scene of the context by a new one of the CPU renderer
    /// that covers `clip_rect` and what `filter` spreads of it beyond the
    /// rectangle (the whole scene without one), and returns the scene that
    /// was replaced.
    ///
    /// The clip rectangle cuts the content of the effect, not what the
    /// filter makes of it: a blur of content that is cut spreads beyond the
    /// cut, as far as a blur reaches, and a shadow lies where its offset
    /// puts it.
    fn begin_effect_scene(&mut self, clip_rect: Option<Rect>, filter: &VelloSceneFilter) -> OuterScene {
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

        let scene = self.create_filter_scene(bounds.width() as u16, bounds.height() as u16);
        let sink = match self.sink.replace(scene) {
            Some(sink) => sink,
            None => panic!("The drawing context has been disposed"),
        };

        let outer = OuterScene { sink, origin: self.sink_origin, open_clips: self.open_clips };
        self.sink_origin += Vec2::new(bounds.x0, bounds.y0);
        self.open_clips = 0;
        outer
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

        let mut scene = match self.sink.replace(outer.sink) {
            Some(scene) => scene,
            None => panic!("The drawing context has been disposed"),
        };
        let position = self.sink_origin - outer.origin;
        self.sink_origin = outer.origin;
        self.open_clips = outer.open_clips;

        self.draw_filter_scene(&mut *scene, position.x, position.y, crate::perf::Phase::EffectAsImage);
    }

    /// A scene of its own for something that is drawn through a filter and
    /// composed into the scene of the context as a picture: on the device
    /// of the scene of the context by the hybrid renderer, which has filter
    /// layers, when the scene is drawn on a device (the hybrid mode, and the
    /// GPU mode, whose renderer has no filters, in a build with both); by
    /// the CPU renderer otherwise.
    pub(super) fn create_filter_scene(&mut self, width: u16, height: u16) -> Box<dyn IVelloSceneSink> {
        #[cfg(feature = "hybrid")]
        if let Some(device) = self.filter_scene_device() {
            return Box::new(crate::scene::VelloHybridSceneSink::new(device, width, height));
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
    /// and draws its picture into the scene of the context, its top left
    /// pixel at (`x`, `y`) of that scene: as a texture of the device when
    /// it was drawn there, as an image otherwise.
    pub(super) fn draw_filter_scene(&mut self, scene: &mut dyn IVelloSceneSink, x: f64, y: f64, phase: crate::perf::Phase) {
        let (width, height) = (scene.width(), scene.height());
        crate::perf::count(phase, u64::from(width) * u64::from(height) * 4);

        #[cfg(any(feature = "hybrid", feature = "gpu"))]
        if let Some(device) = scene.device().cloned() {
            if self.draws_on_device(&device) {
                let texture = crate::gpu::VelloDeviceTexture::new(
                    &device,
                    u32::from(width),
                    u32::from(height),
                    crate::gpu::VelloTextureAlpha::Premultiplied,
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

        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        scene.render_to_pixels(&mut rgba);
        let image = to_image(rgba, PixelSize::new(width as i32, height as i32));
        self.draw_image_at_pixels(image, x, y);
    }
}

impl IDrawingContextImplWithEffects for DrawingContextImpl {
    fn push_effect(&mut self, clip_rect: Option<Rect>, effect: &dyn IEffect) {
        let Some(filter) = self.create_effect(effect) else {
            // An effect that changes nothing is a layer.
            self.sink().push_layer(BlendMode::default(), 1.0);
            self.effect_stack.push(EffectFrame { cropped: false, outer: None });
            self.save(SavedKind::Effect);
            return;
        };

        let outer =
            if self.draws_effects_in_the_scene() { None } else { Some(self.begin_effect_scene(clip_rect, &filter)) };

        let transform = self.device_transform();
        self.sink().push_filter_layer(&filter, transform);
        self.effect_stack.push(EffectFrame { cropped: clip_rect.is_some(), outer });
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

/// A colour matrix layer that is pushed: what is drawn inside it is
/// recorded into a scene of the CPU renderer, whose pixels are transformed
/// by the matrix and composed into the scene the layer was pushed in when
/// it is popped.
///
/// vello 0.3 has no colour matrix filter (`vello_common` leaves
/// `ColorMatrix` unimplemented), so the matrix is applied to the pixels.
pub(super) struct ColorMatrixFrame {
    outer: OuterScene,
    matrix: ferroui_base::media::ColorMatrix,
    alpha_mode: ferroui_base::media::ColorMatrixAlphaMode,
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

        let scene: Box<dyn IVelloSceneSink> =
            Box::new(VelloCpuSceneSink::single_threaded(device.width() as u16, device.height() as u16));
        let sink = match self.sink.replace(scene) {
            Some(sink) => sink,
            None => panic!("The drawing context has been disposed"),
        };
        let outer = OuterScene { sink, origin: self.sink_origin, open_clips: self.open_clips };
        self.sink_origin += Vec2::new(device.x0, device.y0);
        self.open_clips = 0;
        outer
    }

    /// Ends the innermost colour matrix layer: its scene is rendered, its
    /// pixels transformed and drawn into the scene it was pushed in.
    pub(super) fn end_color_matrix(&mut self) {
        let frame = self.color_matrix_stack.pop().unwrap_or_else(|| panic!("The colour matrix stack is empty"));
        let mut scene = match self.sink.replace(frame.outer.sink) {
            Some(scene) => scene,
            None => panic!("The drawing context has been disposed"),
        };
        let position = self.sink_origin - frame.outer.origin;
        self.sink_origin = frame.outer.origin;
        self.open_clips = frame.outer.open_clips;

        let (width, height) = (scene.width(), scene.height());
        crate::perf::count(crate::perf::Phase::EffectAsImage, u64::from(width) * u64::from(height) * 4);
        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        scene.render_to_pixels(&mut rgba);
        frame.matrix.apply_to_rgba8(&mut rgba, frame.alpha_mode);
        let image = to_image(rgba, PixelSize::new(width as i32, height as i32));
        self.draw_image_at_pixels(image, position.x, position.y);
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

    fn push_color_matrix_layer(
        &mut self,
        bounds: Rect,
        matrix: &ferroui_base::media::ColorMatrix,
        alpha_mode: ferroui_base::media::ColorMatrixAlphaMode,
    ) {
        let outer = self.begin_cpu_scene(bounds);
        self.color_matrix_stack.push(ColorMatrixFrame { outer, matrix: *matrix, alpha_mode });
        self.save(SavedKind::ColorMatrix);
        // Nothing outside the bounds is kept.
        self.push_clip(bounds);
    }

    fn pop_color_matrix_layer(&mut self) {
        // The clip, then the layer.
        self.restore();
        self.restore();
    }
}
