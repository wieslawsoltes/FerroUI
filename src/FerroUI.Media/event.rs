use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
};
type Handler<T> = Rc<dyn Fn(&T)>;
struct EventInner<T> {
    next: Cell<u64>,
    handlers: RefCell<BTreeMap<u64, Handler<T>>>,
}
/// UI-thread event with reentrant delivery and deterministic subscription Drop.
pub struct MediaEvent<T>(Rc<EventInner<T>>);
impl<T> Clone for MediaEvent<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> Default for MediaEvent<T> {
    fn default() -> Self {
        Self(Rc::new(EventInner {
            next: Cell::new(0),
            handlers: RefCell::new(BTreeMap::new()),
        }))
    }
}
pub struct MediaSubscription {
    unsubscribe: Option<Box<dyn FnOnce()>>,
}
impl Drop for MediaSubscription {
    fn drop(&mut self) {
        if let Some(f) = self.unsubscribe.take() {
            f();
        }
    }
}
impl<T: 'static> MediaEvent<T> {
    pub fn subscribe(&self, handler: impl Fn(&T) + 'static) -> MediaSubscription {
        let id = self
            .0
            .next
            .get()
            .checked_add(1)
            .expect("subscription identity exhausted");
        self.0.next.set(id);
        self.0.handlers.borrow_mut().insert(id, Rc::new(handler));
        let inner: Weak<EventInner<T>> = Rc::downgrade(&self.0);
        MediaSubscription {
            unsubscribe: Some(Box::new(move || {
                if let Some(inner) = inner.upgrade() {
                    inner.handlers.borrow_mut().remove(&id);
                }
            })),
        }
    }
    pub fn emit(&self, value: &T) {
        let handlers: Vec<_> = self
            .0
            .handlers
            .borrow()
            .iter()
            .map(|(id, h)| (*id, h.clone()))
            .collect();
        for (id, handler) in handlers {
            if self.0.handlers.borrow().contains_key(&id) {
                handler(value);
            }
        }
    }
}
