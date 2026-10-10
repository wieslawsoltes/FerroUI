//! The multicast events of the backend's own classes (a C# `event Action`
//! of the reference): a list of handlers that are called in the order they
//! were added.

use ferroui_base::utilities::HandlerList;
use std::rc::Rc;

/// An event with one argument (`()` for none).
pub struct Event<A = ()> {
    handlers: HandlerList<dyn Fn(A)>,
}

impl<A> Default for Event<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A> Event<A> {
    pub fn new() -> Self {
        Self { handlers: HandlerList::new() }
    }

    /// Adds a handler (`+=`); the token removes it again.
    pub fn subscribe(&self, handler: impl Fn(A) + 'static) -> u64 {
        self.handlers.add(Rc::new(handler))
    }

    /// Removes a handler (`-=`).
    pub fn unsubscribe(&self, token: u64) -> bool {
        self.handlers.remove(token)
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl<A: Clone> Event<A> {
    /// Calls the handlers that are subscribed at the time of the call.
    pub fn invoke(&self, arg: A) {
        let snapshot = self.handlers.snapshot();
        for (_, handler) in snapshot.iter() {
            handler(arg.clone());
        }
    }
}

impl Event<()> {
    pub fn raise(&self) {
        self.invoke(());
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference: this type stands for a language feature of it.
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn handlers_run_in_order_until_they_are_removed() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let event: Event<i32> = Event::new();
        assert!(event.is_empty());
        let first = {
            let log = log.clone();
            event.subscribe(move |value| log.borrow_mut().push(("first", value)))
        };
        {
            let log = log.clone();
            event.subscribe(move |value| log.borrow_mut().push(("second", value)));
        }
        event.invoke(1);
        assert!(event.unsubscribe(first));
        assert!(!event.unsubscribe(first));
        event.invoke(2);
        assert_eq!(*log.borrow(), vec![("first", 1), ("second", 1), ("second", 2)]);
    }
}
