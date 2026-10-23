use super::{IServerRenderResource, IServerRenderResourceObserver, IServerVisualContent, ServerCompositionVisual, ServerVisualRenderContext};
use crate::platform::{IDrawingFragment, LtrbRect};
use crate::rendering::composition::drawing::ServerCompositionRenderData;
use crate::rendering::composition::transport::BatchStreamReader;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Server-side counterpart of `CompositionDrawListVisual`.
#[derive(Default)]
pub struct ServerCompositionDrawListVisual {
    render_commands: RefCell<Option<Rc<ServerCompositionRenderData>>>,
    /// What the backend kept of the commands when they were drawn, to draw
    /// them again from while they do not change: dropped when the commands
    /// or a resource of them change.
    fragment: RefCell<Option<Rc<dyn IDrawingFragment>>>,
}

impl ServerCompositionDrawListVisual {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render_commands(&self) -> Option<Rc<ServerCompositionRenderData>> {
        self.render_commands.borrow().clone()
    }
}

impl IServerVisualContent for ServerCompositionDrawListVisual {
    fn compute_own_content_bounds(&self, _visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        self.render_commands.borrow().as_ref().and_then(|commands| commands.bounds())
    }

    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        if reader.read::<u8>() == 1 {
            if let Some(old) = self.render_commands.borrow_mut().take() {
                super::IServerObject::dispose(&*old);
            }
            let commands = reader.read_server_object().map(|id| {
                match visual.compositor().and_then(|c| c.get::<ServerCompositionRenderData>(id)) {
                    Some(commands) => commands,
                    None => panic!("a batch refers to render data {id:?}, which does not exist"),
                }
            });
            if let (Some(commands), Some(this)) = (&commands, visual.weak().upgrade()) {
                let observer: Rc<dyn IServerRenderResourceObserver> = this;
                commands.add_observer(&observer);
            }
            *self.render_commands.borrow_mut() = commands;
            *self.fragment.borrow_mut() = None;
            visual.invalidate_content();
        }
        visual.base_deserialize_changes_core(reader, committed_at);
    }

    fn render_core(
        &self,
        _visual: &ServerCompositionVisual,
        context: &mut ServerVisualRenderContext<'_>,
        _current_transformed_clip: LtrbRect,
    ) {
        let commands = self.render_commands.borrow().clone();
        let Some(commands) = commands else { return };
        let canvas = context.canvas();

        // Deviation (DEVIATIONS.md, Server composition visuals): upstream
        // replays the commands every time (`RenderCore`). A backend that
        // keeps fragments draws them from the one it kept when it can, and
        // keeps a new one when it cannot.
        if let Some(fragments) = canvas.as_drawing_context_impl_with_fragments() {
            let kept = self.fragment.borrow().clone();
            if kept.is_some_and(|fragment| fragments.draw_fragment(&fragment)) {
                return;
            }
            fragments.begin_fragment();
            commands.render(fragments);
            *self.fragment.borrow_mut() = fragments.end_fragment();
            return;
        }

        commands.render(canvas);
    }

    fn dependency_queued_invalidate(&self, visual: &ServerCompositionVisual) {
        *self.fragment.borrow_mut() = None;
        visual.invalidate_content();
    }

    fn dispose(&self, _visual: &ServerCompositionVisual) {
        *self.render_commands.borrow_mut() = None;
        *self.fragment.borrow_mut() = None;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
