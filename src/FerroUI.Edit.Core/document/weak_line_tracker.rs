use super::{DocumentSubscription, ILineTracker, TextDocument};
use std::cell::RefCell;
use std::rc::Rc;

/// Weak tracker registration. Disposing is explicit; ignoring the handle keeps
/// tracking active while the target remains alive. An expired target removes
/// its subscription on the next notification.
#[derive(Clone)]
pub struct WeakLineTracker(Rc<RefCell<Option<DocumentSubscription>>>);
impl WeakLineTracker {
    pub fn register(document: &TextDocument, tracker: &Rc<dyn ILineTracker>) -> Self {
        let weak = Rc::downgrade(tracker);
        let slot = Rc::new(RefCell::new(None::<DocumentSubscription>));
        let callback_slot = slot.clone();
        let subscription = document.subscribe_line_events(move |event| {
            if let Some(tracker) = weak.upgrade() {
                event.dispatch(tracker.as_ref());
            } else {
                let subscription = callback_slot.borrow_mut().take();
                if let Some(subscription) = subscription {
                    subscription.dispose();
                }
            }
        });
        *slot.borrow_mut() = Some(subscription);
        Self(slot)
    }
    pub fn deregister(&self) {
        let subscription = self.0.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
    pub fn is_registered(&self) -> bool {
        self.0.borrow().as_ref().is_some_and(|s| !s.is_disposed())
    }
}
