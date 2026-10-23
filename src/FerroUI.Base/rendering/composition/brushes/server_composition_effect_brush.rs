//! The server-side counterparts of the WinUI effect and backdrop brushes:
//! the drawing of an effect graph over its sources, as a brush and as the
//! effect of a layer visual.

use super::paint_composition_brush;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{BoxShadows, Color, EffectExtensions, IEffect, ImmutableBlurEffect};
use crate::platform::IDrawingContextImpl;
use crate::rendering::composition::effects::{
    blur_amount_to_radius, EffectColor, EffectGraph, EffectNode, EffectNodeKind, EffectScalar,
};
use crate::rendering::composition::generated::{ServerCompositionBrushHooks, ServerCompositionBrushProps};
use crate::rendering::composition::server::{
    impl_server_render_resource, IAnimatedServerObject, IServerObject, IServerRenderResource,
    IServerRenderResourceObserver, ServerCompositionPropertySet, ServerCompositor, ServerObjectId,
    ServerRenderResource,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Rect, RoundedRect, Thickness};
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

    /// Paints `node` over `rect`.
    fn paint_node(&self, node: &EffectNode, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        match &node.kind {
            EffectNodeKind::Parameter(name) => {
                if let Some(source) = self.source(name).filter(|source| !self.is_self(source)) {
                    paint_composition_brush(canvas, &source, rect);
                }
            }
            EffectNodeKind::GaussianBlur { amount, source } => {
                let pushed = Self::push_blur(canvas, rect, self.blur_radius(amount));
                self.paint_node(source, canvas, rect);
                Self::pop_blur(canvas, pushed);
            }
            EffectNodeKind::Opacity { opacity, source } => {
                self.paint_node_with_opacity(source, canvas, rect, self.unit(opacity));
            }
            EffectNodeKind::CrossFade { weight, source1, source2 } => {
                // Deviation (winui-composition.md): the second source is drawn
                // over the first, each with its weight.
                let weight = self.unit(weight);
                self.paint_node_with_opacity(source1, canvas, rect, 1.0 - weight);
                self.paint_node_with_opacity(source2, canvas, rect, weight);
            }
            EffectNodeKind::Composite(sources) => {
                for source in sources {
                    self.paint_node(source, canvas, rect);
                }
            }
            EffectNodeKind::ColorSource(color) => {
                let color = self.color(color);
                if color.a > 0 {
                    let brush = ImmutableSolidColorBrush::new(color);
                    canvas.draw_rectangle(Some(&brush), None, RoundedRect::from_rect(rect), &BoxShadows::default());
                }
            }
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
        match &node.kind {
            EffectNodeKind::Parameter(_) | EffectNodeKind::ColorSource(_) => {}
            EffectNodeKind::GaussianBlur { amount, source } => {
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
            EffectNodeKind::Composite(sources) => {
                let Some(layer) = sources.get(index) else { return };
                match phase {
                    LayerPhase::Before => {
                        for source in &sources[..index] {
                            self.paint_node(source, canvas, rect);
                        }
                        self.paint_around_layer(layer, rest, canvas, rect, phase);
                    }
                    LayerPhase::After => {
                        self.paint_around_layer(layer, rest, canvas, rect, phase);
                        for source in &sources[index + 1..] {
                            self.paint_node(source, canvas, rect);
                        }
                    }
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
/// nothing, as the drawing contexts cannot read back what is behind a
/// visual.
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

    /// Paints nothing.
    //
    // Deviation (winui-composition.md): no backdrop can be read back.
    pub fn paint(&self, _canvas: &mut dyn IDrawingContextImpl, _rect: Rect) {}
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
