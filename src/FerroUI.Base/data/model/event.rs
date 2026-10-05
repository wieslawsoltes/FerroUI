use crate::utilities::HandlerList;
use std::rc::Rc;

/// A multicast event of a model object: the equivalent of a managed `event`
/// member on a type that is not part of the class hierarchy.
///
/// Handlers are identified by the token returned from [`add`](Self::add).
/// Raising works on a snapshot, so handlers may add or remove handlers.
pub struct Event<A: ?Sized> {
    handlers: HandlerList<dyn Fn(&A)>,
}

impl<A: ?Sized> Default for Event<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: ?Sized> Event<A> {
    pub fn new() -> Self {
        Self { handlers: HandlerList::new() }
    }

    /// An event whose handler tokens come from `ids`, a counter shared with
    /// other handler lists (see [`HandlerList::with_shared_ids`]).
    pub fn with_shared_ids(ids: std::rc::Rc<std::cell::Cell<u64>>) -> Self {
        Self { handlers: HandlerList::with_shared_ids(ids) }
    }

    /// The current handlers with their tokens; later changes do not affect
    /// it.
    pub fn snapshot(&self) -> Rc<Vec<(u64, Rc<dyn Fn(&A)>)>> {
        self.handlers.snapshot()
    }

    /// Adds a handler; returns the token that removes it.
    pub fn add(&self, handler: Rc<dyn Fn(&A)>) -> u64 {
        self.handlers.add(handler)
    }

    /// Removes a handler. Returns false if the token is unknown.
    pub fn remove(&self, token: u64) -> bool {
        self.handlers.remove(token)
    }

    /// Whether any handler is subscribed.
    pub fn has_handlers(&self) -> bool {
        !self.handlers.is_empty()
    }

    /// The number of subscribed handlers.
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Invokes the handlers.
    pub fn raise(&self, args: &A) {
        if self.handlers.is_empty() {
            return;
        }
        for (_, handler) in self.handlers.snapshot().iter() {
            handler(args);
        }
    }
}
