use std::cell::Cell;
use std::rc::Rc;

/// Deferral class for notify that a work done in RefreshRequested event is
/// done.
pub struct RefreshCompletionDeferral {
    deferred_action: Option<Rc<dyn Fn()>>,
    defer_count: Cell<i32>,
}

impl RefreshCompletionDeferral {
    /// Creates a deferral that runs `deferred_action` when every deferral
    /// taken with [`get`](Self::get) has been completed.
    pub fn new(deferred_action: impl Fn() + 'static) -> Rc<Self> {
        Self::from_action(Some(Rc::new(deferred_action)))
    }

    /// Creates a deferral for an action that may be absent.
    pub fn from_action(deferred_action: Option<Rc<dyn Fn()>>) -> Rc<Self> {
        Rc::new(Self { deferred_action, defer_count: Cell::new(0) })
    }

    pub fn complete(&self) {
        self.defer_count.set(self.defer_count.get() - 1);

        if self.defer_count.get() == 0 {
            if let Some(deferred_action) = &self.deferred_action {
                deferred_action();
            }
        }
    }

    pub fn get(self: &Rc<Self>) -> Rc<Self> {
        self.defer_count.set(self.defer_count.get() + 1);

        self.clone()
    }
}

/// A deferral is a reference object: it compares by identity.
impl PartialEq for RefreshCompletionDeferral {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
