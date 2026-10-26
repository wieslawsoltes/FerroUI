use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::{Rc, Weak};

type Callback<T> = dyn Fn(&T);
struct EventInner<T> {
    next_id: Cell<u64>,
    callbacks: RefCell<BTreeMap<u64, Weak<Callback<T>>>>,
    pending: RefCell<VecDeque<T>>,
    emitting: Cell<bool>,
}
pub(crate) struct Event<T>(Rc<EventInner<T>>);
impl<T> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> Default for Event<T> {
    fn default() -> Self {
        Self(Rc::new(EventInner {
            next_id: Cell::new(0),
            callbacks: RefCell::new(BTreeMap::new()),
            pending: RefCell::new(VecDeque::new()),
            emitting: Cell::new(false),
        }))
    }
}
/// Disconnects its callback on drop. Keep this value while observing a service.
///
/// Event sources retain callbacks weakly, so dropping an observer also releases
/// its captures even when the event source remains alive.
#[must_use = "dropping the subscription disconnects the callback"]
pub struct Subscription {
    _disconnect: Box<dyn FnOnce()>,
}
impl Drop for Subscription {
    fn drop(&mut self) {
        let disconnect = std::mem::replace(&mut self._disconnect, Box::new(|| {}));
        disconnect();
    }
}
impl<T: 'static> Event<T> {
    pub fn subscribe(&self, callback: impl Fn(&T) + 'static) -> Subscription {
        let id = self.0.next_id.get();
        self.0.next_id.set(
            id.checked_add(1)
                .expect("subscription identifier exhausted"),
        );
        let callback: Rc<Callback<T>> = Rc::new(callback);
        self.0
            .callbacks
            .borrow_mut()
            .insert(id, Rc::downgrade(&callback));
        let event = Rc::downgrade(&self.0);
        Subscription {
            _disconnect: Box::new(move || {
                if let Some(event) = event.upgrade() {
                    event.callbacks.borrow_mut().remove(&id);
                }
                drop(callback);
            }),
        }
    }
    pub fn emit(&self, value: T) {
        self.0.pending.borrow_mut().push_back(value);
        if self.0.emitting.replace(true) {
            return;
        }
        struct Emitting<'a>(&'a Cell<bool>);
        impl Drop for Emitting<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Emitting(&self.0.emitting);
        loop {
            let value = self.0.pending.borrow_mut().pop_front();
            let Some(value) = value else {
                break;
            };
            let ids: Vec<_> = self.0.callbacks.borrow().keys().copied().collect();
            for id in ids {
                let callback = self.0.callbacks.borrow().get(&id).and_then(Weak::upgrade);
                if let Some(callback) = callback {
                    callback(&value);
                }
            }
        }
    }
    pub fn listener_count(&self) -> usize {
        self.0.callbacks.borrow().len()
    }
}
