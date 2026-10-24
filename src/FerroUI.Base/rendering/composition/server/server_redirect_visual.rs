use super::server_layer_visual::observe_visual_resource;
use super::{
    CompositionProperty, IServerVisualContent, ServerCompositionVisual, ServerCompositionVisualSurface,
    ServerSizeDependantVisual, ServerVisualRenderContext,
};
use crate::platform::LtrbRect;
use crate::rendering::composition::generated::{
    ServerCompositionContainerVisualProps, ServerRedirectVisualHooks, ServerRedirectVisualProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::time::Duration;

/// Server-side counterpart of a
/// [`RedirectVisual`](crate::rendering::composition::RedirectVisual): draws
/// the subtree of its source in its own coordinates, under its children.
///
/// The source is rendered through the hidden visual surface of the visual
/// ([`ServerCompositionVisualSurface::render_source_subtree`]), which keeps
/// the guard against a redirect visual inside the subtree of its own source
/// and follows the changes of the source (the visual observes the surface).
/// The bounds of the visual are its own rectangle and the bounds of the
/// content and children of the source, taken when the update pass of the
/// target reaches the visual; inside the subtree of its own source they are
/// its own rectangle only, so it draws nothing of the source there.
///
/// Deviation (winui-composition.md): when the source comes after the
/// redirect visual in the tree, a change of the extent of the source reaches
/// the bounds of the redirect visual one frame later.
#[derive(Default)]
pub struct ServerRedirectVisual {
    props: ServerRedirectVisualProps,
}

impl ServerRedirectVisual {
    pub fn new() -> Self {
        Self::default()
    }

    /// The properties of the visual.
    pub fn props(&self) -> &ServerRedirectVisualProps {
        &self.props
    }

    /// The visual whose subtree is shown.
    pub fn source(&self) -> Option<Rc<ServerCompositionVisual>> {
        self.props.source().and_then(|visual| visual.into_any_rc().downcast::<ServerCompositionVisual>().ok())
    }

    /// The hidden surface that renders the source.
    pub fn source_surface(&self) -> Option<Rc<ServerCompositionVisualSurface>> {
        self.props
            .source_surface()
            .and_then(|surface| surface.into_any_rc().downcast::<ServerCompositionVisualSurface>().ok())
    }
}

impl ServerRedirectVisualHooks for ServerCompositionVisual {
    fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        ServerCompositionVisual::base_deserialize_changes_core(self, reader, committed_at);
    }

    fn on_source_changed(&self) {
        self.invalidate_content();
    }

    fn on_source_surface_changing(&self) {
        if let Some(content) = self.content_as::<ServerRedirectVisual>() {
            observe_visual_resource(self, content.props.source_surface(), false);
        }
    }

    fn on_source_surface_changed(&self) {
        if let Some(content) = self.content_as::<ServerRedirectVisual>() {
            if let Some(surface) = content.source_surface() {
                surface.set_follows_source_bounds(true);
            }
            observe_visual_resource(self, content.props.source_surface(), true);
        }
        self.invalidate_content();
    }
}

impl IServerVisualContent for ServerRedirectVisual {
    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        self.props.deserialize_changes_core(visual, reader, committed_at);
    }

    /// The rectangle of the visual and the extent of the source subtree (not
    /// when the visual is inside that subtree: it is not drawn inside itself).
    fn compute_own_content_bounds(&self, visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        let own = ServerSizeDependantVisual::compute_own_content_bounds(visual);
        let Some(source) = self.source() else { return own };
        if visual.is_self_or_descendant_of(&source) {
            return own;
        }
        LtrbRect::full_union(own, source.unclipped_sub_tree_bounds())
    }

    fn has_update_pass_content_bounds(&self) -> bool {
        true
    }

    fn size_changed(&self, visual: &ServerCompositionVisual) {
        ServerSizeDependantVisual::size_changed(visual);
    }

    fn render_core(
        &self,
        _visual: &ServerCompositionVisual,
        context: &mut ServerVisualRenderContext<'_>,
        current_transformed_clip: LtrbRect,
    ) {
        if let Some(surface) = self.source_surface() {
            surface.render_source_subtree(context.canvas(), current_transformed_clip);
        }
    }

    /// The source subtree changed (the surface was invalidated).
    fn dependency_queued_invalidate(&self, visual: &ServerCompositionVisual) {
        visual.invalidate_content();
    }

    fn dispose(&self, visual: &ServerCompositionVisual) {
        observe_visual_resource(visual, self.props.source_surface(), false);
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
