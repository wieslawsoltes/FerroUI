//! The server-side counterparts of the WinUI effect and backdrop brushes:
//! the drawing of an effect graph over its sources, as a brush and as the
//! effect of a layer visual.

use super::{paint_composition_brush, ServerCompositionSurfaceBrush};
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::imaging::BitmapBlendingMode;
use crate::media::{BoxShadows, Color, ColorMatrix, ColorMatrixAlphaMode, EffectExtensions, IEffect, ImmutableBlurEffect};
use crate::platform::{AlphaFormat, IDrawingContextImpl, PixelFormat, PixelFormats};
use crate::rendering::composition::effects::{
    blur_amount_to_radius, BlendEffectMode, EffectColor, EffectGraph, EffectMatrix, EffectNode, EffectNodeKind,
    EffectScalar, PixelOp,
};
use crate::rendering::composition::generated::{ServerCompositionBrushHooks, ServerCompositionBrushProps};
use crate::rendering::composition::server::{
    impl_server_render_resource, IAnimatedServerObject, IServerObject, IServerRenderResource,
    IServerRenderResourceObserver, ServerCompositionPropertySet, ServerCompositor, ServerObjectId,
    ServerRenderResource,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Matrix, PixelSize, Rect, RoundedRect, Thickness, Vector};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// The changes of an effect brush that one batch carries: the property set
/// of the brush (with the first batch) and the brushes bound to its source
/// parameters (when they changed).
pub(crate) struct EffectBrushChanges {
    pub(crate) properties: Option<ServerObjectId>,
    pub(crate) sources: Option<Vec<(String, Option<ServerObjectId>)>>,
}

/// Which side of the composed layer of a layer visual is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LayerPhase {
    /// Before the children: what is under the layer, and what wraps it is
    /// pushed.
    Before,
    /// After the children: what wraps the layer is popped, and what is over
    /// it is drawn.
    After,
}

fn is_empty(rect: &Rect) -> bool {
    rect.width <= 0.0 || rect.height <= 0.0
}

/// The server-side counterpart of a
/// [`CompositionEffectBrush`](super::CompositionEffectBrush), painted by
/// the visuals that show it ([`paint_composition_brush`]) and applied by
/// the layer visuals whose effect it is.
///
/// The animatable properties of the graph are read from the property set
/// of the brush (its `Properties`), whose changes invalidate the brush.
pub struct ServerCompositionEffectBrush {
    this: Weak<ServerCompositionEffectBrush>,
    base: ServerRenderResource,
    props: ServerCompositionBrushProps,
    graph: Arc<EffectGraph>,
    properties: RefCell<Option<Rc<ServerCompositionPropertySet>>>,
    sources: RefCell<Vec<(String, Option<Rc<dyn IServerObject>>)>>,
    /// Set while the brush paints: a source that contains the brush is not
    /// painted inside itself.
    painting: Cell<bool>,
}

impl ServerCompositionEffectBrush {
    /// Creates the brush for the graph of its factory, active: the UI-thread
    /// brush activates its server object when it is created.
    pub(crate) fn new(compositor: &Rc<ServerCompositor>, graph: Arc<EffectGraph>) -> Rc<ServerCompositionEffectBrush> {
        let brush = Rc::new_cyclic(|this: &Weak<ServerCompositionEffectBrush>| ServerCompositionEffectBrush {
            this: this.clone(),
            base: ServerRenderResource::new(
                compositor,
                this.clone() as Weak<dyn IAnimatedServerObject>,
                this.clone() as Weak<dyn IServerRenderResource>,
            ),
            props: ServerCompositionBrushProps::new(),
            graph,
            properties: RefCell::new(None),
            sources: RefCell::new(Vec::new()),
            painting: Cell::new(false),
        });
        brush.base.object().activate();
        brush
    }

    /// The properties of the generated `ServerCompositionBrush`.
    pub fn brush_props(&self) -> &ServerCompositionBrushProps {
        &self.props
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// Whether the factory of the brush loaded its effect (otherwise the
    /// brush draws nothing).
    pub fn is_loaded(&self) -> bool {
        self.graph.is_loaded()
    }

    /// The property set of the brush, once the first batch arrived.
    pub fn properties(&self) -> Option<Rc<ServerCompositionPropertySet>> {
        self.properties.borrow().clone()
    }

    /// The brush bound to the source parameter `name`.
    pub fn source(&self, name: &str) -> Option<Rc<dyn IServerObject>> {
        self.sources.borrow().iter().find(|(existing, _)| existing == name).and_then(|(_, source)| source.clone())
    }

    fn subscription_key(&self) -> usize {
        self.this.as_ptr() as *const () as usize
    }

    fn observer(&self) -> Option<Rc<dyn IServerRenderResourceObserver>> {
        self.this.upgrade().map(|this| this as Rc<dyn IServerRenderResourceObserver>)
    }

    /// Follows the changes of the entries of the property set: they are the
    /// animatable properties of the graph.
    fn set_properties(&self, properties: Option<Rc<ServerCompositionPropertySet>>) {
        if let Some(old) = self.properties.borrow().as_ref() {
            old.changed().remove(self.subscription_key());
        }
        if let Some(new) = &properties {
            let weak = self.this.clone();
            new.changed().add(
                self.subscription_key(),
                Rc::new(move || {
                    if let Some(brush) = weak.upgrade() {
                        brush.base.core().invalidated(&*brush);
                    }
                }),
            );
        }
        *self.properties.borrow_mut() = properties;
    }

    /// Whether `source` is this brush.
    fn is_self(&self, source: &Rc<dyn IServerObject>) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(source), self.this.as_ptr())
    }

    /// Binds the source parameters, observing the render resources among the
    /// bound brushes (a change of a source redraws the effect).
    fn set_sources(&self, sources: Vec<(String, Option<Rc<dyn IServerObject>>)>) {
        let old = std::mem::replace(&mut *self.sources.borrow_mut(), sources);
        let Some(observer) = self.observer() else { return };
        let resources = |sources: &[(String, Option<Rc<dyn IServerObject>>)]| -> Vec<Rc<dyn IServerRenderResource>> {
            sources
                .iter()
                .filter_map(|(_, source)| source.clone())
                .filter(|source| !self.is_self(source))
                .filter_map(|source| source.as_render_resource())
                .collect()
        };
        for resource in resources(old.as_slice()) {
            resource.remove_observer(&observer);
        }
        let new = resources(self.sources.borrow().as_slice());
        for resource in new {
            resource.add_observer(&observer);
        }
    }

    /// The current value of a scalar member: the entry of the property set
    /// for an animatable member, the value of the description otherwise.
    fn scalar(&self, value: &EffectScalar) -> f64 {
        if let (Some(key), Some(properties)) = (&value.key, self.properties.borrow().as_ref()) {
            if let Some(current) = properties.get_value(key).try_cast::<f64>() {
                return current;
            }
        }
        value.value as f64
    }

    /// The current value of a colour member; see [`scalar`](Self::scalar).
    fn color(&self, value: &EffectColor) -> Color {
        if let (Some(key), Some(properties)) = (&value.key, self.properties.borrow().as_ref()) {
            if let Some(current) = properties.get_value(key).try_cast::<Color>() {
                return current;
            }
        }
        value.value
    }

    /// The blur radius of the drawing contexts for a blur node, 0 for none.
    fn blur_radius(&self, amount: &EffectScalar) -> f64 {
        blur_amount_to_radius(self.scalar(amount))
    }

    /// A weight or opacity, clamped to 0..1.
    fn unit(&self, value: &EffectScalar) -> f64 {
        let value = self.scalar(value);
        if value.is_nan() {
            return 0.0;
        }
        value.clamp(0.0, 1.0)
    }

    /// Pushes a blur of `radius` when the canvas has effects; returns
    /// whether it was pushed (the matching `pop_blur` is called with the
    /// same answer).
    fn push_blur(canvas: &mut dyn IDrawingContextImpl, rect: Rect, radius: f64) -> bool {
        if radius <= 0.0 {
            return false;
        }
        match canvas.as_drawing_context_impl_with_effects() {
            Some(effects) => {
                effects.push_effect(Some(rect), &ImmutableBlurEffect::new(radius));
                true
            }
            // Deviation (winui-composition.md): a drawing context without
            // effects draws the source unblurred.
            None => false,
        }
    }

    fn pop_blur(canvas: &mut dyn IDrawingContextImpl, pushed: bool) {
        if pushed {
            if let Some(effects) = canvas.as_drawing_context_impl_with_effects() {
                effects.pop_effect();
            }
        }
    }

    /// Whether a blur of `radius` is pushed on `canvas` (the same answer as
    /// `push_blur` gives).
    fn blur_is_pushed(canvas: &mut dyn IDrawingContextImpl, radius: f64) -> bool {
        radius > 0.0 && canvas.as_drawing_context_impl_with_effects().is_some()
    }

    /// Paints `node` over `rect` with `opacity` (nothing at 0).
    fn paint_node_with_opacity(&self, node: &EffectNode, canvas: &mut dyn IDrawingContextImpl, rect: Rect, opacity: f64) {
        if opacity <= 0.0 {
            return;
        }
        if opacity < 1.0 {
            canvas.push_opacity(opacity, Some(rect));
        }
        self.paint_node(node, canvas, rect);
        if opacity < 1.0 {
            canvas.pop_opacity();
        }
    }

    /// The reach of a blur of `radius` beyond its area: how far a blur that
    /// reads its source beyond the area (the `Hard` border mode) reads it.
    fn blur_reach(radius: f64) -> f64 {
        if radius <= 0.0 {
            return 0.0;
        }
        let blur = ImmutableBlurEffect::new(radius);
        let padding = EffectExtensions::get_effect_output_padding(Some(&blur as &dyn IEffect));
        padding.left.max(padding.top).max(padding.right).max(padding.bottom)
    }

    /// The current matrix of a colour matrix node.
    fn matrix(&self, matrix: &EffectMatrix) -> ColorMatrix {
        match matrix {
            EffectMatrix::Fixed(matrix) => *matrix,
            EffectMatrix::Saturation(saturation) => ColorMatrix::saturation(self.scalar(saturation) as f32),
            EffectMatrix::Tint(color) => {
                let color = self.color(color);
                ColorMatrix::tint([color.r, color.g, color.b, color.a].map(|channel| channel as f32 / 255.0))
            }
        }
    }

    /// The colour of a colour source node (premultiplied channels, 0 to 1).
    fn node_color(&self, node: &EffectNode) -> [f32; 4] {
        let EffectNodeKind::ColorSource(color) = &node.kind else { return [0.0; 4] };
        let color = self.color(color);
        let alpha = color.a as f32 / 255.0;
        [color.r as f32 / 255.0 * alpha, color.g as f32 / 255.0 * alpha, color.b as f32 / 255.0 * alpha, alpha]
    }

    /// The colour matrix (on premultiplied colours) an arithmetic composite
    /// of an image and a colour is: `image * (k1 c + k_image) + (k_color c + k4)`.
    fn arithmetic_matrix(
        &self,
        multiply: &EffectScalar,
        source1: &EffectScalar,
        source2: &EffectScalar,
        offset: &EffectScalar,
        image_is_source1: bool,
        color: &EffectNode,
    ) -> ColorMatrix {
        let c = self.node_color(color);
        let k1 = self.scalar(multiply) as f32;
        let (k_image, k_color) = if image_is_source1 {
            (self.scalar(source1) as f32, self.scalar(source2) as f32)
        } else {
            (self.scalar(source2) as f32, self.scalar(source1) as f32)
        };
        let k4 = self.scalar(offset) as f32;
        ColorMatrix::scale_and_offset(c.map(|c| k1 * c + k_image), c.map(|c| k_color * c + k4))
    }

    /// Pushes a blend layer when the canvas has them; returns whether it did.
    fn push_blend(canvas: &mut dyn IDrawingContextImpl, rect: Rect, mode: BitmapBlendingMode) -> bool {
        match canvas.as_drawing_context_impl_with_layer_effects() {
            Some(layers) => {
                layers.push_blend_layer(rect, mode);
                true
            }
            // Deviation (winui-materials.md): without the layers of the
            // drawing context the foreground is drawn source-over.
            None => false,
        }
    }

    fn pop_blend(canvas: &mut dyn IDrawingContextImpl, pushed: bool) {
        if pushed {
            if let Some(layers) = canvas.as_drawing_context_impl_with_layer_effects() {
                layers.pop_blend_layer();
            }
        }
    }

    /// Pushes a colour matrix layer when the canvas has them and the matrix
    /// changes something; returns whether it did.
    fn push_matrix(canvas: &mut dyn IDrawingContextImpl, rect: Rect, matrix: &ColorMatrix, alpha_mode: ColorMatrixAlphaMode) -> bool {
        if matrix.is_identity() {
            return false;
        }
        match canvas.as_drawing_context_impl_with_layer_effects() {
            Some(layers) => {
                layers.push_color_matrix_layer(rect, matrix, alpha_mode);
                true
            }
            // Deviation (winui-materials.md): without the layers of the
            // drawing context the source is drawn unchanged.
            None => false,
        }
    }

    fn pop_matrix(canvas: &mut dyn IDrawingContextImpl, pushed: bool) {
        if pushed {
            if let Some(layers) = canvas.as_drawing_context_impl_with_layer_effects() {
                layers.pop_color_matrix_layer();
            }
        }
    }

    /// Paints `node` over `rect`.
    fn paint_node(&self, node: &EffectNode, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        match &node.kind {
            EffectNodeKind::Parameter(name) => {
                if let Some(source) = self.source(name).filter(|source| !self.is_self(source)) {
                    paint_composition_brush(canvas, &source, rect);
                }
            }
            EffectNodeKind::GaussianBlur { amount, hard, source } => {
                let radius = self.blur_radius(amount);
                if *hard && Self::blur_is_pushed(canvas, radius) {
                    // The `Hard` border mode: the blur reads its source beyond
                    // the area, so its edges do not fade; its output is cut
                    // to the area.
                    let reach = Self::blur_reach(radius);
                    let wide = rect.inflate(reach);
                    canvas.push_clip(rect);
                    let pushed = Self::push_blur(canvas, wide, radius);
                    self.paint_node(source, canvas, wide);
                    Self::pop_blur(canvas, pushed);
                    canvas.pop_clip();
                } else {
                    let pushed = Self::push_blur(canvas, rect, radius);
                    self.paint_node(source, canvas, rect);
                    Self::pop_blur(canvas, pushed);
                }
            }
            EffectNodeKind::Opacity { opacity, source } => {
                self.paint_node_with_opacity(source, canvas, rect, self.unit(opacity));
            }
            EffectNodeKind::CrossFade { weight, source1, source2 } => {
                // `source1 * (1 - weight) + source2 * weight` (premultiplied):
                // the second source is added to the first in a layer of their
                // own. Without blend layers the second is drawn over the first.
                let weight = self.unit(weight);
                if weight <= 0.0 || weight >= 1.0 {
                    self.paint_node_with_opacity(source1, canvas, rect, 1.0 - weight);
                    self.paint_node_with_opacity(source2, canvas, rect, weight);
                    return;
                }
                canvas.push_layer(rect);
                self.paint_node_with_opacity(source1, canvas, rect, 1.0 - weight);
                let pushed = Self::push_blend(canvas, rect, BitmapBlendingMode::Plus);
                self.paint_node_with_opacity(source2, canvas, rect, weight);
                Self::pop_blend(canvas, pushed);
                canvas.pop_layer();
            }
            EffectNodeKind::Composite { mode, sources } => {
                if *mode == BitmapBlendingMode::SourceOver {
                    for source in sources {
                        self.paint_node(source, canvas, rect);
                    }
                    return;
                }
                // Each source composed onto what the sources before it made,
                // in a layer of their own.
                canvas.push_layer(rect);
                for (index, source) in sources.iter().enumerate() {
                    let pushed = index > 0 && Self::push_blend(canvas, rect, *mode);
                    self.paint_node(source, canvas, rect);
                    Self::pop_blend(canvas, pushed);
                }
                canvas.pop_layer();
            }
            EffectNodeKind::ColorSource(color) => {
                let color = self.color(color);
                if color.a > 0 {
                    let brush = ImmutableSolidColorBrush::new(color);
                    canvas.draw_rectangle(Some(&brush), None, RoundedRect::from_rect(rect), &BoxShadows::default());
                }
            }
            EffectNodeKind::Blend { mode, background, foreground } => {
                // The background and the foreground mixed in a layer of their
                // own, which is drawn source-over.
                canvas.push_layer(rect);
                self.paint_node(background, canvas, rect);
                let pushed = Self::push_blend(canvas, rect, *mode);
                self.paint_node(foreground, canvas, rect);
                Self::pop_blend(canvas, pushed);
                canvas.pop_layer();
            }
            EffectNodeKind::ColorMatrix { matrix, alpha_mode, source } => {
                let matrix = self.matrix(matrix);
                let pushed = Self::push_matrix(canvas, rect, &matrix, *alpha_mode);
                self.paint_node(source, canvas, rect);
                Self::pop_matrix(canvas, pushed);
            }
            EffectNodeKind::AlphaMask { source, mask } => {
                canvas.push_layer(rect);
                self.paint_node(source, canvas, rect);
                let pushed = Self::push_blend(canvas, rect, BitmapBlendingMode::DestinationIn);
                if pushed {
                    self.paint_node(mask, canvas, rect);
                }
                Self::pop_blend(canvas, pushed);
                canvas.pop_layer();
            }
            EffectNodeKind::Border { tile, source } => {
                let surface = match &source.kind {
                    EffectNodeKind::Parameter(name) if *tile => self.source(name),
                    _ => None,
                };
                match surface.as_ref().and_then(|surface| surface.as_any().downcast_ref::<ServerCompositionSurfaceBrush>()) {
                    Some(surface) => surface.paint_tiled(canvas, rect),
                    None => self.paint_node(source, canvas, rect),
                }
            }
            EffectNodeKind::ArithmeticWithColor { multiply, source1, source2, offset, image_is_source1, image, color } => {
                let matrix = self.arithmetic_matrix(multiply, source1, source2, offset, *image_is_source1, color);
                let pushed = Self::push_matrix(canvas, rect, &matrix, ColorMatrixAlphaMode::Premultiplied);
                self.paint_node(image, canvas, rect);
                Self::pop_matrix(canvas, pushed);
            }
            EffectNodeKind::Shadow { amount, color, source } => {
                let radius = self.blur_radius(amount);
                let matrix = shadow_matrix(self.color(color));
                let blurred = Self::push_blur(canvas, rect, radius);
                let colored = Self::push_matrix(canvas, rect, &matrix, ColorMatrixAlphaMode::Straight);
                self.paint_node(source, canvas, rect);
                Self::pop_matrix(canvas, colored);
                Self::pop_blur(canvas, blurred);
            }
            EffectNodeKind::Pixels { op, sources } => self.paint_pixels(op, sources, canvas, rect),
            EffectNodeKind::SceneLighting { ambient, diffuse, specular, shine } => {
                let material = crate::rendering::composition::server::LightingMaterial {
                    ambient: self.scalar(ambient),
                    diffuse: self.scalar(diffuse),
                    specular: self.scalar(specular),
                    shine: self.scalar(shine),
                };
                crate::rendering::composition::server::paint_scene_lighting(canvas, rect, &material);
            }
        }
    }

    /// Draws `node` over `rect` into a bitmap of its own at the resolution of
    /// `canvas` and returns its premultiplied RGBA pixels, or `None` when the
    /// compositor has no render interface.
    fn render_node_pixels(&self, node: &EffectNode, canvas: &dyn IDrawingContextImpl, rect: Rect, size: PixelSize) -> Option<Vec<u8>> {
        let compositor = self.base.compositor()?;
        let transform = Matrix::create_translation(-rect.x, -rect.y)
            * Matrix::create_scale(size.width as f64 / rect.width, size.height as f64 / rect.height);
        let _ = canvas;
        compositor.render_interface().with_platform_render_interface(|render_interface| {
            let bitmap = render_interface.create_render_target_bitmap(size, Vector::new(96.0, 96.0));
            {
                let mut context = bitmap.create_drawing_context();
                context.set_transform(transform);
                self.paint_node(node, &mut *context, rect);
                context.dispose();
            }
            let framebuffer = bitmap.lock();
            let (width, height) = (size.width as usize, size.height as usize);
            let row_bytes = framebuffer.row_bytes() as usize;
            let bgra = framebuffer.format() == PixelFormat::BGRA8888;
            let premultiplied = framebuffer.alpha_format() != AlphaFormat::Unpremul;
            let mut rgba = vec![0u8; width * height * 4];
            framebuffer.with_data(&mut |data| {
                for y in 0..height {
                    for x in 0..width {
                        let at = y * row_bytes + x * 4;
                        let Some(pixel) = data.get(at..at + 4) else { continue };
                        let (mut r, mut g, mut b, a) =
                            if bgra { (pixel[2], pixel[1], pixel[0], pixel[3]) } else { (pixel[0], pixel[1], pixel[2], pixel[3]) };
                        if !premultiplied {
                            let scale = |c: u8| ((u32::from(c) * u32::from(a) + 127) / 255) as u8;
                            (r, g, b) = (scale(r), scale(g), scale(b));
                        }
                        rgba[(y * width + x) * 4..(y * width + x) * 4 + 4].copy_from_slice(&[r, g, b, a]);
                    }
                }
            });
            framebuffer.dispose();
            rgba
        })
    }

    /// Paints an operation on the pixels of `sources`: each source is drawn
    /// apart into a bitmap at the resolution of `canvas` (at most 4096
    /// pixels a side), the pixels are combined, and the result is drawn over
    /// `rect`. Nothing is drawn without a render interface.
    fn paint_pixels(&self, op: &PixelOp, sources: &[EffectNode], canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        if sources.is_empty() || is_empty(&rect) {
            return;
        }
        let transform = canvas.transform();
        let scale_x = (transform.m11 * transform.m11 + transform.m12 * transform.m12).sqrt().max(1e-6);
        let scale_y = (transform.m21 * transform.m21 + transform.m22 * transform.m22).sqrt().max(1e-6);
        let size = PixelSize::new(
            ((rect.width * scale_x).ceil() as i32).clamp(1, 4096),
            ((rect.height * scale_y).ceil() as i32).clamp(1, 4096),
        );
        let mut images = Vec::with_capacity(sources.len());
        for source in sources {
            match self.render_node_pixels(source, canvas, rect, size) {
                Some(pixels) => images.push(pixels),
                None => return,
            }
        }
        let mut images = images.into_iter();
        let Some(mut result) = images.next() else { return };
        let width = size.width as usize;
        for (index, other) in images.enumerate() {
            match op {
                PixelOp::Blend(mode) => blend_pixels(*mode, &mut result, &other, width),
                PixelOp::Arithmetic { multiply, source1, source2, offset } => {
                    let k = [self.scalar(multiply), self.scalar(source1), self.scalar(source2), self.scalar(offset)];
                    arithmetic_pixels(k.map(|k| k as f32), &mut result, &other);
                }
                PixelOp::MaskInvert => mask_invert_pixels(&mut result, &other),
            }
            // A blend and an arithmetic composite have two sources.
            if !matches!(op, PixelOp::MaskInvert) && index == 0 {
                break;
            }
        }
        let Some(compositor) = self.base.compositor() else { return };
        let bitmap = compositor.render_interface().with_platform_render_interface(|render_interface| {
            render_interface.load_bitmap_from_pixels(
                PixelFormats::RGBA8888,
                AlphaFormat::Premul,
                &result,
                size,
                Vector::new(96.0, 96.0),
                size.width * 4,
            )
        });
        if let Some(Some(bitmap)) = bitmap {
            let source = Rect::new(0.0, 0.0, size.width as f64, size.height as f64);
            canvas.draw_bitmap(&*bitmap, 1.0, source, rect);
        }
    }

    /// Paints the brush over `rect`, in the coordinates of `canvas`: the
    /// graph over its bound sources. A brush whose factory did not load its
    /// effect paints nothing.
    pub fn paint(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        let Some(root) = self.graph.root.as_ref() else { return };
        let opacity = self.props.opacity();
        if opacity <= 0.0 || is_empty(&rect) || self.painting.replace(true) {
            return;
        }
        if opacity != 1.0 {
            canvas.push_opacity(opacity, Some(rect));
        }
        self.paint_node(root, canvas, rect);
        if opacity != 1.0 {
            canvas.pop_opacity();
        }
        self.painting.set(false);
    }

    /// One side of the drawing of `node` around the composed layer, which is
    /// at `path` (the indices of the inputs) below `node`.
    fn paint_around_layer(
        &self,
        node: &EffectNode,
        path: &[usize],
        canvas: &mut dyn IDrawingContextImpl,
        rect: Rect,
        phase: LayerPhase,
    ) {
        let Some((&index, rest)) = path.split_first() else {
            // The source parameter of the layer: the children of the visual
            // are drawn here by the render walk.
            return;
        };
        let before = phase == LayerPhase::Before;
        let has_layers = canvas.as_drawing_context_impl_with_layer_effects().is_some();
        match &node.kind {
            EffectNodeKind::Parameter(_) | EffectNodeKind::ColorSource(_) | EffectNodeKind::SceneLighting { .. } => {}
            EffectNodeKind::GaussianBlur { amount, source, .. } => {
                let radius = self.blur_radius(amount);
                match phase {
                    LayerPhase::Before => {
                        Self::push_blur(canvas, rect, radius);
                        self.paint_around_layer(source, rest, canvas, rect, phase);
                    }
                    LayerPhase::After => {
                        self.paint_around_layer(source, rest, canvas, rect, phase);
                        let pushed = Self::blur_is_pushed(canvas, radius);
                        Self::pop_blur(canvas, pushed);
                    }
                }
            }
            EffectNodeKind::Opacity { opacity, source } => {
                let opacity = self.unit(opacity);
                self.wrap_layer_in_opacity(source, rest, canvas, rect, phase, opacity);
            }
            EffectNodeKind::CrossFade { weight, source1, source2 } => {
                let weight = self.unit(weight);
                if index == 0 {
                    self.wrap_layer_in_opacity(source1, rest, canvas, rect, phase, 1.0 - weight);
                    if phase == LayerPhase::After {
                        self.paint_node_with_opacity(source2, canvas, rect, weight);
                    }
                } else {
                    if phase == LayerPhase::Before {
                        self.paint_node_with_opacity(source1, canvas, rect, 1.0 - weight);
                    }
                    self.wrap_layer_in_opacity(source2, rest, canvas, rect, phase, weight);
                }
            }
            EffectNodeKind::Composite { mode, sources } => {
                let Some(layer) = sources.get(index) else { return };
                let blended = *mode != BitmapBlendingMode::SourceOver && has_layers;
                if before {
                    if blended {
                        canvas.push_layer(rect);
                    }
                    for (position, source) in sources[..index].iter().enumerate() {
                        let pushed = blended && position > 0 && Self::push_blend(canvas, rect, *mode);
                        self.paint_node(source, canvas, rect);
                        Self::pop_blend(canvas, pushed);
                    }
                    if blended && index > 0 {
                        Self::push_blend(canvas, rect, *mode);
                    }
                    self.paint_around_layer(layer, rest, canvas, rect, phase);
                } else {
                    self.paint_around_layer(layer, rest, canvas, rect, phase);
                    Self::pop_blend(canvas, blended && index > 0);
                    for source in &sources[index + 1..] {
                        let pushed = blended && Self::push_blend(canvas, rect, *mode);
                        self.paint_node(source, canvas, rect);
                        Self::pop_blend(canvas, pushed);
                    }
                    if blended {
                        canvas.pop_layer();
                    }
                }
            }
            EffectNodeKind::Blend { mode, background, foreground } => {
                if index == 0 {
                    // The layer is the background.
                    if before {
                        canvas.push_layer(rect);
                        self.paint_around_layer(background, rest, canvas, rect, phase);
                    } else {
                        self.paint_around_layer(background, rest, canvas, rect, phase);
                        let pushed = Self::push_blend(canvas, rect, *mode);
                        self.paint_node(foreground, canvas, rect);
                        Self::pop_blend(canvas, pushed);
                        canvas.pop_layer();
                    }
                } else if before {
                    // The layer is the foreground.
                    canvas.push_layer(rect);
                    self.paint_node(background, canvas, rect);
                    Self::push_blend(canvas, rect, *mode);
                    self.paint_around_layer(foreground, rest, canvas, rect, phase);
                } else {
                    self.paint_around_layer(foreground, rest, canvas, rect, phase);
                    Self::pop_blend(canvas, has_layers);
                    canvas.pop_layer();
                }
            }
            EffectNodeKind::ColorMatrix { matrix, alpha_mode, source } => {
                let matrix = self.matrix(matrix);
                if before {
                    Self::push_matrix(canvas, rect, &matrix, *alpha_mode);
                    self.paint_around_layer(source, rest, canvas, rect, phase);
                } else {
                    self.paint_around_layer(source, rest, canvas, rect, phase);
                    Self::pop_matrix(canvas, has_layers && !matrix.is_identity());
                }
            }
            EffectNodeKind::ArithmeticWithColor { multiply, source1, source2, offset, image_is_source1, image, color } => {
                let image_index = if *image_is_source1 { 0 } else { 1 };
                if index != image_index {
                    // The layer is the colour: not an image.
                    return;
                }
                let matrix = self.arithmetic_matrix(multiply, source1, source2, offset, *image_is_source1, color);
                if before {
                    Self::push_matrix(canvas, rect, &matrix, ColorMatrixAlphaMode::Premultiplied);
                    self.paint_around_layer(image, rest, canvas, rect, phase);
                } else {
                    self.paint_around_layer(image, rest, canvas, rect, phase);
                    Self::pop_matrix(canvas, has_layers && !matrix.is_identity());
                }
            }
            EffectNodeKind::AlphaMask { source, mask } => {
                if index == 0 {
                    // The layer is the source.
                    if before {
                        canvas.push_layer(rect);
                        self.paint_around_layer(source, rest, canvas, rect, phase);
                    } else {
                        self.paint_around_layer(source, rest, canvas, rect, phase);
                        let pushed = Self::push_blend(canvas, rect, BitmapBlendingMode::DestinationIn);
                        if pushed {
                            self.paint_node(mask, canvas, rect);
                        }
                        Self::pop_blend(canvas, pushed);
                        canvas.pop_layer();
                    }
                } else if before {
                    // The layer is the mask.
                    canvas.push_layer(rect);
                    self.paint_node(source, canvas, rect);
                    Self::push_blend(canvas, rect, BitmapBlendingMode::DestinationIn);
                    self.paint_around_layer(mask, rest, canvas, rect, phase);
                } else {
                    self.paint_around_layer(mask, rest, canvas, rect, phase);
                    Self::pop_blend(canvas, has_layers);
                    canvas.pop_layer();
                }
            }
            EffectNodeKind::Border { source, .. } => {
                self.paint_around_layer(source, rest, canvas, rect, phase);
            }
            EffectNodeKind::Shadow { amount, color, source } => {
                let radius = self.blur_radius(amount);
                let matrix = shadow_matrix(self.color(color));
                if before {
                    Self::push_blur(canvas, rect, radius);
                    Self::push_matrix(canvas, rect, &matrix, ColorMatrixAlphaMode::Straight);
                    self.paint_around_layer(source, rest, canvas, rect, phase);
                } else {
                    self.paint_around_layer(source, rest, canvas, rect, phase);
                    Self::pop_matrix(canvas, has_layers && !matrix.is_identity());
                    let pushed = Self::blur_is_pushed(canvas, radius);
                    Self::pop_blur(canvas, pushed);
                }
            }
            // Deviation (winui-materials.md): the children of a layer visual
            // are drawn by the render walk, not into a bitmap of their own,
            // so an operation on their pixels draws them unchanged.
            EffectNodeKind::Pixels { sources, .. } => {
                if let Some(layer) = sources.get(index) {
                    self.paint_around_layer(layer, rest, canvas, rect, phase);
                }
            }
        }
    }

    /// One side of an opacity around the input of a node that leads to the
    /// layer.
    fn wrap_layer_in_opacity(
        &self,
        source: &EffectNode,
        rest: &[usize],
        canvas: &mut dyn IDrawingContextImpl,
        rect: Rect,
        phase: LayerPhase,
        opacity: f64,
    ) {
        match phase {
            LayerPhase::Before => {
                if opacity < 1.0 {
                    canvas.push_opacity(opacity.max(0.0), Some(rect));
                }
                self.paint_around_layer(source, rest, canvas, rect, phase);
            }
            LayerPhase::After => {
                self.paint_around_layer(source, rest, canvas, rect, phase);
                if opacity < 1.0 {
                    canvas.pop_opacity();
                }
            }
        }
    }

    /// What a layer visual whose effect is the brush draws before its
    /// children, over `rect` (the bounds of the composed layer): the parts
    /// of the graph under the layer, and what wraps the layer (blurs,
    /// opacities) pushed. The children are the first source parameter of the
    /// graph; [`pop_for_layer`](Self::pop_for_layer) is called after them
    /// with the same rectangle.
    ///
    /// A graph without a source parameter is drawn whole and the children
    /// are not shown; a brush whose factory did not load its effect shows
    /// nothing.
    pub fn push_for_layer(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        let opacity = self.props.opacity();
        if opacity != 1.0 {
            canvas.push_opacity(opacity.max(0.0), Some(rect));
        }
        match (self.graph.root.as_ref(), self.graph.layer_path.as_deref()) {
            (Some(root), Some(path)) => self.paint_around_layer(root, path, canvas, rect, LayerPhase::Before),
            (root, _) => {
                if let Some(root) = root {
                    self.paint_node(root, canvas, rect);
                }
                // Nothing of the children is shown.
                canvas.push_clip(Rect::new(rect.x, rect.y, 0.0, 0.0));
            }
        }
    }

    /// What a layer visual whose effect is the brush draws after its
    /// children: see [`push_for_layer`](Self::push_for_layer).
    pub fn pop_for_layer(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        match (self.graph.root.as_ref(), self.graph.layer_path.as_deref()) {
            (Some(root), Some(path)) => self.paint_around_layer(root, path, canvas, rect, LayerPhase::After),
            _ => canvas.pop_clip(),
        }
        if self.props.opacity() != 1.0 {
            canvas.pop_opacity();
        }
    }

    /// How far the output of the brush as the effect of a layer extends
    /// beyond the composed layer: the padding of the blurs around it.
    pub fn layer_padding(&self) -> Thickness {
        let mut padding = Thickness::default();
        let (Some(mut node), Some(path)) = (self.graph.root.as_ref(), self.graph.layer_path.as_deref()) else {
            return padding;
        };
        for &index in path {
            if let EffectNodeKind::GaussianBlur { amount, .. } = &node.kind {
                let radius = self.blur_radius(amount);
                if radius > 0.0 {
                    let blur = ImmutableBlurEffect::new(radius);
                    padding = padding + EffectExtensions::get_effect_output_padding(Some(&blur as &dyn IEffect));
                }
            }
            match node.sources().get(index).copied() {
                Some(source) => node = source,
                None => break,
            }
        }
        padding
    }
}

impl_server_render_resource!(
    ServerCompositionEffectBrush,
    base,
    composition_property: ServerCompositionBrushProps::get_composition_property
);

impl ServerCompositionBrushHooks for ServerCompositionEffectBrush {}

impl IServerObject for ServerCompositionEffectBrush {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
        let Some(changes) = reader.read_value::<EffectBrushChanges>() else { return };
        let compositor = self.base.compositor();
        let resolve = |id: ServerObjectId| compositor.as_ref().and_then(|compositor| compositor.get_object(id));
        if let Some(id) = changes.properties {
            let properties = resolve(id).and_then(|set| set.into_any_rc().downcast::<ServerCompositionPropertySet>().ok());
            self.set_properties(properties);
        }
        if let Some(sources) = changes.sources {
            self.set_sources(sources.into_iter().map(|(name, id)| (name, id.and_then(resolve))).collect());
        }
    }

    fn values_invalidated(&self) {
        self.base.core().invalidated(self);
    }

    fn dispose(&self) {
        self.set_properties(None);
        self.set_sources(Vec::new());
        self.base.core().dispose();
    }

    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
        Some(self)
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// The server-side counterpart of a
/// [`CompositionBackdropBrush`](super::CompositionBackdropBrush): it paints
/// what was drawn under the visual that paints it, drawn again by the
/// compositor ([`draw_backdrop`](crate::rendering::composition::server::draw_backdrop),
/// winui-materials.md).
pub struct ServerCompositionBackdropBrush {
    base: ServerRenderResource,
    props: ServerCompositionBrushProps,
}

impl ServerCompositionBackdropBrush {
    /// Creates the brush, active.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionBackdropBrush> {
        let brush = Rc::new_cyclic(|this: &Weak<ServerCompositionBackdropBrush>| ServerCompositionBackdropBrush {
            base: ServerRenderResource::new(
                compositor,
                this.clone() as Weak<dyn IAnimatedServerObject>,
                this.clone() as Weak<dyn IServerRenderResource>,
            ),
            props: ServerCompositionBrushProps::new(),
        });
        brush.base.object().activate();
        brush
    }

    /// The properties of the generated `ServerCompositionBrush`.
    pub fn brush_props(&self) -> &ServerCompositionBrushProps {
        &self.props
    }

    /// Paints the backdrop of the visual being drawn over `rect`, with the
    /// opacity of the brush.
    pub fn paint(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        let opacity = self.props.opacity();
        if opacity <= 0.0 {
            return;
        }
        if opacity < 1.0 {
            canvas.push_opacity(opacity, Some(rect));
        }
        crate::rendering::composition::server::draw_backdrop(canvas, rect);
        if opacity < 1.0 {
            canvas.pop_opacity();
        }
    }
}

impl_server_render_resource!(
    ServerCompositionBackdropBrush,
    base,
    composition_property: ServerCompositionBrushProps::get_composition_property
);

impl ServerCompositionBrushHooks for ServerCompositionBackdropBrush {}

impl IServerObject for ServerCompositionBackdropBrush {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }

    fn values_invalidated(&self) {
        self.base.core().invalidated(self);
    }

    fn dispose(&self) {
        self.base.core().dispose();
    }

    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
        Some(self)
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// The server-side counterpart of a
/// [`CompositionHostBackdropBrush`](super::CompositionHostBackdropBrush):
/// what is behind the window. Over a window that the platform composes over
/// what is behind it (a transparency level other than none), the area of
/// the brush is made transparent, so that shows through; over an opaque
/// window the brush paints nothing.
pub struct ServerCompositionHostBackdropBrush {
    base: ServerRenderResource,
    props: ServerCompositionBrushProps,
}

impl ServerCompositionHostBackdropBrush {
    /// Creates the brush, active.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionHostBackdropBrush> {
        let brush = Rc::new_cyclic(|this: &Weak<ServerCompositionHostBackdropBrush>| ServerCompositionHostBackdropBrush {
            base: ServerRenderResource::new(
                compositor,
                this.clone() as Weak<dyn IAnimatedServerObject>,
                this.clone() as Weak<dyn IServerRenderResource>,
            ),
            props: ServerCompositionBrushProps::new(),
        });
        brush.base.object().activate();
        brush
    }

    /// The properties of the generated `ServerCompositionBrush`.
    pub fn brush_props(&self) -> &ServerCompositionBrushProps {
        &self.props
    }

    /// Makes `rect` transparent in a transparent window.
    pub fn paint(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        let transparent = crate::rendering::composition::server::painting_visual()
            .and_then(|visual| visual.root())
            .is_some_and(|target| target.transparency_level() != crate::rendering::composition::CompositionTransparencyLevel::None);
        if !transparent || self.props.opacity() <= 0.0 {
            return;
        }
        canvas.push_clip(rect);
        if let Some(layers) = canvas.as_drawing_context_impl_with_layer_effects() {
            // A layer composed with `Source`: what it holds (nothing)
            // replaces what is under it.
            layers.push_blend_layer(rect, BitmapBlendingMode::Source);
            layers.pop_blend_layer();
        }
        canvas.pop_clip();
    }
}

impl_server_render_resource!(
    ServerCompositionHostBackdropBrush,
    base,
    composition_property: ServerCompositionBrushProps::get_composition_property
);

impl ServerCompositionBrushHooks for ServerCompositionHostBackdropBrush {}

impl IServerObject for ServerCompositionHostBackdropBrush {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }

    fn values_invalidated(&self) {
        self.base.core().invalidated(self);
    }

    fn dispose(&self) {
        self.base.core().dispose();
    }

    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
        Some(self)
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// The colour matrix of a shadow (on straight colours): the colour of the
/// shadow, with the alpha of the source times the alpha of the colour.
fn shadow_matrix(color: Color) -> ColorMatrix {
    let channel = |value: u8| f32::from(value) / 255.0;
    ColorMatrix::new([
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, channel(color.a)],
        [channel(color.r), channel(color.g), channel(color.b), 0.0],
    ])
}

/// A premultiplied RGBA pixel as straight colour channels and alpha, 0 to 1.
fn unpremultiply(pixel: &[u8]) -> ([f32; 3], f32) {
    let alpha = f32::from(pixel[3]) / 255.0;
    if alpha <= 0.0 {
        return ([0.0; 3], 0.0);
    }
    let channel = |value: u8| (f32::from(value) / 255.0 / alpha).min(1.0);
    ([channel(pixel[0]), channel(pixel[1]), channel(pixel[2])], alpha)
}

fn store(pixel: &mut [u8], color: [f32; 3], alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    for channel in 0..3 {
        pixel[channel] = (color[channel].clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
    }
    pixel[3] = (alpha * 255.0).round() as u8;
}

fn luminosity(color: [f32; 3]) -> f32 {
    0.3 * color[0] + 0.59 * color[1] + 0.11 * color[2]
}

fn color_dodge(b: f32, s: f32) -> f32 {
    if b <= 0.0 {
        0.0
    } else if s >= 1.0 {
        1.0
    } else {
        (b / (1.0 - s)).min(1.0)
    }
}

fn color_burn(b: f32, s: f32) -> f32 {
    if b >= 1.0 {
        1.0
    } else if s <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - b) / s).min(1.0)
    }
}

/// A small hash of a pixel position, 0 to 1: the random choice of
/// `Dissolve`, the same for a pixel in every frame.
fn dissolve_noise(index: usize) -> f32 {
    let mut x = index as u32 ^ 0x9E37_79B9;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    x = x.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 16;
    (x & 0xFFFF) as f32 / 65535.0
}

/// Blends the premultiplied pixels of `foreground` into `background` with
/// one of the modes of Direct2D's blend effect that the blend layers do not
/// have; the result is composed as a separable blend is:
/// `(1 - as) ab Cb + (1 - ab) as Cs + as ab B(Cb, Cs)`.
fn blend_pixels(mode: BlendEffectMode, background: &mut [u8], foreground: &[u8], width: usize) {
    let _ = width;
    for (index, (back, fore)) in background.chunks_exact_mut(4).zip(foreground.chunks_exact(4)).enumerate() {
        let (cb, ab) = unpremultiply(back);
        let (cs, as_) = unpremultiply(fore);
        if mode == BlendEffectMode::Dissolve {
            // The foreground where a random number falls under its alpha,
            // opaque; the background elsewhere.
            if as_ > 0.0 && dissolve_noise(index) < as_ {
                store(back, cs, 1.0);
            }
            continue;
        }
        let blended: [f32; 3] = match mode {
            BlendEffectMode::DarkerColor => {
                if luminosity(cs) < luminosity(cb) {
                    cs
                } else {
                    cb
                }
            }
            BlendEffectMode::LighterColor => {
                if luminosity(cs) > luminosity(cb) {
                    cs
                } else {
                    cb
                }
            }
            _ => std::array::from_fn(|channel| {
                let (b, s) = (cb[channel], cs[channel]);
                match mode {
                    BlendEffectMode::LinearBurn => (b + s - 1.0).max(0.0),
                    BlendEffectMode::VividLight => {
                        if s <= 0.5 {
                            color_burn(b, 2.0 * s)
                        } else {
                            color_dodge(b, 2.0 * (s - 0.5))
                        }
                    }
                    BlendEffectMode::LinearLight => (b + 2.0 * s - 1.0).clamp(0.0, 1.0),
                    BlendEffectMode::PinLight => {
                        if s <= 0.5 {
                            b.min(2.0 * s)
                        } else {
                            b.max(2.0 * s - 1.0)
                        }
                    }
                    BlendEffectMode::HardMix => {
                        if b + s >= 1.0 {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    BlendEffectMode::Subtract => (b - s).max(0.0),
                    BlendEffectMode::Division => {
                        if s <= 0.0 {
                            if b <= 0.0 {
                                0.0
                            } else {
                                1.0
                            }
                        } else {
                            (b / s).min(1.0)
                        }
                    }
                    // The other modes are drawn by the blend layers.
                    _ => s,
                }
            }),
        };
        let alpha = as_ + ab - as_ * ab;
        if alpha <= 0.0 {
            store(back, [0.0; 3], 0.0);
            continue;
        }
        let premultiplied: [f32; 3] = std::array::from_fn(|channel| {
            (1.0 - as_) * ab * cb[channel] + (1.0 - ab) * as_ * cs[channel] + as_ * ab * blended[channel]
        });
        store(back, premultiplied.map(|value| value / alpha), alpha);
    }
}

/// `k1 * s1 * s2 + k2 * s1 + k3 * s2 + k4` of premultiplied pixels, per
/// channel, clamped to 0..1 and the colour to the alpha (Direct2D's
/// arithmetic composite).
fn arithmetic_pixels(k: [f32; 4], first: &mut [u8], second: &[u8]) {
    for (one, two) in first.chunks_exact_mut(4).zip(second.chunks_exact(4)) {
        let mut out = [0.0f32; 4];
        for channel in 0..4 {
            let (a, b) = (f32::from(one[channel]) / 255.0, f32::from(two[channel]) / 255.0);
            out[channel] = (k[0] * a * b + k[1] * a + k[2] * b + k[3]).clamp(0.0, 1.0);
        }
        for channel in 0..3 {
            out[channel] = out[channel].min(out[3]);
        }
        for channel in 0..4 {
            one[channel] = (out[channel] * 255.0).round() as u8;
        }
    }
}

/// The `MaskInvert` composite of the premultiplied pixels of `source` onto
/// `destination`: the colour of the destination inverted where the source
/// covers it (`as (ad - Cd) + (1 - as) Cd`), the alpha of the destination
/// kept.
fn mask_invert_pixels(destination: &mut [u8], source: &[u8]) {
    for (dest, src) in destination.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
        let source_alpha = f32::from(src[3]) / 255.0;
        let dest_alpha = f32::from(dest[3]) / 255.0;
        for channel in 0..3 {
            let d = f32::from(dest[channel]) / 255.0;
            let out = source_alpha * (dest_alpha - d) + (1.0 - source_alpha) * d;
            dest[channel] = (out.clamp(0.0, dest_alpha) * 255.0).round() as u8;
        }
    }
}

#[cfg(test)]
mod pixel_tests {
    use super::*;

    #[test]
    fn the_extra_blend_modes_follow_their_formulas() {
        let opaque = |r: u8, g: u8, b: u8| vec![r, g, b, 255];
        let mut back = opaque(200, 100, 50);
        blend_pixels(BlendEffectMode::Subtract, &mut back, &opaque(100, 150, 25), 1);
        assert_eq!(opaque(100, 0, 25), back);
        let mut back = opaque(200, 100, 50);
        blend_pixels(BlendEffectMode::LinearBurn, &mut back, &opaque(100, 255, 0), 1);
        assert_eq!(opaque(45, 100, 0), back);
        let mut back = opaque(100, 100, 100);
        blend_pixels(BlendEffectMode::HardMix, &mut back, &opaque(200, 100, 0), 1);
        assert_eq!(opaque(255, 0, 0), back);
        let mut back = opaque(10, 10, 10);
        blend_pixels(BlendEffectMode::LighterColor, &mut back, &opaque(200, 0, 0), 1);
        assert_eq!(opaque(200, 0, 0), back);
    }

    #[test]
    fn a_blend_over_transparency_is_the_foreground() {
        let mut back = vec![0, 0, 0, 0];
        blend_pixels(BlendEffectMode::Division, &mut back, &[100, 50, 0, 255], 1);
        assert_eq!(vec![100, 50, 0, 255], back);
    }

    #[test]
    fn the_arithmetic_composite_of_two_images() {
        let mut first = vec![255, 0, 128, 255];
        arithmetic_pixels([0.0, 0.5, 0.5, 0.0], &mut first, &[0, 255, 128, 255]);
        assert_eq!(vec![128, 128, 128, 255], first);
    }

    #[test]
    fn mask_invert_inverts_the_destination_under_the_source() {
        let mut destination = vec![255, 0, 64, 255, 255, 0, 64, 255];
        mask_invert_pixels(&mut destination, &[0, 0, 0, 255, 0, 0, 0, 0]);
        assert_eq!(vec![0, 255, 191, 255, 255, 0, 64, 255], destination);
    }
}
