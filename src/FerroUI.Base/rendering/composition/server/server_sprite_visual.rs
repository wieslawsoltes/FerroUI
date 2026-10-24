use super::{
    paint_composition_brush, CompositionProperty, IServerObject, IServerRenderResourceObserver, IServerVisualContent,
    ServerCompositionVisual, ServerDropShadow, ServerSizeDependantVisual,
    ServerVisualRenderContext,
};
use crate::platform::LtrbRect;
use crate::rendering::composition::generated::{
    ServerCompositionContainerVisualProps, ServerSpriteVisualHooks, ServerSpriteVisualProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::Rect;
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::time::Duration;

/// Server-side counterpart of a
/// [`SpriteVisual`](crate::rendering::composition::SpriteVisual): the shadow
/// of its rectangle (or of its mask, or of the alpha of the brush, see
/// [`ServerDropShadow::render_under_rect`] and
/// [`ServerDropShadow::render_with_content`]), then its brush over
/// `(0, 0, size)`, then its children.
///
/// The visual observes its brush and its shadow, which are render
/// resources: a change of either (an animation included) redraws it.
#[derive(Default)]
pub struct ServerSpriteVisual {
    props: ServerSpriteVisualProps,
}

impl ServerSpriteVisual {
    pub fn new() -> Self {
        Self::default()
    }

    /// The properties of the visual.
    pub fn props(&self) -> &ServerSpriteVisualProps {
        &self.props
    }

    /// The brush: a server-side composition brush.
    pub fn brush(&self) -> Option<Rc<dyn IServerObject>> {
        self.props.brush()
    }

    /// The shadow.
    pub fn shadow(&self) -> Option<Rc<ServerDropShadow>> {
        self.props.shadow().and_then(|shadow| shadow.into_any_rc().downcast::<ServerDropShadow>().ok())
    }

    /// Starts or stops observing a render resource the visual shows.
    fn observe(visual: &ServerCompositionVisual, resource: Option<Rc<dyn IServerObject>>, observe: bool) {
        let Some(resource) = resource.and_then(|resource| resource.as_render_resource()) else { return };
        let Some(this) = visual.weak().upgrade() else { return };
        let observer: Rc<dyn IServerRenderResourceObserver> = this;
        if observe {
            resource.add_observer(&observer);
        } else {
            resource.remove_observer(&observer);
        }
    }

    fn rect(visual: &ServerCompositionVisual) -> Rect {
        let size = visual.size();
        Rect::new(0.0, 0.0, size.x, size.y)
    }
}

impl ServerSpriteVisualHooks for ServerCompositionVisual {
    fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        ServerCompositionVisual::base_deserialize_changes_core(self, reader, committed_at);
    }

    fn on_brush_changing(&self) {
        if let Some(content) = self.content_as::<ServerSpriteVisual>() {
            ServerSpriteVisual::observe(self, content.brush(), false);
        }
    }

    fn on_brush_changed(&self) {
        if let Some(content) = self.content_as::<ServerSpriteVisual>() {
            ServerSpriteVisual::observe(self, content.brush(), true);
        }
        self.invalidate_content();
    }

    fn on_shadow_changing(&self) {
        if let Some(content) = self.content_as::<ServerSpriteVisual>() {
            ServerSpriteVisual::observe(self, content.props.shadow(), false);
        }
    }

    fn on_shadow_changed(&self) {
        if let Some(content) = self.content_as::<ServerSpriteVisual>() {
            ServerSpriteVisual::observe(self, content.props.shadow(), true);
        }
        self.invalidate_content();
    }
}

impl IServerVisualContent for ServerSpriteVisual {
    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        self.props.deserialize_changes_core(visual, reader, committed_at);
    }

    /// The rectangle of the visual, with the area of its shadow.
    fn compute_own_content_bounds(&self, visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        let bounds = ServerSizeDependantVisual::compute_own_content_bounds(visual)?;
        match self.shadow() {
            Some(shadow) => Some(LtrbRect::from_rect(shadow.bounds(bounds.to_rect()))),
            None => Some(bounds),
        }
    }

    fn size_changed(&self, visual: &ServerCompositionVisual) {
        ServerSizeDependantVisual::size_changed(visual);
    }

    fn render_core(
        &self,
        visual: &ServerCompositionVisual,
        context: &mut ServerVisualRenderContext<'_>,
        _current_transformed_clip: LtrbRect,
    ) {
        let rect = Self::rect(visual);
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }
        let brush = self.brush();
        if let Some(shadow) = self.shadow().filter(|shadow| shadow.is_visible()) {
            // `InheritFromVisualContent`: the shadow of the alpha of the
            // brush, drawn with the brush through a drop shadow effect.
            if shadow.inherits_from_visual_content() {
                if let Some(brush) = &brush {
                    if shadow.render_with_content(context.canvas(), rect, |canvas| {
                        paint_composition_brush(canvas, brush, rect)
                    }) {
                        return;
                    }
                }
            }
            // The shadow of the mask, or of the rectangle.
            shadow.render_under_rect(context.canvas(), rect);
        }
        if let Some(brush) = brush {
            paint_composition_brush(context.canvas(), &brush, rect);
        }
    }

    /// The brush or the shadow changed.
    fn dependency_queued_invalidate(&self, visual: &ServerCompositionVisual) {
        visual.invalidate_content();
    }

    fn dispose(&self, visual: &ServerCompositionVisual) {
        ServerSpriteVisual::observe(visual, self.brush(), false);
        ServerSpriteVisual::observe(visual, self.props.shadow(), false);
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
