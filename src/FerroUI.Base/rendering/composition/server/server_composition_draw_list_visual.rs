use super::server_draw_items::draw_with_fragment;
use super::{
    IServerRenderResource, IServerRenderResourceObserver, IServerVisualContent, ServerCompositionVisual, ServerDrawItems,
    ServerVisualRenderContext,
};
use crate::platform::{IDrawingFragment, LtrbRect};
use crate::rendering::composition::drawing::ServerCompositionRenderData;
use crate::rendering::composition::transport::BatchStreamReader;
use crate::utilities::OptionCell;
use std::any::Any;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Server-side counterpart of `CompositionDrawListVisual`.
#[derive(Default)]
pub struct ServerCompositionDrawListVisual {
    render_commands: RefCell<Option<Rc<ServerCompositionRenderData>>>,
    /// What the backend kept of the commands when they were drawn, to draw
    /// them again from while they do not change: dropped when the commands
    /// or a resource of them change.
    fragment: OptionCell<Rc<dyn IDrawingFragment>>,
    /// The drawings of the visuals below that have no composition visual (section 9.42):
    /// made when the first one arrives.
    items: OnceCell<Box<ServerDrawItems>>,
}

/// The bits of the byte the changes of a draw list visual start with: what changed of its own
/// drawing (`DRAW_LIST_NEW`, `DRAW_LIST_REDRAWN` or neither), and whether changes of its
/// drawing items follow.
pub(crate) const DRAW_LIST_NEW: u8 = 1;
pub(crate) const DRAW_LIST_REDRAWN: u8 = 2;
pub(crate) const DRAW_LIST_CHANGE_MASK: u8 = 3;
pub(crate) const DRAW_LIST_ITEMS: u8 = 4;

impl ServerCompositionDrawListVisual {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render_commands(&self) -> Option<Rc<ServerCompositionRenderData>> {
        self.render_commands.borrow().clone()
    }

    /// The drawing items of the visual, when it has had any.
    pub fn draw_items(&self) -> Option<&ServerDrawItems> {
        self.items.get().map(|items| &**items)
    }
}

impl IServerVisualContent for ServerCompositionDrawListVisual {
    fn compute_own_content_bounds(&self, _visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        let own = self.render_commands.borrow().as_ref().and_then(|commands| commands.bounds());
        match self.items.get() {
            Some(items) => LtrbRect::full_union(own, items.bounds()),
            None => own,
        }
    }

    fn draw_items(&self) -> Option<&ServerDrawItems> {
        ServerCompositionDrawListVisual::draw_items(self)
    }

    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        let changes = reader.read::<u8>();
        let change = changes & DRAW_LIST_CHANGE_MASK;
        if change == DRAW_LIST_REDRAWN {
            // The same drawing, drawn again (section 9.14).
            self.fragment.set(None);
            visual.invalidate_content();
        }
        if change == DRAW_LIST_NEW {
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
            self.fragment.set(None);
            visual.invalidate_content();
        }
        if changes & DRAW_LIST_ITEMS != 0 {
            self.items.get_or_init(Default::default).deserialize(visual, reader);
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
        // Deviation (DEVIATIONS.md, Server composition visuals): upstream
        // replays the commands every time (`RenderCore`). A backend that
        // keeps fragments draws them from the one it kept when it can, and
        // keeps a new one when it cannot.
        draw_with_fragment(&commands, &self.fragment, context.canvas());
    }

    fn dependency_queued_invalidate(&self, visual: &ServerCompositionVisual) {
        self.fragment.set(None);
        visual.invalidate_content();
    }

    fn dispose(&self, visual: &ServerCompositionVisual) {
        *self.render_commands.borrow_mut() = None;
        self.fragment.set(None);
        if let Some(items) = self.items.get() {
            items.clear();
            visual.set_has_draw_items(false);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
