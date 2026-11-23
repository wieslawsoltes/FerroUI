//! An animated visual as a tree of composition visuals and shapes (the WinUI composition API).
//!
//! The tree is built once. Every property that changes over the animation becomes a key frame
//! animation under one paused controller, so a frame sets one value: the progress of the
//! controller. What the composition API has no key frames for is listed in [`LiveValue`] and
//! written when it changes: a path whose shape changes, the matrix of a skewed transform or of a
//! repeater copy, the stops of an animated gradient, dashes, and a text document.

use crate::content::{document_at, lines_of, TextDocument};
use crate::evaluate::{evaluate_items, gradient_paint, merged, solid_color, stroke_style, GradientKind, Paint};
use crate::model::{
    AnimatedVisual, GradientModel, Layer, LayerKind, MaskModel, MatteMode, ShapeItem, StrokeModel, Transform,
};
use crate::path::{build_geometry, PathData};
use crate::value::Animated;
use ferroui_base::animation::easings::LinearEasing;
use ferroui_base::media::imaging::{Bitmap, BitmapBlendingMode};
use ferroui_base::media::{
    Color, FlowDirection, FontFamily, FormattedText, Geometry, GradientSpreadMethod, IGradientStop, Typeface,
};
use ferroui_base::rendering::composition::animations::AnimationController;
use ferroui_base::rendering::composition::effects::{
    BlendEffect, BlendEffectMode, CanvasComposite, CompositeEffect, CompositionEffectSourceParameter,
    GraphicsEffectSource, LuminanceToAlphaEffect,
};
use ferroui_base::rendering::composition::{
    CompositionBrush, CompositionContainerShape, CompositionLinearGradientBrush, CompositionPath,
    CompositionPathGeometry, CompositionRadialGradientBrush, CompositionShape, CompositionShapeCollection,
    CompositionSolidColorBrush, CompositionSpriteShape, CompositionStrokeCap, CompositionStrokeLineJoin,
    CompositionVisual, Compositor, GeometricClip, ICompositionObjectAnimations, LoadedImageSurface,
};
use ferroui_base::utilities::Locale;
use ferroui_base::collections::FerroList;
use ferroui_base::{Matrix, Point, Ref, RelativePoint, RelativeScalar, RelativeUnit, Vector, Vector3D};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

/// How many times per frame a value is looked at when its key frames are made.
const SAMPLES_PER_FRAME: usize = 2;
/// How often an interval is halved around a change of a value, such as a hold.
const MOST_HALVINGS: u32 = 10;

/// The progress the controller is at for the first frame.
const FIRST_PROGRESS: f32 = 1e-6;

/// The frame of a timeline at a progress of the animation.
type Clock = Rc<dyn Fn(f64) -> f64>;
/// A factor at a frame of the timeline of the shapes.
type Factor = Rc<dyn Fn(f64) -> f64>;

/// The key frames of a value: progress and components. The value between two keys is linear, and
/// is within `tolerance` of `value` wherever it was looked at.
fn fit<const N: usize>(frames: f64, tolerance: f64, value: &dyn Fn(f64) -> [f64; N]) -> Vec<(f64, [f64; N])> {
    fn near<const N: usize>(a: &[f64; N], b: &[f64; N], t: f64, at: &[f64; N], tolerance: f64) -> bool {
        (0..N).all(|index| (a[index] + (b[index] - a[index]) * t - at[index]).abs() <= tolerance)
    }
    fn refine<const N: usize>(
        from: (f64, [f64; N]),
        to: (f64, [f64; N]),
        depth: u32,
        tolerance: f64,
        value: &dyn Fn(f64) -> [f64; N],
        out: &mut Vec<(f64, [f64; N])>,
    ) {
        let middle = (from.0 + to.0) * 0.5;
        let at = value(middle);
        if depth > 0 && !near(&from.1, &to.1, 0.5, &at, tolerance) {
            refine(from, (middle, at), depth - 1, tolerance, value, out);
            refine((middle, at), to, depth - 1, tolerance, value, out);
        } else {
            out.push(to);
        }
    }
    let steps = ((frames.ceil() as usize).max(1) * SAMPLES_PER_FRAME).min(8192);
    let mut keys = vec![(0.0, value(0.0))];
    for step in 1..=steps {
        let from = keys[keys.len() - 1];
        let progress = step as f64 / steps as f64;
        refine(from, (progress, value(progress)), MOST_HALVINGS, tolerance, value, &mut keys);
    }
    // A key on the line between the keys around it says nothing.
    let mut kept: Vec<(f64, [f64; N])> = Vec::with_capacity(keys.len());
    for (index, key) in keys.iter().enumerate() {
        if let (Some(before), Some(after)) = (kept.last(), keys.get(index + 1)) {
            let t = (key.0 - before.0) / (after.0 - before.0);
            if near(&before.1, &after.1, t, &key.1, tolerance * 0.25) {
                continue;
            }
        }
        kept.push(*key);
    }
    kept
}

fn is_constant<const N: usize>(keys: &[(f64, [f64; N])]) -> bool {
    keys.windows(2).all(|pair| pair[0].1 == pair[1].1)
}

fn to_color(value: [f64; 4]) -> Color {
    let channel = |v: f64| v.clamp(0.0, 255.0).round() as u8;
    Color::from_argb(channel(value[0]), channel(value[1]), channel(value[2]), channel(value[3]))
}

fn components(color: Color) -> [f64; 4] {
    [f64::from(color.a), f64::from(color.r), f64::from(color.g), f64::from(color.b)]
}

/// What is written when it changes, because the composition API has no key frames for it.
enum LiveValue {
    Path { geometry: CompositionPathGeometry, clock: Clock, items: Rc<Vec<ShapeItem>>, end: usize, even_odd: bool, last: PathData },
    MaskPath { geometry: CompositionPathGeometry, clock: Clock, masks: Vec<MaskModel>, last: PathData },
    ShapeMatrix { shape: Rc<CompositionShape>, value: Rc<dyn Fn(f64) -> Matrix>, last: Matrix },
    VisualMatrix { visual: Rc<CompositionVisual>, value: Rc<dyn Fn(f64) -> Matrix>, last: Matrix },
    ClipMatrix { clip: GeometricClip, value: Rc<dyn Fn(f64) -> Matrix>, last: Matrix },
    Gradient { brush: GradientBrush, model: GradientModel, clock: Clock, alpha: Factor, last: Option<Paint> },
    Dashes { sprite: CompositionSpriteShape, model: StrokeModel, clock: Clock, last: Vec<f32>, last_offset: f32 },
    BoundAlpha { brush: CompositionSolidColorBrush, name: Rc<str>, alpha: Rc<dyn Fn(f64) -> f64>, last: Option<Color> },
    Text { collection: Rc<CompositionShapeCollection>, documents: Rc<Vec<(f64, TextDocument)>>, clock: Clock, shown: Option<usize> },
}

#[derive(Clone)]
enum GradientBrush {
    Linear(CompositionLinearGradientBrush),
    Radial(CompositionRadialGradientBrush),
}

impl GradientBrush {
    fn brush(&self) -> Rc<CompositionBrush> {
        match self {
            Self::Linear(brush) => (***brush).clone(),
            Self::Radial(brush) => (***brush).clone(),
        }
    }
}

/// An animated visual as composition visuals and shapes.
pub struct CompositionScene {
    compositor: Rc<Compositor>,
    visual: Rc<AnimatedVisual>,
    root: Rc<CompositionVisual>,
    controller: Rc<AnimationController>,
    duration: Duration,
    frames: f64,
    live: Vec<LiveValue>,
    /// The brushes of each colour property, with the alpha each keeps when it is static.
    bound: HashMap<Rc<str>, Vec<(CompositionSolidColorBrush, f64)>>,
    colors: Rc<RefCell<HashMap<Rc<str>, Color>>>,
    animations: usize,
    progress: f64,
}

impl CompositionScene {
    /// Builds the tree of `visual`. Nothing is built again for a later frame.
    pub fn build(compositor: &Rc<Compositor>, visual: &Rc<AnimatedVisual>) -> Self {
        let root = compositor.create_container_visual();
        root.set_size(Vector::new(visual.width, visual.height));
        let controller = compositor.create_animation_controller();
        controller.pause();
        let mut scene = Self {
            compositor: compositor.clone(),
            visual: visual.clone(),
            root: root.clone(),
            controller,
            duration: Duration::from_secs_f64(visual.duration_seconds().max(0.001)),
            frames: (visual.out_point - visual.in_point).max(1.0),
            live: Vec::new(),
            bound: HashMap::new(),
            colors: Rc::new(RefCell::new(HashMap::new())),
            animations: 0,
            progress: -1.0,
        };
        let document = visual.clone();
        let clock: Clock = Rc::new(move |progress| document.frame_at(progress));
        let size = Vector::new(visual.width, visual.height);
        scene.add_layers(&visual.layers, &clock, &root.children_visual(), size, 0);
        scene.set_progress(0.0);
        scene
    }

    /// The root of the tree, of the size of the composition.
    pub fn root(&self) -> &Rc<CompositionVisual> {
        &self.root
    }

    /// How many key frame animations the tree runs under its controller.
    pub fn animation_count(&self) -> usize {
        self.animations
    }

    /// How many values are written by hand when they change.
    pub fn live_value_count(&self) -> usize {
        self.live.len()
    }

    /// Shows the frame at `progress`: the progress of the controller, and the few values the
    /// composition API has no key frames for, when they changed.
    pub fn set_progress(&mut self, progress: f64) {
        let progress = progress.clamp(0.0, 1.0);
        if progress == self.progress {
            return;
        }
        self.progress = progress;
        // At exactly 0 a key frame animation shows the value it started from, not its first key.
        self.controller.set_progress((progress as f32).max(FIRST_PROGRESS));
        let mut live = std::mem::take(&mut self.live);
        for value in &mut live {
            self.update(value, progress);
        }
        self.live = live;
    }

    /// Sends the progress again. Animations of a tree that was not on screen start from the
    /// values of their properties: the tree calls this when it is attached.
    pub fn refresh(&self) {
        self.controller.set_progress((self.progress.max(0.0) as f32).max(FIRST_PROGRESS));
    }

    /// Sets the colour of the paints bound to `name`. `None` keeps the colour they have.
    pub fn set_color(&mut self, name: &str, color: Option<Color>) {
        let Some(color) = color else {
            return;
        };
        self.colors.borrow_mut().insert(Rc::from(name), color);
        if let Some(brushes) = self.bound.get(name) {
            for (brush, alpha) in brushes {
                brush.set_color(with_alpha(color, *alpha));
            }
        }
        let progress = self.progress.max(0.0);
        let mut live = std::mem::take(&mut self.live);
        for value in &mut live {
            if let LiveValue::BoundAlpha { name: bound, last, .. } = value {
                if &**bound == name {
                    *last = None;
                }
            }
            if matches!(value, LiveValue::BoundAlpha { .. }) {
                self.update(value, progress);
            }
        }
        self.live = live;
    }

    fn update(&self, value: &mut LiveValue, progress: f64) {
        match value {
            LiveValue::Path { geometry, clock, items, end, even_odd, last } => {
                let path = path_of(&items[..*end], clock(progress));
                if path != *last {
                    set_path(geometry, &path, *even_odd);
                    *last = path;
                }
            }
            LiveValue::MaskPath { geometry, clock, masks, last } => {
                let path = added_masks(masks, clock(progress));
                if path != *last {
                    set_path(geometry, &path, false);
                    *last = path;
                }
            }
            LiveValue::ShapeMatrix { shape, value, last } => {
                let matrix = value(progress);
                if matrix != *last {
                    shape.set_transform_matrix(matrix);
                    *last = matrix;
                }
            }
            LiveValue::VisualMatrix { visual, value, last } => {
                let matrix = value(progress);
                if matrix != *last {
                    visual.set_transform_matrix(matrix);
                    *last = matrix;
                }
            }
            LiveValue::ClipMatrix { clip, value, last } => {
                let matrix = value(progress);
                if matrix != *last {
                    clip.set_transform_matrix(matrix);
                    *last = matrix;
                }
            }
            LiveValue::Gradient { brush, model, clock, alpha, last } => {
                let frame = clock(progress);
                let paint = gradient_paint(model, frame, Matrix::IDENTITY);
                let opacity = alpha(frame);
                let paint = fade(paint, opacity);
                if last.as_ref() != Some(&paint) {
                    apply_gradient(&self.compositor, brush, &paint);
                    *last = Some(paint);
                }
            }
            LiveValue::Dashes { sprite, model, clock, last, last_offset } => {
                let style = stroke_style(model, clock(progress), 1.0);
                let width = style.width.max(f64::MIN_POSITIVE);
                let dashes: Vec<f32> = style.dashes.iter().map(|dash| (dash / width) as f32).collect();
                let offset = (style.dash_offset / width) as f32;
                if dashes != *last {
                    sprite.set_stroke_dash_array(dashes.clone());
                    *last = dashes;
                }
                if offset != *last_offset {
                    sprite.set_stroke_dash_offset(offset);
                    *last_offset = offset;
                }
            }
            LiveValue::BoundAlpha { brush, name, alpha, last } => {
                let base = self.colors.borrow().get(name).copied().unwrap_or(Color::from_rgb(0, 0, 0));
                let color = with_alpha(base, alpha(progress));
                if *last != Some(color) {
                    brush.set_color(color);
                    *last = Some(color);
                }
            }
            LiveValue::Text { collection, documents, clock, shown } => {
                let frame = clock(progress);
                let index = documents.partition_point(|(time, _)| *time <= frame).saturating_sub(1);
                if *shown != Some(index) {
                    *shown = Some(index);
                    collection.clear();
                    if let Some(document) = document_at(documents, frame) {
                        self.add_text(document, frame, collection);
                    }
                }
            }
        }
    }

    /// A container that does not clip: a visual of this API clips to its size unless told not to.
    fn container(&self, size: Vector) -> Rc<CompositionVisual> {
        let visual = self.compositor.create_container_visual();
        visual.set_size(size);
        visual.set_clip_to_bounds(false);
        visual
    }

    // --- layers ---------------------------------------------------------------------------

    fn add_layers(
        &mut self,
        layers: &[Layer],
        clock: &Clock,
        parent: &ChildVisuals,
        size: Vector,
        depth: usize,
    ) {
        if depth >= crate::evaluate::MAX_DEPTH {
            return;
        }
        // The first layer of a document is on top: the last is added first.
        for (index, layer) in layers.iter().enumerate().rev() {
            if layer.is_matte {
                continue;
            }
            let Some(content) = self.layer_visual(layers, layer, clock, size, depth) else {
                continue;
            };
            let matte = layer.matte.and_then(|mode| {
                let source = match layer.matte_parent {
                    Some(id) => layers.iter().find(|candidate| candidate.index == Some(id)),
                    None => index.checked_sub(1).map(|before| &layers[before]),
                };
                Some((mode, source?))
            });
            let shown = match matte {
                Some((mode, source)) => {
                    let source = self.layer_visual(layers, source, clock, size, depth);
                    self.matted(content, source, mode, size)
                }
                None => content,
            };
            parent.add(shown);
        }
    }

    /// The visual of a layer: its time on screen and opacity, its masks, its blend mode and what
    /// it draws.
    fn layer_visual(
        &mut self,
        layers: &[Layer],
        layer: &Layer,
        clock: &Clock,
        size: Vector,
        depth: usize,
    ) -> Option<Rc<CompositionVisual>> {
        if matches!(layer.kind, LayerKind::Null | LayerKind::NotDrawn) {
            return None;
        }
        let outer = self.container(size);
        self.bind_visibility(&outer, layer, clock);

        let local = layer_clock(layer, clock);
        // The masks are in the space of the layer: a clip follows the transforms around them.
        let mut inner: Rc<CompositionVisual> = outer.clone();
        let world = world_clock(layers, layer, clock);
        for cut in self.add_masks(layer, &local, &world, size) {
            inner.children().insert_at_top(cut.clone());
            inner = cut;
        }

        let content: Option<Rc<CompositionVisual>> = match &layer.kind {
            LayerKind::Shape(items) => {
                let visual = self.compositor.create_shape_visual();
                visual.set_size(size);
                let holder = self.transform_chain(layers, layer, clock, &visual.shapes());
                let items = Rc::new(items.clone());
                self.add_items(&items, items.len(), &local, &one(), &holder);
                Some((*visual).clone())
            }
            LayerKind::Solid { color, width, height } => {
                let visual = self.compositor.create_shape_visual();
                visual.set_size(size);
                let holder = self.transform_chain(layers, layer, clock, &visual.shapes());
                let path = PathData::rectangle(Point::new(width * 0.5, height * 0.5), *width, *height, 0.0);
                let geometry = self.compositor.create_path_geometry();
                set_path(&geometry, &path, false);
                let sprite = self.compositor.create_sprite_shape_with_geometry(Some((*geometry).clone()));
                let brush = self.compositor.create_color_brush_with_color(solid_color(*color));
                sprite.set_fill_brush(Some((*brush).clone()));
                holder.append((*sprite).clone());
                Some((*visual).clone())
            }
            LayerKind::Image { reference } => self.image_visual(layers, layer, clock, reference),
            LayerKind::Text(documents) => {
                let visual = self.compositor.create_shape_visual();
                visual.set_size(size);
                let holder = self.transform_chain(layers, layer, clock, &visual.shapes());
                let container = self.compositor.create_container_shape();
                holder.append((*container).clone());
                let collection = container.shapes().clone();
                if documents.len() > 1 {
                    let mut value = LiveValue::Text {
                        collection,
                        documents: Rc::new(documents.clone()),
                        clock: local.clone(),
                        shown: None,
                    };
                    self.update(&mut value, 0.0);
                    self.live.push(value);
                } else if let Some(document) = documents.first().map(|(_, document)| document) {
                    self.add_text(document, 0.0, &collection);
                }
                Some((*visual).clone())
            }
            LayerKind::Precomposition { reference, width, height, time_remap } => {
                self.precomposition(layers, layer, clock, reference, *width, *height, time_remap.as_ref(), size, depth)
            }
            LayerKind::Null | LayerKind::NotDrawn => None,
        };
        let content = content?;
        let content = match layer.blend {
            Some(mode) => self.blended(content, mode, size),
            None => content,
        };
        inner.children().insert_at_top(content);
        Some(outer.clone())
    }

    fn precomposition(
        &mut self,
        layers: &[Layer],
        layer: &Layer,
        clock: &Clock,
        reference: &str,
        width: f64,
        height: f64,
        time_remap: Option<&Animated>,
        size: Vector,
        depth: usize,
    ) -> Option<Rc<CompositionVisual>> {
        let visual = self.visual.clone();
        let inner_layers = visual.precompositions.get(reference)?;
        let (mut outermost, mut holder) = (None::<Rc<CompositionVisual>>, None::<Rc<CompositionVisual>>);
        // One visual for each transform, the parent of the parent first.
        for owner in transform_owners(layers, layer) {
            let next = self.container(size);
            self.bind_visual_transform(&next, &owner.transform, &layer_clock(owner, clock));
            let next: Rc<CompositionVisual> = next.clone();
            match &holder {
                Some(parent) => parent.children().insert_at_top(next.clone()),
                None => outermost = Some(next.clone()),
            }
            holder = Some(next);
        }
        let holder = holder?;
        let local = layer_clock(layer, clock);
        let rate = self.visual.frame_rate;
        let inner_clock: Clock = match time_remap {
            Some(remap) => {
                let remap = remap.clone();
                Rc::new(move |progress| remap.at(local(progress))[0] * rate)
            }
            None => local,
        };
        let inner_size = if width > 0.0 && height > 0.0 { Vector::new(width, height) } else { size };
        let group = self.container(inner_size);
        group.set_clip_to_bounds(width > 0.0 && height > 0.0);
        self.add_layers(inner_layers, &inner_clock, &group.children_visual(), inner_size, depth + 1);
        holder.children().insert_at_top(group.clone());
        outermost
    }

    fn image_visual(
        &mut self,
        layers: &[Layer],
        layer: &Layer,
        clock: &Clock,
        reference: &str,
    ) -> Option<Rc<CompositionVisual>> {
        let image = self.visual.images.get(reference)?.clone();
        let bytes = image.bytes.as_ref()?;
        let bitmap = Bitmap::from_stream(&mut &bytes[..]).ok()?;
        let surface = LoadedImageSurface::from_bitmap(&self.compositor, &bitmap);
        let brush = self.compositor.create_surface_brush_with_surface(surface);
        let sprite = self.compositor.create_sprite_visual();
        sprite.set_size(Vector::new(image.width, image.height));
        sprite.set_brush(Some((*brush).clone()));
        let (mut outermost, mut holder) = (None::<Rc<CompositionVisual>>, None::<Rc<CompositionVisual>>);
        for owner in transform_owners(layers, layer) {
            let next = self.container(Vector::new(self.visual.width, self.visual.height));
            self.bind_visual_transform(&next, &owner.transform, &layer_clock(owner, clock));
            let next: Rc<CompositionVisual> = next.clone();
            match &holder {
                Some(parent) => parent.children().insert_at_top(next.clone()),
                None => outermost = Some(next.clone()),
            }
            holder = Some(next);
        }
        holder?.children().insert_at_top((*sprite).clone());
        outermost
    }

    /// The content in a layer, its matte composed into the layer by the effect of the layer. The
    /// matte is a hidden visual that a visual surface shows.
    fn matted(
        &mut self,
        content: Rc<CompositionVisual>,
        source: Option<Rc<CompositionVisual>>,
        mode: MatteMode,
        size: Vector,
    ) -> Rc<CompositionVisual> {
        let holder = self.container(size);
        let layer = self.compositor.create_layer_visual();
        layer.set_size(size);
        layer.set_clip_to_bounds(false);
        layer.children().insert_at_top(content);
        let matte_visual = source.unwrap_or_else(|| {
            self.container(size)
        });
        matte_visual.set_visible(false);
        holder.children().insert_at_top(matte_visual.clone());
        let surface = self.compositor.create_visual_surface();
        surface.set_source_visual(Some(matte_visual));
        surface.set_source_size(size);
        let brush = self.compositor.create_surface_brush_with_surface(surface);
        let parameter = || -> GraphicsEffectSource { CompositionEffectSourceParameter::new("matte").into() };
        let mask: GraphicsEffectSource = match mode {
            MatteMode::Luma | MatteMode::LumaInverted => {
                LuminanceToAlphaEffect { name: "Luminance".into(), source: Some(parameter()) }.into()
            }
            MatteMode::Alpha | MatteMode::AlphaInverted => parameter(),
        };
        let composite = match mode {
            MatteMode::Alpha | MatteMode::Luma => CanvasComposite::DestinationIn,
            MatteMode::AlphaInverted | MatteMode::LumaInverted => CanvasComposite::DestinationOut,
        };
        let factory = self.compositor.create_effect_factory(CompositeEffect {
            name: "Matte".into(),
            mode: composite,
            sources: vec![CompositionEffectSourceParameter::new("layer").into(), mask],
        });
        let effect = factory.create_brush();
        effect.set_source_parameter("matte", Some((*brush).clone()));
        layer.set_effect(Some(effect));
        holder.children().insert_at_top((*layer).clone());
        holder.clone()
    }

    /// A layer mixed with what is under it, by an effect over the backdrop.
    fn blended(&mut self, content: Rc<CompositionVisual>, mode: BitmapBlendingMode, size: Vector) -> Rc<CompositionVisual> {
        let Some(mode) = blend_effect_mode(mode) else {
            return content;
        };
        let layer = self.compositor.create_layer_visual();
        layer.set_size(size);
        layer.set_clip_to_bounds(false);
        layer.children().insert_at_top(content);
        let factory = self.compositor.create_effect_factory(BlendEffect {
            name: "Blend".into(),
            mode,
            background: Some(CompositionEffectSourceParameter::new("backdrop").into()),
            foreground: Some(CompositionEffectSourceParameter::new("layer").into()),
        });
        let effect = factory.create_brush();
        let backdrop = self.compositor.create_backdrop_brush();
        effect.set_source_parameter("backdrop", Some((*backdrop).clone()));
        layer.set_effect(Some(effect));
        (*layer).clone()
    }

    /// One visual per mask that is not an opaque union with the others: each clips what is in it.
    fn add_masks(&mut self, layer: &Layer, local: &Clock, world: &Rc<dyn Fn(f64) -> Matrix>, size: Vector) -> Vec<Rc<CompositionVisual>> {
        let added: Vec<MaskModel> =
            layer.masks.iter().filter(|mask| mask.mode == b'a' && !mask.inverted).cloned().collect();
        let mut visuals = Vec::new();
        if !added.is_empty() {
            visuals.push(self.clip_visual(ClipShape::Added(added), local, world, size));
        }
        for mask in layer.masks.iter().filter(|mask| matches!(mask.mode, b'a' | b's' | b'i')) {
            if mask.mode == b'a' && !mask.inverted {
                continue;
            }
            visuals.push(self.clip_visual(ClipShape::One(mask.clone()), local, world, size));
        }
        visuals
    }

    fn clip_visual(&mut self, shape: ClipShape, local: &Clock, world: &Rc<dyn Fn(f64) -> Matrix>, size: Vector) -> Rc<CompositionVisual> {
        let visual = self.container(size);
        let (masks, even_odd) = match &shape {
            ClipShape::Added(masks) => (masks.clone(), false),
            ClipShape::One(mask) => (vec![mask.clone()], (mask.mode == b's') != mask.inverted),
        };
        let geometry = self.compositor.create_path_geometry();
        let value = |frame: f64| -> PathData {
            let path = added_masks(&masks, frame);
            if even_odd { outside(&path) } else { path }
        };
        let path = value(local(0.0));
        set_path(&geometry, &path, even_odd);
        if masks.iter().any(|mask| mask.path.is_animated()) && !even_odd {
            self.live.push(LiveValue::MaskPath { geometry: geometry.clone(), clock: local.clone(), masks: masks.clone(), last: path });
        }
        let clip = self.compositor.create_geometric_clip_with_geometry(Some((*geometry).clone()));
        let matrix = world.clone();
        clip.set_transform_matrix(matrix(0.0));
        self.live.push(LiveValue::ClipMatrix { clip: clip.clone(), value: matrix.clone(), last: matrix(0.0) });
        visual.set_composition_clip(Some((*clip).clone()));
        visual
    }

    // --- transforms -----------------------------------------------------------------------

    /// One container shape for each transform of a layer, the parent of the parent first. What
    /// the layer draws goes into the collection this returns.
    fn transform_chain(
        &mut self,
        layers: &[Layer],
        layer: &Layer,
        clock: &Clock,
        root: &Rc<CompositionShapeCollection>,
    ) -> Rc<CompositionShapeCollection> {
        let mut holder = root.clone();
        for owner in transform_owners(layers, layer) {
            let container = self.compositor.create_container_shape();
            self.bind_shape_transform(&container, &owner.transform, &layer_clock(owner, clock));
            holder.append((*container).clone());
            holder = container.shapes().clone();
        }
        holder
    }

    fn start(&mut self, target: &dyn ICompositionObjectAnimations, property: &str, animation: &dyn ferroui_base::rendering::composition::animations::ICompositionAnimation) {
        target.start_animation_with_controller(property, animation, &self.controller);
        self.animations += 1;
    }

    fn scalar(&mut self, target: &dyn ICompositionObjectAnimations, property: &str, keys: &[(f64, [f64; 1])]) {
        let animation = self.compositor.create_scalar_key_frame_animation();
        animation.set_duration(self.duration);
        for (progress, value) in keys {
            animation.insert_key_frame_with_easing(*progress as f32, value[0] as f32, Rc::new(LinearEasing::new()));
        }
        self.start(target, property, &*animation);
    }

    fn vector(&mut self, target: &dyn ICompositionObjectAnimations, property: &str, keys: &[(f64, [f64; 2])]) {
        let animation = self.compositor.create_vector_key_frame_animation();
        animation.set_duration(self.duration);
        for (progress, value) in keys {
            animation.insert_key_frame_with_easing(*progress as f32, Vector::new(value[0], value[1]), Rc::new(LinearEasing::new()));
        }
        self.start(target, property, &*animation);
    }

    fn vector3(&mut self, target: &dyn ICompositionObjectAnimations, property: &str, keys: &[(f64, [f64; 2])], z: f64) {
        let animation = self.compositor.create_vector3d_key_frame_animation();
        animation.set_duration(self.duration);
        for (progress, value) in keys {
            animation.insert_key_frame_with_easing(*progress as f32, Vector3D::new(value[0], value[1], z), Rc::new(LinearEasing::new()));
        }
        self.start(target, property, &*animation);
    }

    fn bind_shape_transform(&mut self, shape: &CompositionContainerShape, transform: &Transform, clock: &Clock) {
        if transform.has_skew() {
            let (owner, time) = (transform.clone(), clock.clone());
            let value: Rc<dyn Fn(f64) -> Matrix> = Rc::new(move |progress| owner.matrix_at(time(progress)));
            let first = value(0.0);
            shape.set_transform_matrix(first);
            self.live.push(LiveValue::ShapeMatrix { shape: (**shape).clone(), value, last: first });
            return;
        }
        let parts = TransformFit::new(transform, clock, self.frames);
        // The first values are set too: an animation shows them only once it has been committed.
        shape.set_center_point(point_of(parts.anchor[0].1));
        shape.set_offset(point_of(parts.offset[0].1));
        shape.set_scale(point_of(parts.scale[0].1));
        shape.set_rotation_angle_in_degrees(parts.rotation[0].1[0] as f32);
        if !is_constant(&parts.anchor) {
            self.vector(&***shape, "CenterPoint", &parts.anchor);
        }
        if !is_constant(&parts.offset) {
            self.vector(&***shape, "Offset", &parts.offset);
        }
        if !is_constant(&parts.scale) {
            self.vector(&***shape, "Scale", &parts.scale);
        }
        if !is_constant(&parts.rotation) {
            self.scalar(&***shape, "RotationAngle", &in_radians(&parts.rotation));
        }
    }

    fn bind_visual_transform(&mut self, visual: &Rc<ferroui_base::rendering::composition::CompositionContainerVisual>, transform: &Transform, clock: &Clock) {
        let target: Rc<CompositionVisual> = visual.clone();
        if transform.has_skew() {
            let (owner, time) = (transform.clone(), clock.clone());
            let value: Rc<dyn Fn(f64) -> Matrix> = Rc::new(move |progress| owner.matrix_at(time(progress)));
            let first = value(0.0);
            target.set_transform_matrix(first);
            self.live.push(LiveValue::VisualMatrix { visual: target, value, last: first });
            return;
        }
        let parts = TransformFit::new(transform, clock, self.frames);
        let vector3 = |value: [f64; 2], z: f64| Vector3D::new(value[0], value[1], z);
        target.set_center_point(vector3(parts.anchor[0].1, 0.0));
        if !is_constant(&parts.anchor) {
            self.vector3(&*target, "CenterPoint", &parts.anchor, 0.0);
        }
        target.set_offset(vector3(parts.offset[0].1, 0.0));
        target.set_scale(vector3(parts.scale[0].1, 1.0));
        target.set_rotation_angle_in_degrees(parts.rotation[0].1[0] as f32);
        if !is_constant(&parts.offset) {
            self.vector3(&*target, "Offset", &parts.offset, 0.0);
        }
        if !is_constant(&parts.scale) {
            self.vector3(&*target, "Scale", &parts.scale, 1.0);
        }
        if !is_constant(&parts.rotation) {
            self.scalar(&*target, "RotationAngle", &in_radians(&parts.rotation));
        }
    }

    /// The opacity of a layer over its time on screen.
    fn bind_visibility(&mut self, visual: &Rc<ferroui_base::rendering::composition::CompositionContainerVisual>, layer: &Layer, clock: &Clock) {
        let (owner, time) = (layer.clone(), clock.clone());
        let value = move |progress: f64| -> [f64; 1] {
            let frame = time(progress);
            if frame < owner.in_point || frame >= owner.out_point {
                return [0.0];
            }
            [owner.transform.opacity_at((frame - owner.start_time) / owner.stretch)]
        };
        let keys = fit(self.frames, 0.002, &value);
        let target: Rc<CompositionVisual> = visual.clone();
        target.set_opacity(keys[0].1[0] as f32);
        if !is_constant(&keys) {
            self.scalar(&*target, "Opacity", &keys);
        }
    }

    // --- shapes ---------------------------------------------------------------------------

    /// The items before `end`, the first on top. A repeater repeats what is before it.
    fn add_items(&mut self, items: &Rc<Vec<ShapeItem>>, end: usize, clock: &Clock, alpha: &Factor, holder: &Rc<CompositionShapeCollection>) {
        let repeater = items[..end].iter().rposition(|item| matches!(item, ShapeItem::Repeater { .. }));
        let first = repeater.map_or(0, |index| index + 1);
        // What is below goes in first, so the last item of the document is at the bottom.
        for index in (first..end).rev() {
            self.add_item(items, index, clock, alpha, holder);
        }
        if let Some(index) = repeater {
            self.add_repeater(items, index, clock, alpha, holder);
        }
    }

    fn add_item(&mut self, items: &Rc<Vec<ShapeItem>>, index: usize, clock: &Clock, alpha: &Factor, holder: &Rc<CompositionShapeCollection>) {
        match &items[index] {
            ShapeItem::Group { items: inner, transform } => {
                let container = self.compositor.create_container_shape();
                self.bind_shape_transform(&container, transform, clock);
                let (own, outer) = (transform.clone(), alpha.clone());
                let chained: Factor = Rc::new(move |frame| outer(frame) * own.opacity_at(frame));
                let inner = Rc::new(inner.clone());
                self.add_items(&inner, inner.len(), clock, &chained, container.shapes());
                holder.append((*container).clone());
            }
            ShapeItem::Fill { color, opacity, even_odd, binding } => {
                let sprite = self.sprite(items, index, *even_odd, clock);
                let brush = self.solid_brush(color, opacity, binding.as_ref(), clock, alpha);
                sprite.set_fill_brush(Some(brush));
                holder.append((*sprite).clone());
            }
            ShapeItem::Stroke { color, opacity, stroke, binding } => {
                let sprite = self.sprite(items, index, false, clock);
                let brush = self.solid_brush(color, opacity, binding.as_ref(), clock, alpha);
                sprite.set_stroke_brush(Some(brush));
                self.bind_stroke(&sprite, stroke, clock);
                holder.append((*sprite).clone());
            }
            ShapeItem::GradientFill { gradient, opacity, even_odd } => {
                let sprite = self.sprite(items, index, *even_odd, clock);
                let brush = self.gradient_brush(gradient, opacity, clock, alpha);
                sprite.set_fill_brush(Some(brush));
                holder.append((*sprite).clone());
            }
            ShapeItem::GradientStroke { gradient, opacity, stroke } => {
                let sprite = self.sprite(items, index, false, clock);
                let brush = self.gradient_brush(gradient, opacity, clock, alpha);
                sprite.set_stroke_brush(Some(brush));
                self.bind_stroke(&sprite, stroke, clock);
                holder.append((*sprite).clone());
            }
            _ => {}
        }
    }

    /// The copies of what is before a repeater, each a container with its matrix.
    fn add_repeater(&mut self, items: &Rc<Vec<ShapeItem>>, index: usize, clock: &Clock, alpha: &Factor, holder: &Rc<CompositionShapeCollection>) {
        let ShapeItem::Repeater { copies, offset, above, transform, start_opacity, end_opacity } = &items[index] else {
            return;
        };
        let samples = (self.frames.ceil() as usize).max(1) * SAMPLES_PER_FRAME;
        let most = (0..=samples)
            .map(|step| copies.at(clock(step as f64 / samples as f64))[0].floor())
            .fold(0.0_f64, f64::max)
            .clamp(0.0, 4096.0) as usize;
        let order: Vec<usize> = if *above { (0..most).collect() } else { (0..most).rev().collect() };
        for copy in order {
            let container = self.compositor.create_container_shape();
            let (time, count, first, own) = (clock.clone(), copies.clone(), offset.clone(), transform.clone());
            let value: Rc<dyn Fn(f64) -> Matrix> = Rc::new(move |progress| {
                let frame = time(progress);
                if copy as f64 >= count.at(frame)[0].floor() {
                    return Matrix::create_scale(0.0, 0.0);
                }
                own.repeated_at(frame, first.at(frame)[0] + copy as f64)
            });
            let matrix = value(0.0);
            container.set_transform_matrix(matrix);
            self.live.push(LiveValue::ShapeMatrix { shape: (*container).clone(), value, last: matrix });
            let (outer, count, from, to) = (alpha.clone(), copies.clone(), start_opacity.clone(), end_opacity.clone());
            let copy_alpha: Factor = Rc::new(move |frame| {
                let total = count.at(frame)[0].floor();
                let t = if total > 1.0 { copy as f64 / (total - 1.0) } else { 0.0 };
                let (start, end) = (from.at(frame)[0] / 100.0, to.at(frame)[0] / 100.0);
                outer(frame) * (start + (end - start) * t).clamp(0.0, 1.0)
            });
            self.add_items(items, index, clock, &copy_alpha, container.shapes());
            holder.append((*container).clone());
        }
    }

    /// A sprite with the geometry gathered before the paint at `index`. A path that does not
    /// change is built once; one that changes is written when it does.
    fn sprite(&mut self, items: &Rc<Vec<ShapeItem>>, index: usize, even_odd: bool, clock: &Clock) -> CompositionSpriteShape {
        let geometry = self.compositor.create_path_geometry();
        let frame = clock(0.0);
        let path = path_of(&items[..index], frame);
        set_path(&geometry, &path, even_odd);
        if geometry_changes(&items[..index]) {
            self.live.push(LiveValue::Path {
                geometry: geometry.clone(),
                clock: clock.clone(),
                items: items.clone(),
                end: index,
                even_odd,
                last: path,
            });
        }
        self.compositor.create_sprite_shape_with_geometry(Some((*geometry).clone()))
    }

    fn solid_brush(
        &mut self,
        color: &Animated,
        opacity: &Animated,
        binding: Option<&Rc<str>>,
        clock: &Clock,
        alpha: &Factor,
    ) -> Rc<CompositionBrush> {
        let (paint, own, time, chain) = (color.clone(), opacity.clone(), clock.clone(), alpha.clone());
        let opacity_at: Rc<dyn Fn(f64) -> f64> = {
            let (own, time, chain) = (own.clone(), time.clone(), chain.clone());
            Rc::new(move |progress| {
                let frame = time(progress);
                (own.at(frame)[0] / 100.0).clamp(0.0, 1.0) * chain(frame)
            })
        };
        if let Some(name) = binding {
            let keys = fit(self.frames, 0.002, &|progress| [opacity_at(progress)]);
            let base = self.colors.borrow().get(name).copied();
            let start = base.unwrap_or_else(|| solid_color(paint.at(time(0.0))));
            let brush = self.compositor.create_color_brush_with_color(with_alpha(start, keys[0].1[0]));
            if is_constant(&keys) {
                self.bound.entry(name.clone()).or_default().push((brush.clone(), keys[0].1[0]));
            } else {
                self.colors.borrow_mut().entry(name.clone()).or_insert(start);
                self.live.push(LiveValue::BoundAlpha { brush: brush.clone(), name: name.clone(), alpha: opacity_at, last: None });
            }
            return (*brush).clone();
        }
        let value = |progress: f64| -> [f64; 4] {
            let frame = time(progress);
            let color = with_alpha(solid_color(paint.at(frame)), (own.at(frame)[0] / 100.0).clamp(0.0, 1.0) * chain(frame));
            components(color)
        };
        let keys = fit(self.frames, 0.5, &value);
        let brush = self.compositor.create_color_brush_with_color(to_color(keys[0].1));
        if !is_constant(&keys) {
            let animation = self.compositor.create_color_key_frame_animation();
            animation.set_duration(self.duration);
            for (progress, value) in &keys {
                animation.insert_key_frame_with_easing(*progress as f32, to_color(*value), Rc::new(LinearEasing::new()));
            }
            self.start(&**brush, "Color", &*animation);
        }
        (*brush).clone()
    }

    fn gradient_brush(&mut self, model: &GradientModel, opacity: &Animated, clock: &Clock, alpha: &Factor) -> Rc<CompositionBrush> {
        let brush = if model.radial {
            GradientBrush::Radial(self.compositor.create_radial_gradient_brush())
        } else {
            GradientBrush::Linear(self.compositor.create_linear_gradient_brush())
        };
        let (own, chain) = (opacity.clone(), alpha.clone());
        let factor: Factor = Rc::new(move |frame| (own.at(frame)[0] / 100.0).clamp(0.0, 1.0) * chain(frame));
        let frame = clock(0.0);
        let paint = fade(gradient_paint(model, frame, Matrix::IDENTITY), factor(frame));
        apply_gradient(&self.compositor, &brush, &paint);
        if model.is_animated() || opacity.is_animated() {
            self.live.push(LiveValue::Gradient {
                brush: brush.clone(),
                model: model.clone(),
                clock: clock.clone(),
                alpha: factor,
                last: Some(paint),
            });
        }
        brush.brush()
    }

    fn bind_stroke(&mut self, sprite: &CompositionSpriteShape, model: &StrokeModel, clock: &Clock) {
        let cap = match model.cap {
            2 => CompositionStrokeCap::Round,
            3 => CompositionStrokeCap::Square,
            _ => CompositionStrokeCap::Flat,
        };
        sprite.set_stroke_start_cap(cap);
        sprite.set_stroke_end_cap(cap);
        sprite.set_stroke_dash_cap(cap);
        sprite.set_stroke_line_join(match model.join {
            1 => CompositionStrokeLineJoin::Miter,
            3 => CompositionStrokeLineJoin::Bevel,
            _ => CompositionStrokeLineJoin::Round,
        });
        sprite.set_stroke_miter_limit(model.miter.max(1.0) as f32);
        let (width, time) = (model.width.clone(), clock.clone());
        let keys = fit(self.frames, 0.01, &|progress| [width.at(time(progress))[0]]);
        sprite.set_stroke_thickness(keys[0].1[0] as f32);
        if !is_constant(&keys) {
            self.scalar(&***sprite, "StrokeThickness", &keys);
        }
        if !model.dashes.is_empty() {
            let mut value = LiveValue::Dashes {
                sprite: sprite.clone(),
                model: model.clone(),
                clock: clock.clone(),
                last: Vec::new(),
                last_offset: 0.0,
            };
            self.update(&mut value, 0.0);
            self.live.push(value);
        }
    }

    // --- text -----------------------------------------------------------------------------

    /// A text document as shapes: from the glyphs of the document, or from the outlines of the
    /// text stack.
    fn add_text(&self, document: &TextDocument, frame: f64, collection: &Rc<CompositionShapeCollection>) {
        let fonts = &self.visual.fonts;
        let tracking = document.tracking * document.size / 1000.0;
        let mut geometries: Vec<Ref<Geometry>> = Vec::new();
        if let Some(glyphs) = fonts.glyphs_for(&document.font, &document.text) {
            let scale = document.size / 100.0;
            let mut path = PathData::default();
            for (index, line) in lines_of(&document.text).enumerate() {
                let width: f64 = line.chars().map(|character| glyphs[&character].advance * scale + tracking).sum();
                let mut x = justified(document.justify, width);
                let y = index as f64 * document.line_height - document.baseline_shift;
                for character in line.chars() {
                    let glyph = &glyphs[&character];
                    let matrix = Matrix::create_scale(scale, scale) * Matrix::create_translation(x, y);
                    let (mut parts, mut unused) = (Vec::new(), Vec::new());
                    evaluate_items(&glyph.shapes, frame, matrix, 1.0, &mut parts, &mut unused);
                    for part in &parts {
                        path.append(part);
                    }
                    x += glyph.advance * scale + tracking;
                }
            }
            if !path.is_empty() {
                geometries.push(build_geometry(&path, false, true));
            }
        } else if document.size > 0.0 {
            let family = FontFamily::parse(fonts.family_of(&document.font)).or_else(|_| FontFamily::parse("Default"));
            if let Ok(family) = family {
                let typeface = Typeface::new(family);
                for (index, line) in lines_of(&document.text).enumerate() {
                    let parts: Vec<String> =
                        if tracking != 0.0 { line.chars().map(String::from).collect() } else { vec![line.to_owned()] };
                    let placed: Vec<(FormattedText, f64)> = parts
                        .iter()
                        .map(|part| {
                            let text = FormattedText::new(part, Locale::UNKNOWN, FlowDirection::LeftToRight, typeface.clone(), document.size, None);
                            let advance = text.width_including_trailing_whitespace() + tracking;
                            (text, advance)
                        })
                        .collect();
                    let width: f64 = placed.iter().map(|(_, advance)| advance).sum();
                    let mut x = justified(document.justify, width);
                    let y = index as f64 * document.line_height - document.baseline_shift;
                    for (text, advance) in &placed {
                        if let Some(geometry) = text.build_geometry(Point::new(x, y - text.baseline())) {
                            geometries.push(geometry);
                        }
                        x += advance;
                    }
                }
            }
        }
        for geometry in geometries {
            let Some(path) = CompositionPath::from_geometry(&geometry) else {
                continue;
            };
            let shape = self.compositor.create_path_geometry_with_path(Some(path));
            let sprite = self.compositor.create_sprite_shape_with_geometry(Some((*shape).clone()));
            if let Some(fill) = document.fill {
                let brush = self.compositor.create_color_brush_with_color(with_alpha(solid_color(fill), fill[3]));
                sprite.set_fill_brush(Some((*brush).clone()));
            }
            if let (Some(stroke), true) = (document.stroke, document.stroke_width > 0.0) {
                let brush = self.compositor.create_color_brush_with_color(with_alpha(solid_color(stroke), stroke[3]));
                sprite.set_stroke_brush(Some((*brush).clone()));
                sprite.set_stroke_thickness(document.stroke_width as f32);
            }
            collection.append((*sprite).clone());
        }
    }
}

enum ClipShape {
    Added(Vec<MaskModel>),
    One(MaskModel),
}

/// A collection of child visuals, whatever visual holds it.
type ChildVisuals = Rc<ferroui_base::rendering::composition::CompositionVisualCollection>;

trait ChildVisualsExt {
    fn add(&self, visual: Rc<CompositionVisual>);
}

impl ChildVisualsExt for ChildVisuals {
    fn add(&self, visual: Rc<CompositionVisual>) {
        self.insert_at_top(visual);
    }
}

trait ContainerChildren {
    fn children_visual(&self) -> ChildVisuals;
}

impl ContainerChildren for Rc<ferroui_base::rendering::composition::CompositionContainerVisual> {
    fn children_visual(&self) -> ChildVisuals {
        self.children().clone()
    }
}

/// The values of a transform as key frames.
struct TransformFit {
    anchor: Vec<(f64, [f64; 2])>,
    offset: Vec<(f64, [f64; 2])>,
    scale: Vec<(f64, [f64; 2])>,
    rotation: Vec<(f64, [f64; 1])>,
}

impl TransformFit {
    fn new(transform: &Transform, clock: &Clock, frames: f64) -> Self {
        let parts = |progress: f64| transform.parts_at(clock(progress));
        Self {
            anchor: fit(frames, 0.005, &|progress| parts(progress).anchor),
            offset: fit(frames, 0.005, &|progress| {
                let at = parts(progress);
                [at.position[0] - at.anchor[0], at.position[1] - at.anchor[1]]
            }),
            scale: fit(frames, 0.0005, &|progress| parts(progress).scale),
            rotation: fit(frames, 0.02, &|progress| [parts(progress).rotation]),
        }
    }
}

fn in_radians(keys: &[(f64, [f64; 1])]) -> Vec<(f64, [f64; 1])> {
    keys.iter().map(|(progress, value)| (*progress, [value[0].to_radians()])).collect()
}

fn point_of(value: [f64; 2]) -> Vector {
    Vector::new(value[0], value[1])
}

fn one() -> Factor {
    Rc::new(|_| 1.0)
}

fn with_alpha(color: Color, alpha: f64) -> Color {
    let value = (f64::from(color.a) * alpha.clamp(0.0, 1.0)).round() as u8;
    Color::from_argb(value, color.r, color.g, color.b)
}

fn justified(justify: u8, width: f64) -> f64 {
    match justify {
        1 => -width,
        2 => -width * 0.5,
        _ => 0.0,
    }
}

/// The frame in the time of `layer`.
fn layer_clock(layer: &Layer, clock: &Clock) -> Clock {
    let (start, stretch, clock) = (layer.start_time, layer.stretch, clock.clone());
    Rc::new(move |progress| (clock(progress) - start) / stretch)
}

/// `layer` and the layers it is parented to, the outermost first.
fn transform_owners<'a>(layers: &'a [Layer], layer: &'a Layer) -> Vec<&'a Layer> {
    let mut owners = vec![layer];
    let mut parent = layer.parent;
    for _ in 0..layers.len() {
        let Some(id) = parent else {
            break;
        };
        let Some(next) = layers.iter().find(|candidate| candidate.index == Some(id)) else {
            break;
        };
        owners.push(next);
        parent = next.parent;
    }
    owners.reverse();
    owners
}

/// The matrix of `layer` with its parents, over the progress.
fn world_clock(layers: &[Layer], layer: &Layer, clock: &Clock) -> Rc<dyn Fn(f64) -> Matrix> {
    let owners: Vec<(Transform, Clock)> = transform_owners(layers, layer)
        .into_iter()
        .map(|owner| (owner.transform.clone(), layer_clock(owner, clock)))
        .collect();
    Rc::new(move |progress| {
        // The outermost is applied last.
        owners.iter().rev().fold(Matrix::IDENTITY, |matrix, (transform, time)| matrix * transform.matrix_at(time(progress)))
    })
}

fn path_of(items: &[ShapeItem], frame: f64) -> PathData {
    let (mut geometry, mut unused) = (Vec::new(), Vec::new());
    evaluate_items(items, frame, Matrix::IDENTITY, 1.0, &mut geometry, &mut unused);
    merged(&geometry)
}

fn set_path(geometry: &CompositionPathGeometry, path: &PathData, even_odd: bool) {
    let built = build_geometry(path, even_odd, true);
    geometry.set_path(CompositionPath::from_geometry(&built));
}

fn added_masks(masks: &[MaskModel], frame: f64) -> PathData {
    let mut path = PathData::default();
    for mask in masks {
        path.append(&mask.path.path_at(frame));
    }
    path
}

/// Everything but `path`, for an even-odd fill.
fn outside(path: &PathData) -> PathData {
    let mut all = PathData::rectangle(Point::new(0.0, 0.0), 2_000_000.0, 2_000_000.0, 0.0);
    all.append(path);
    all
}

/// Whether the geometry gathered from `items` changes over the animation.
fn geometry_changes(items: &[ShapeItem]) -> bool {
    items.iter().any(|item| match item {
        ShapeItem::Group { items, transform } => geometry_changes(items) || transform.is_animated(),
        ShapeItem::Rectangle { position, size, radius } => position.is_animated() || size.is_animated() || radius.is_animated(),
        ShapeItem::Ellipse { position, size } => position.is_animated() || size.is_animated(),
        ShapeItem::Path(shape) => shape.is_animated(),
        ShapeItem::Polystar { position, points, rotation, outer_radius, outer_roundness, inner_radius, inner_roundness, .. } => {
            [position, points, rotation, outer_radius, outer_roundness, inner_radius, inner_roundness]
                .iter()
                .any(|value| value.is_animated())
        }
        ShapeItem::Trim { start, end, offset } => start.is_animated() || end.is_animated() || offset.is_animated(),
        ShapeItem::Repeater { .. } => true,
        _ => false,
    })
}

fn fade(paint: Paint, opacity: f64) -> Paint {
    match paint {
        Paint::Solid(color) => Paint::Solid(with_alpha(color, opacity)),
        Paint::Gradient { kind, start, end, stops } => Paint::Gradient {
            kind,
            start,
            end,
            stops: stops.into_iter().map(|(offset, color)| (offset, with_alpha(color, opacity))).collect(),
        },
    }
}

fn apply_gradient(compositor: &Rc<Compositor>, brush: &GradientBrush, paint: &Paint) {
    let Paint::Gradient { kind, start, end, stops } = paint else {
        return;
    };
    let absolute = |point: Point| RelativePoint::new(point.x, point.y, RelativeUnit::Absolute);
    let list: FerroList<Rc<dyn IGradientStop>> = FerroList::new();
    for (offset, color) in stops {
        list.add(compositor.create_gradient_stop_with(*offset, *color));
    }
    match (brush, kind) {
        (GradientBrush::Linear(brush), GradientKind::Linear) => {
            brush.set_start_point(absolute(*start));
            brush.set_end_point(absolute(*end));
            brush.set_gradient_stops(list);
            brush.set_spread_method(GradientSpreadMethod::Pad);
        }
        (GradientBrush::Radial(brush), GradientKind::Radial) => {
            let radius = RelativeScalar::new((end.x - start.x).hypot(end.y - start.y), RelativeUnit::Absolute);
            brush.set_center(absolute(*start));
            brush.set_gradient_origin(absolute(*start));
            brush.set_radius_x(radius);
            brush.set_radius_y(radius);
            brush.set_gradient_stops(list);
            brush.set_spread_method(GradientSpreadMethod::Pad);
        }
        _ => {}
    }
}

fn blend_effect_mode(mode: BitmapBlendingMode) -> Option<BlendEffectMode> {
    Some(match mode {
        BitmapBlendingMode::Multiply => BlendEffectMode::Multiply,
        BitmapBlendingMode::Screen => BlendEffectMode::Screen,
        BitmapBlendingMode::Darken => BlendEffectMode::Darken,
        BitmapBlendingMode::Lighten => BlendEffectMode::Lighten,
        BitmapBlendingMode::Overlay => BlendEffectMode::Overlay,
        BitmapBlendingMode::ColorDodge => BlendEffectMode::ColorDodge,
        BitmapBlendingMode::ColorBurn => BlendEffectMode::ColorBurn,
        BitmapBlendingMode::HardLight => BlendEffectMode::HardLight,
        BitmapBlendingMode::SoftLight => BlendEffectMode::SoftLight,
        BitmapBlendingMode::Difference => BlendEffectMode::Difference,
        BitmapBlendingMode::Exclusion => BlendEffectMode::Exclusion,
        BitmapBlendingMode::Hue => BlendEffectMode::Hue,
        BitmapBlendingMode::Saturation => BlendEffectMode::Saturation,
        BitmapBlendingMode::Color => BlendEffectMode::Color,
        BitmapBlendingMode::Luminosity => BlendEffectMode::Luminosity,
        _ => return None,
    })
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
