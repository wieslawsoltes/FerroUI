use ferroui_system::{events::Event, IDisposable};
use std::{
    cell::Cell,
    ops::Range,
    rc::{Rc, Weak},
};

/// Completed token ranges from one synchronous tokenization batch. All ranges
/// are zero-based and half-open, like the editor's Rust line APIs. The revision
/// and line count identify the input without a reference cycle back to its model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelTokensChangedEvent {
    pub ranges: Vec<Range<usize>>,
    pub revision: u64,
    pub line_count: usize,
    pub pending: bool,
}

pub trait IModelTokensChangedListener {
    fn model_tokens_changed(&self, event: &ModelTokensChangedEvent);
}
impl<F: Fn(&ModelTokensChangedEvent)> IModelTokensChangedListener for F {
    fn model_tokens_changed(&self, event: &ModelTokensChangedEvent) {
        self(event);
    }
}

struct ModelSubscription {
    event: Weak<Event<ModelTokensChangedEvent>>,
    token: u64,
    disposed: Cell<bool>,
}
impl IDisposable for ModelSubscription {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        if let Some(event) = self.event.upgrade() {
            event.remove(self.token);
        }
    }
}

pub(crate) fn subscribe(
    event: &Rc<Event<ModelTokensChangedEvent>>,
    handler: Rc<dyn Fn(&ModelTokensChangedEvent)>,
) -> Rc<dyn IDisposable> {
    Rc::new(ModelSubscription {
        event: Rc::downgrade(event),
        token: event.add(handler),
        disposed: Cell::new(false),
    })
}
