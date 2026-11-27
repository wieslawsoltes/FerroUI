use std::cell::RefCell;
use std::rc::Rc;

/// Keeps a document listener registered for the lifetime of the guard.
#[must_use = "keep the guard for as long as the listener is needed"]
pub struct DocumentSubscription(RefCell<Option<Box<dyn FnOnce()>>>);

impl DocumentSubscription {
    pub fn dispose(&self) {
        let unsubscribe = self.0.borrow_mut().take();
        if let Some(unsubscribe) = unsubscribe {
            unsubscribe();
        }
    }

    pub fn is_disposed(&self) -> bool {
        self.0.borrow().is_none()
    }

    /// Keeps the listener registered for the lifetime of the event source.
    pub fn detach(mut self) {
        self.0.get_mut().take();
    }
}

impl Drop for DocumentSubscription {
    fn drop(&mut self) {
        if let Some(unsubscribe) = self.0.get_mut().take() {
            unsubscribe();
        }
    }
}

type Handler<T> = Rc<dyn Fn(&T)>;
type Handlers<T> = Rc<RefCell<Vec<(Rc<()>, Handler<T>)>>>;

pub(crate) struct Event<T: 'static>(Handlers<T>);

impl<T: 'static> Default for Event<T> {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(Vec::new())))
    }
}

impl<T: 'static> Event<T> {
    pub fn subscribe(&self, handler: impl Fn(&T) + 'static) -> DocumentSubscription {
        let id = Rc::new(());
        self.0.borrow_mut().push((id.clone(), Rc::new(handler)));
        let weak = Rc::downgrade(&self.0);
        DocumentSubscription(RefCell::new(Some(Box::new(move || {
            if let Some(handlers) = weak.upgrade() {
                let removed = {
                    let mut handlers = handlers.borrow_mut();
                    handlers
                        .iter()
                        .position(|(entry, _)| Rc::ptr_eq(entry, &id))
                        .map(|index| handlers.remove(index))
                };
                drop(removed);
            }
        }))))
    }

    pub fn raise(&self, value: &T) {
        // Subscribers may add/remove listeners or inspect their document. Never
        // call application code with the listener collection borrowed.
        let handlers: Vec<_> = self.0.borrow().iter().map(|(_, h)| h.clone()).collect();
        for handler in handlers {
            handler(value);
        }
    }
}
