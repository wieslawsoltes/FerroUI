//! The layers with an effect the engine has no group effect for: custom shader effects (#455),
//! colour matrices, multiply layers, convolutions, lighting and pixel operations, and acrylic.
//!
//! While such a layer is open, what is drawn is recorded into a scene of its own over the pixels
//! of the layer. When it closes, that scene is rendered into a texture of the pool, a pass of the
//! device (`effect_passes.rs`) makes the result, and the scene the layer was pushed in samples it,
//! one texel per pixel. All of it is on the GPU.

use super::tile_brush::{address_modes, positive_start};
use super::*;
use crate::effect_passes::{LayerPass, ShaderPlacement, NOISE_TILE_SIDE};
use crate::effect_records::{self as records, LightOutput};
use ferroui_base::media::effects::ImmutableCustomEffect;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{
    AcrylicBackgroundSource, ColorMatrix, ColorMatrixAlphaMode, ConvolveMatrix, IExperimentalAcrylicMaterial, TileMode,
};
use ferroui_base::platform::{
    IDrawingContextImplWithLayerEffects, IDrawingContextImplWithScene3D, IDrawingContextWithAcrylicLikeSupport,
    LightingParameters, MultiplyLayerFactors, PixelOperation,
};
use ferroui_base::rendering::scene3d::Scene3DFrame;

/// What a layer does to its content when it closes.
pub(super) enum Operation {
    Matrix { matrix: ColorMatrix, alpha_mode: ColorMatrixAlphaMode },
    Factors { factors: MultiplyLayerFactors, bounds: Rect },
    Lights(LightingParameters),
    Convolve(ConvolveMatrix),
    /// A pixel operation of two or more sources, with what the sources before made.
    Pixel { operation: PixelOperation, result: Option<Rc<GpuTexture>> },
    /// A custom effect; `whole` is its area in the pixels of the scene it was pushed in.
    Shader { effect: ImmutableCustomEffect, whole: Rect, scale: f32 },
    /// Nothing of the layer is on the target: its content is dropped.
    Discard,
}

/// What the context was recording when a layer with an effect opened.
struct OuterRecording {
    scene: Option<SceneBuilder>,
    textures: Vec<(SampledTexture, SceneImage)>,
    bitmaps: HashMap<(usize, i32), SceneImage>,
    current: Current,
    state: Option<u32>,
    glyph_state: Option<u32>,
    text_styles: HashMap<([u32; 4], bool), u32>,
    clips: Vec<ClipEntry>,
    analytic: Vec<progpu_native_scene_layer_mask>,
    layers: Vec<LayerEntry>,
    path_resources: HashMap<*const PathData, (Arc<PathData>, u32)>,
    origin: (f64, f64),
    target_size: Option<(u32, u32)>,
}

/// An open layer with an effect: the entry at the bottom of the stack of layers of its scene.
pub(super) struct SubScene {
    outer: OuterRecording,
    /// The pixels of the layer, in the scene it was pushed in.
    area: Rect,
    /// From the pixels of the layer to the coordinates of the drawing.
    to_local: Matrix,
    /// What cuts the content, in the coordinates of the drawing.
    clip: Option<Rect>,
    operation: Operation,
}

fn rounded_out(rect: Rect) -> Rect {
    let (left, top) = (rect.x.floor(), rect.y.floor());
    Rect::new(left, top, (rect.x + rect.width).ceil() - left, (rect.y + rect.height).ceil() - top)
}

fn has_pixels(rect: Rect) -> bool {
    rect.width >= 1.0 && rect.height >= 1.0
}

fn pixel_size(area: Rect) -> (u32, u32) {
    (area.width as u32, area.height as u32)
}

/// The straight colour of `top` over `bottom`, through their premultiplied values.
fn composed(top: Color, bottom: Color) -> Color {
    let stored = |color: Color| {
        let alpha = f32::from(color.a) / 255.0;
        [f32::from(color.r) * alpha, f32::from(color.g) * alpha, f32::from(color.b) * alpha, alpha]
    };
    let (upper, lower) = (stored(top), stored(bottom));
    let through = 1.0 - upper[3];
    let alpha = upper[3] + lower[3] * through;
    if alpha <= 0.0 {
        return Color::from_argb(0, 0, 0, 0);
    }
    let channel = |index: usize| ((upper[index] + lower[index] * through) / alpha).round() as u8;
    Color::from_argb((alpha * 255.0).round() as u8, channel(0), channel(1), channel(2))
}

impl ProGpuDrawingContext {
    /// The device, when it can run passes.
    pub(super) fn pass_device(&self) -> Option<Rc<Gpu>> {
        self.gpu.clone().filter(|gpu| gpu.offscreen().is_some())
    }

    /// The area of a layer over `bounds` (the whole target without them) in whole pixels of this
    /// scene, and the part of it on the target. `None` for an area that is not finite, or on a
    /// context that does not know its target.
    fn layer_area(&self, bounds: Option<Rect>) -> Option<(Rect, Rect)> {
        let (width, height) = self.target_size?;
        let target = Rect::new(0.0, 0.0, f64::from(width), f64::from(height));
        let whole = match bounds {
            Some(bounds) => rounded_out(transform_bounds(bounds, &self.current.transform)),
            None => target,
        };
        [whole.x, whole.y, whole.width, whole.height]
            .iter()
            .all(|value| value.is_finite())
            .then(|| (whole, intersect(whole, target)))
    }

    /// Starts recording the content of a layer over `area` of this scene, in a scene of its own.
    fn enter_sub_scene(&mut self, gpu: &Gpu, area: Rect, clip: Option<Rect>, operation: Operation) {
        let mut scene = gpu.passes().take_scene();
        let begun = scene.begin_generation();
        self.record(begun);
        let transform = self.current.transform * Matrix::create_translation(-area.x, -area.y);
        let outer = OuterRecording {
            scene: self.scene.replace(scene),
            textures: std::mem::take(&mut self.textures),
            bitmaps: std::mem::take(&mut self.bitmaps),
            current: self.current,
            state: self.state.take(),
            glyph_state: self.glyph_state.take(),
            text_styles: std::mem::take(&mut self.text_styles),
            clips: std::mem::take(&mut self.clips),
            analytic: std::mem::take(&mut self.analytic),
            layers: std::mem::take(&mut self.layers),
            path_resources: std::mem::take(&mut self.path_resources),
            origin: self.origin,
            target_size: self.target_size,
        };
        self.origin = (self.origin.0 + area.x, self.origin.1 + area.y);
        self.target_size = Some(pixel_size(area));
        self.current = Current { transform, clip_rect: None, mask: NO_INDEX };
        let to_local = transform.try_invert().unwrap_or(Matrix::IDENTITY);
        self.layers.push(LayerEntry::SubScene(Box::new(SubScene { outer, area, to_local, clip, operation })));
        // Nothing outside the bounds is kept.
        if let Some(clip) = clip {
            self.push_clip(clip);
        }
    }

    /// Ends the recording of a layer: its content rendered into a texture of the pool (unless it
    /// is dropped), and the recording it was pushed in back in place.
    fn leave_sub_scene(&mut self, outer: OuterRecording, area: Rect, drop_content: bool) -> Option<Rc<GpuTexture>> {
        while !self.clips.is_empty() {
            self.pop_clip();
        }
        let gpu = self.gpu.clone();
        let mut scene = self.scene.take().expect("the scene of a layer");
        let images: Vec<SampledTexture> = self.textures.drain(..).map(|(texture, _)| texture).collect();
        let mut content = None;
        if let Some(gpu) = gpu.as_ref().filter(|_| !drop_content) {
            if let Some(engine) = gpu.offscreen() {
                let (width, height) = pixel_size(area);
                let target = gpu.passes().texture(gpu, width, height);
                let mut recording = Recording { scene, error: None, images, damage: None };
                match recording.render(engine, target.view(), target.width(), target.height(), false) {
                    Ok(()) => {
                        target.changed();
                        content = Some(target);
                    }
                    Err(error) => notes::failure(&format!("the content of a layer was not drawn: {error}")),
                }
                scene = recording.scene;
            }
        }
        if let Some(gpu) = &gpu {
            gpu.passes().give_scene(gpu, scene);
        }
        self.scene = outer.scene;
        self.textures = outer.textures;
        self.bitmaps = outer.bitmaps;
        self.current = outer.current;
        self.state = outer.state;
        self.glyph_state = outer.glyph_state;
        self.text_styles = outer.text_styles;
        self.clips = outer.clips;
        self.analytic = outer.analytic;
        self.layers = outer.layers;
        self.path_resources = outer.path_resources;
        self.origin = outer.origin;
        self.target_size = outer.target_size;
        // The scene now samples a texture made for this frame.
        self.needs_whole_replay = true;
        content
    }

    /// The result of a layer in `area` of this scene, one texel per pixel, under the clips.
    fn draw_area(&mut self, texture: &Rc<GpuTexture>, area: Rect) {
        let Some(image) = self.sampled_texture(texture, false) else { return };
        let draw = progpu_native_scene_image_draw {
            struct_size: std::mem::size_of::<progpu_native_scene_image_draw>() as u32,
            flags: PROGPU_NATIVE_SCENE_IMAGE_SOURCE_PREMULTIPLIED as u32,
            image_width: image.width,
            image_height: image.height,
            row_bytes: image.row_bytes,
            sampling: PROGPU_NATIVE_IMAGE_SAMPLING_NEAREST as u32,
            source_rect: image_rect(Rect::new(0.0, 0.0, area.width, area.height)),
            destination_rect: image_rect(area),
            transform: identity(),
            opacity: 1.0,
            max_anisotropy: 1,
        };
        let Some(state) = self.glyph_state() else { return };
        let result = self.scene().draw_image(image.index, &draw, image_rect(area), state);
        self.record(result);
    }

    /// Opens a layer that changes its content when it closes. `false`, with nothing pushed, on a
    /// context without a device.
    fn push_operation(&mut self, bounds: Option<Rect>, operation: Operation) -> bool {
        self.refuse_fragment();
        let Some(gpu) = self.pass_device() else { return false };
        let Some((_, visible)) = self.layer_area(bounds) else { return false };
        if has_pixels(visible) {
            self.enter_sub_scene(&gpu, visible, bounds, operation);
        } else {
            self.enter_sub_scene(&gpu, Rect::new(0.0, 0.0, 1.0, 1.0), None, Operation::Discard);
        }
        true
    }

    /// A layer that keeps its content as it is, where the effect cannot be drawn.
    fn push_plain_layer(&mut self, bounds: Rect, key: &str, message: &str) {
        notes::note_once(key, message);
        self.push_scene_layer(Some(bounds), 1.0, PROGPU_NATIVE_BLEND_SRC_OVER as u32);
    }

    /// The luminance of the content as its alpha, over `bounds`.
    pub(super) fn push_luminance_layer(&mut self, bounds: Option<Rect>) -> bool {
        let matrix = records::luminance_mask();
        self.push_operation(bounds, Operation::Matrix { matrix, alpha_mode: ColorMatrixAlphaMode::Straight })
    }

    /// Opens a custom effect over `clip_rect` (the whole target without one). `false`, with
    /// nothing pushed, when it cannot be drawn: it is then drawn as an effect without a factory.
    pub(super) fn push_shader_effect(&mut self, clip_rect: Option<Rect>, effect: &ImmutableCustomEffect) -> bool {
        if effect.check_shader_children().is_err() {
            effect.report_unhandled("ProGPU");
            return false;
        }
        self.refuse_fragment();
        let Some(gpu) = self.pass_device() else { return false };
        let m = self.current.transform;
        let scale = ((m.m11.hypot(m.m12) + m.m21.hypot(m.m22)) / 2.0) as f32;
        let Some((whole, visible)) = self.layer_area(clip_rect) else { return false };
        if !has_pixels(whole) || !scale.is_finite() || scale <= 0.0 {
            return false;
        }
        if !has_pixels(visible) {
            self.enter_sub_scene(&gpu, Rect::new(0.0, 0.0, 1.0, 1.0), None, Operation::Discard);
            return true;
        }
        if !gpu.passes().prepare_shader(&gpu, effect) {
            effect.report_unhandled("ProGPU");
            return false;
        }
        let operation = Operation::Shader { effect: effect.clone(), whole, scale };
        self.enter_sub_scene(&gpu, visible, None, operation);
        true
    }

    /// Two results of a pixel operation combined: what the sources before made and the next one.
    fn combine(
        gpu: &Gpu,
        operation: &PixelOperation,
        first: Option<Rc<GpuTexture>>,
        next: Rc<GpuTexture>,
        size: (u32, u32),
        to_local: Matrix,
    ) -> Rc<GpuTexture> {
        let Some(first) = first else { return next };
        records::composite(operation, &to_local)
            .and_then(|record| gpu.passes().run(gpu, &LayerPass::Composite(record), &[&first, &next], size))
            .unwrap_or(next)
    }

    /// Closes a layer with an effect: its content, through its pass, drawn into the scene it was
    /// pushed in. The content unchanged should the pass fail.
    pub(super) fn end_sub_scene(&mut self, layer: SubScene) {
        let SubScene { outer, area, to_local, clip: _, operation } = layer;
        let discard = matches!(operation, Operation::Discard);
        let content = self.leave_sub_scene(outer, area, discard);
        let (Some(gpu), Some(content)) = (self.gpu.clone(), content) else { return };
        let size = pixel_size(area);
        let passes = gpu.passes();
        let result = match &operation {
            Operation::Matrix { matrix, alpha_mode } => {
                let pass = LayerPass::ColorMatrix(records::color_matrix(matrix, *alpha_mode));
                passes.run(&gpu, &pass, &[&content], size)
            }
            Operation::Factors { factors, bounds } => passes.factor_texture(&gpu, factors).and_then(|texture| {
                let pass = LayerPass::FactorMultiply(records::factor_multiply(factors.scale, *bounds, &to_local));
                passes.run(&gpu, &pass, &[&content, &texture], size)
            }),
            Operation::Lights(lighting) => records::lighting(lighting, LightOutput::Multiply, (0, 0), &to_local)
                .and_then(|record| passes.run(&gpu, &LayerPass::Lighting(record), &[&content], size)),
            Operation::Convolve(convolution) => {
                passes.run(&gpu, &LayerPass::Convolution(records::convolution(convolution)), &[&content], size)
            }
            Operation::Pixel { operation, result } => {
                Some(Self::combine(&gpu, operation, result.clone(), content.clone(), size, to_local))
            }
            Operation::Shader { effect, whole, scale } => {
                let placement = ShaderPlacement {
                    first_texel: [(area.x - whole.x) as f32, (area.y - whole.y) as f32],
                    whole: [whole.width as f32, whole.height as f32],
                    scale: *scale,
                    pixels: size,
                };
                passes.run_shader(&gpu, &content, effect, placement)
            }
            Operation::Discard => None,
        };
        self.draw_area(&result.unwrap_or(content), area);
    }
}

impl IDrawingContextImplWithLayerEffects for ProGpuDrawingContext {
    fn push_blend_layer(&mut self, _bounds: Rect, mode: BitmapBlendingMode) {
        self.push_scene_layer(None, 1.0, native_blend_mode(mode));
    }

    fn pop_blend_layer(&mut self) {
        self.pop_scene_layer();
    }

    fn push_color_matrix_layer(&mut self, bounds: Rect, matrix: &ColorMatrix, alpha_mode: ColorMatrixAlphaMode) {
        if !self.push_operation(Some(bounds), Operation::Matrix { matrix: *matrix, alpha_mode }) {
            self.push_plain_layer(bounds, "color-matrix-layer", "a colour matrix layer needs a device");
        }
    }

    fn pop_color_matrix_layer(&mut self) {
        self.pop_scene_layer();
    }

    fn push_multiply_layer(&mut self, bounds: Rect, factors: &MultiplyLayerFactors) {
        let usable = bounds.width > 0.0 && bounds.height > 0.0;
        let operation = Operation::Factors { factors: factors.clone(), bounds };
        if !(usable && self.push_operation(Some(bounds), operation)) {
            self.push_plain_layer(bounds, "multiply-layer", "a multiply layer needs a device and an area");
        }
    }

    fn pop_multiply_layer(&mut self) {
        self.pop_scene_layer();
    }

    fn push_convolve_matrix_layer(&mut self, bounds: Rect, convolution: &ConvolveMatrix) {
        if !self.push_operation(Some(bounds), Operation::Convolve(*convolution)) {
            self.push_plain_layer(bounds, "convolve-layer", "a convolution layer needs a device");
        }
    }

    fn pop_convolve_matrix_layer(&mut self) {
        self.pop_scene_layer();
    }

    fn draw_lighting(&mut self, rect: Rect, lighting: &LightingParameters) -> bool {
        if lighting.lights.len() > LightingParameters::MAX_LIGHTS {
            return false;
        }
        let Some(gpu) = self.pass_device() else { return false };
        self.refuse_fragment();
        let Some((_, visible)) = self.layer_area(Some(rect)) else { return false };
        if !has_pixels(visible) {
            return true;
        }
        let Some(to_local) = self.current.transform.try_invert() else { return true };
        let origin = (visible.x as u32, visible.y as u32);
        let Some(record) = records::lighting(lighting, LightOutput::Fill, origin, &to_local) else { return false };
        let size = pixel_size(visible);
        let Some(result) = gpu.passes().run(&gpu, &LayerPass::Lighting(record), &[], size) else {
            return false;
        };
        self.push_clip(rect);
        self.draw_area(&result, visible);
        self.pop_clip();
        true
    }

    fn push_lighting_multiply_layer(&mut self, bounds: Rect, lighting: &LightingParameters) -> bool {
        lighting.lights.len() <= LightingParameters::MAX_LIGHTS
            && self.push_operation(Some(bounds), Operation::Lights(lighting.clone()))
    }

    fn push_pixel_operation(&mut self, bounds: Rect, operation: &PixelOperation) -> bool {
        if let PixelOperation::Blend { noise_grid, .. } = operation {
            if noise_grid.width <= 0.0 || noise_grid.height <= 0.0 {
                return false;
            }
        }
        self.push_operation(Some(bounds), Operation::Pixel { operation: *operation, result: None })
    }

    fn next_pixel_operation_source(&mut self) {
        if !matches!(self.layers.last(), Some(LayerEntry::SubScene(_))) {
            return;
        }
        let Some(LayerEntry::SubScene(layer)) = self.layers.pop() else { return };
        let SubScene { outer, area, to_local, clip, operation } = *layer;
        let discard = matches!(operation, Operation::Discard);
        let content = self.leave_sub_scene(outer, area, discard);
        let operation = match (operation, self.gpu.clone(), content) {
            (Operation::Pixel { operation, result }, Some(gpu), Some(content)) => {
                let result = Self::combine(&gpu, &operation, result, content, pixel_size(area), to_local);
                Operation::Pixel { operation, result: Some(result) }
            }
            (other, ..) => other,
        };
        if let Some(gpu) = self.gpu.clone() {
            self.enter_sub_scene(&gpu, area, clip, operation);
        }
    }

    fn pop_pixel_operation(&mut self) {
        self.pop_scene_layer();
    }
}

impl IDrawingContextImplWithScene3D for ProGpuDrawingContext {
    /// The scene drawn on the device (`scene3d.rs`) into a texture, which is drawn over the bounds
    /// like an image, under the transform, clips and layers of the context.
    fn draw_scene_3d(&mut self, frame: &Scene3DFrame) -> bool {
        let bounds = frame.bounds;
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return true;
        }
        let Some(gpu) = self.pass_device() else { return false };
        self.refuse_fragment();
        let m = self.current.transform;
        let (size, scale) = ferroui_render3d::scene_pixel_size(bounds, (m.m11, m.m12), (m.m21, m.m22));
        let antialiased = self.render_options.last().is_none_or(|options| options.edge_mode != EdgeMode::Aliased);
        // The pixel of the target at the corner of the viewport names the viewport.
        let whole = self.transform();
        let corner = (
            bounds.x * whole.m11 + bounds.y * whole.m21 + whole.m31,
            bounds.x * whole.m12 + bounds.y * whole.m22 + whole.m32,
        );
        let origin = (corner.0.round() as i32, corner.1.round() as i32);
        let Some(drawn) = gpu.scenes_3d().scene_texture(&gpu, &frame.scene, size, scale, antialiased, origin) else {
            return false;
        };
        let Some(image) = self.sampled_texture(&drawn.texture, false) else { return false };
        let source = Rect::new(0.0, 0.0, f64::from(drawn.size.0), f64::from(drawn.size.1));
        let draw = progpu_native_scene_image_draw {
            struct_size: std::mem::size_of::<progpu_native_scene_image_draw>() as u32,
            flags: PROGPU_NATIVE_SCENE_IMAGE_SOURCE_PREMULTIPLIED as u32,
            image_width: image.width,
            image_height: image.height,
            row_bytes: image.row_bytes,
            sampling: PROGPU_NATIVE_IMAGE_SAMPLING_LINEAR as u32,
            source_rect: image_rect(source),
            destination_rect: image_rect(bounds),
            transform: identity(),
            opacity: 1.0,
            max_anisotropy: 1,
        };
        let device_bounds = transform_bounds(bounds, &self.current.transform);
        let Some(state) = self.state() else { return false };
        let result = self.scene().draw_image(image.index, &draw, image_rect(device_bounds), state);
        self.record(result).is_some()
    }
}

impl IDrawingContextWithAcrylicLikeSupport for ProGpuDrawingContext {
    /// The tint over the colour of the material, then the tile of the engine's material noise
    /// repeated over both. A material that digs through to what is behind the window replaces
    /// what is under it.
    fn draw_rectangle_with_material(&mut self, material: &dyn IExperimentalAcrylicMaterial, rect: RoundedRect) {
        if rect.rect.width <= 0.0 || rect.rect.height <= 0.0 {
            return;
        }
        self.refuse_fragment();
        let tint = composed(material.tint_color(), material.material_color());
        if material.background_source() == AcrylicBackgroundSource::Digger {
            self.push_clip_rounded(rect);
            self.clear(tint);
            self.pop_clip();
        } else {
            let brush = ImmutableSolidColorBrush::new(tint);
            self.draw_rectangle(Some(&brush), None, rect, &BoxShadows::default());
        }
        let Some(gpu) = self.pass_device() else { return };
        let Some(noise) = gpu.passes().noise_tile(&gpu) else { return };
        self.push_clip_rounded(rect);
        if let Some(image) = self.sampled_texture(&noise, false) {
            let (address_u, address_v) = address_modes(TileMode::Tile);
            let area = rect.rect;
            let side = f64::from(NOISE_TILE_SIDE);
            let (start_x, start_y) = (positive_start(area.x, side), positive_start(area.y, side));
            let source = Rect::new(start_x, start_y, area.width, area.height);
            let draw = progpu_native_scene_image_draw {
                struct_size: std::mem::size_of::<progpu_native_scene_image_draw>() as u32,
                flags: PROGPU_NATIVE_SCENE_IMAGE_SOURCE_PREMULTIPLIED as u32
                    | (address_u << PROGPU_NATIVE_SCENE_IMAGE_ADDRESS_U_SHIFT)
                    | (address_v << PROGPU_NATIVE_SCENE_IMAGE_ADDRESS_V_SHIFT)
                    | PROGPU_NATIVE_SCENE_IMAGE_EXTENDED_SOURCE_RECT as u32,
                image_width: image.width,
                image_height: image.height,
                row_bytes: image.row_bytes,
                sampling: PROGPU_NATIVE_IMAGE_SAMPLING_NEAREST as u32,
                source_rect: image_rect(source),
                destination_rect: image_rect(area),
                transform: identity(),
                opacity: 1.0,
                max_anisotropy: 1,
            };
            let bounds = transform_bounds(area, &self.current.transform);
            if let Some(state) = self.state() {
                let result = self.scene().draw_image(image.index, &draw, image_rect(bounds), state);
                self.record(result);
            }
        }
        self.pop_clip();
    }
}
