//! The drawing context (docs/porting/progpu-backend.md, section 2.3): the drawing calls of the
//! base library recorded into a retained scene of the engine.
//!
//! - The transform and the clips are draw states: an axis-aligned rectangle clip under an
//!   axis-aligned transform is the state's clip rectangle; a rounded or geometry clip is the
//!   state's mask (coverage per draw, no layer); a second mask nests as a layer.
//! - Rectangles, rounded rectangles with one radius, ellipses and their strokes are analytic
//!   primitives (batched by the engine); a box shadow is the engine's analytic shadow, clipped
//!   out of its element by a difference mask when the element is not opaque.
//! - Geometries fill their canonical segments; their strokes fill the engine's widening of the
//!   pen (computed once per geometry and pen).
//! - Group opacity, layers and compositing blend modes are layers of the scene.

use crate::bitmap::ProGpuBitmap;
use crate::fragment::{moved, moved_rect, FragmentBrush, FragmentOp, FragmentRecorder, KeptRun, ProGpuFragment};
use crate::geometry::{path_data_of, PathData};
use tile_brush::is_tile_brush;
use crate::glyph_run::{Placement, ProGpuGlyphRun};
use crate::gpu::{Gpu, GpuTexture, Recording, SampledTexture};
use crate::layer::ProGpuLayer;
use crate::notes;
use crate::scene::{SceneBuildError, SceneBuilder, NO_INDEX};
use ferroui_base::media::effects::IEffect;
use ferroui_base::media::imaging::{BitmapBlendingMode, BitmapInterpolationMode};
use ferroui_base::media::{BoxShadows, Color, EdgeMode, GradientSpreadMethod, IBrush, IPen, RenderOptions, TextOptions, TextRenderingMode};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IDrawingContextImplWithEffects, IDrawingContextImplWithFragments,
    IDrawingContextLayerImpl, IDrawingFragment, IGeometryImpl, IGlyphRunImpl,
    IPlatformRenderInterfaceRegion, PixelFormat,
};
use ferroui_base::{Matrix, PixelSize, Point, Rect, RoundedRect, Vector};
use progpu_native_sys::progpu::*;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// The coverage sampling of path fills and clips: a 4x4 grid per pixel (the engine accepts 1, 4
/// or 8).
const PATH_SAMPLE_GRID: u32 = 4;

/// The coverage transfers of greyscale text (`progpu_native_scene_text_style::coverage_transfer`)
/// by the lightness of its colour, darkest first: alpha = 1 - (1 - coverage)^e with e = 0.5 +
/// (value - 14) / 32. The text the Skia backend draws on macOS gets heavier the lighter its
/// colour is, in four steps; the exponents are measured against it
/// (docs/porting/progpu-backend.md, the text quality check).
const TEXT_COVERAGE_TRANSFERS: [u32; 4] = [28, 35, 45, 56];

/// The coverage transfer of greyscale text of a colour: by a quarter of its red and three
/// quarters of its green, in four steps of 64 (measured on 25 colours; blue did not count).
fn text_coverage_transfer(color: progpu_native_color) -> u32 {
    let lightness = (0.25 * color.r + 0.75 * color.g).clamp(0.0, 1.0) * 255.0;
    TEXT_COVERAGE_TRANSFERS[((lightness as u32) >> 6).min(3) as usize]
}

/// The steps of the raster scale of text under a transform that is not upright: eight to a
/// doubling, so a scale that animates does not fill the atlas with a raster per frame.
const FREE_SCALE_STEPS: f64 = 8.0;

/// The largest sigma the engine takes for a Gaussian, in device pixels.
const MAX_EFFECT_SIGMA: f32 = 128.0 / 3.0;

/// The blend mode of the engine for a blend mode of the base library.
fn native_blend_mode(mode: BitmapBlendingMode) -> u32 {
    (match mode {
        BitmapBlendingMode::Unspecified | BitmapBlendingMode::SourceOver => PROGPU_NATIVE_BLEND_SRC_OVER,
        BitmapBlendingMode::Source => PROGPU_NATIVE_BLEND_SRC,
        BitmapBlendingMode::Destination => PROGPU_NATIVE_BLEND_DST,
        BitmapBlendingMode::DestinationOver => PROGPU_NATIVE_BLEND_DST_OVER,
        BitmapBlendingMode::SourceIn => PROGPU_NATIVE_BLEND_SRC_IN,
        BitmapBlendingMode::DestinationIn => PROGPU_NATIVE_BLEND_DST_IN,
        BitmapBlendingMode::SourceOut => PROGPU_NATIVE_BLEND_SRC_OUT,
        BitmapBlendingMode::DestinationOut => PROGPU_NATIVE_BLEND_DST_OUT,
        BitmapBlendingMode::SourceAtop => PROGPU_NATIVE_BLEND_SRC_ATOP,
        BitmapBlendingMode::DestinationAtop => PROGPU_NATIVE_BLEND_DST_ATOP,
        BitmapBlendingMode::Xor => PROGPU_NATIVE_BLEND_XOR,
        BitmapBlendingMode::Plus => PROGPU_NATIVE_BLEND_PLUS,
        BitmapBlendingMode::Screen => PROGPU_NATIVE_BLEND_SCREEN,
        BitmapBlendingMode::Overlay => PROGPU_NATIVE_BLEND_OVERLAY,
        BitmapBlendingMode::Darken => PROGPU_NATIVE_BLEND_DARKEN,
        BitmapBlendingMode::Lighten => PROGPU_NATIVE_BLEND_LIGHTEN,
        BitmapBlendingMode::ColorDodge => PROGPU_NATIVE_BLEND_COLOR_DODGE,
        BitmapBlendingMode::ColorBurn => PROGPU_NATIVE_BLEND_COLOR_BURN,
        BitmapBlendingMode::HardLight => PROGPU_NATIVE_BLEND_HARD_LIGHT,
        BitmapBlendingMode::SoftLight => PROGPU_NATIVE_BLEND_SOFT_LIGHT,
        BitmapBlendingMode::Difference => PROGPU_NATIVE_BLEND_DIFFERENCE,
        BitmapBlendingMode::Exclusion => PROGPU_NATIVE_BLEND_EXCLUSION,
        BitmapBlendingMode::Multiply => PROGPU_NATIVE_BLEND_MULTIPLY,
        BitmapBlendingMode::Hue => PROGPU_NATIVE_BLEND_HUE,
        BitmapBlendingMode::Saturation => PROGPU_NATIVE_BLEND_SATURATION,
        BitmapBlendingMode::Color => PROGPU_NATIVE_BLEND_COLOR,
        BitmapBlendingMode::Luminosity => PROGPU_NATIVE_BLEND_LUMINOSITY,
    }) as u32
}

/// The sigma of a blur radius, as the other backends convert it.
pub(crate) fn blur_radius_to_sigma(radius: f64) -> f32 {
    if radius <= 0.0 {
        0.0
    } else {
        0.288675 * radius as f32 + 0.5
    }
}

pub(crate) fn affine(matrix: &Matrix) -> progpu_native_affine_2d {
    progpu_native_affine_2d {
        m11: matrix.m11 as f32,
        m12: matrix.m12 as f32,
        m21: matrix.m21 as f32,
        m22: matrix.m22 as f32,
        m31: matrix.m31 as f32,
        m32: matrix.m32 as f32,
    }
}

#[path = "tile_brush.rs"]
mod tile_brush;
#[path = "effect_layers.rs"]
mod effect_layers;

fn identity() -> progpu_native_affine_2d {
    affine(&Matrix::IDENTITY)
}

fn image_rect(rect: Rect) -> progpu_native_image_rect {
    progpu_native_image_rect { x: rect.x as f32, y: rect.y as f32, width: rect.width as f32, height: rect.height as f32 }
}

fn native_color(color: Color, opacity: f64) -> progpu_native_color {
    progpu_native_color {
        r: color.r as f32 / 255.0,
        g: color.g as f32 / 255.0,
        b: color.b as f32 / 255.0,
        a: (color.a as f64 / 255.0 * opacity) as f32,
    }
}

fn axis_aligned(matrix: &Matrix) -> bool {
    matrix.m12 == 0.0 && matrix.m21 == 0.0 && matrix.m13 == 0.0 && matrix.m23 == 0.0
}

/// Whether a rectangle covers anything: a width and a height above zero, all finite.
fn has_area(rect: Rect) -> bool {
    rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite() && rect.width > 0.0 && rect.height > 0.0
}

/// Whether a transform maps areas to areas: what the engine takes as the transform of a mask.
fn invertible(matrix: &Matrix) -> bool {
    let determinant = matrix.m11 * matrix.m22 - matrix.m12 * matrix.m21;
    determinant.is_finite() && determinant.abs() > 0.000001
}

/// The axis-aligned bounds of a rectangle under a transform.
fn transform_bounds(rect: Rect, matrix: &Matrix) -> Rect {
    let corners = [
        matrix.transform(Point::new(rect.x, rect.y)),
        matrix.transform(Point::new(rect.x + rect.width, rect.y)),
        matrix.transform(Point::new(rect.x, rect.y + rect.height)),
        matrix.transform(Point::new(rect.x + rect.width, rect.y + rect.height)),
    ];
    let (mut left, mut top, mut right, mut bottom) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for corner in corners {
        left = left.min(corner.x);
        top = top.min(corner.y);
        right = right.max(corner.x);
        bottom = bottom.max(corner.y);
    }
    Rect::new(left, top, right - left, bottom - top)
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    Rect::new(left, top, (right - left).max(0.0), (bottom - top).max(0.0))
}

/// One clip of the stack: what it set, to restore it.
#[derive(Clone, Copy)]
enum ClipEntry {
    /// The state's clip rectangle and mask before the clip, and how many analytic masks the
    /// mask was made of.
    State { clip_rect: Option<Rect>, mask: u32, analytic: usize },
    /// A clip that needed a layer.
    Layer,
}

/// What a layer of the stack is, to pop it.
enum LayerEntry {
    Layer,
    /// A group that an image or visual brush masks when it closes: the tile is drawn over the
    /// content and keeps it where it has alpha.
    TileMask(Box<tile_brush::TileMask>),
    /// A layer with an effect: what is above it in the stack is recorded into a scene of its
    /// own, and this entry holds the recording it was pushed in.
    SubScene(Box<effect_layers::SubScene>),
}

/// The current state of the context (what the next draw's state is made of).
#[derive(Clone, Copy)]
struct Current {
    transform: Matrix,
    clip_rect: Option<Rect>,
    mask: u32,
}

/// The image a scene draws: its resource and the measures its draws repeat.
#[derive(Clone, Copy)]
struct SceneImage {
    index: u32,
    width: u32,
    height: u32,
    row_bytes: u32,
    premultiplied: bool,
    opaque: bool,
}

/// A drawing context recording into a scene of the engine. The owner takes the scene back with
/// [`ProGpuDrawingContext::finish`] (or receives it through the completion of `dispose`).
pub struct ProGpuDrawingContext {
    scene: Option<SceneBuilder>,
    /// The device the scene is rendered on: what its layers are made on.
    gpu: Option<Rc<Gpu>>,
    /// The textures the scene samples, with their image resources.
    textures: Vec<(SampledTexture, SceneImage)>,
    /// The image resources of the bitmaps of pixels the scene draws, by bitmap and version.
    bitmaps: HashMap<(usize, i32), SceneImage>,
    current: Current,
    state: Option<u32>,
    /// The state of the current clips without the transform: what glyphs at whole device
    /// pixels are drawn with.
    glyph_state: Option<u32>,
    /// The text styles of the scene, by colour and mode.
    text_styles: HashMap<([u32; 4], bool), u32>,
    /// The glyphs of the run being drawn (kept for its allocation).
    positioned: Vec<progpu_native_positioned_glyph>,
    clips: Vec<ClipEntry>,
    /// The analytic masks the mask of the state is made of: the rounded and the rotated
    /// rectangle clips in force. Empty under no mask and under a mask with a texture.
    analytic: Vec<progpu_native_scene_layer_mask>,
    layers: Vec<LayerEntry>,
    render_options: Vec<RenderOptions>,
    text_options: Vec<TextOptions>,
    error: Option<SceneBuildError>,
    complete: Option<Box<dyn FnOnce(Recording)>>,
    /// What is being kept as a fragment (`fragment.rs`).
    fragment: Option<FragmentRecorder>,
    /// What this frame changes of a target that keeps its content.
    damage: Option<Rect>,
    /// The scene has a layer or a state with a mask: the engine cannot replay it in part.
    needs_whole_replay: bool,
    /// The geometries drawn in this frame, by address, with the resource of their first draw.
    path_resources: HashMap<*const PathData, (Arc<PathData>, u32)>,
    /// The pixel of the target at the top left of the scene being recorded: not zero inside a
    /// layer with an effect (`effect_layers.rs`), whose scene covers only the layer.
    origin: (f64, f64),
    /// The size in pixels of what the scene being recorded is rendered into.
    target_size: Option<(u32, u32)>,
}

impl ProGpuDrawingContext {
    /// A context recording the next generation of `scene`; `complete` receives the scene at
    /// `dispose`. The context has no device: it records, and draws no layers.
    pub fn new(scene: SceneBuilder, complete: Option<Box<dyn FnOnce(SceneBuilder, Option<SceneBuildError>)>>) -> Self {
        Self::with_gpu(
            scene,
            None,
            complete.map(|complete| Box::new(move |recording: Recording| complete(recording.scene, recording.error)) as Box<dyn FnOnce(Recording)>),
        )
    }

    /// A context recording the next generation of `scene` for a frame on the device of `gpu`;
    /// `complete` receives what was recorded at `dispose`.
    pub(crate) fn with_gpu(mut scene: SceneBuilder, gpu: Option<Rc<Gpu>>, complete: Option<Box<dyn FnOnce(Recording)>>) -> Self {
        let error = scene.begin_generation().err();
        if let Some(gpu) = &gpu {
            gpu.images().begin_frame();
            gpu.passes().begin_frame();
        }
        ProGpuDrawingContext {
            scene: Some(scene),
            gpu,
            textures: Vec::new(),
            bitmaps: HashMap::new(),
            current: Current { transform: Matrix::IDENTITY, clip_rect: None, mask: NO_INDEX },
            state: None,
            glyph_state: None,
            text_styles: HashMap::new(),
            positioned: Vec::new(),
            clips: Vec::new(),
            analytic: Vec::new(),
            layers: Vec::new(),
            render_options: Vec::new(),
            text_options: Vec::new(),
            error,
            complete,
            fragment: None,
            damage: None,
            needs_whole_replay: false,
            path_resources: HashMap::new(),
            origin: (0.0, 0.0),
            target_size: None,
        }
    }

    /// Says how large the target of the frame is: what a layer with an effect that has no bounds
    /// covers, and what cuts the others.
    pub(crate) fn with_target_size(mut self, width: u32, height: u32) -> Self {
        self.target_size = Some((width, height));
        self
    }

    /// The recorded scene and the first error of the recording, if any.
    pub fn finish(mut self) -> (SceneBuilder, Option<SceneBuildError>) {
        (self.scene.take().expect("the scene of the context"), self.error.take())
    }

    fn scene(&mut self) -> &mut SceneBuilder {
        self.scene.as_mut().expect("the scene of a disposed context")
    }

    fn record<T>(&mut self, result: Result<T, SceneBuildError>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.error.get_or_insert(error);
                None
            }
        }
    }

    /// The state resource of the current transform and clips, added once per change.
    fn state(&mut self) -> Option<u32> {
        if let Some(index) = self.state {
            return Some(index);
        }
        let index = self.add_state(self.current.transform)?;
        self.state = Some(index);
        Some(index)
    }

    /// The state resource of the current clips under the identity transform, added once per
    /// change: the clips of a state are in the coordinates of the target.
    fn glyph_state(&mut self) -> Option<u32> {
        if self.current.transform.is_identity() {
            return self.state();
        }
        if let Some(index) = self.glyph_state {
            return Some(index);
        }
        let index = self.add_state(Matrix::IDENTITY)?;
        self.glyph_state = Some(index);
        Some(index)
    }

    fn add_state(&mut self, transform: Matrix) -> Option<u32> {
        let mut state = SceneBuilder::identity_state();
        state.transform = affine(&transform);
        state.opacity = 1.0;
        if let Some(clip) = self.current.clip_rect {
            state.flags |= PROGPU_NATIVE_SCENE_STATE_CLIP_RECT as u32;
            state.clip_rect = image_rect(clip);
        }
        if self.current.mask != NO_INDEX {
            self.needs_whole_replay = true;
            state.flags |= PROGPU_NATIVE_SCENE_STATE_MASK as u32;
            state.mask_resource_index = self.current.mask;
        }
        let result = self.scene().add_state(&state);
        self.record(result)
    }

    fn changed(&mut self) {
        self.state = None;
        self.glyph_state = None;
    }

    /// Whether text is drawn aliased: the text options pushed last, then the render options,
    /// as the other backends resolve them. Everything else is greyscale antialiasing.
    fn aliased_text(&self) -> bool {
        let text = self.text_options.last().map(|options| options.text_rendering_mode).unwrap_or_default();
        let render = self.render_options.last().copied().unwrap_or_default();
        match (text, render.text_rendering_mode) {
            (TextRenderingMode::Alias, _) | (TextRenderingMode::Unspecified, TextRenderingMode::Alias) => true,
            (TextRenderingMode::Unspecified, TextRenderingMode::Unspecified) => render.edge_mode == EdgeMode::Aliased,
            _ => false,
        }
    }

    /// The text style of a colour, added to the scene once.
    fn text_style(&mut self, color: progpu_native_color, aliased: bool) -> Option<u32> {
        let key = ([color.r.to_bits(), color.g.to_bits(), color.b.to_bits(), color.a.to_bits()], aliased);
        if let Some(index) = self.text_styles.get(&key) {
            return Some(*index);
        }
        let style = progpu_native_scene_text_style {
            color,
            text_rendering_mode: if aliased { PROGPU_NATIVE_SCENE_TEXT_ALIASED } else { PROGPU_NATIVE_SCENE_TEXT_GRAYSCALE } as u32,
            coverage_transfer: if aliased { 0 } else { text_coverage_transfer(color) },
            reserved1: 0,
            reserved2: 0,
        };
        let result = self.scene().add_text_style(&style);
        let index = self.record(result)?;
        self.text_styles.insert(key, index);
        Some(index)
    }

    /// Draws a run in one colour from the glyph atlas of the engine.
    fn draw_run(&mut self, run: &ProGpuGlyphRun, color: progpu_native_color) {
        let transform = self.current.transform;
        let origin = ferroui_base::platform::IGlyphRunImpl::baseline_origin(run);
        let aliased = self.aliased_text();
        let upright = axis_aligned(&transform) && transform.m11 > 0.0 && (transform.m11 - transform.m22).abs() <= 1e-6 * transform.m11;
        // Upright text: glyphs at whole device pixels under the identity transform. Any other
        // transform: glyphs in the units of the run, through the transform.
        let (placement, base, atlas_to_logical_scale) = if upright {
            let device = transform.transform(origin);
            let base = Point::new(device.x.floor(), device.y.floor());
            if !(base.x.is_finite() && base.y.is_finite()) {
                return;
            }
            let placement =
                Placement::Snapped { scale: transform.m11 as f32, fraction_x: (device.x - base.x) as f32, fraction_y: (device.y - base.y) as f32, aliased };
            (placement, base, 1.0)
        } else {
            let scale = transform.get_determinant().abs().sqrt();
            if !(scale.is_finite() && scale > 0.0) {
                return;
            }
            let scale = 2f64.powf((scale.log2() * FREE_SCALE_STEPS).round() / FREE_SCALE_STEPS) as f32;
            (Placement::Free { scale }, origin, 1.0 / scale)
        };
        let Some(raster) = run.raster(placement) else { return };
        if raster.glyphs.is_empty() {
            return;
        }
        let Some(font) = run.font() else { return };
        let bounds = transform_bounds(ferroui_base::platform::IGlyphRunImpl::bounds(run), &transform).inflate(1.0);
        let device = if upright { transform.transform(origin) } else { origin };
        let kept = KeptRun {
            face: font.face.clone(),
            raster,
            color,
            aliased,
            upright,
            base,
            device,
            atlas_to_logical_scale,
            bounds,
            transform,
        };
        if self.draw_kept_run(&kept, base, bounds) {
            if let Some(fragment) = &mut self.fragment {
                fragment.run(kept);
            }
        }
    }

    /// Draws the glyphs of `run` placed from `base`, over `bounds` in device pixels.
    fn draw_kept_run(&mut self, run: &KeptRun, base: Point, bounds: Rect) -> bool {
        let state = if run.upright { self.glyph_state() } else { self.state() };
        let Some(state) = state else { return false };
        let Some(style) = self.text_style(run.color, run.aliased) else { return false };
        let result = self.scene().add_glyph_keys(&run.face, &run.raster.keys);
        let Some(resource) = self.record(result) else { return false };

        let (color, atlas_to_logical_scale) = (run.color, run.atlas_to_logical_scale);
        let mut positioned = std::mem::take(&mut self.positioned);
        positioned.clear();
        positioned.extend(run.raster.glyphs.iter().map(|glyph| progpu_native_positioned_glyph {
            outline_index: glyph.outline,
            reserved: 0,
            position: progpu_native_point { x: (base.x + f64::from(glyph.x)) as f32, y: (base.y + f64::from(glyph.y)) as f32 },
            basis_x: progpu_native_point { x: 1.0, y: 0.0 },
            basis_y: progpu_native_point { x: 0.0, y: 1.0 },
            color,
            atlas_to_logical_scale,
            bold_offset: 0.0,
            italic_skew: 0.0,
            reserved2: 0.0,
        }));
        let result = self.scene().draw_glyph_run(resource, &positioned, image_rect(bounds), state, style);
        let drawn = self.record(result).is_some();
        self.positioned = positioned;
        drawn
    }

    /// The brush of the scene for a brush of the base library, over `bounds` (the local box a
    /// relative brush is laid out on). The engine evaluates a brush in the coordinates of the
    /// shape or the path it fills, before the transform of the state. None for a brush that
    /// paints nothing.
    fn brush(&mut self, brush: &dyn IBrush, bounds: Rect) -> Option<u32> {
        let opacity = brush.opacity();
        if let Some(solid) = brush.as_solid_color_brush() {
            let color = native_color(solid.color(), 1.0);
            let result = self.scene().add_solid_brush(color, opacity as f32);
            if let Some(fragment) = &mut self.fragment {
                fragment.set_brush(FragmentBrush::Solid { color, opacity: opacity as f32 });
            }
            return self.record(result);
        }
        let Some((native, stops)) = Self::gradient_record(brush, bounds) else {
            notes::note_once("brush-kind", "a brush of a kind the ProGPU backend does not paint is not drawn");
            return None;
        };
        let result = self.scene().add_brush(&native, &stops);
        if let Some(fragment) = &mut self.fragment {
            fragment.set_brush(FragmentBrush::Gradient { brush: Box::new(native), stops: stops.into_boxed_slice() });
        }
        self.record(result)
    }

    /// The record and the stops of a gradient brush over `bounds`. None for another brush.
    fn gradient_record(
        brush: &dyn IBrush,
        bounds: Rect,
    ) -> Option<(progpu_native_scene_brush, Vec<progpu_native_scene_gradient_stop>)> {
        let opacity = brush.opacity();
        let gradient = brush.as_gradient_brush()?;
        let mut stops: Vec<progpu_native_scene_gradient_stop> = gradient
            .gradient_stops()
            .iter()
            .map(|stop| progpu_native_scene_gradient_stop {
                color: native_color(stop.color(), 1.0),
                offset: stop.offset() as f32,
                reserved0: 0,
                reserved1: 0,
                reserved2: 0,
            })
            .collect();
        if stops.is_empty() {
            return None;
        }
        stops.sort_by(|a, b| a.offset.total_cmp(&b.offset));
        // SAFETY: an all-zero brush record is a valid start (the reserved fields stay zero).
        let mut native: progpu_native_scene_brush = unsafe { std::mem::zeroed() };
        native.opacity = opacity as f32;
        native.spread_method = match gradient.spread_method() {
            GradientSpreadMethod::Pad => PROGPU_NATIVE_SCENE_GRADIENT_PAD,
            GradientSpreadMethod::Reflect => PROGPU_NATIVE_SCENE_GRADIENT_REFLECT,
            GradientSpreadMethod::Repeat => PROGPU_NATIVE_SCENE_GRADIENT_REPEAT,
        } as u32;
        if native.spread_method == PROGPU_NATIVE_SCENE_GRADIENT_PAD as u32
            && (stops[0].offset < 0.0 || stops[stops.len() - 1].offset > 1.0)
        {
            native.spread_method = PROGPU_NATIVE_SCENE_GRADIENT_PAD_UNIT_INTERVAL as u32;
        }
        native.color_interpolation_mode = PROGPU_NATIVE_SCENE_GRADIENT_INTERPOLATE_SRGB as u32;
        native.stop_count = stops.len() as u32;
        for (index, stop) in stops.iter().take(8).enumerate() {
            native.colors[index] = stop.color;
            if index < 4 {
                native.offsets0[index] = stop.offset;
            } else {
                native.offsets1[index - 4] = stop.offset;
            }
        }
        let to_point = |value: Point| progpu_native_point { x: value.x as f32, y: value.y as f32 };
        if let Some(linear) = brush.as_linear_gradient_brush() {
            native.type_ = PROGPU_NATIVE_SCENE_BRUSH_LINEAR_GRADIENT as u32;
            native.start_point = to_point(linear.start_point().to_pixels_rect(bounds));
            native.end_point = to_point(linear.end_point().to_pixels_rect(bounds));
        } else if let Some(radial) = brush.as_radial_gradient_brush() {
            let center = radial.center().to_pixels_rect(bounds);
            let origin = radial.gradient_origin().to_pixels_rect(bounds);
            native.type_ = PROGPU_NATIVE_SCENE_BRUSH_RADIAL_GRADIENT as u32;
            native.start_point = to_point(origin);
            native.center = to_point(center);
            native.radius = radial.radius_x().to_value(bounds.width) as f32;
            native.radius_y = radial.radius_y().to_value(bounds.height) as f32;
        } else {
            return None;
        }
        // Evaluation coordinates to brush coordinates: the inverse of the transforms of the
        // brush. Its relative transform acts on the box as a unit square and comes first, its
        // transform acts about its origin in the coordinates of the box.
        let relative = brush
            .relative_transform()
            .filter(|_| bounds.width > 0.0 && bounds.height > 0.0)
            .map(|transform| {
                Matrix::create_translation(-bounds.x, -bounds.y)
                    * Matrix::create_scale(1.0 / bounds.width, 1.0 / bounds.height)
                    * transform.value()
                    * Matrix::create_scale(bounds.width, bounds.height)
                    * Matrix::create_translation(bounds.x, bounds.y)
            })
            .unwrap_or(Matrix::IDENTITY);
        let absolute = brush
            .transform()
            .map(|transform| {
                let origin = brush.transform_origin().to_pixels_rect(bounds);
                Matrix::create_translation(-origin.x, -origin.y) * transform.value() * Matrix::create_translation(origin.x, origin.y)
            })
            .unwrap_or(Matrix::IDENTITY);
        let brush_transform = relative * absolute;
        let coordinate = brush_transform.try_invert().unwrap_or(Matrix::IDENTITY);
        native.coordinate_transform0 = [coordinate.m11 as f32, coordinate.m21 as f32, coordinate.m31 as f32, 0.0];
        native.coordinate_transform1 = [coordinate.m12 as f32, coordinate.m22 as f32, coordinate.m32 as f32, 0.0];
        Some((native, stops))
    }

    /// Adds the brush of a kept draw.
    fn kept_brush(&mut self, brush: &FragmentBrush) -> Option<u32> {
        let result = match brush {
            FragmentBrush::Solid { color, opacity } => self.scene().add_solid_brush(*color, *opacity),
            FragmentBrush::Gradient { brush, stops } => self.scene().add_brush(brush, stops),
        };
        self.record(result)
    }

    /// Draws `fragment` moved by `offset` device pixels, under the clips in force now.
    fn replay(&mut self, fragment: &ProGpuFragment, offset: (f64, f64)) {
        let transform = self.current.transform;
        for op in &fragment.ops {
            match op {
                FragmentOp::Shape { primitive, brush, bounds, transform } => {
                    let Some(index) = self.kept_brush(brush) else { continue };
                    self.place(moved(transform, offset));
                    self.analytic(*primitive, index, moved_rect(*bounds, offset));
                }
                FragmentOp::Path { path, segments, brush, bounds, transform } => {
                    let Some(index) = self.kept_brush(brush) else { continue };
                    self.place(moved(transform, offset));
                    let Some(state) = self.state() else { continue };
                    let result = self.scene().draw_paths(&[*path], segments, &[index], image_rect(*bounds), state);
                    self.record(result);
                }
                FragmentOp::Run(run) => {
                    self.place(moved(&run.transform, offset));
                    let base = if run.upright { Point::new(run.base.x + offset.0, run.base.y + offset.1) } else { run.base };
                    self.draw_kept_run(run, base, moved_rect(run.bounds, offset));
                }
                FragmentOp::PushClip(rect) => self.push_device_clip(moved_rect(*rect, offset)),
                FragmentOp::PopClip => self.pop_clip(),
                FragmentOp::PushLayer { bounds, opacity, blend_mode } => {
                    self.push_device_layer(bounds.map(|bounds| moved_rect(bounds, offset)), *opacity, *blend_mode);
                }
                FragmentOp::PopLayer => self.pop_scene_layer(),
            }
        }
        self.place(transform);
    }

    /// Sets the transform of the next draws.
    fn place(&mut self, transform: Matrix) {
        if self.current.transform != transform {
            self.current.transform = transform;
            self.changed();
        }
    }

    /// What is drawn from here on cannot be kept in the fragment being recorded.
    fn refuse_fragment(&mut self) {
        if let Some(fragment) = &mut self.fragment {
            fragment.refuse();
        }
    }

    /// A clip rectangle in device pixels, within the clip in force.
    fn push_device_clip(&mut self, target: Rect) {
        self.clips.push(self.state_entry());
        self.current.clip_rect = Some(match self.current.clip_rect {
            Some(current) => intersect(current, target),
            None => target,
        });
        self.changed();
    }

    fn analytic(&mut self, primitive: progpu_native_analytic_primitive, brush: u32, bounds: Rect) {
        let Some(state) = self.state() else { return };
        let result = self.scene().draw_analytic(&[primitive], &[brush], image_rect(bounds), state);
        self.record(result);
        if let Some(fragment) = &mut self.fragment {
            fragment.shape(primitive, bounds, self.current.transform);
        }
    }

    /// Fills a geometry that may be drawn several times in a frame (the same icon in every row):
    /// after its first draw the stream names the resource of that draw. The map holds the
    /// geometry, so that its address names it for the whole frame. Inside a fragment the commands
    /// are kept to be sent again in another frame, where the resource is not there.
    fn fill_shared_path(&mut self, data: &Arc<PathData>, brush: &dyn IBrush) {
        if self.fragment.is_some() || is_tile_brush(brush) {
            self.fill_path(data, brush);
            return;
        }
        let key = Arc::as_ptr(data);
        if let Some(resource) = self.path_resources.get(&key).map(|(_, resource)| *resource) {
            let bounds = data.bounds();
            let Some(brush_index) = self.brush(brush, bounds) else { return };
            let Some(state) = self.state() else { return };
            let result = self.scene().draw_path_resource(resource, &[brush_index], image_rect(bounds), state);
            self.record(result);
            return;
        }
        // Only a call that added a draw: an empty geometry adds none.
        if self.fill_path(data, brush) {
            if let Some(resource) = self.scene.as_ref().and_then(SceneBuilder::last_path_resource) {
                self.path_resources.insert(key, (data.clone(), resource));
            }
        }
    }

    /// Fills the canonical segments of `data` (its filled figures) with a brush.
    fn fill_path(&mut self, data: &PathData, brush: &dyn IBrush) -> bool {
        self.fill_path_on(data, brush, data.bounds())
    }

    /// Fills `data` with a brush placed on the box `target`: the shape a stroke is the outline of.
    fn fill_path_on(&mut self, data: &PathData, brush: &dyn IBrush, target: Rect) -> bool {
        if is_tile_brush(brush) {
            return self.fill_with_tile_brush(data, brush, target);
        }
        let segments = data.filled_segments();
        if segments.is_empty() {
            return false;
        }
        let bounds = data.bounds();
        if !has_area(bounds) {
            // Figures without an area (a line, a point) cover nothing, and the engine refuses
            // a path without bounds.
            return false;
        }
        let Some(brush_index) = self.brush(brush, bounds) else { return false };
        let Some(state) = self.state() else { return false };
        let path = progpu_native_scene_path_fill {
            segment_offset: 0,
            segment_count: segments.len() as u64,
            boolean_node_offset: 0,
            boolean_node_count: 0,
            min_x: bounds.x as f32,
            min_y: bounds.y as f32,
            max_x: (bounds.x + bounds.width) as f32,
            max_y: (bounds.y + bounds.height) as f32,
            color: progpu_native_color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            transform: identity(),
            fill_rule: match data.fill_rule() {
                ferroui_base::media::FillRule::EvenOdd => PROGPU_NATIVE_FILL_RULE_EVEN_ODD,
                ferroui_base::media::FillRule::NonZero => PROGPU_NATIVE_FILL_RULE_NON_ZERO,
            } as u32,
            sample_grid: PATH_SAMPLE_GRID,
        };
        let result = self.scene().draw_paths(&[path], &segments, &[brush_index], image_rect(bounds), state);
        let drawn = result.is_ok();
        self.record(result);
        if let Some(fragment) = &mut self.fragment {
            fragment.path(path, segments, bounds, self.current.transform);
        }
        drawn
    }

    fn primitive(kind: u32, rect: Rect, radius: f64, stroke: f64) -> progpu_native_analytic_primitive {
        progpu_native_analytic_primitive {
            kind,
            flags: 0,
            x: rect.x as f32,
            y: rect.y as f32,
            width: rect.width as f32,
            height: rect.height as f32,
            corner_radius: radius as f32,
            stroke_thickness: stroke as f32,
            color: progpu_native_color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            transform: identity(),
        }
    }

    /// Whether a brush paints every pixel of its shape opaquely (a shadow under it is hidden).
    fn opaque(brush: Option<&dyn IBrush>) -> bool {
        brush.is_some_and(|brush| {
            brush.opacity() >= 1.0 && brush.as_solid_color_brush().is_some_and(|solid| solid.color().a == 255)
        })
    }

    fn rounded_rect_path(rect: &RoundedRect) -> PathData {
        let mut builder = crate::geometry::PathBuilder::default();
        use ferroui_base::media::SweepDirection;
        use ferroui_base::platform::IGeometryContext;
        let r = rect.rect;
        let (tl, tr, br, bl) = (rect.radii_top_left, rect.radii_top_right, rect.radii_bottom_right, rect.radii_bottom_left);
        builder.begin_figure(Point::new(r.x + tl.x, r.y), true);
        builder.line_to(Point::new(r.x + r.width - tr.x, r.y), true);
        if tr.x > 0.0 && tr.y > 0.0 {
            builder.arc_to(Point::new(r.x + r.width, r.y + tr.y), ferroui_base::Size::new(tr.x, tr.y), 0.0, false, SweepDirection::Clockwise, true);
        }
        builder.line_to(Point::new(r.x + r.width, r.y + r.height - br.y), true);
        if br.x > 0.0 && br.y > 0.0 {
            builder.arc_to(Point::new(r.x + r.width - br.x, r.y + r.height), ferroui_base::Size::new(br.x, br.y), 0.0, false, SweepDirection::Clockwise, true);
        }
        builder.line_to(Point::new(r.x + bl.x, r.y + r.height), true);
        if bl.x > 0.0 && bl.y > 0.0 {
            builder.arc_to(Point::new(r.x, r.y + r.height - bl.y), ferroui_base::Size::new(bl.x, bl.y), 0.0, false, SweepDirection::Clockwise, true);
        }
        builder.line_to(Point::new(r.x, r.y + tl.y), true);
        if tl.x > 0.0 && tl.y > 0.0 {
            builder.arc_to(Point::new(r.x + tl.x, r.y), ferroui_base::Size::new(tl.x, tl.y), 0.0, false, SweepDirection::Clockwise, true);
        }
        builder.end_figure(true);
        builder.finish()
    }

    /// A clip of the shapes in `data` (vector mask), with an operation (intersect or difference).
    /// The mask is a texture of the size of the target: the clips of rectangles and rounded
    /// rectangles do not come here (see `push_analytic_clip`).
    fn vector_mask(&mut self, data: &PathData, operation: u32) -> Option<u32> {
        self.refuse_fragment();
        let segments = data.filled_segments();
        let bounds = data.bounds();
        let path = progpu_native_scene_clip_path {
            segment_offset: 0,
            segment_count: segments.len() as u64,
            boolean_node_offset: 0,
            boolean_node_count: 0,
            min_x: bounds.x as f32,
            min_y: bounds.y as f32,
            max_x: (bounds.x + bounds.width) as f32,
            max_y: (bounds.y + bounds.height) as f32,
            transform: affine(&self.current.transform),
            fill_rule: match data.fill_rule() {
                ferroui_base::media::FillRule::EvenOdd => PROGPU_NATIVE_FILL_RULE_EVEN_ODD,
                ferroui_base::media::FillRule::NonZero => PROGPU_NATIVE_FILL_RULE_NON_ZERO,
            } as u32,
            sample_grid: PATH_SAMPLE_GRID,
            operation,
            reserved: 0,
        };
        let result = self.scene().add_vector_clip_mask(&[path], &segments, 1.0);
        self.record(result)
    }

    /// The outer shadows of a rectangle, which are under it, or its inset shadows, over it.
    fn box_shadows(&mut self, brush: Option<&dyn IBrush>, rect: &RoundedRect, shadows: &BoxShadows, inset: bool) {
        let uniform = rect.is_uniform() && rect.radii_top_left.x == rect.radii_top_left.y;
        let opaque = Self::opaque(brush);
        for shadow in shadows.iter() {
            if shadow.color.a == 0 || shadow.is_inset != inset {
                continue;
            }
            let sigma = blur_radius_to_sigma(shadow.blur);
            if shadow.is_inset {
                self.inset_shadow(rect, &shadow, sigma);
                continue;
            }
            if !uniform {
                self.outer_shadow_of_uneven_corners(rect, &shadow, sigma, opaque);
                continue;
            }
            let spread = shadow.spread;
            let radius = if rect.radii_top_left.x > 0.0 { (rect.radii_top_left.x + spread).max(0.0) } else { 0.0 };
            let box_rect = Rect::new(
                rect.rect.x - spread + shadow.offset_x,
                rect.rect.y - spread + shadow.offset_y,
                (rect.rect.width + 2.0 * spread).max(0.0),
                (rect.rect.height + 2.0 * spread).max(0.0),
            );
            let color = native_color(shadow.color, 1.0);
            let result = self.scene().add_solid_brush(color, 1.0);
            if let Some(fragment) = &mut self.fragment {
                fragment.set_brush(FragmentBrush::Solid { color, opacity: 1.0 });
            }
            let Some(brush_index) = self.record(result) else { return };
            // The shadow is outside its element: under an element that is not opaque, a
            // difference mask of the element keeps it out.
            let saved = self.current;
            if !opaque {
                // Under a clip mask the shadow is drawn whole: the engine takes one mask per
                // state.
                if self.current.mask == NO_INDEX && has_area(rect.rect) {
                    let element = Self::rounded_rect_path(rect);
                    if let Some(mask) = self.vector_mask(&element, PROGPU_NATIVE_CLIP_DIFFERENCE as u32) {
                        self.current.mask = mask;
                        self.changed();
                    }
                }
            }
            let reach = 3.0 * sigma as f64;
            let bounds = transform_bounds(
                Rect::new(box_rect.x - reach, box_rect.y - reach, box_rect.width + 2.0 * reach, box_rect.height + 2.0 * reach),
                &self.current.transform,
            );
            let primitive = if sigma > 0.0 {
                Self::primitive(PROGPU_NATIVE_PRIMITIVE_SHADOW as u32, box_rect, radius, sigma as f64)
            } else {
                Self::primitive(PROGPU_NATIVE_PRIMITIVE_ROUNDED_RECTANGLE as u32, box_rect, radius, 0.0)
            };
            self.analytic(primitive, brush_index, bounds);
            if self.current.mask != saved.mask {
                self.current = saved;
                self.changed();
            }
        }
    }

    /// The fill and the stroke of a rectangle.
    fn rectangle_shape(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: &RoundedRect) {
        // A tile brush paints through the outline of the shape.
        let stroke = pen.filter(|pen| pen.thickness() > 0.0).and_then(|pen| Some((pen, pen.brush()?)));
        if brush.is_some_and(is_tile_brush) || stroke.as_ref().is_some_and(|(_, brush)| is_tile_brush(brush.as_ref())) {
            let path = Self::rounded_rect_path(rect);
            if let Some(brush) = brush {
                self.fill_path_on(&path, brush, rect.rect);
            }
            if let Some((pen, stroke_brush)) = stroke {
                self.fill_path_on(&path.widen(pen), stroke_brush.as_ref(), rect.rect);
            }
            return;
        }
        let uniform = rect.is_uniform() && rect.radii_top_left.x == rect.radii_top_left.y;
        let local = rect.rect;
        let bounds = transform_bounds(local, &self.current.transform);
        if uniform {
            let radius = rect.radii_top_left.x;
            let kind = if radius > 0.0 { PROGPU_NATIVE_PRIMITIVE_ROUNDED_RECTANGLE } else { PROGPU_NATIVE_PRIMITIVE_RECTANGLE } as u32;
            if let Some(brush) = brush {
                if let Some(index) = self.brush(brush, local) {
                    self.analytic(Self::primitive(kind, local, radius, 0.0), index, bounds);
                }
            }
            if let Some(pen) = pen.filter(|pen| pen.thickness() > 0.0 && pen.dash_style().is_none()) {
                if let Some(stroke_brush) = pen.brush() {
                    if let Some(index) = self.brush(stroke_brush.as_ref(), local) {
                        let half = pen.thickness() / 2.0;
                        let outer = transform_bounds(Rect::new(local.x - half, local.y - half, local.width + pen.thickness(), local.height + pen.thickness()), &self.current.transform);
                        self.analytic(Self::primitive(kind, local, radius, pen.thickness()), index, outer);
                    }
                }
                return;
            }
        } else if let Some(brush) = brush {
            self.fill_path(&Self::rounded_rect_path(rect), brush);
        }
        // Dashed strokes and the strokes of rectangles with different radii: the widened outline.
        if let Some(pen) = pen.filter(|pen| pen.thickness() > 0.0) {
            if let Some(stroke_brush) = pen.brush() {
                let outline = Self::rounded_rect_path(rect).widen(pen);
                self.fill_path(&outline, stroke_brush.as_ref());
            }
        }
    }

    /// The rectangle of `rect` grown by `by` on every side, its radii with it (a corner without
    /// a radius stays square; a radius does not go below zero).
    fn spread_rect(rect: &RoundedRect, by: f64, x: f64, y: f64) -> RoundedRect {
        let grown = |radius: Vector| {
            if radius.x > 0.0 && radius.y > 0.0 {
                Vector::new((radius.x + by).max(0.0), (radius.y + by).max(0.0))
            } else {
                radius
            }
        };
        RoundedRect::new(
            Rect::new(rect.rect.x - by + x, rect.rect.y - by + y, (rect.rect.width + 2.0 * by).max(0.0), (rect.rect.height + 2.0 * by).max(0.0)),
            grown(rect.radii_top_left),
            grown(rect.radii_top_right),
            grown(rect.radii_bottom_right),
            grown(rect.radii_bottom_left),
        )
    }

    /// The record of an analytic clip of a rectangle under `transform`.
    fn analytic_mask_record(rect: Rect, transform: &Matrix, corner_radii_x: [f32; 4], corner_radii_y: [f32; 4]) -> progpu_native_scene_layer_mask {
        progpu_native_scene_layer_mask {
            struct_size: std::mem::size_of::<progpu_native_scene_layer_mask>() as u32,
            kind: PROGPU_NATIVE_SCENE_LAYER_MASK_ROUNDED_RECTANGLE as u32,
            flags: 0,
            reserved: 0,
            bounds: image_rect(rect),
            transform: affine(transform),
            corner_radii_x,
            corner_radii_y,
            opacity: 1.0,
            reserved0: 0,
            reserved1: 0,
            reserved2: 0,
        }
    }

    /// The mask of a blurred layer: the clips in force and `shape`, all applied to the result
    /// of the blur, since a clip of the draws inside would cut the source of the blur. `None`
    /// when they do not fit one chain of analytic masks; `Some(NO_INDEX)` for no clip at all.
    fn blurred_layer_mask(&mut self, shape: Option<progpu_native_scene_layer_mask>) -> Option<u32> {
        if self.current.mask != NO_INDEX && self.analytic.is_empty() {
            return None;
        }
        let mut masks = self.analytic.clone();
        if let Some(clip) = self.current.clip_rect {
            masks.push(Self::analytic_mask_record(clip, &Matrix::IDENTITY, [0.0; 4], [0.0; 4]));
        }
        masks.extend(shape);
        if masks.len() > PROGPU_NATIVE_SCENE_MAX_ANALYTIC_MASKS as usize {
            return None;
        }
        let result = match masks.as_slice() {
            [] => return Some(NO_INDEX),
            [mask] => self.scene().add_rounded_rectangle_mask(mask),
            masks => self.scene().add_analytic_mask_chain(masks),
        };
        self.record(result)
    }

    /// Draws with `draw`, unclipped, in a layer blurred by `sigma` and then masked by `mask`.
    fn draw_blurred(&mut self, sigma: f32, mask: u32, draw: impl FnOnce(&mut Self)) {
        let chain = (sigma > 0.0)
            .then(|| self.effect_chain(PROGPU_NATIVE_GROUP_EFFECT_GAUSSIAN_BLUR as u32, sigma, (0.0, 0.0), [0.0; 4]))
            .flatten();
        self.push_effect_layer(None, chain.unwrap_or(NO_INDEX), mask);
        let saved = self.current;
        self.current.clip_rect = None;
        self.current.mask = NO_INDEX;
        self.changed();
        draw(self);
        self.current = saved;
        self.changed();
        self.pop_scene_layer();
    }

    /// An inset shadow: a frame around the shape with a hole at the shadow's offset, shrunk by
    /// the spread and blurred, seen through the shape.
    fn inset_shadow(&mut self, rect: &RoundedRect, shadow: &ferroui_base::media::BoxShadow, sigma: f32) {
        if !has_area(rect.rect) || !invertible(&self.current.transform) {
            return;
        }
        self.refuse_fragment();
        let shape = Self::analytic_mask_record(
            rect.rect,
            &self.current.transform,
            [rect.radii_top_left.x as f32, rect.radii_top_right.x as f32, rect.radii_bottom_right.x as f32, rect.radii_bottom_left.x as f32],
            [rect.radii_top_left.y as f32, rect.radii_top_right.y as f32, rect.radii_bottom_right.y as f32, rect.radii_bottom_left.y as f32],
        );
        if let Some(mask) = self.blurred_layer_mask(Some(shape)) {
            self.draw_inset_shadow(rect, shadow, sigma, mask);
            return;
        }
        // Under a clip with a texture: the shadow is drawn in a layer that clip masks.
        if !self.analytic.is_empty() {
            notes::note_once(
                "inset-shadow-mask",
                "an inset shadow under more nested rounded clips than one chain holds is not drawn",
            );
            return;
        }
        let saved = self.current;
        self.push_masked_layer(saved.mask);
        self.current.mask = NO_INDEX;
        self.current.clip_rect = None;
        self.changed();
        if let Some(mask) = self.blurred_layer_mask(Some(shape)) {
            self.draw_inset_shadow(rect, shadow, sigma, mask);
        }
        self.current = saved;
        self.changed();
        self.pop_clip();
    }

    /// The frame around the shape, blurred and seen through `mask`.
    fn draw_inset_shadow(
        &mut self,
        rect: &RoundedRect,
        shadow: &ferroui_base::media::BoxShadow,
        sigma: f32,
        mask: u32,
    ) {
        let reach = 3.0 * f64::from(sigma) + shadow.offset_x.abs().max(shadow.offset_y.abs()) + shadow.spread.abs() + 1.0;
        let hole = Self::spread_rect(rect, -shadow.spread, shadow.offset_x, shadow.offset_y);
        let frame = Self::frame_path(rect.rect.inflate(reach), &hole);
        let brush = ferroui_base::media::immutable::ImmutableSolidColorBrush::new(shadow.color);
        self.draw_blurred(sigma, mask, |context| {
            context.fill_path(&frame, &brush);
        });
    }

    /// The path of `outer` with `hole` cut out of it.
    fn frame_path(outer: Rect, hole: &RoundedRect) -> PathData {
        use ferroui_base::media::{FillRule, SweepDirection};
        use ferroui_base::platform::IGeometryContext;
        let mut builder = crate::geometry::PathBuilder::default();
        builder.set_fill_rule(FillRule::EvenOdd);
        builder.begin_figure(Point::new(outer.x, outer.y), true);
        builder.line_to(Point::new(outer.x + outer.width, outer.y), true);
        builder.line_to(Point::new(outer.x + outer.width, outer.y + outer.height), true);
        builder.line_to(Point::new(outer.x, outer.y + outer.height), true);
        builder.end_figure(true);
        if has_area(hole.rect) {
            let r = hole.rect;
            let (tl, tr, br, bl) = (hole.radii_top_left, hole.radii_top_right, hole.radii_bottom_right, hole.radii_bottom_left);
            let arc = |builder: &mut crate::geometry::PathBuilder, to: Point, radius: Vector| {
                if radius.x > 0.0 && radius.y > 0.0 {
                    builder.arc_to(to, ferroui_base::Size::new(radius.x, radius.y), 0.0, false, SweepDirection::Clockwise, true);
                }
            };
            builder.begin_figure(Point::new(r.x + tl.x, r.y), true);
            builder.line_to(Point::new(r.x + r.width - tr.x, r.y), true);
            arc(&mut builder, Point::new(r.x + r.width, r.y + tr.y), tr);
            builder.line_to(Point::new(r.x + r.width, r.y + r.height - br.y), true);
            arc(&mut builder, Point::new(r.x + r.width - br.x, r.y + r.height), br);
            builder.line_to(Point::new(r.x + bl.x, r.y + r.height), true);
            arc(&mut builder, Point::new(r.x, r.y + r.height - bl.y), bl);
            builder.line_to(Point::new(r.x, r.y + tl.y), true);
            arc(&mut builder, Point::new(r.x + tl.x, r.y), tl);
            builder.end_figure(true);
        }
        builder.finish()
    }

    /// The outer shadow of a rectangle whose corners differ: its shape blurred in a layer. Under
    /// a shape that is not opaque, a difference mask of the shape on the layer keeps the shadow
    /// out of it; the clips in force then apply to the source of the blur.
    fn outer_shadow_of_uneven_corners(&mut self, rect: &RoundedRect, shadow: &ferroui_base::media::BoxShadow, sigma: f32, opaque: bool) {
        let shape = Self::spread_rect(rect, shadow.spread, shadow.offset_x, shadow.offset_y);
        if !has_area(shape.rect) {
            return;
        }
        self.refuse_fragment();
        let path = Self::rounded_rect_path(&shape);
        let brush = ferroui_base::media::immutable::ImmutableSolidColorBrush::new(shadow.color);
        if !opaque && has_area(rect.rect) {
            let element = Self::rounded_rect_path(rect);
            let Some(mask) = self.vector_mask(&element, PROGPU_NATIVE_CLIP_DIFFERENCE as u32) else { return };
            let chain = (sigma > 0.0)
                .then(|| self.effect_chain(PROGPU_NATIVE_GROUP_EFFECT_GAUSSIAN_BLUR as u32, sigma, (0.0, 0.0), [0.0; 4]))
                .flatten();
            self.push_effect_layer(None, chain.unwrap_or(NO_INDEX), mask);
            self.fill_path(&path, &brush);
            self.pop_scene_layer();
            return;
        }
        let Some(mask) = self.blurred_layer_mask(None) else {
            // Under a clip of a geometry the clip stays on the draw.
            let chain = (sigma > 0.0)
                .then(|| self.effect_chain(PROGPU_NATIVE_GROUP_EFFECT_GAUSSIAN_BLUR as u32, sigma, (0.0, 0.0), [0.0; 4]))
                .flatten();
            self.push_effect_layer(None, chain.unwrap_or(NO_INDEX), NO_INDEX);
            self.fill_path(&path, &brush);
            self.pop_scene_layer();
            return;
        };
        self.draw_blurred(sigma, mask, |context| {
            context.fill_path(&path, &brush);
        });
    }

    /// The effect chain of one effect, its sigma and offset in the coordinates of the drawing
    /// taken to device pixels, or None for an effect the engine refuses.
    fn effect_chain(&mut self, kind: u32, sigma: f32, offset: (f32, f32), color: [f32; 4]) -> Option<u32> {
        let m = self.current.transform;
        let scale = (m.m11 * m.m22 - m.m12 * m.m21).abs().sqrt() as f32;
        let device_sigma = sigma * scale;
        if !device_sigma.is_finite() || device_sigma < 0.0 {
            return None;
        }
        if device_sigma > MAX_EFFECT_SIGMA {
            notes::note_once("effect-sigma", "a blur above the largest sigma of the engine is drawn at that sigma");
        }
        let (x, y) = (f64::from(offset.0), f64::from(offset.1));
        let effect = progpu_native_group_effect {
            struct_size: std::mem::size_of::<progpu_native_group_effect>() as u32,
            kind,
            flags: 0,
            revision: 1,
            sigma_x: device_sigma.min(MAX_EFFECT_SIGMA),
            sigma_y: device_sigma.min(MAX_EFFECT_SIGMA),
            reserved: 0,
            reserved2: 0,
            offset_x: (x * m.m11 + y * m.m21) as f32,
            offset_y: (x * m.m12 + y * m.m22) as f32,
            color_r: color[0],
            color_g: color[1],
            color_b: color[2],
            color_a: color[3],
        };
        let result = self.scene().add_effect_chain(&[effect], 1);
        self.record(result)
    }

    /// A layer through the effect chain `effect` and then the mask `mask` (`NO_INDEX` for none),
    /// with its bounds in device pixels.
    fn push_effect_layer(&mut self, target: Option<Rect>, effect: u32, mask: u32) {
        self.refuse_fragment();
        let layer = progpu_native_scene_layer {
            struct_size: std::mem::size_of::<progpu_native_scene_layer>() as u32,
            flags: if target.is_some() { PROGPU_NATIVE_SCENE_LAYER_BOUNDS as u32 } else { 0 },
            bounds: target.map(image_rect).unwrap_or(progpu_native_image_rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }),
            opacity: 1.0,
            blend_mode: PROGPU_NATIVE_BLEND_SRC_OVER as u32,
            mask_resource_index: mask,
            effect_resource_index: effect,
            content_revision: 0,
            composite_revision: 0,
            reserved0: 0,
            reserved1: 0,
        };
        self.needs_whole_replay = true;
        let result = self.scene().push_layer(&layer);
        self.record(result);
        self.layers.push(LayerEntry::Layer);
    }

    /// A layer masked by the gradient `mask` over `bounds`. A solid brush is a plain opacity; a
    /// brush the engine has no mask for leaves the layer without one, with a note.
    fn push_brush_mask_layer(&mut self, mask: &dyn IBrush, bounds: Rect) {
        self.refuse_fragment();
        if let Some(solid) = mask.as_solid_color_brush() {
            let alpha = f64::from(solid.color().a) / 255.0 * mask.opacity();
            self.push_scene_layer(Some(bounds), alpha.clamp(0.0, 1.0), PROGPU_NATIVE_BLEND_SRC_OVER as u32);
            return;
        }
        if is_tile_brush(mask) {
            self.push_tile_mask(mask, bounds);
            return;
        }
        let Some((brush, stops)) = Self::gradient_record(mask, bounds) else {
            notes::note_once(
                "opacity-mask",
                "an opacity mask of a brush of an unknown kind is not applied by the ProGPU backend",
            );
            self.push_scene_layer(Some(bounds), 1.0, PROGPU_NATIVE_BLEND_SRC_OVER as u32);
            return;
        };
        // SAFETY: an all-zero record is a valid start (the reserved fields stay zero).
        let mut record: progpu_native_scene_layer_brush_mask = unsafe { std::mem::zeroed() };
        record.bounds = image_rect(bounds);
        record.transform = affine(&self.current.transform);
        record.opacity = 1.0;
        record.brush = brush;
        let result = self.scene().add_brush_mask(&record, &stops);
        let Some(index) = self.record(result) else {
            self.push_scene_layer(Some(bounds), 1.0, PROGPU_NATIVE_BLEND_SRC_OVER as u32);
            return;
        };
        let target = transform_bounds(bounds, &self.current.transform);
        let layer = progpu_native_scene_layer {
            struct_size: std::mem::size_of::<progpu_native_scene_layer>() as u32,
            flags: PROGPU_NATIVE_SCENE_LAYER_BOUNDS as u32,
            bounds: image_rect(target),
            opacity: 1.0,
            blend_mode: PROGPU_NATIVE_BLEND_SRC_OVER as u32,
            mask_resource_index: index,
            effect_resource_index: NO_INDEX,
            content_revision: 0,
            composite_revision: 0,
            reserved0: 0,
            reserved1: 0,
        };
        self.needs_whole_replay = true;
        let result = self.scene().push_layer(&layer);
        self.record(result);
        self.layers.push(LayerEntry::Layer);
    }

    /// The image resource of the scene for a bitmap: a layer of the same device (sampled from
    /// its texture) or a bitmap of pixels (copied into the stream), added once per scene. None
    /// (with a note) for what the backend cannot sample.
    fn image(&mut self, source: &dyn IBitmapImpl) -> Option<SceneImage> {
        self.refuse_fragment();
        let any = source.as_any();
        if let Some(layer) = any.downcast_ref::<ProGpuLayer>() {
            return self.sampled_texture(layer.texture()?, false);
        }
        if let Some(bitmap) = any.downcast_ref::<ProGpuBitmap>() {
            if !bitmap.pixels_ready() {
                // The content is still on the GPU: it is drawn from there.
                if let Some(frame) = bitmap.kept_frame() {
                    return self.sampled_texture(&frame, bitmap.pixel_alpha_format() == AlphaFormat::Opaque);
                }
                notes::note_once("pixels-pending", "a bitmap whose pixels could not be read from the GPU is not drawn");
                return None;
            }
            let key = (bitmap as *const ProGpuBitmap as usize, bitmap.version());
            if let Some(image) = self.bitmaps.get(&key) {
                return Some(*image);
            }
            let size = bitmap.pixel_size();
            if size.width <= 0 || size.height <= 0 {
                return None;
            }
            // On a device a bitmap is one texture from frame to frame, named by the scene.
            if let Some(gpu) = self.gpu.clone() {
                if let Some(texture) = gpu.images().texture_of(gpu.device(), bitmap) {
                    let (width, height) = (texture.width(), texture.height());
                    let result = self.scene().add_external_image(width, height);
                    let (index, resource_id, generation) = self.record(result)?;
                    let image = SceneImage {
                        index,
                        width,
                        height,
                        row_bytes: width * 4,
                        premultiplied: true,
                        opaque: bitmap.pixel_alpha_format() == AlphaFormat::Opaque,
                    };
                    self.textures.push((SampledTexture { texture, resource_id, generation }, image));
                    self.bitmaps.insert(key, image);
                    return Some(image);
                }
            }
            let (width, height, row_bytes) = (size.width as u32, size.height as u32, bitmap.row_bytes() as u32);
            let bgra = bitmap.pixel_format() == PixelFormat::BGRA8888;
            let result = bitmap.with_pixels(|pixels| self.scene().add_pixel_image(width, height, row_bytes, pixels, bgra));
            let index = self.record(result)?;
            let alpha = bitmap.pixel_alpha_format();
            let image = SceneImage {
                index,
                width,
                height,
                row_bytes,
                premultiplied: alpha == AlphaFormat::Premul,
                opaque: alpha == AlphaFormat::Opaque,
            };
            self.bitmaps.insert(key, image);
            return Some(image);
        }
        notes::note_once("bitmap-kind", "a bitmap of another backend is not drawn by the ProGPU backend");
        None
    }

    /// The image resource of the scene for a premultiplied texture of the device.
    fn sampled_texture(&mut self, texture: &Rc<GpuTexture>, opaque: bool) -> Option<SceneImage> {
        if let Some((_, image)) = self.textures.iter().find(|(own, _)| Rc::ptr_eq(&own.texture, texture)) {
            return Some(*image);
        }
        // A page has one WebGPU device, however many handles of it there are.
        let same_device = cfg!(target_os = "emscripten")
            || self.gpu.as_ref().is_some_and(|gpu| Rc::ptr_eq(gpu.device(), texture.device()));
        if !same_device {
            notes::note_once("layer-device", "a layer of another device is not drawn by the ProGPU backend");
            return None;
        }
        let (width, height) = (texture.width(), texture.height());
        let result = self.scene().add_external_image(width, height);
        let (index, resource_id, generation) = self.record(result)?;
        let image = SceneImage { index, width, height, row_bytes: width * 4, premultiplied: true, opaque };
        self.textures.push((SampledTexture { texture: texture.clone(), resource_id, generation }, image));
        Some(image)
    }

    fn state_entry(&self) -> ClipEntry {
        ClipEntry::State { clip_rect: self.current.clip_rect, mask: self.current.mask, analytic: self.analytic.len() }
    }

    /// A clip that leaves nothing: the clip of a shape without an area.
    fn push_empty_clip(&mut self) {
        self.refuse_fragment();
        self.clips.push(self.state_entry());
        self.current.clip_rect = Some(Rect::new(0.0, 0.0, 0.0, 0.0));
        self.changed();
    }

    /// A clip of a mask with a texture (a geometry): the mask of the state, or under another
    /// mask a layer masked by it.
    fn push_mask_clip(&mut self, mask: Option<u32>) {
        self.refuse_fragment();
        let Some(mask) = mask else {
            // The scene has failed: the frame is not drawn, and the clip is kept for its pop.
            self.clips.push(self.state_entry());
            return;
        };
        if self.current.mask == NO_INDEX {
            self.clips.push(self.state_entry());
            self.current.mask = mask;
            self.changed();
        } else {
            self.push_masked_layer(mask);
        }
    }

    /// A clip as a layer masked by `mask`: how a mask nests in a mask it cannot join.
    fn push_masked_layer(&mut self, mask: u32) {
        self.refuse_fragment();
        let layer = progpu_native_scene_layer {
            struct_size: std::mem::size_of::<progpu_native_scene_layer>() as u32,
            flags: 0,
            bounds: progpu_native_image_rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            opacity: 1.0,
            blend_mode: PROGPU_NATIVE_BLEND_SRC_OVER as u32,
            mask_resource_index: mask,
            effect_resource_index: NO_INDEX,
            content_revision: 0,
            composite_revision: 0,
            reserved0: 0,
            reserved1: 0,
        };
        self.needs_whole_replay = true;
        let result = self.scene().push_layer(&layer);
        self.record(result);
        self.clips.push(ClipEntry::Layer);
    }

    /// A clip of a rectangle or a rounded rectangle under the current transform, evaluated by
    /// the engine per pixel of each draw: no mask texture, whatever the rotation. Clips nested
    /// in each other join in one chain of masks; past the length of a chain, and under a mask
    /// with a texture, the clip is a masked layer.
    fn push_analytic_clip(&mut self, rect: Rect, corner_radii_x: [f32; 4], corner_radii_y: [f32; 4]) {
        self.refuse_fragment();
        if !has_area(rect) || !invertible(&self.current.transform) {
            self.push_empty_clip();
            return;
        }
        let mask = progpu_native_scene_layer_mask {
            struct_size: std::mem::size_of::<progpu_native_scene_layer_mask>() as u32,
            kind: PROGPU_NATIVE_SCENE_LAYER_MASK_ROUNDED_RECTANGLE as u32,
            flags: 0,
            reserved: 0,
            bounds: image_rect(rect),
            transform: affine(&self.current.transform),
            corner_radii_x,
            corner_radii_y,
            opacity: 1.0,
            reserved0: 0,
            reserved1: 0,
            reserved2: 0,
        };
        let textured = self.current.mask != NO_INDEX && self.analytic.is_empty();
        if textured || self.analytic.len() >= PROGPU_NATIVE_SCENE_MAX_ANALYTIC_MASKS as usize {
            let result = self.scene().add_rounded_rectangle_mask(&mask);
            match self.record(result) {
                Some(mask) => self.push_masked_layer(mask),
                None => self.clips.push(self.state_entry()),
            }
            return;
        }
        let entry = self.state_entry();
        let mut masks = std::mem::take(&mut self.analytic);
        masks.push(mask);
        let result = match masks.len() {
            1 => self.scene().add_rounded_rectangle_mask(&mask),
            _ => self.scene().add_analytic_mask_chain(&masks),
        };
        match self.record(result) {
            Some(index) => {
                self.current.mask = index;
                self.changed();
            }
            None => {
                masks.pop();
            }
        }
        self.analytic = masks;
        self.clips.push(entry);
    }

    fn push_scene_layer(&mut self, bounds: Option<Rect>, opacity: f64, blend_mode: u32) {
        let target = bounds.map(|bounds| transform_bounds(bounds, &self.current.transform));
        self.push_device_layer(target, opacity, blend_mode);
    }

    /// A layer with its bounds in device pixels.
    fn push_device_layer(&mut self, target: Option<Rect>, opacity: f64, blend_mode: u32) {
        if let Some(fragment) = &mut self.fragment {
            fragment.push_layer(target, opacity, blend_mode);
        }
        let layer = progpu_native_scene_layer {
            struct_size: std::mem::size_of::<progpu_native_scene_layer>() as u32,
            flags: if target.is_some() { PROGPU_NATIVE_SCENE_LAYER_BOUNDS as u32 } else { 0 },
            bounds: target.map(image_rect).unwrap_or(progpu_native_image_rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }),
            opacity: opacity as f32,
            blend_mode,
            mask_resource_index: NO_INDEX,
            effect_resource_index: NO_INDEX,
            content_revision: 0,
            composite_revision: 0,
            reserved0: 0,
            reserved1: 0,
        };
        self.needs_whole_replay = true;
        let result = self.scene().push_layer(&layer);
        self.record(result);
        self.layers.push(LayerEntry::Layer);
    }

    fn pop_scene_layer(&mut self) {
        if let Some(entry) = self.layers.pop() {
            let entry = match entry {
                LayerEntry::SubScene(layer) => {
                    self.end_sub_scene(*layer);
                    return;
                }
                other => other,
            };
            if let LayerEntry::TileMask(mask) = entry {
                self.apply_tile_mask(*mask);
            }
            if let Some(fragment) = &mut self.fragment {
                fragment.pop_layer();
            }
            let result = self.scene().pop_layer();
            self.record(result);
        }
    }

    /// A layer that exists whatever its opacity and blend, for what is composed into it.
    fn push_isolated_layer(&mut self, target: Rect) {
        self.refuse_fragment();
        let layer = progpu_native_scene_layer {
            struct_size: std::mem::size_of::<progpu_native_scene_layer>() as u32,
            flags: (PROGPU_NATIVE_SCENE_LAYER_BOUNDS | PROGPU_NATIVE_SCENE_LAYER_FORCE_ISOLATION) as u32,
            bounds: image_rect(target),
            opacity: 1.0,
            blend_mode: PROGPU_NATIVE_BLEND_SRC_OVER as u32,
            mask_resource_index: NO_INDEX,
            effect_resource_index: NO_INDEX,
            content_revision: 0,
            composite_revision: 0,
            reserved0: 0,
            reserved1: 0,
        };
        self.needs_whole_replay = true;
        let result = self.scene().push_layer(&layer);
        self.record(result);
    }

    /// What the mask of the state covers replaced by `color`, on the GPU: the coverage is taken
    /// out of what is there and the colour times the coverage is added, so an antialiased edge
    /// is the mix of the two.
    fn clear_under_mask(&mut self, color: progpu_native_color) {
        // Past any target; the scissor of the target and the state clip it.
        const REACH: f64 = 32768.0;
        let area = self.current.clip_rect.unwrap_or(Rect::new(-REACH, -REACH, 2.0 * REACH, 2.0 * REACH));
        let saved = self.current.transform;
        self.current.transform = Matrix::IDENTITY;
        self.changed();
        let opaque = progpu_native_color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
        for (blend, fill) in [(PROGPU_NATIVE_BLEND_DST_OUT, opaque), (PROGPU_NATIVE_BLEND_PLUS, color)] {
            self.push_device_layer(self.current.clip_rect, 1.0, blend as u32);
            let result = self.scene().add_solid_brush(fill, 1.0);
            if let Some(index) = self.record(result) {
                self.analytic(Self::primitive(PROGPU_NATIVE_PRIMITIVE_RECTANGLE, area, 0.0, 0.0), index, area);
            }
            self.pop_scene_layer();
        }
        self.current.transform = saved;
        self.changed();
    }
}

impl IDrawingContextImpl for ProGpuDrawingContext {
    fn transform(&self) -> Matrix {
        if self.origin == (0.0, 0.0) {
            return self.current.transform;
        }
        self.current.transform * Matrix::create_translation(self.origin.0, self.origin.1)
    }

    fn set_frame_damage(&mut self, damage: Rect) {
        self.damage = Some(damage);
    }

    fn set_transform(&mut self, value: Matrix) {
        // Inside a layer with an effect the scene starts at the layer.
        let value = if self.origin == (0.0, 0.0) {
            value
        } else {
            value * Matrix::create_translation(-self.origin.0, -self.origin.1)
        };
        if self.current.transform != value {
            self.current.transform = value;
            self.changed();
        }
    }

    fn clear(&mut self, color: Color) {
        self.refuse_fragment();
        if self.current.mask != NO_INDEX {
            self.clear_under_mask(native_color(color, 1.0));
            return;
        }
        let color = native_color(color, 1.0);
        if self.current.clip_rect.is_none() {
            let result = self.scene().clear_target(color);
            self.record(result);
            return;
        }
        // The clear goes through the clip rectangle of the state of its scope.
        let Some(state) = self.state() else { return };
        let result = self.scene().save(state);
        if self.record(result).is_some() {
            let result = self.scene().clear_target(color);
            self.record(result);
            let result = self.scene().restore();
            self.record(result);
        }
    }

    fn draw_bitmap(&mut self, source: &dyn IBitmapImpl, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        if opacity <= 0.0 || source_rect.width <= 0.0 || source_rect.height <= 0.0 || dest_rect.width <= 0.0 || dest_rect.height <= 0.0 {
            return;
        }
        let Some(image) = self.image(source) else { return };
        let mut flags = 0u32;
        if image.premultiplied {
            flags |= PROGPU_NATIVE_SCENE_IMAGE_SOURCE_PREMULTIPLIED as u32;
        }
        if image.opaque {
            flags |= PROGPU_NATIVE_SCENE_IMAGE_SOURCE_ALPHA_IGNORE as u32;
        }
        let sampling = match self.render_options.last().map(|options| options.bitmap_interpolation_mode) {
            Some(BitmapInterpolationMode::None) => PROGPU_NATIVE_IMAGE_SAMPLING_NEAREST,
            _ => PROGPU_NATIVE_IMAGE_SAMPLING_LINEAR,
        } as u32;
        let draw = progpu_native_scene_image_draw {
            struct_size: std::mem::size_of::<progpu_native_scene_image_draw>() as u32,
            flags,
            image_width: image.width,
            image_height: image.height,
            row_bytes: image.row_bytes,
            sampling,
            source_rect: image_rect(source_rect),
            destination_rect: image_rect(dest_rect),
            transform: identity(),
            opacity: opacity.min(1.0) as f32,
            max_anisotropy: 1,
        };
        let bounds = transform_bounds(dest_rect, &self.current.transform);
        let Some(state) = self.state() else { return };
        let result = self.scene().draw_image(image.index, &draw, image_rect(bounds), state);
        self.record(result);
    }

    fn draw_bitmap_with_mask(&mut self, source: &dyn IBitmapImpl, opacity_mask: &dyn IBrush, opacity_mask_rect: Rect, dest_rect: Rect) {
        let size = source.pixel_size();
        let source_rect = Rect::new(0.0, 0.0, f64::from(size.width), f64::from(size.height));
        self.push_brush_mask_layer(opacity_mask, opacity_mask_rect);
        self.draw_bitmap(source, 1.0, source_rect, dest_rect);
        self.pop_scene_layer();
    }

    fn draw_line(&mut self, pen: Option<&dyn IPen>, p1: Point, p2: Point) {
        let Some(pen) = pen.filter(|pen| pen.thickness() > 0.0) else { return };
        let Some(brush) = pen.brush() else { return };
        let outline = PathData::line(p1, p2).widen(pen);
        self.fill_path(&outline, brush.as_ref());
    }

    fn draw_geometry(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, geometry: &dyn IGeometryImpl) {
        let Some(data) = path_data_of(geometry) else { return };
        if let Some(brush) = brush {
            self.fill_shared_path(&data, brush);
        }
        if let Some(pen) = pen.filter(|pen| pen.thickness() > 0.0) {
            if let Some(stroke_brush) = pen.brush() {
                let outline = data.widen(pen);
                self.fill_path_on(&outline, stroke_brush.as_ref(), data.bounds());
            }
        }
    }

    fn draw_rectangle(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: RoundedRect, box_shadows: &BoxShadows) {
        let shadowed = box_shadows.count() != 0;
        if shadowed {
            self.box_shadows(brush, &rect, box_shadows, false);
        }
        self.rectangle_shape(brush, pen, &rect);
        if shadowed {
            self.box_shadows(brush, &rect, box_shadows, true);
        }
    }

    fn draw_region(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, region: &dyn IPlatformRenderInterfaceRegion) {
        for rect in region.rects() {
            let rect = Rect::new(rect.left as f64, rect.top as f64, (rect.right - rect.left) as f64, (rect.bottom - rect.top) as f64);
            self.draw_rectangle(brush, pen, RoundedRect::from_rect(rect), &BoxShadows::default());
        }
    }

    fn draw_ellipse(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: Rect) {
        // A tile brush paints through the outline of the shape.
        let stroke = pen.filter(|pen| pen.thickness() > 0.0).and_then(|pen| Some((pen, pen.brush()?)));
        if brush.is_some_and(is_tile_brush) || stroke.as_ref().is_some_and(|(_, brush)| is_tile_brush(brush.as_ref())) {
            let path = PathData::ellipse(rect);
            if let Some(brush) = brush {
                self.fill_path_on(&path, brush, rect);
            }
            if let Some((pen, stroke_brush)) = stroke {
                self.fill_path_on(&path.widen(pen), stroke_brush.as_ref(), rect);
            }
            return;
        }
        let bounds = transform_bounds(rect, &self.current.transform);
        if let Some(brush) = brush {
            if let Some(index) = self.brush(brush, rect) {
                self.analytic(Self::primitive(PROGPU_NATIVE_PRIMITIVE_ELLIPSE as u32, rect, 0.0, 0.0), index, bounds);
            }
        }
        if let Some(pen) = pen.filter(|pen| pen.thickness() > 0.0) {
            let Some(stroke_brush) = pen.brush() else { return };
            if pen.dash_style().is_none() {
                if let Some(index) = self.brush(stroke_brush.as_ref(), rect) {
                    let half = pen.thickness() / 2.0;
                    let outer = transform_bounds(Rect::new(rect.x - half, rect.y - half, rect.width + pen.thickness(), rect.height + pen.thickness()), &self.current.transform);
                    self.analytic(Self::primitive(PROGPU_NATIVE_PRIMITIVE_ELLIPSE as u32, rect, 0.0, pen.thickness()), index, outer);
                }
            } else {
                let outline = PathData::ellipse(rect).widen(pen);
                self.fill_path(&outline, stroke_brush.as_ref());
            }
        }
    }

    fn draw_glyph_run(&mut self, foreground: Option<&dyn IBrush>, glyph_run: &dyn IGlyphRunImpl) {
        let Some(foreground) = foreground else { return };
        let Some(run) = glyph_run.as_any().downcast_ref::<ProGpuGlyphRun>() else {
            notes::note_once("text-run", "a glyph run of another backend is not drawn by the ProGPU backend");
            return;
        };
        if run.font().is_none() {
            notes::note_once("text-typeface", "text of a typeface of another font manager is not drawn by the ProGPU backend");
            return;
        }
        match foreground.as_solid_color_brush() {
            Some(solid) => {
                let color = native_color(solid.color(), foreground.opacity());
                if color.a > 0.0 {
                    self.draw_run(run, color);
                }
            }
            // A gradient fills the outlines of the run as a path: its coordinates are those of
            // the run.
            None => {
                if let Some(path) = run.outline_path() {
                    self.fill_path(&path, foreground);
                }
            }
        }
    }

    fn create_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(ProGpuLayer::new(self.gpu.clone(), size, Vector::new(96.0, 96.0)))
    }

    fn push_clip(&mut self, clip: Rect) {
        if axis_aligned(&self.current.transform) {
            let target = transform_bounds(clip, &self.current.transform);
            if let Some(fragment) = &mut self.fragment {
                fragment.push_clip(target);
            }
            self.push_device_clip(target);
        } else {
            self.push_analytic_clip(clip, [0.0; 4], [0.0; 4]);
        }
    }

    fn push_clip_rounded(&mut self, clip: RoundedRect) {
        if !clip.is_rounded() {
            self.push_clip(clip.rect);
            return;
        }
        self.push_analytic_clip(
            clip.rect,
            [
                clip.radii_top_left.x as f32,
                clip.radii_top_right.x as f32,
                clip.radii_bottom_right.x as f32,
                clip.radii_bottom_left.x as f32,
            ],
            [
                clip.radii_top_left.y as f32,
                clip.radii_top_right.y as f32,
                clip.radii_bottom_right.y as f32,
                clip.radii_bottom_left.y as f32,
            ],
        );
    }

    fn push_clip_region(&mut self, region: &dyn IPlatformRenderInterfaceRegion) {
        let bounds = region.bounds();
        self.push_clip(Rect::new(bounds.left as f64, bounds.top as f64, (bounds.right - bounds.left) as f64, (bounds.bottom - bounds.top) as f64));
    }

    fn pop_clip(&mut self) {
        if let Some(fragment) = &mut self.fragment {
            fragment.pop_clip();
        }
        match self.clips.pop() {
            Some(ClipEntry::State { clip_rect, mask, analytic }) => {
                self.current.clip_rect = clip_rect;
                self.current.mask = mask;
                self.analytic.truncate(analytic);
                self.changed();
            }
            Some(ClipEntry::Layer) => {
                let result = self.scene().pop_layer();
                self.record(result);
            }
            None => {}
        }
    }

    fn push_layer(&mut self, bounds: Rect) {
        self.push_scene_layer(Some(bounds), 1.0, PROGPU_NATIVE_BLEND_SRC_OVER as u32);
    }

    fn pop_layer(&mut self) {
        self.pop_scene_layer();
    }

    fn push_opacity(&mut self, opacity: f64, bounds: Option<Rect>) {
        self.push_scene_layer(bounds, opacity, PROGPU_NATIVE_BLEND_SRC_OVER as u32);
    }

    fn pop_opacity(&mut self) {
        self.pop_scene_layer();
    }

    fn push_opacity_mask(&mut self, mask: &dyn IBrush, bounds: Rect) {
        self.push_brush_mask_layer(mask, bounds);
    }

    fn pop_opacity_mask(&mut self) {
        self.pop_scene_layer();
    }

    fn push_geometry_clip(&mut self, clip: &dyn IGeometryImpl) {
        self.refuse_fragment();
        let Some(data) = path_data_of(clip) else {
            self.clips.push(self.state_entry());
            return;
        };
        if !has_area(data.bounds()) || !invertible(&self.current.transform) {
            self.push_empty_clip();
            return;
        }
        let mask = self.vector_mask(&data, PROGPU_NATIVE_CLIP_INTERSECT as u32);
        self.push_mask_clip(mask);
    }

    fn pop_geometry_clip(&mut self) {
        self.pop_clip();
    }

    fn push_render_options(&mut self, render_options: RenderOptions) {
        self.render_options.push(render_options);
    }

    fn pop_render_options(&mut self) {
        self.render_options.pop();
    }

    fn push_text_options(&mut self, text_options: TextOptions) {
        self.text_options.push(text_options);
    }

    fn pop_text_options(&mut self) {
        self.text_options.pop();
    }

    fn try_push_compositing_layer(&mut self, blend: BitmapBlendingMode, opacity: f64, bounds: Rect) -> bool {
        if ![bounds.x, bounds.y, bounds.width, bounds.height, opacity].iter().all(|value| value.is_finite())
            || bounds.width < 0.0
            || bounds.height < 0.0
            || !(0.0..=1.0).contains(&opacity)
        {
            return false;
        }
        // The bounds are an allocation hint and not a clip: the layer is not cut to them.
        self.push_scene_layer(None, opacity, native_blend_mode(blend));
        true
    }

    fn pop_compositing_layer(&mut self) {
        self.pop_scene_layer();
    }

    fn get_feature(&mut self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }

    fn as_drawing_context_impl_with_effects(&mut self) -> Option<&mut dyn IDrawingContextImplWithEffects> {
        Some(self)
    }

    fn as_drawing_context_impl_with_fragments(&mut self) -> Option<&mut dyn IDrawingContextImplWithFragments> {
        Some(self)
    }

    fn as_drawing_context_impl_with_layer_effects(
        &mut self,
    ) -> Option<&mut dyn ferroui_base::platform::IDrawingContextImplWithLayerEffects> {
        Some(self)
    }

    fn as_drawing_context_with_acrylic_like_support(
        &mut self,
    ) -> Option<&mut dyn ferroui_base::platform::IDrawingContextWithAcrylicLikeSupport> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn dispose(&mut self) {
        while !self.layers.is_empty() {
            self.pop_scene_layer();
        }
        while !self.clips.is_empty() {
            self.pop_clip();
        }
        if let (Some(scene), Some(complete)) = (self.scene.take(), self.complete.take()) {
            let images = self.textures.drain(..).map(|(texture, _)| texture).collect();
            let damage = self.damage.filter(|_| !self.needs_whole_replay);
            complete(Recording { scene, error: self.error.take(), images, damage });
        }
    }
}

impl IDrawingContextImplWithEffects for ProGpuDrawingContext {
    fn push_effect(&mut self, clip_rect: Option<Rect>, effect: &dyn IEffect) {
        if effect.as_any().is::<ferroui_base::media::ImmutableLuminanceToAlphaEffect>()
            && self.push_luminance_layer(clip_rect)
        {
            return;
        }
        if let Some(custom) = effect.as_custom_effect() {
            if custom.has_shader() && self.push_shader_effect(clip_rect, custom) {
                return;
            }
        }
        let target = clip_rect.map(|clip| transform_bounds(clip, &self.current.transform));
        let chain = if let Some(blur) = effect.as_blur_effect() {
            let sigma = blur_radius_to_sigma(blur.radius());
            (blur.radius() > 0.0)
                .then(|| self.effect_chain(PROGPU_NATIVE_GROUP_EFFECT_GAUSSIAN_BLUR as u32, sigma, (0.0, 0.0), [0.0; 4]))
                .flatten()
        } else if let Some(shadow) = effect.as_drop_shadow_effect() {
            let sigma = if shadow.blur_radius() > 0.0 { blur_radius_to_sigma(shadow.blur_radius()) } else { 0.0 };
            let color = shadow.color();
            let channels = [
                f32::from(color.r) / 255.0,
                f32::from(color.g) / 255.0,
                f32::from(color.b) / 255.0,
                ((f64::from(color.a) / 255.0) * shadow.opacity()).clamp(0.0, 1.0) as f32,
            ];
            self.effect_chain(
                PROGPU_NATIVE_GROUP_EFFECT_DROP_SHADOW as u32,
                sigma,
                (shadow.offset_x() as f32, shadow.offset_y() as f32),
                channels,
            )
        } else {
            notes::note_once("effect-kind", "this effect is drawn without it by the ProGPU backend");
            None
        };
        self.push_effect_layer(target, chain.unwrap_or(NO_INDEX), NO_INDEX);
    }

    fn pop_effect(&mut self) {
        self.pop_scene_layer();
    }
}

impl IDrawingContextImplWithFragments for ProGpuDrawingContext {
    fn begin_fragment(&mut self) {
        // What a layer with an effect holds is in the coordinates of that layer.
        if self.origin != (0.0, 0.0) {
            return;
        }
        self.fragment = Some(FragmentRecorder::new(self.current.transform, self.aliased_text()));
    }

    fn end_fragment(&mut self) -> Option<Rc<dyn IDrawingFragment>> {
        let fragment = self.fragment.take()?.finish()?;
        Some(Rc::new(fragment))
    }

    fn draw_fragment(&mut self, fragment: &Rc<dyn IDrawingFragment>) -> bool {
        // Fragments do not nest: what is being kept is drawn from its commands. Their places are
        // in the coordinates of the target, not of a layer with an effect.
        if self.fragment.is_some() || self.origin != (0.0, 0.0) {
            return false;
        }
        let Some(fragment) = fragment.as_any().downcast_ref::<ProGpuFragment>() else {
            return false;
        };
        let Some(offset) = fragment.offset_to(&self.current.transform, self.aliased_text()) else {
            return false;
        };
        self.replay(fragment, offset);
        true
    }
}
