use super::server::ServerCompositionVisual;
use crate::media::ImmediateDrawingContext;
use crate::platform::LtrbRect;
use crate::{Matrix, Point, Rect, Vector};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The state upstream's abstract `CompositionCustomVisualHandler` keeps,
/// and its protected members: what a handler class embeds and returns from
/// [`ICompositionCustomVisualHandler::handler_base`].
///
/// The handler is created on the UI thread and used on the server only: it
/// is attached to the server-side custom visual when that visual is created.
#[derive(Default)]
pub struct CompositionCustomVisualHandler {
    host: RefCell<Option<Weak<ServerCompositionVisual>>>,
    in_render: Cell<bool>,
    current_transformed_clip: Cell<Rect>,
    current_transform: Cell<Matrix>,
}

/// The overridable members of a custom visual handler. A handler class
/// embeds a [`CompositionCustomVisualHandler`] and implements this trait.
pub trait ICompositionCustomVisualHandler: 'static {
    /// The embedded handler state.
    fn handler_base(&self) -> &CompositionCustomVisualHandler;

    /// A message sent with `CompositionCustomVisual::send_handler_message`.
    fn on_message(&self, _message: Rc<dyn Any>) {}

    fn on_animation_frame_update(&self) {}

    fn on_render(&self, drawing_context: &mut ImmediateDrawingContext<'_>);

    /// The bounds of what the handler draws; the effective size of the
    /// visual by default.
    fn get_render_bounds(&self) -> Rect {
        let size = self.handler_base().effective_size();
        Rect::new(0.0, 0.0, size.x, size.y)
    }
}

impl CompositionCustomVisualHandler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders through `handler`: the render APIs are available while the
    /// handler draws.
    pub(crate) fn render(
        handler: &dyn ICompositionCustomVisualHandler,
        drawing_context: &mut ImmediateDrawingContext<'_>,
        current_transformed_clip: Rect,
    ) {
        let base = handler.handler_base();
        base.in_render.set(true);
        base.current_transformed_clip.set(current_transformed_clip);
        base.current_transform.set(drawing_context.current_transform());
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&base.in_render);
        handler.on_render(drawing_context);
    }

    fn host(&self) -> Rc<ServerCompositionVisual> {
        match self.host.borrow().as_ref().and_then(Weak::upgrade) {
            Some(host) => host,
            None => panic!("Object is not yet attached to the compositor"),
        }
    }

    /// `VerifyAccess`: the handler must be attached. Upstream also checks
    /// that the caller is on the render thread; the server compositor runs
    /// on the thread of its compositor here, so there is nothing more to
    /// check.
    fn verify_access(&self) -> Rc<ServerCompositionVisual> {
        self.host()
    }

    fn verify_in_render(&self) {
        self.verify_access();
        if !self.in_render.get() {
            panic!("This API is only available from OnRender");
        }
    }

    /// The size of the visual.
    pub fn effective_size(&self) -> Vector {
        self.verify_access().size()
    }

    /// The time of the server compositor.
    pub fn composition_now(&self) -> Duration {
        self.verify_access().compositor().map(|c| c.server_now()).unwrap_or_default()
    }

    pub(crate) fn attach(&self, visual: Weak<ServerCompositionVisual>) {
        *self.host.borrow_mut() = Some(visual);
    }

    /// Asks for the visual to be redrawn.
    pub fn invalidate(&self) {
        self.verify_access().invalidate_content();
    }

    /// Asks for an area of the visual to be redrawn.
    pub fn invalidate_rect(&self, rc: Rect) {
        self.verify_access().add_extra_dirty_rect(LtrbRect::from_rect(rc));
    }

    /// Asks for [`ICompositionCustomVisualHandler::on_animation_frame_update`]
    /// to be called on the next frame.
    pub fn register_for_next_animation_frame_update(&self) {
        let host = self.verify_access();
        super::server::ServerCompositionCustomVisual::handler_register_for_next_animation_frame_update(&host);
    }

    /// Whether a point of the visual is inside the current clip.
    pub fn render_clip_contains(&self, pt: Point) -> bool {
        self.verify_in_render();
        let pt = pt.transform(self.current_transform.get());
        self.current_transformed_clip.get().contains(pt)
    }

    /// Whether a rectangle of the visual intersects the current clip.
    pub fn render_clip_intersectes(&self, rc: Rect) -> bool {
        self.verify_in_render();
        let rc = rc.transform_to_aabb(self.current_transform.get());
        self.current_transformed_clip.get().intersects(rc)
    }
}
