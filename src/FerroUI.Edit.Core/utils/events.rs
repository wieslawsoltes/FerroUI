use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Explicit event subscription. Dropping a subscription does not unsubscribe it.
/// Call [`dispose`](Self::dispose) when the listener is no longer required.
#[derive(Clone)]
pub struct DocumentSubscription(Rc<SubscriptionInner>);

struct SubscriptionInner {
    disposed: Cell<bool>,
    dispose: Box<dyn Fn()>,
}

impl DocumentSubscription {
    pub fn dispose(&self) {
        if !self.0.disposed.replace(true) {
            (self.0.dispose)();
        }
    }

    pub fn is_disposed(&self) -> bool {
        self.0.disposed.get()
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
        DocumentSubscription(Rc::new(SubscriptionInner {
            disposed: Cell::new(false),
            dispose: Box::new(move || {
                if let Some(handlers) = weak.upgrade() {
                    handlers
                        .borrow_mut()
                        .retain(|(entry, _)| !Rc::ptr_eq(entry, &id));
                }
            }),
        }))
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
