use super::{IServerRenderResource, IServerRenderResourceObserver, IServerVisualContent, ServerCompositionVisual, ServerVisualRenderContext};
use crate::platform::LtrbRect;
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
        if let Some(commands) = commands {
            commands.render(context.canvas());
        }
    }

    fn dependency_queued_invalidate(&self, visual: &ServerCompositionVisual) {
        visual.invalidate_content();
    }

    fn dispose(&self, _visual: &ServerCompositionVisual) {
        *self.render_commands.borrow_mut() = None;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
