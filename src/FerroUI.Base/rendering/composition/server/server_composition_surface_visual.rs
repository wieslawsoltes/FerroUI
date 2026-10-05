use super::{IServerCompositionSurface, IServerVisualContent, ServerCompositionTarget, ServerCompositionVisual, ServerSizeDependantVisual, ServerVisualRenderContext};
use crate::platform::LtrbRect;
use crate::rendering::composition::generated::{
    ServerCompositionSurfaceVisualHooks, ServerCompositionSurfaceVisualProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Rect, Size};
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::time::Duration;

/// Server-side counterpart of `CompositionSurfaceVisual`.
#[derive(Default)]
pub struct ServerCompositionSurfaceVisual {
    props: ServerCompositionSurfaceVisualProps,
}

impl ServerCompositionSurfaceVisual {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn surface(&self) -> Option<Rc<dyn IServerCompositionSurface>> {
        self.props.surface().and_then(|surface| surface.as_surface())
    }

    fn subscribe(&self, visual: &ServerCompositionVisual) {
        if let Some(surface) = self.surface() {
            let weak = visual.weak();
            surface.changed().add(
                weak.as_ptr() as usize,
                Rc::new(move || {
                    // OnSurfaceInvalidated
                    if let Some(visual) = weak.upgrade() {
                        visual.invalidate_content();
                    }
                }),
            );
        }
    }

    fn unsubscribe(&self, visual: &ServerCompositionVisual) {
        if let Some(surface) = self.surface() {
            surface.changed().remove(visual.weak().as_ptr() as usize);
        }
    }
}

impl ServerCompositionSurfaceVisualHooks for ServerCompositionVisual {
    fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        ServerCompositionVisual::base_deserialize_changes_core(self, reader, committed_at);
    }

    fn on_surface_changed(&self) {
        if let Some(content) = self.content_as::<ServerCompositionSurfaceVisual>() {
            content.subscribe(self);
        }
    }

    fn on_surface_changing(&self) {
        if let Some(content) = self.content_as::<ServerCompositionSurfaceVisual>() {
            content.unsubscribe(self);
        }
    }
}

impl IServerVisualContent for ServerCompositionSurfaceVisual {
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
        let Some(surface) = self.surface() else { return };
        let Some(bmp) = surface.bitmap() else { return };

        //TODO: add a way to always render the whole bitmap instead of just assuming 96 DPI
        let size = visual.size();
        context.canvas().draw_bitmap(
            &*bmp,
            1.0,
            Rect::from_size(bmp.pixel_size().to_size(1.0)),
            Rect::from_size(Size::new(size.x, size.y)),
        );
    }

    fn on_attached_to_root(&self, visual: &ServerCompositionVisual, _target: &Rc<ServerCompositionTarget>) {
        self.subscribe(visual);
    }

    fn on_detached_from_root(&self, visual: &ServerCompositionVisual, _target: &Rc<ServerCompositionTarget>) {
        self.unsubscribe(visual);
    }

    fn dispose(&self, visual: &ServerCompositionVisual) {
        self.unsubscribe(visual);
    }

    fn find_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
