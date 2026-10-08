use super::generated::CompositionContainerVisualProps;
use super::server::{ServerCompositionCustomVisual, ServerCompositionVisual};
use super::visual::CompositionVisualKind;
use super::{CompositionVisual, Compositor, ICompositionCustomVisualHandler};
use std::any::Any;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// What a custom visual adds to the container visual: the messages for its
/// handler that wait for the next commit.
pub(crate) struct CustomVisualData {
    props: CompositionContainerVisualProps,
    messages: RefCell<Option<Vec<std::sync::Arc<dyn Any + Send + Sync>>>>,
}

impl CustomVisualData {
    pub(crate) fn props(&self) -> &CompositionContainerVisualProps {
        &self.props
    }
}

/// A container visual that draws through a handler on the render thread.
#[derive(Clone)]
pub struct CompositionCustomVisual(Rc<CompositionVisual>);

impl Deref for CompositionCustomVisual {
    type Target = Rc<CompositionVisual>;

    fn deref(&self) -> &Rc<CompositionVisual> {
        &self.0
    }
}

impl CompositionCustomVisual {
    pub(crate) fn new(
        compositor: &Rc<Compositor>,
        handler: impl FnOnce() -> Rc<dyn ICompositionCustomVisualHandler> + Send + 'static,
    ) -> CompositionCustomVisual {
        let data = CustomVisualData { props: CompositionContainerVisualProps::new(), messages: RefCell::new(None) };
        CompositionCustomVisual(CompositionVisual::create_with(
            compositor,
            CompositionVisualKind::Custom(data),
            move || Box::new(ServerCompositionCustomVisual::new(handler())),
            ServerCompositionCustomVisual::attach,
        ))
    }

    /// The handle of `visual` as a custom visual, if it is one.
    pub fn from_visual(visual: &Rc<CompositionVisual>) -> Option<CompositionCustomVisual> {
        matches!(visual.kind, CompositionVisualKind::Custom(_)).then(|| CompositionCustomVisual(visual.clone()))
    }

    fn data(&self) -> &CustomVisualData {
        match &self.0.kind {
            CompositionVisualKind::Custom(data) => data,
            _ => unreachable!("a custom visual handle wraps a custom visual"),
        }
    }

    /// Sends a message to the handler, which receives it on the render
    /// thread with the next commit.
    pub fn send_handler_message(&self, message: std::sync::Arc<dyn Any + Send + Sync>) {
        let first = self.data().messages.borrow().is_none();
        if first {
            *self.data().messages.borrow_mut() = Some(Vec::new());
            let visual = Rc::downgrade(&self.0);
            self.compositor().request_composition_update(move || {
                if let Some(visual) = visual.upgrade() {
                    CompositionCustomVisual(visual).on_composition_update();
                }
            });
        }
        if let Some(messages) = self.data().messages.borrow_mut().as_mut() {
            messages.push(message);
        }
    }

    fn on_composition_update(&self) {
        let Some(messages) = self.data().messages.borrow_mut().take() else { return };

        let server = self.server();
        self.compositor().post_server_job(
            move |compositor| {
                let visual = compositor.get::<ServerCompositionVisual>(server);
                if let Some(content) = visual.as_ref().and_then(|v| v.content_as::<ServerCompositionCustomVisual>()) {
                    content.dispatch_messages(messages);
                }
            },
            false,
        );
    }
}
