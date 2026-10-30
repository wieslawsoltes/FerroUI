use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use std::rc::Rc;

/// An explicit subscription, with snapshot delivery and no borrow held across callbacks.
pub(crate) struct InkEvent<T: 'static>(Rc<HandlerList<dyn Fn(&T)>>);

impl<T: 'static> Default for InkEvent<T> {
    fn default() -> Self {
        Self(Rc::new(HandlerList::new()))
    }
}

impl<T: 'static> InkEvent<T> {
    pub fn subscribe(&self, handler: impl Fn(&T) + 'static) -> Rc<dyn IDisposable> {
        let token = self.0.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.0);
        Disposable::create(move || {
            if let Some(list) = weak.upgrade() {
                list.remove(token);
            }
        })
    }

    pub fn raise(&self, value: &T) {
        for (_, handler) in self.0.snapshot().iter() {
            handler(value);
        }
    }
}
