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
use crate::geometry::{path_data_of, PathData};
use crate::glyph_run::{Placement, ProGpuGlyphRun};
use crate::gpu::{Gpu, Recording, SampledTexture};
use crate::layer::ProGpuLayer;
use crate::notes;
use crate::scene::{SceneBuildError, SceneBuilder, NO_INDEX};
use ferroui_base::media::imaging::{BitmapBlendingMode, BitmapInterpolationMode};
use ferroui_base::media::{BoxShadows, Color, EdgeMode, GradientSpreadMethod, IBrush, IPen, RenderOptions, TextOptions, TextRenderingMode};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, IGeometryImpl, IGlyphRunImpl,
    IPlatformRenderInterfaceRegion, PixelFormat,
};
use ferroui_base::{Matrix, PixelSize, Point, Rect, RoundedRect, Vector};
use progpu_native_sys::progpu::*;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;

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
    /// The state's clip rectangle and mask before the clip.
    State { clip_rect: Option<Rect>, mask: u32 },
    /// A clip that needed a layer.
    Layer,
}

/// What a layer of the stack is, to pop it.
enum LayerEntry {
    Layer,
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
    layers: Vec<LayerEntry>,
    render_options: Vec<RenderOptions>,
    text_options: Vec<TextOptions>,
    error: Option<SceneBuildError>,
    complete: Option<Box<dyn FnOnce(Recording)>>,
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
            layers: Vec::new(),
            render_options: Vec::new(),
            text_options: Vec::new(),
            error,
            complete,
        }
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
        let state = if upright { self.glyph_state() } else { self.state() };
        let Some(state) = state else { return };
        let Some(style) = self.text_style(color, aliased) else { return };
        let result = self.scene().add_glyph_outlines(&raster.outlines, &raster.segments);
        let Some(resource) = self.record(result) else { return };

        let mut positioned = std::mem::take(&mut self.positioned);
        positioned.clear();
        positioned.extend(raster.glyphs.iter().map(|glyph| progpu_native_positioned_glyph {
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
        let bounds = transform_bounds(ferroui_base::platform::IGlyphRunImpl::bounds(run), &transform).inflate(1.0);
        let result = self.scene().draw_glyph_run(resource, &positioned, image_rect(bounds), state, style);
        self.record(result);
        self.positioned = positioned;
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
            return self.record(result);
        }
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
        let result = self.scene().add_brush(&native, &stops);
        self.record(result)
    }

    fn analytic(&mut self, primitive: progpu_native_analytic_primitive, brush: u32, bounds: Rect) {
        let Some(state) = self.state() else { return };
        let result = self.scene().draw_analytic(&[primitive], &[brush], image_rect(bounds), state);
        self.record(result);
    }

    /// Fills the canonical segments of `data` (its filled figures) with a brush.
    fn fill_path(&mut self, data: &PathData, brush: &dyn IBrush) {
        let segments = data.filled_segments();
        if segments.is_empty() {
            return;
        }
        let bounds = data.bounds();
        let Some(brush_index) = self.brush(brush, bounds) else { return };
        let Some(state) = self.state() else { return };
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
        self.record(result);
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
    fn vector_mask(&mut self, data: &PathData, operation: u32) -> Option<u32> {
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

    fn box_shadows(&mut self, brush: Option<&dyn IBrush>, rect: &RoundedRect, shadows: &BoxShadows) {
        let uniform = rect.is_uniform() && rect.radii_top_left.x == rect.radii_top_left.y;
        let opaque = Self::opaque(brush);
        for shadow in shadows.iter() {
            if shadow.color.a == 0 {
                continue;
            }
            if shadow.is_inset || !uniform {
                // Inset shadows and the shadows of rounded rectangles with different radii: the
                // group effect (step P5 of the plan).
                notes::note_once("box-shadow", "inset box shadows and the shadows of rectangles with different corner radii are not drawn by the ProGPU backend yet");
                continue;
            }
            let sigma = blur_radius_to_sigma(shadow.blur);
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
            let Some(brush_index) = self.record(result) else { return };
            // The shadow is outside its element: under an element that is not opaque, a
            // difference mask of the element keeps it out.
            let saved = self.current;
            if !opaque {
                let element = Self::rounded_rect_path(rect);
                if let Some(mask) = self.vector_mask(&element, PROGPU_NATIVE_CLIP_DIFFERENCE as u32) {
                    if self.current.mask == NO_INDEX {
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

    /// The image resource of the scene for a bitmap: a layer of the same device (sampled from
    /// its texture) or a bitmap of pixels (copied into the stream), added once per scene. None
    /// (with a note) for what the backend cannot sample.
    fn image(&mut self, source: &dyn IBitmapImpl) -> Option<SceneImage> {
        let any = source.as_any();
        if let Some(layer) = any.downcast_ref::<ProGpuLayer>() {
            let texture = layer.texture()?;
            if let Some((_, image)) = self.textures.iter().find(|(own, _)| Rc::ptr_eq(&own.texture, texture)) {
                return Some(*image);
            }
            if !self.gpu.as_ref().is_some_and(|gpu| Rc::ptr_eq(gpu.device(), texture.device())) {
                notes::note_once("layer-device", "a layer of another device is not drawn by the ProGPU backend");
                return None;
            }
            let (width, height) = (texture.width(), texture.height());
            let result = self.scene().add_external_image(width, height);
            let (index, resource_id, generation) = self.record(result)?;
            let image = SceneImage { index, width, height, row_bytes: width * 4, premultiplied: true, opaque: false };
            self.textures.push((SampledTexture { texture: texture.clone(), resource_id, generation }, image));
            return Some(image);
        }
        if let Some(bitmap) = any.downcast_ref::<ProGpuBitmap>() {
            let key = (bitmap as *const ProGpuBitmap as usize, bitmap.version());
            if let Some(image) = self.bitmaps.get(&key) {
                return Some(*image);
            }
            let size = bitmap.pixel_size();
            if size.width <= 0 || size.height <= 0 {
                return None;
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

    fn push_mask_clip(&mut self, mask: Option<u32>) {
        let Some(mask) = mask else {
            self.clips.push(ClipEntry::Layer);
            return;
        };
        if self.current.mask == NO_INDEX {
            self.clips.push(ClipEntry::State { clip_rect: self.current.clip_rect, mask: self.current.mask });
            self.current.mask = mask;
            self.changed();
        } else {
            // A second mask nests as a layer masked by it.
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
            let result = self.scene().push_layer(&layer);
            self.record(result);
            self.clips.push(ClipEntry::Layer);
        }
    }

    fn push_scene_layer(&mut self, bounds: Option<Rect>, opacity: f64, blend_mode: u32) {
        let target = bounds.map(|bounds| transform_bounds(bounds, &self.current.transform));
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
        let result = self.scene().push_layer(&layer);
        self.record(result);
        self.layers.push(LayerEntry::Layer);
    }

    fn pop_scene_layer(&mut self) {
        if self.layers.pop().is_some() {
            let result = self.scene().pop_layer();
            self.record(result);
        }
    }
}

impl IDrawingContextImpl for ProGpuDrawingContext {
    fn transform(&self) -> Matrix {
        self.current.transform
    }

    fn set_transform(&mut self, value: Matrix) {
        if self.current.transform != value {
            self.current.transform = value;
            self.changed();
        }
    }

    fn clear(&mut self, color: Color) {
        if self.current.mask != NO_INDEX {
            // The engine clears through a clip rectangle; under a coverage mask it needs a layer.
            notes::note_once("clear-under-mask", "a clear under a rounded or geometry clip is not drawn by the ProGPU backend yet");
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

    fn draw_bitmap_with_mask(&mut self, _source: &dyn IBitmapImpl, _opacity_mask: &dyn IBrush, _opacity_mask_rect: Rect, _dest_rect: Rect) {
        // Step P5 of the plan.
        notes::note_once("bitmap-mask", "bitmaps under an opacity mask are not drawn by the ProGPU backend yet");
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
            self.fill_path(&data, brush);
        }
        if let Some(pen) = pen.filter(|pen| pen.thickness() > 0.0) {
            if let Some(stroke_brush) = pen.brush() {
                let outline = data.widen(pen);
                self.fill_path(&outline, stroke_brush.as_ref());
            }
        }
    }

    fn draw_rectangle(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: RoundedRect, box_shadows: &BoxShadows) {
        if box_shadows.count() != 0 {
            self.box_shadows(brush, &rect, box_shadows);
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
            self.fill_path(&Self::rounded_rect_path(&rect), brush);
        }
        // Dashed strokes and the strokes of rectangles with different radii: the widened outline.
        if let Some(pen) = pen.filter(|pen| pen.thickness() > 0.0) {
            if let Some(stroke_brush) = pen.brush() {
                let outline = Self::rounded_rect_path(&rect).widen(pen);
                self.fill_path(&outline, stroke_brush.as_ref());
            }
        }
    }

    fn draw_region(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, region: &dyn IPlatformRenderInterfaceRegion) {
        for rect in region.rects() {
            let rect = Rect::new(rect.left as f64, rect.top as f64, (rect.right - rect.left) as f64, (rect.bottom - rect.top) as f64);
            self.draw_rectangle(brush, pen, RoundedRect::from_rect(rect), &BoxShadows::default());
        }
    }

    fn draw_ellipse(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: Rect) {
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
            self.clips.push(ClipEntry::State { clip_rect: self.current.clip_rect, mask: self.current.mask });
            self.current.clip_rect = Some(match self.current.clip_rect {
                Some(current) => intersect(current, target),
                None => target,
            });
            self.changed();
        } else {
            let mask = self.vector_mask(&PathData::rectangle(clip), PROGPU_NATIVE_CLIP_INTERSECT as u32);
            self.push_mask_clip(mask);
        }
    }

    fn push_clip_rounded(&mut self, clip: RoundedRect) {
        if !clip.is_rounded() {
            self.push_clip(clip.rect);
            return;
        }
        let mask = progpu_native_scene_layer_mask {
            struct_size: std::mem::size_of::<progpu_native_scene_layer_mask>() as u32,
            kind: PROGPU_NATIVE_SCENE_LAYER_MASK_ROUNDED_RECTANGLE as u32,
            flags: 0,
            reserved: 0,
            bounds: image_rect(clip.rect),
            transform: affine(&self.current.transform),
            corner_radii_x: [
                clip.radii_top_left.x as f32,
                clip.radii_top_right.x as f32,
                clip.radii_bottom_right.x as f32,
                clip.radii_bottom_left.x as f32,
            ],
            corner_radii_y: [
                clip.radii_top_left.y as f32,
                clip.radii_top_right.y as f32,
                clip.radii_bottom_right.y as f32,
                clip.radii_bottom_left.y as f32,
            ],
            opacity: 1.0,
            reserved0: 0,
            reserved1: 0,
            reserved2: 0,
        };
        let result = self.scene().add_rounded_rectangle_mask(&mask);
        let mask = self.record(result);
        self.push_mask_clip(mask);
    }

    fn push_clip_region(&mut self, region: &dyn IPlatformRenderInterfaceRegion) {
        let bounds = region.bounds();
        self.push_clip(Rect::new(bounds.left as f64, bounds.top as f64, (bounds.right - bounds.left) as f64, (bounds.bottom - bounds.top) as f64));
    }

    fn pop_clip(&mut self) {
        match self.clips.pop() {
            Some(ClipEntry::State { clip_rect, mask }) => {
                self.current.clip_rect = clip_rect;
                self.current.mask = mask;
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

    fn push_opacity_mask(&mut self, _mask: &dyn IBrush, bounds: Rect) {
        // Opacity masks (a brush layer mask): step P5 of the plan; the layer keeps the stack.
        notes::note_once("opacity-mask", "opacity masks are not applied by the ProGPU backend yet");
        self.push_scene_layer(Some(bounds), 1.0, PROGPU_NATIVE_BLEND_SRC_OVER as u32);
    }

    fn pop_opacity_mask(&mut self) {
        self.pop_scene_layer();
    }

    fn push_geometry_clip(&mut self, clip: &dyn IGeometryImpl) {
        let mask = path_data_of(clip).and_then(|data| self.vector_mask(&data, PROGPU_NATIVE_CLIP_INTERSECT as u32));
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

    fn try_push_compositing_layer(&mut self, _blend: BitmapBlendingMode, _opacity: f64, _bounds: Rect) -> bool {
        // Blend modes of compositing layers: step P5 of the plan.
        false
    }

    fn get_feature(&mut self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
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
            complete(Recording { scene, error: self.error.take(), images });
        }
    }
}
