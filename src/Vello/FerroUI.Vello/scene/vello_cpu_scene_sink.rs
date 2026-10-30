use crate::scene::{
    IVelloSceneSink, VelloSceneBrush, VelloSceneCapabilities, VelloSceneFilter, VelloSceneFilterCapabilities,
    VelloSceneGlyphRun, VelloScenePaint, VelloScenePixelRect,
};
use crate::vello_options::VelloRenderingMode;
use glifo::{FontEmbolden, Glyph};
use kurbo::{Affine, BezPath, Diagonal2, PathEl, Shape, Stroke, StrokeOpts};
use peniko::color::{AlphaColor, Srgb};
use peniko::{BlendMode, Fill, ImageBrush, ImageSampler};
use std::borrow::Cow;
use std::cell::RefCell;
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

/// A rectangle as the renderers fill and clip to it fastest, with the
/// transform to draw it with, or `None` for a transform that turns its sides
/// off the axes.
///
/// The renderers take their own way for a rectangle whose sides stay on the
/// axes when its edges are anti-aliased only. An edge without anti-aliasing
/// paints the pixels whose centers the rectangle holds (the threshold of the
/// sinks is half the coverage): those of the rectangle in the pixels of the
/// target with its sides moved to the nearest edge between pixels, which
/// the anti-aliased way paints exactly.
pub(super) fn axis_aligned_rect(rect: kurbo::Rect, transform: Affine, anti_alias: bool) -> Option<(kurbo::Rect, Affine)> {
    let [a, b, c, d, _, _] = transform.as_coeffs();
    if b != 0.0 || c != 0.0 || a == 0.0 || d == 0.0 || !transform.is_finite() {
        return None;
    }
    if anti_alias {
        return Some((rect, transform));
    }

    let pixels = transform.transform_rect_bbox(rect);
    let snap = |value: f64| (value - 0.5).ceil();
    let snapped = kurbo::Rect::new(snap(pixels.x0), snap(pixels.y0), snap(pixels.x1), snap(pixels.y1));
    Some((snapped, Affine::IDENTITY))
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
    /// The size of the target.
    target_size: (u16, u16),
    /// The workers of the render context of the whole target
    /// ([`render_threads`]).
    threads: u16,
    /// Where the render context lies on the target: its top-left pixel. The
    /// context covers the whole target unless the scene was restricted to
    /// the rectangles it clears ([`IVelloSceneSink::retain_target`]).
    origin: (u16, u16),
    /// The clips that were opened before anything else was recorded, to
    /// be opened again in a render context of the rectangles a kept target
    /// clears; `None` once something else was recorded.
    first_clips: Option<Vec<FirstClip>>,
    /// The clips that are open.
    open_clips: usize,
    /// The clips that bound a render context of the cleared rectangles:
    /// those that were open when it was made.
    bounding_clips: Vec<FirstClip>,
    /// A render context of the cleared rectangles, with its place, whose
    /// clips were ended while the scene went on: what was recorded in it is
    /// rendered first, and the rest of the scene in a render context of
    /// the whole target over it.
    bounded: Option<(RenderContext, (u16, u16))>,
    /// Whether the clips that bound the render context of the cleared
    /// rectangles were ended: what is recorded next goes to a render
    /// context of the whole target.
    unbounded: bool,
    /// What the last render may have changed: the render context of the
    /// cleared rectangles, or `None` for the whole target.
    rendered: Option<VelloScenePixelRect>,
}

/// A clip opened before anything was drawn (see
/// [`VelloCpuSceneSink::first_clips`]).
#[derive(Clone)]
enum FirstClip {
    Path { path: BezPath, fill_rule: Fill, transform: Affine, anti_alias: bool },
    Rect { rect: kurbo::Rect, transform: Affine, anti_alias: bool },
}

impl VelloCpuSceneSink {
    /// Creates the scene of a target of the given size: a large one is
    /// rendered by the thread pool of the renderer ([`render_threads`]),
    /// a small one on the thread that draws.
    pub fn new(width: u16, height: u16) -> Self {
        Self::with_threads(width, height, render_threads(width, height))
    }

    /// Creates the scene of a target of the given size rendered on the
    /// thread that draws: the renderer has filter layers then (its thread
    /// pool has none).
    pub fn single_threaded(width: u16, height: u16) -> Self {
        Self::with_threads(width, height, 0)
    }

    fn with_threads(width: u16, height: u16, threads: u16) -> Self {
        let (context, resources) = RendererPool::take(width, height, threads);

        Self {
            context,
            resources,
            images: HashMap::new(),
            retained: None,
            target_size: (width, height),
            threads,
            origin: (0, 0),
            first_clips: Some(Vec::new()),
            open_clips: 0,
            bounding_clips: Vec::new(),
            bounded: None,
            unbounded: false,
            rendered: None,
        }
    }

    /// A transform to the pixels of the target as a transform to the pixels
    /// of the render context.
    fn placed(&self, transform: Affine) -> Affine {
        match self.origin {
            (0, 0) => transform,
            (x, y) => Affine::translate((-f64::from(x), -f64::from(y))) * transform,
        }
    }

    /// The pixels of the target as the renderer draws into them.
    fn pixmap(width: u16, height: u16, pixels: &mut [u8]) -> PixmapMut<'_> {
        PixmapMut::new(width, height, pixels)
            .unwrap_or_else(|| panic!("The pixels of the target do not have the size of the scene"))
    }

    /// Something other than a clip is recorded: the scene keeps its size.
    fn drawing(&mut self) {
        self.first_clips = None;
        self.recording();
    }

    /// Something is recorded: after the clips that bound the render context
    /// were ended, in a render context of the whole target.
    fn recording(&mut self) {
        if std::mem::take(&mut self.unbounded) {
            self.unbound();
        }
    }

    /// Gives the scene a render context of `size` at `origin`, with the
    /// clips that were opened before anything was drawn opened again in it.
    fn place_context(&mut self, origin: (u16, u16), size: (u16, u16)) {
        // A context of the size of the rectangle is made for it now and
        // again: on the thread that draws, without a thread pool to start.
        let context = RendererPool::take_context(size.0, size.1, 0);
        RendererPool::give_back_context(std::mem::replace(&mut self.context, context));
        self.origin = origin;

        let clips = self.first_clips.take().unwrap_or_default();
        self.open_again(&clips);
        self.first_clips = Some(clips.clone());
        self.bounding_clips = clips;
    }

    /// Opens clips again in the render context.
    fn open_again(&mut self, clips: &[FirstClip]) {
        for clip in clips {
            match clip {
                FirstClip::Path { path, fill_rule, transform, anti_alias } => {
                    self.open_clip(path, *fill_rule, *transform, *anti_alias)
                }
                FirstClip::Rect { rect, transform, anti_alias } => self.open_clip_rect(*rect, *transform, *anti_alias),
            }
        }
    }

    /// The clips that bound the render context of the cleared rectangles
    /// were ended: the scene goes on in a render context of the whole
    /// target, with the clips that are still open, and the bounded one is
    /// rendered first.
    fn unbound(&mut self) {
        let context = RendererPool::take_context(self.target_size.0, self.target_size.1, self.threads);
        let bounded = std::mem::replace(&mut self.context, context);
        self.bounded = Some((bounded, self.origin));
        self.origin = (0, 0);

        let clips = std::mem::take(&mut self.bounding_clips);
        self.open_again(&clips[..self.open_clips.min(clips.len())]);
    }

    fn open_clip(&mut self, path: &BezPath, fill_rule: Fill, transform: Affine, anti_alias: bool) {
        let (path, path_transform) = prepare(path, self.placed(transform));

        self.set_anti_alias(anti_alias);
        self.context.set_transform(path_transform);
        self.context.set_fill_rule(fill_rule);
        self.context.push_clip_path(&path);
    }

    fn open_clip_rect(&mut self, rect: kurbo::Rect, transform: Affine, anti_alias: bool) {
        let Some((rect, rect_transform)) = axis_aligned_rect(rect, self.placed(transform), anti_alias) else {
            self.open_clip(&rect.to_path(0.1), Fill::NonZero, transform, anti_alias);
            return;
        };

        self.set_anti_alias(true);
        self.context.set_transform(rect_transform);
        self.context.push_clip_rect(&rect);
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

/// The render contexts and the resources of the renderer a thread keeps
/// between scenes.
///
/// A scene is made for every frame of a target. A render context holds the
/// storage of the strips and of the tiles of its size (megabytes for a
/// window), and the resources hold what the renderer keeps of glyphs (their
/// outlines and the hinting instances of the fonts, which run a program of
/// the font to be made): a sink takes them from here and gives them back
/// when it is dropped, so that neither is made again for every frame.
struct RendererPool {
    /// Render contexts that were reset, the one used last at the end.
    contexts: Vec<RenderContext>,
    /// The resources of the renderer, while no sink of the thread has them.
    resources: Option<Resources>,
}

ferroui_base::ferro_thread_local! {
    static RENDERER_POOL: RefCell<RendererPool> =
        const { RefCell::new(RendererPool { contexts: Vec::new(), resources: None }) };
}

impl RendererPool {
    /// The most render contexts a thread keeps: a window, its layers and
    /// the surfaces of its effects at their sizes.
    const CONTEXTS: usize = 4;

    /// A render context of the size and of `threads` workers, and the
    /// resources of the thread, or new ones.
    fn take(width: u16, height: u16, threads: u16) -> (RenderContext, Resources) {
        let resources = RENDERER_POOL.try_with(|pool| pool.borrow_mut().resources.take()).ok().flatten();
        (Self::take_context(width, height, threads), resources.unwrap_or_default())
    }

    /// A render context of the size and of `threads` workers from the
    /// pool, or a new one.
    fn take_context(width: u16, height: u16, threads: u16) -> RenderContext {
        let pooled = RENDERER_POOL
            .try_with(|pool| {
                let mut pool = pool.borrow_mut();
                let index = pool.contexts.iter().rposition(|context| {
                    (context.width(), context.height(), context.render_settings().num_threads) == (width, height, threads)
                });
                index.map(|index| pool.contexts.remove(index))
            })
            .ok()
            .flatten();
        pooled.unwrap_or_else(|| Self::new_context(width, height, threads))
    }

    /// Keeps a render context, reset, for a later scene of the thread.
    fn give_back_context(mut context: RenderContext) {
        context.reset();
        let _ = RENDERER_POOL.try_with(|pool| {
            let mut pool = pool.borrow_mut();
            if pool.contexts.len() == Self::CONTEXTS {
                pool.contexts.remove(0);
            }
            pool.contexts.push(context);
        });
    }

    /// A render context of the size, rendered by `threads` workers of a
    /// thread pool of its own, or on the thread that draws for none.
    fn new_context(width: u16, height: u16, threads: u16) -> RenderContext {
        RenderContext::new_with(width, height, RenderSettings { num_threads: threads, ..RenderSettings::default() })
    }

    /// Keeps a render context, reset, and the resources for the next scene
    /// of the thread.
    fn give_back(context: RenderContext, resources: Resources) {
        Self::give_back_context(context);
        let _ = RENDERER_POOL.try_with(|pool| {
            let mut pool = pool.borrow_mut();
            if pool.resources.is_none() {
                pool.resources = Some(resources);
            }
        });
    }
}

/// The workers that render a scene of `width` by `height` pixels in the
/// CPU mode: none (the thread that draws renders it) for a small scene,
/// where starting them costs more than they save, and in a web page, where
/// the renderer is built without its thread pool (the manifest); otherwise
/// the cores of the machine but one, at most seven.
fn render_threads(width: u16, height: u16) -> u16 {
    #[cfg(not(target_family = "wasm"))]
    {
        const LARGE: u32 = 512 * 512;
        if u32::from(width) * u32::from(height) >= LARGE {
            let cores = std::thread::available_parallelism().map_or(1, |cores| cores.get());
            return (cores.saturating_sub(1)).min(7) as u16;
        }
    }
    let _ = (width, height);
    0
}

impl Drop for VelloCpuSceneSink {
    fn drop(&mut self) {
        // The context left in the sink is of one pixel: its storage is a
        // few bytes.
        if let Some((bounded, _)) = self.bounded.take() {
            RendererPool::give_back_context(bounded);
        }
        let context = std::mem::replace(&mut self.context, RendererPool::new_context(1, 1, 0));
        RendererPool::give_back(context, std::mem::take(&mut self.resources));
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
        // Before anything but clips was recorded, a scene that is composed
        // over its target in the rectangles it clears is rendered there
        // only: a render context of the rectangle around them, at its
        // place on the target. The renderer walks every tile of its
        // context, whatever the scene covers.
        // The clips that are open bound what is drawn while they are.
        let first = self.retained.is_none()
            && self.origin == (0, 0)
            && self.bounded.is_none()
            && self.first_clips.as_ref().is_some_and(|clips| !clips.is_empty() && clips.len() == self.open_clips);
        self.retained.get_or_insert_with(Vec::new).extend_from_slice(cleared);
        if !first || cleared.is_empty() {
            return;
        }

        let (width, height) = self.target_size;
        let bounds = cleared.iter().fold(None::<VelloScenePixelRect>, |bounds, rect| {
            let rect = VelloScenePixelRect {
                x0: rect.x0.min(width),
                y0: rect.y0.min(height),
                x1: rect.x1.min(width),
                y1: rect.y1.min(height),
            };
            Some(match bounds {
                None => rect,
                Some(bounds) => VelloScenePixelRect {
                    x0: bounds.x0.min(rect.x0),
                    y0: bounds.y0.min(rect.y0),
                    x1: bounds.x1.max(rect.x1),
                    y1: bounds.y1.max(rect.y1),
                },
            })
        });
        let Some(bounds) = bounds.filter(|bounds| bounds.x1 > bounds.x0 && bounds.y1 > bounds.y0) else {
            return;
        };
        let size = (bounds.x1 - bounds.x0, bounds.y1 - bounds.y0);
        if u32::from(size.0) * u32::from(size.1) * 4 > u32::from(width) * u32::from(height) * 3 {
            // Most of the target: the whole of it is as quick.
            return;
        }
        self.place_context((bounds.x0, bounds.y0), size);
    }

    fn rendered_rect(&self) -> Option<VelloScenePixelRect> {
        self.rendered
    }

    fn width(&self) -> u16 {
        self.target_size.0
    }

    fn height(&self) -> u16 {
        self.target_size.1
    }

    fn reset(&mut self) {
        if let Some((bounded, _)) = self.bounded.take() {
            RendererPool::give_back_context(bounded);
        }
        self.open_clips = 0;
        self.unbounded = false;
        self.bounding_clips.clear();
        if self.origin != (0, 0) || (self.context.width(), self.context.height()) != self.target_size {
            let context = RendererPool::take_context(self.target_size.0, self.target_size.1, self.threads);
            RendererPool::give_back_context(std::mem::replace(&mut self.context, context));
            self.origin = (0, 0);
        }
        self.context.reset();
        self.images.clear();
        self.retained = None;
        self.first_clips = Some(Vec::new());
    }

    fn fill_glyph_outline(
        &mut self,
        path: &BezPath,
        transform: Affine,
        paint: &VelloScenePaint,
        anti_alias: bool,
        prepare: &dyn Fn(Affine) -> Option<(std::sync::Arc<BezPath>, Affine)>,
    ) {
        super::fill_cached_glyph_outline(self, path, transform, paint, anti_alias, prepare);
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
        self.drawing();
        let transform = self.placed(transform);
        let (path, path_transform) = prepare(path, transform);

        self.set_anti_alias(anti_alias);
        self.set_paint(paint, transform, path_transform);
        self.context.set_transform(path_transform);
        self.context.set_fill_rule(fill_rule);
        self.context.set_blend_mode(blend_mode);
        self.context.fill_path(&path);
        self.context.set_blend_mode(BlendMode::default());
    }

    fn fill_rect(
        &mut self,
        rect: kurbo::Rect,
        transform: Affine,
        paint: &VelloScenePaint,
        blend_mode: BlendMode,
        anti_alias: bool,
    ) {
        self.drawing();
        let Some((rect, rect_transform)) = axis_aligned_rect(rect, self.placed(transform), anti_alias) else {
            self.fill(&rect.to_path(0.1), Fill::NonZero, transform, paint, blend_mode, anti_alias);
            return;
        };
        let transform = self.placed(transform);

        self.set_anti_alias(true);
        self.set_paint(paint, transform, rect_transform);
        self.context.set_transform(rect_transform);
        self.context.set_blend_mode(blend_mode);
        self.context.fill_rect(&rect);
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
        self.drawing();
        let transform = self.placed(transform);
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
        self.recording();
        if let Some(clips) = &mut self.first_clips {
            clips.push(FirstClip::Path { path: path.clone(), fill_rule, transform, anti_alias });
        }
        self.open_clips += 1;
        self.open_clip(path, fill_rule, transform, anti_alias);
    }

    fn push_clip_rect(&mut self, rect: kurbo::Rect, transform: Affine, anti_alias: bool) {
        self.recording();
        if let Some(clips) = &mut self.first_clips {
            clips.push(FirstClip::Rect { rect, transform, anti_alias });
        }
        self.open_clips += 1;
        self.open_clip_rect(rect, transform, anti_alias);
    }

    fn pop_clip(&mut self) {
        self.first_clips = None;
        self.context.pop_clip();
        self.open_clips = self.open_clips.saturating_sub(1);
        if self.origin != (0, 0) && self.open_clips < self.bounding_clips.len() {
            self.unbounded = true;
        }
    }

    fn push_layer(&mut self, blend_mode: BlendMode, opacity: f32) {
        self.drawing();
        self.context.push_layer(None, Some(blend_mode), Some(opacity), None, None);
    }

    fn pop_layer(&mut self) {
        self.context.pop_layer();
    }

    fn render_to_pixels(&mut self, pixels: &mut [u8]) {
        let (width, height) = self.target_size;
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

        let settings = RasterizerSettings { target_init, offset: self.origin, ..RasterizerSettings::default() };

        let _perf = crate::perf::scope(crate::perf::Phase::CpuRender, pixels_len);
        self.rendered = match (self.origin, &self.bounded) {
            ((0, 0), _) | (_, Some(_)) => None,
            ((x0, y0), None) => Some(VelloScenePixelRect {
                x0,
                y0,
                x1: x0 + self.context.width(),
                y1: y0 + self.context.height(),
            }),
        };
        if let Some((mut bounded, offset)) = self.bounded.take() {
            bounded.flush();
            bounded.render_with(Self::pixmap(width, height, pixels), &mut self.resources, RasterizerSettings { offset, ..settings });
            RendererPool::give_back_context(bounded);
        }
        self.context.flush();
        self.context.render_with(Self::pixmap(width, height, pixels), &mut self.resources, settings);
    }

    fn filter_capabilities(&self) -> VelloSceneFilterCapabilities {
        // The renderer has no filter layers with a thread pool: an effect is
        // drawn into a scene of its own on the thread that draws, and
        // composed (`drawing_context_impl/effects.rs`).
        VelloSceneFilterCapabilities { filter_layers: self.threads == 0, blurred_rounded_rects: true }
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
        self.drawing();
        let transform = self.placed(transform);
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
        self.drawing();
        let transform = self.placed(transform);
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

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: u16 = 96;

    fn solid(red: u8, green: u8, blue: u8, alpha: u8) -> VelloScenePaint {
        VelloScenePaint::solid(peniko::Color::from_rgba8(red, green, blue, alpha))
    }

    /// The first frame of the target: shapes over all of it.
    fn first_frame() -> Vec<u8> {
        let mut pixels = vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4];
        let mut sink = VelloCpuSceneSink::new(SIZE, SIZE);
        sink.fill_rect(kurbo::Rect::new(0.0, 0.0, 96.0, 96.0), Affine::IDENTITY, &solid(200, 30, 0, 255), BlendMode::default(), true);
        let circle = kurbo::Circle::new((48.0, 48.0), 40.0).to_path(0.1);
        sink.fill(&circle, Fill::NonZero, Affine::IDENTITY, &solid(0, 40, 220, 160), BlendMode::default(), true);
        sink.render_to_pixels(&mut pixels);
        pixels
    }

    /// The second frame: the compositor's clip of the dirty rectangles, the
    /// clear of them, a translucent ellipse and a stroked path that cross
    /// their edges, glyph-free; then, with `after_clips`, the clips ended
    /// and a shape drawn over all of the target. `bounded` lets the sink
    /// render the cleared rectangles only; without it a transparent fill
    /// before the clips keeps the scene the size of the target.
    fn second_frame(pixels: &mut [u8], bounded: bool, after_clips: bool) {
        let dirty = [VelloScenePixelRect { x0: 10, y0: 12, x1: 50, y1: 40 }, VelloScenePixelRect { x0: 30, y0: 30, x1: 70, y1: 64 }];
        let mut sink = VelloCpuSceneSink::new(SIZE, SIZE);
        if !bounded {
            sink.fill_rect(kurbo::Rect::new(0.0, 0.0, 1.0, 1.0), Affine::IDENTITY, &solid(0, 0, 0, 0), BlendMode::default(), true);
        }
        sink.push_clip_rect(kurbo::Rect::new(10.0, 12.0, 70.0, 64.0), Affine::IDENTITY, false);
        let mut region = BezPath::new();
        for rect in &dirty {
            let rect = kurbo::Rect::new(f64::from(rect.x0), f64::from(rect.y0), f64::from(rect.x1), f64::from(rect.y1));
            region.extend(rect.to_path(0.1));
        }
        sink.push_clip(&region, Fill::NonZero, Affine::IDENTITY, false);
        sink.retain_target(&dirty);

        let ellipse = kurbo::Ellipse::new((40.0, 38.0), (34.0, 18.0), 0.3).to_path(0.1);
        sink.fill(&ellipse, Fill::NonZero, Affine::translate((1.5, 0.25)), &solid(20, 180, 40, 190), BlendMode::default(), true);
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.curve_to((40.0, 90.0), (60.0, -20.0), (96.0, 80.0));
        sink.stroke(&line, &Stroke::new(3.0), Affine::IDENTITY, &solid(250, 250, 0, 255), true);
        sink.pop_clip();
        sink.pop_clip();

        if after_clips {
            let bar = kurbo::Rect::new(5.0, 44.0, 91.0, 52.0);
            sink.fill_rect(bar, Affine::IDENTITY, &solid(90, 0, 120, 200), BlendMode::default(), true);
        }
        sink.render_to_pixels(pixels);
    }

    fn largest_difference(a: &[u8], b: &[u8]) -> u8 {
        a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0)
    }

    /// A kept target that is drawn again in a part is rendered there only,
    /// to the same pixels as a scene of the whole target.
    #[test]
    fn a_scene_bounded_by_its_cleared_rectangles_draws_the_pixels_of_the_whole_scene() {
        let (mut bounded, mut whole) = (first_frame(), first_frame());
        second_frame(&mut bounded, true, false);
        second_frame(&mut whole, false, false);
        assert_ne!(bounded, first_frame(), "the second frame drew nothing");
        assert_eq!(0, largest_difference(&bounded, &whole));
    }

    /// The clips that bound the scene are ended and drawing goes on over
    /// the whole target: what was drawn inside them is rendered first, the
    /// rest over it.
    #[test]
    fn a_bounded_scene_goes_on_over_the_whole_target_when_its_clips_end() {
        let (mut bounded, mut whole) = (first_frame(), first_frame());
        second_frame(&mut bounded, true, true);
        second_frame(&mut whole, false, true);
        assert_eq!(0, largest_difference(&bounded, &whole));
    }

    /// Render contexts are taken from the pool of the thread by their size
    /// and given back when a scene ends.
    #[test]
    fn a_render_context_is_used_again_by_the_next_scene_of_its_size() {
        let mut pixels = first_frame();
        for _ in 0..3 {
            second_frame(&mut pixels, true, true);
        }
        let sizes = RENDERER_POOL.with(|pool| {
            pool.borrow().contexts.iter().map(|context| (context.width(), context.height())).collect::<Vec<_>>()
        });
        assert!(sizes.contains(&(SIZE, SIZE)), "{sizes:?}");
        assert!(sizes.len() <= RendererPool::CONTEXTS, "{sizes:?}");
    }
}
