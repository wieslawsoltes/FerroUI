use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

#[derive(Default)]
pub(crate) struct FoldingEvents {
    next: Cell<usize>,
    handlers: RefCell<BTreeMap<usize, Rc<dyn Fn()>>>,
    depth: Cell<usize>,
    pending: Cell<bool>,
}

impl FoldingEvents {
    pub fn subscribe(self: &Rc<Self>, handler: impl Fn() + 'static) -> FoldingSubscription {
        let id = self.next.get();
        self.next.set(id + 1);
        self.handlers.borrow_mut().insert(id, Rc::new(handler));
        FoldingSubscription {
            events: Rc::downgrade(self),
            id,
        }
    }

    pub fn notify(&self) {
        if self.depth.get() != 0 {
            self.pending.set(true);
            return;
        }
        let handlers: Vec<_> = self.handlers.borrow().values().cloned().collect();
        for handler in handlers {
            handler();
        }
    }

    pub fn batch(self: &Rc<Self>) -> FoldingUpdate {
        self.depth.set(self.depth.get() + 1);
        FoldingUpdate(self.clone())
    }
}

pub(crate) struct FoldingUpdate(Rc<FoldingEvents>);
impl Drop for FoldingUpdate {
    fn drop(&mut self) {
        self.0.depth.set(self.0.depth.get() - 1);
        if self.0.depth.get() == 0 && self.0.pending.replace(false) {
            self.0.notify();
        }
    }
}

/// A folding notification subscription. Dropping or disposing it unsubscribes.
pub struct FoldingSubscription {
    events: Weak<FoldingEvents>,
    id: usize,
}
impl FoldingSubscription {
    pub fn dispose(&self) {
        if let Some(events) = self.events.upgrade() {
            events.handlers.borrow_mut().remove(&self.id);
        }
    }
}
impl Drop for FoldingSubscription {
    fn drop(&mut self) {
        self.dispose();
    }
}
