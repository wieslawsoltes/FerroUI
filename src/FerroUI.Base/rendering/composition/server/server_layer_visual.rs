use super::{
    CompositionProperty, IServerObject, IServerRenderResourceObserver, IServerVisualContent, ServerCompositionEffectBrush,
    ServerCompositionVisual, ServerDropShadow, ServerSizeDependantVisual, ServerVisualRenderContext,
};
use crate::media::{Color, EffectExtensions, IEffect, ImmutableDropShadowEffect};
use crate::platform::{IDrawingContextImpl, LtrbRect};
use crate::rendering::composition::generated::{
    ServerCompositionContainerVisualProps, ServerLayerVisualHooks, ServerLayerVisualProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Rect, Thickness};
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::time::Duration;

/// Starts or stops observing a render resource a visual shows: a change of
/// the resource (an animation included) reaches the content of the visual
/// through `dependency_queued_invalidate`.
pub(crate) fn observe_visual_resource(
    visual: &ServerCompositionVisual,
    resource: Option<Rc<dyn IServerObject>>,
    observe: bool,
) {
    let Some(resource) = resource.and_then(|resource| resource.as_render_resource()) else { return };
    let Some(this) = visual.weak().upgrade() else { return };
    let observer: Rc<dyn IServerRenderResourceObserver> = this;
    if observe {
        resource.add_observer(&observer);
    } else {
        resource.remove_observer(&observer);
    }
}

/// The drop shadow of the alpha of pushed content that draws `shadow`: its
/// offset (X and Y), blur radius, colour and opacity.
pub fn drop_shadow_effect(shadow: &ServerDropShadow) -> ImmutableDropShadowEffect {
    let props = shadow.props();
    let offset = props.offset();
    let color: Color = props.color();
    ImmutableDropShadowEffect::new(
        offset.x,
        offset.y,
        props.blur_radius().max(0.0) as f64,
        color,
        props.opacity().clamp(0.0, 1.0) as f64,
    )
}

/// How a layer visual composes its children.
enum LayerMode {
    /// Through a drop shadow effect of the drawing context: the shadow follows
    /// the alpha of the composed children.
    Shadow(ImmutableDropShadowEffect),
    /// Into a plain layer.
    Layer,
}

/// Server-side counterpart of a
/// [`LayerVisual`](crate::rendering::composition::LayerVisual): its children
/// are composed into an offscreen layer (`push_layer` over the bounds of the
/// subtree), which the opacity of the visual applies to as a whole.
///
/// With a shadow, the children are composed through a drop shadow effect
/// when the drawing context has effects, so the shadow follows the alpha of
/// the composed content; otherwise the box shadow of the rectangle of the
/// visual is drawn under the layer. A shadow with a `Mask` (and the
/// `Default` source policy) is the shadow of the mask over the rectangle of
/// the visual, drawn under the layer
/// ([`ServerDropShadow::render_under_rect`]).
///
/// With an effect (an effect brush), the composed children are the first
/// source parameter of its graph: what the graph draws under the layer and
/// what wraps it are drawn and pushed before the children, inside the layer
/// (or the shadow), and popped and drawn after them.
///
/// Deviation (winui-composition.md): without effect support in the drawing
/// context the shadow is the box shadow of the rectangle of the visual, as
/// for a sprite visual.
#[derive(Default)]
pub struct ServerLayerVisual {
    props: ServerLayerVisualProps,
}

impl ServerLayerVisual {
    pub fn new() -> Self {
        Self::default()
    }

    /// The properties of the visual.
    pub fn props(&self) -> &ServerLayerVisualProps {
        &self.props
    }

    /// The shadow.
    pub fn shadow(&self) -> Option<Rc<ServerDropShadow>> {
        self.props.shadow().and_then(|shadow| shadow.into_any_rc().downcast::<ServerDropShadow>().ok())
    }

    /// The effect brush applied to the composed children (the WinUI
    /// `Effect`).
    pub fn effect_brush(&self) -> Option<Rc<ServerCompositionEffectBrush>> {
        self.props
            .effect_brush()
            .and_then(|brush| brush.into_any_rc().downcast::<ServerCompositionEffectBrush>().ok())
    }

    /// The shadow, when it is visible.
    fn visible_shadow(&self) -> Option<Rc<ServerDropShadow>> {
        self.shadow().filter(|shadow| shadow.box_shadows().count() > 0)
    }

    fn rect(visual: &ServerCompositionVisual) -> Rect {
        let size = visual.size();
        Rect::new(0.0, 0.0, size.x, size.y)
    }

    /// How the children are composed on `canvas`; `None` when the subtree has
    /// no bounds (nothing is pushed). The same before and after the
    /// children, so what is pushed is popped.
    fn layer_mode(&self, visual: &ServerCompositionVisual, canvas: &mut dyn IDrawingContextImpl) -> Option<(LayerMode, Rect)> {
        let bounds = visual.sub_tree_bounds()?.to_rect();
        // A shadow with a mask (and the `Default` source policy) is the
        // shadow of the mask, drawn under a plain layer.
        let mode = match self.visible_shadow() {
            Some(shadow)
                if shadow.effective_mask().is_none() && canvas.as_drawing_context_impl_with_effects().is_some() =>
            {
                LayerMode::Shadow(drop_shadow_effect(&shadow))
            }
            _ => LayerMode::Layer,
        };
        Some((mode, bounds))
    }
}

impl ServerLayerVisualHooks for ServerCompositionVisual {
    fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        ServerCompositionVisual::base_deserialize_changes_core(self, reader, committed_at);
    }

    fn on_shadow_changing(&self) {
        if let Some(content) = self.content_as::<ServerLayerVisual>() {
            observe_visual_resource(self, content.props.shadow(), false);
        }
    }

    fn on_shadow_changed(&self) {
        if let Some(content) = self.content_as::<ServerLayerVisual>() {
            observe_visual_resource(self, content.props.shadow(), true);
        }
        self.invalidate_content();
    }

    fn on_effect_brush_changing(&self) {
        if let Some(content) = self.content_as::<ServerLayerVisual>() {
            observe_visual_resource(self, content.props.effect_brush(), false);
        }
    }

    fn on_effect_brush_changed(&self) {
        if let Some(content) = self.content_as::<ServerLayerVisual>() {
            observe_visual_resource(self, content.props.effect_brush(), true);
        }
        self.invalidate_content();
    }
}

impl IServerVisualContent for ServerLayerVisual {
    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        self.props.deserialize_changes_core(visual, reader, committed_at);
    }

    /// The area of the box shadow of the rectangle, when there is a shadow
    /// (what the fallback draws); a layer visual has no content of its own.
    fn compute_own_content_bounds(&self, visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        let shadow = self.visible_shadow()?;
        let bounds = ServerSizeDependantVisual::compute_own_content_bounds(visual)?;
        Some(LtrbRect::from_rect(shadow.bounds(bounds.to_rect())))
    }

    fn size_changed(&self, visual: &ServerCompositionVisual) {
        ServerSizeDependantVisual::size_changed(visual);
    }

    /// The shadow of the composed children extends the subtree as the drop
    /// shadow effect does, and the blurs of the effect around the layer as
    /// the blur effect does.
    fn sub_tree_bounds_padding(&self, _visual: &ServerCompositionVisual) -> Option<Thickness> {
        let shadow = self.visible_shadow().map(|shadow| {
            let effect = drop_shadow_effect(&shadow);
            EffectExtensions::get_effect_output_padding(Some(&effect as &dyn IEffect))
        });
        let effect = self.effect_brush().map(|effect| effect.layer_padding());
        match (shadow, effect) {
            (None, None) => None,
            (shadow, effect) => Some(shadow.unwrap_or_default() + effect.unwrap_or_default()),
        }
    }

    /// A shadow or an effect depends on every change of the subtree, as an
    /// effect does.
    fn has_effect(&self, visual: &ServerCompositionVisual) -> bool {
        visual.effect().is_some() || self.visible_shadow().is_some() || self.effect_brush().is_some()
    }

    fn render_core(
        &self,
        visual: &ServerCompositionVisual,
        context: &mut ServerVisualRenderContext<'_>,
        _current_transformed_clip: LtrbRect,
    ) {
        let canvas = context.canvas();
        let Some((mode, bounds)) = self.layer_mode(visual, canvas) else { return };
        match mode {
            LayerMode::Shadow(effect) => {
                if let Some(effects) = canvas.as_drawing_context_impl_with_effects() {
                    effects.push_effect(Some(bounds), &effect);
                }
            }
            LayerMode::Layer => {
                if let Some(shadow) = self.visible_shadow() {
                    shadow.render_under_rect(canvas, Self::rect(visual));
                }
                canvas.push_layer(bounds);
            }
        }
        // The effect, inside the layer: the composed children are its first
        // source parameter.
        if let Some(effect) = self.effect_brush() {
            effect.push_for_layer(canvas, bounds);
        }
    }

    fn render_after_children(&self, visual: &ServerCompositionVisual, context: &mut ServerVisualRenderContext<'_>) {
        let canvas = context.canvas();
        let Some((mode, bounds)) = self.layer_mode(visual, canvas) else { return };
        if let Some(effect) = self.effect_brush() {
            effect.pop_for_layer(canvas, bounds);
        }
        match mode {
            LayerMode::Shadow(_) => {
                if let Some(effects) = canvas.as_drawing_context_impl_with_effects() {
                    effects.pop_effect();
                }
            }
            LayerMode::Layer => canvas.pop_layer(),
        }
    }

    /// The shadow or the effect changed.
    fn dependency_queued_invalidate(&self, visual: &ServerCompositionVisual) {
        visual.invalidate_content();
    }

    fn dispose(&self, visual: &ServerCompositionVisual) {
        observe_visual_resource(visual, self.props.shadow(), false);
        observe_visual_resource(visual, self.props.effect_brush(), false);
    }

    fn find_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn get_composition_property(&self, name: &str) -> Option<&'static CompositionProperty> {
        ServerCompositionContainerVisualProps::get_composition_property(name)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
