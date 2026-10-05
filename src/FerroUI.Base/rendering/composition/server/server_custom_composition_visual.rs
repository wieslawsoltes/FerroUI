use super::{
    IServerClockItem, IServerVisualContent, ServerCompositionTarget, ServerCompositionVisual, ServerCompositor,
    ServerVisualRenderContext,
};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::ImmediateDrawingContext;
use crate::platform::LtrbRect;
use crate::rendering::composition::server::CompositorDrawingContextProxy;
use crate::rendering::composition::{CompositionCustomVisualHandler, ICompositionCustomVisualHandler};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "a panic".to_owned()
    }
}

/// Runs a member of the handler; a panic is logged as upstream logs the
/// exception of a handler.
fn guarded(handler: &Rc<dyn ICompositionCustomVisualHandler>, member: &str, f: impl FnOnce()) {
    if let Err(error) = catch_unwind(AssertUnwindSafe(f)) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
            let message = panic_message(&*error);
            logger.log_with_values(
                Some(handler as &dyn Any),
                &format!("Exception in {}.{member} {{Error}}", std::any::type_name_of_val(&**handler)),
                &[&message],
            );
        }
    }
}

/// The clock item of a custom visual: the server compositor holds clock
/// items by handle, and the content of a visual is not one.
struct CustomVisualClock {
    visual: RefCell<Weak<ServerCompositionVisual>>,
    handler: Rc<dyn ICompositionCustomVisualHandler>,
    wants_next_animation_frame_after_tick: Cell<bool>,
    this: Weak<CustomVisualClock>,
}

impl CustomVisualClock {
    fn as_item(&self) -> Option<Rc<dyn IServerClockItem>> {
        self.this.upgrade().map(|this| this as Rc<dyn IServerClockItem>)
    }

    fn compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.visual.borrow().upgrade().and_then(|visual| visual.compositor())
    }

    fn add_to_clock(&self) {
        if let (Some(compositor), Some(item)) = (self.compositor(), self.as_item()) {
            compositor.animations().add_to_clock(item);
        }
    }

    fn remove_from_clock(&self) {
        if let (Some(compositor), Some(item)) = (self.compositor(), self.as_item()) {
            compositor.animations().remove_from_clock(&item);
        }
    }
}

impl IServerClockItem for CustomVisualClock {
    fn on_tick(&self) {
        self.wants_next_animation_frame_after_tick.set(false);
        self.handler.on_animation_frame_update();
        if !self.wants_next_animation_frame_after_tick.get() {
            self.remove_from_clock();
        }
    }
}

/// The content of the server-side counterpart of a
/// [`CompositionCustomVisual`](crate::rendering::composition::CompositionCustomVisual):
/// a container visual that draws through its handler.
pub struct ServerCompositionCustomVisual {
    handler: Rc<dyn ICompositionCustomVisualHandler>,
    clock: Rc<CustomVisualClock>,
}

impl ServerCompositionCustomVisual {
    pub(crate) fn new(handler: Rc<dyn ICompositionCustomVisualHandler>) -> Self {
        let clock = Rc::new_cyclic(|this: &Weak<CustomVisualClock>| CustomVisualClock {
            visual: RefCell::new(Weak::new()),
            handler: handler.clone(),
            wants_next_animation_frame_after_tick: Cell::new(false),
            this: this.clone(),
        });
        Self { handler, clock }
    }

    /// Attaches the handler to the visual that has this content (upstream
    /// does it in the constructor of the visual): done when the visual is
    /// created.
    pub(crate) fn attach(visual: &Rc<ServerCompositionVisual>) {
        let Some(content) = visual.content_as::<ServerCompositionCustomVisual>() else { return };
        *content.clock.visual.borrow_mut() = Rc::downgrade(visual);
        content.handler.handler_base().attach(Rc::downgrade(visual));
    }

    /// Dispatches the messages the UI thread sent to the handler.
    pub fn dispatch_messages(&self, messages: Vec<Rc<dyn Any>>) {
        for message in messages {
            guarded(&self.handler, "OnMessage", || self.handler.on_message(message));
        }
    }

    pub(crate) fn handler_register_for_next_animation_frame_update(visual: &ServerCompositionVisual) {
        let Some(content) = visual.content_as::<ServerCompositionCustomVisual>() else { return };
        content.clock.wants_next_animation_frame_after_tick.set(true);
        if visual.root().is_some() {
            content.clock.add_to_clock();
        }
    }
}

impl IServerVisualContent for ServerCompositionCustomVisual {
    fn compute_own_content_bounds(&self, _visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        Some(LtrbRect::from_rect(self.handler.get_render_bounds()))
    }

    fn on_attached_to_root(&self, _visual: &ServerCompositionVisual, _target: &Rc<ServerCompositionTarget>) {
        if self.clock.wants_next_animation_frame_after_tick.get() {
            self.clock.add_to_clock();
        }
    }

    fn on_detached_from_root(&self, _visual: &ServerCompositionVisual, _target: &Rc<ServerCompositionTarget>) {
        self.clock.remove_from_clock();
    }

    fn render_core(
        &self,
        _visual: &ServerCompositionVisual,
        context: &mut ServerVisualRenderContext<'_>,
        current_transformed_clip: LtrbRect,
    ) {
        let canvas = context.canvas();
        let is_proxy = match canvas.as_any_mut().downcast_mut::<CompositorDrawingContextProxy>() {
            Some(proxy) => {
                proxy.set_auto_flush(true);
                proxy.flush();
                true
            }
            None => false,
        };

        {
            let transform = canvas.transform();
            let mut context = ImmediateDrawingContext::borrowed_with_transform(&mut *canvas, transform);
            guarded(&self.handler, "OnRender", || {
                CompositionCustomVisualHandler::render(&*self.handler, &mut context, current_transformed_clip.to_rect())
            });
        }

        if is_proxy {
            if let Some(proxy) = canvas.as_any_mut().downcast_mut::<CompositorDrawingContextProxy>() {
                proxy.set_auto_flush(false);
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
