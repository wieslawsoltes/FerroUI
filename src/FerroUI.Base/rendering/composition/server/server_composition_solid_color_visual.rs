use super::{CompositionProperty, IServerVisualContent, ServerCompositionVisual, ServerSizeDependantVisual, ServerVisualRenderContext};
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{BoxShadows, Color};
use crate::platform::LtrbRect;
use crate::rendering::composition::generated::{
    ServerCompositionContainerVisualProps, ServerCompositionSolidColorVisualHooks, ServerCompositionSolidColorVisualProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Rect, RoundedRect};
use std::any::{Any, TypeId};
use std::time::Duration;

/// Server-side counterpart of `CompositionSolidColorVisual`.
#[derive(Default)]
pub struct ServerCompositionSolidColorVisual {
    props: ServerCompositionSolidColorVisualProps,
}

impl ServerCompositionSolidColorVisual {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(&self) -> Color {
        self.props.color()
    }
}

impl ServerCompositionSolidColorVisualHooks for ServerCompositionVisual {
    fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        ServerCompositionVisual::base_deserialize_changes_core(self, reader, committed_at);
    }
}

impl IServerVisualContent for ServerCompositionSolidColorVisual {
    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        self.props.deserialize_changes_core(visual, reader, committed_at);
    }

    fn compute_own_content_bounds(&self, visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        ServerSizeDependantVisual::compute_own_content_bounds(visual)
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
        let size = visual.size();
        context.canvas().draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(self.color())),
            None,
            RoundedRect::from_rect(Rect::new(0.0, 0.0, size.x, size.y)),
            &BoxShadows::default(),
        );
    }

    fn find_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn get_composition_property(&self, name: &str) -> Option<&'static CompositionProperty> {
        ServerCompositionSolidColorVisualProps::get_composition_property(name)
            .or_else(|| ServerCompositionContainerVisualProps::get_composition_property(name))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
