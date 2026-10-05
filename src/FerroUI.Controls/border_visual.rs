use ferroui_base::platform::{IDrawingContextImpl, LtrbRect};
use ferroui_base::rendering::composition::server::{
    IServerVisualContent, ServerCompositionDrawListVisual, ServerCompositionVisual, ServerVisualRenderContext,
};
use ferroui_base::rendering::composition::transport::{BatchStreamReader, BatchStreamWriter, IRegisterForSerialization};
use ferroui_base::rendering::composition::{CompositionDrawListVisual, Compositor, ICompositionDrawListVisualExtension};
use ferroui_base::{CornerRadius, Rect, RoundedRect, Visual};
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// What the border visual adds to the UI-thread side of a draw list visual.
#[derive(Default)]
struct BorderVisualState {
    corner_radius: Cell<CornerRadius>,
    corner_radius_changed: Cell<bool>,
}

impl ICompositionDrawListVisualExtension for BorderVisualState {
    fn serialize_changes_core(&self, writer: &mut BatchStreamWriter<'_>) {
        writer.write(self.corner_radius_changed.get());
        if self.corner_radius_changed.get() {
            writer.write(self.corner_radius.get());
        }
    }
}

/// The composition visual of a border: a draw list visual that clips to
/// its bounds with the corner radius of the border.
pub(crate) struct CompositionBorderVisual {
    visual: CompositionDrawListVisual,
    state: Rc<BorderVisualState>,
}

impl CompositionBorderVisual {
    pub(crate) fn new(compositor: &Rc<Compositor>, visual: &Visual) -> Self {
        let state = Rc::new(BorderVisualState::default());
        let visual = CompositionDrawListVisual::with_extension(
            compositor,
            visual,
            || Box::new(ServerBorderVisual::default()),
            state.clone(),
        );
        Self { visual, state }
    }

    /// The visual as the draw list visual it is.
    pub(crate) fn as_draw_list_visual(&self) -> &CompositionDrawListVisual {
        &self.visual
    }

    // The getter of the property upstream; nothing reads it yet.
    #[allow(dead_code)]
    pub(crate) fn corner_radius(&self) -> CornerRadius {
        self.state.corner_radius.get()
    }

    pub(crate) fn set_corner_radius(&self, value: CornerRadius) {
        if self.state.corner_radius.get() != value {
            self.state.corner_radius_changed.set(true);
            self.state.corner_radius.set(value);
            self.visual.register_for_serialization();
        }
    }
}

/// Server-side counterpart of the border visual.
#[derive(Default)]
pub(crate) struct ServerBorderVisual {
    draw_list: ServerCompositionDrawListVisual,
    corner_radius: Cell<CornerRadius>,
}

impl ServerBorderVisual {
    #[cfg(test)]
    pub(crate) fn corner_radius(&self) -> CornerRadius {
        self.corner_radius.get()
    }
}

impl IServerVisualContent for ServerBorderVisual {
    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        self.draw_list.deserialize_changes_core(visual, reader, committed_at);
        if reader.read::<bool>() {
            self.corner_radius.set(reader.read::<CornerRadius>());
        }
    }

    fn compute_own_content_bounds(&self, visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        self.draw_list.compute_own_content_bounds(visual)
    }

    fn render_core(
        &self,
        visual: &ServerCompositionVisual,
        context: &mut ServerVisualRenderContext<'_>,
        current_transformed_clip: LtrbRect,
    ) {
        self.draw_list.render_core(visual, context, current_transformed_clip);
    }

    fn push_clip_to_bounds(&self, visual: &ServerCompositionVisual, canvas: &mut dyn IDrawingContextImpl) {
        let size = visual.size();
        let clip_rect = Rect::new(0.0, 0.0, size.x, size.y);
        if self.corner_radius.get() == CornerRadius::default() {
            canvas.push_clip(clip_rect);
        } else {
            canvas.push_clip_rounded(RoundedRect::from_corner_radius(clip_rect, self.corner_radius.get()));
        }
    }

    fn dependency_queued_invalidate(&self, visual: &ServerCompositionVisual) {
        self.draw_list.dependency_queued_invalidate(visual);
    }

    fn dispose(&self, visual: &ServerCompositionVisual) {
        self.draw_list.dispose(visual);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
