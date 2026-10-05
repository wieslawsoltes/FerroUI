use super::{IServerVisualContent, ServerCompositionDrawListVisual, ServerCompositionVisual, ServerVisualRenderContext};
use crate::platform::LtrbRect;
use crate::rendering::composition::generated::{
    ServerCompositionExperimentalAcrylicVisualHooks, ServerCompositionExperimentalAcrylicVisualProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Rect, RoundedRect};
use std::any::{Any, TypeId};
use std::time::Duration;

/// Server-side counterpart of `CompositionExperimentalAcrylicVisual`: a
/// draw list visual that draws an acrylic rectangle below its content.
#[derive(Default)]
pub struct ServerCompositionExperimentalAcrylicVisual {
    draw_list: ServerCompositionDrawListVisual,
    props: ServerCompositionExperimentalAcrylicVisualProps,
}

impl ServerCompositionExperimentalAcrylicVisual {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn props(&self) -> &ServerCompositionExperimentalAcrylicVisualProps {
        &self.props
    }
}

impl ServerCompositionExperimentalAcrylicVisualHooks for ServerCompositionVisual {
    fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        match self.content_as::<ServerCompositionExperimentalAcrylicVisual>() {
            Some(content) => content.draw_list.deserialize_changes_core(self, reader, committed_at),
            None => ServerCompositionVisual::base_deserialize_changes_core(self, reader, committed_at),
        }
    }
}

impl IServerVisualContent for ServerCompositionExperimentalAcrylicVisual {
    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        self.props.deserialize_changes_core(visual, reader, committed_at);
    }

    fn render_core(
        &self,
        visual: &ServerCompositionVisual,
        context: &mut ServerVisualRenderContext<'_>,
        current_transformed_clip: LtrbRect,
    ) {
        let corner_radius = self.props.corner_radius();
        let material = self.props.material();
        let size = visual.size();
        let rect = RoundedRect::from_corner_radius(Rect::new(0.0, 0.0, size.x, size.y), corner_radius);
        if let Some(supported) = context.canvas().as_drawing_context_with_acrylic_like_support() {
            supported.draw_rectangle_with_material(&material, rect);
        }

        self.draw_list.render_core(visual, context, current_transformed_clip);
    }

    fn compute_own_content_bounds(&self, visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        let size = visual.size();
        LtrbRect::full_union(
            self.draw_list.compute_own_content_bounds(visual),
            Some(LtrbRect::new(0.0, 0.0, size.x, size.y)),
        )
    }

    fn size_changed(&self, visual: &ServerCompositionVisual) {
        visual.enqueue_for_own_bounds_recompute();
    }

    fn dependency_queued_invalidate(&self, visual: &ServerCompositionVisual) {
        self.draw_list.dependency_queued_invalidate(visual);
    }

    fn dispose(&self, visual: &ServerCompositionVisual) {
        self.draw_list.dispose(visual);
    }

    fn find_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
