use ferroui_system::events::Event;
use std::{
    ops::Range,
    rc::{Rc, Weak},
};

/// Lines whose tokens were recomputed or cleared. All ranges are zero-based
/// and half-open. The revision and line count identify the input without a
/// reference cycle back to its model.
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

/// Retains a tokenization listener until dropped.
#[must_use = "keep the guard for as long as the listener is needed"]
pub struct ModelSubscription {
    event: Weak<Event<ModelTokensChangedEvent>>,
    token: u64,
}
impl Drop for ModelSubscription {
    fn drop(&mut self) {
        if let Some(event) = self.event.upgrade() {
            event.remove(self.token);
        }
    }
}

pub(crate) fn subscribe(
    event: &Rc<Event<ModelTokensChangedEvent>>,
    handler: Rc<dyn Fn(&ModelTokensChangedEvent)>,
) -> ModelSubscription {
    ModelSubscription {
        event: Rc::downgrade(event),
        token: event.add(handler),
    }
}
