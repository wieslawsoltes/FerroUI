//! `StubViewThreadInvoker` (`TestModels/StubViewThreadInvoker.cs`).

use ferroui_reactiveui::IViewThreadInvoker;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

/// An invoker that claims the objects it is given (by identity), answers the access it is told to,
/// and queues what is posted until the test runs it (`StubViewThreadInvoker`).
pub struct StubViewThreadInvoker {
    claimed: Vec<Rc<dyn Any>>,
    posted: RefCell<VecDeque<Box<dyn FnOnce()>>>,
    /// Whether the calling thread owns the claimed objects (`HasAccess`).
    pub has_access: Cell<bool>,
    /// The number of callbacks posted (`PostCount`).
    pub post_count: Cell<usize>,
}

#[allow(dead_code)]
impl StubViewThreadInvoker {
    /// The invoker that claims `claimed` (`StubViewThreadInvoker(params object[])`).
    pub fn new(claimed: Vec<Rc<dyn Any>>) -> Rc<Self> {
        Rc::new(Self { claimed, posted: RefCell::new(VecDeque::new()), has_access: Cell::new(false), post_count: Cell::new(0) })
    }

    /// The invoker that claims nothing.
    pub fn unclaimed() -> Rc<Self> {
        Self::new(Vec::new())
    }

    /// Sets whether the calling thread owns the claimed objects.
    pub fn set_has_access(&self, has_access: bool) {
        self.has_access.set(has_access);
    }

    /// The number of callbacks posted.
    pub fn post_count(&self) -> usize {
        self.post_count.get()
    }

    /// Runs the posted callbacks, including those they post (`RunPosted`).
    pub fn run_posted(&self) {
        loop {
            let next = self.posted.borrow_mut().pop_front();
            let Some(callback) = next else {
                return;
            };
            callback();
        }
    }
}

impl IViewThreadInvoker for StubViewThreadInvoker {
    fn claims(&self, target: &Rc<dyn Any>) -> bool {
        self.claimed.iter().any(|claimed| std::ptr::addr_eq(Rc::as_ptr(claimed), Rc::as_ptr(target)))
    }

    fn check_access(&self, _target: &Rc<dyn Any>) -> bool {
        self.has_access.get()
    }

    fn post(&self, _target: &Rc<dyn Any>, callback: Box<dyn FnOnce()>) {
        self.post_count.set(self.post_count.get() + 1);
        self.posted.borrow_mut().push_back(callback);
    }
}
