//! `AsyncLock` (`AsyncLock.cs`): runs actions one after another; an action queued while another
//! runs (by that action, on a single thread) runs after it, in the order it was queued.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

/// Runs queued actions one at a time (`AsyncLock`).
///
/// Upstream serializes threads with a lock; on one thread the lock is the re-entrancy guard: an
/// action queued from inside another runs when that one has returned. A panicking action leaves
/// the lock faulted: the actions still queued are dropped and later ones are ignored, as upstream
/// clears its queue and ignores later calls after an exception.
#[derive(Default)]
pub struct AsyncLock {
    queue: RefCell<VecDeque<Box<dyn FnOnce()>>>,
    is_acquired: Cell<bool>,
    has_faulted: Cell<bool>,
}

impl AsyncLock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `action` and runs the queue if no action is running (`Wait`).
    pub fn wait(&self, action: impl FnOnce() + 'static) {
        if self.has_faulted.get() {
            return;
        }
        self.queue.borrow_mut().push_back(Box::new(action));
        if self.is_acquired.replace(true) {
            return;
        }
        struct Release<'a>(&'a AsyncLock);
        impl Drop for Release<'_> {
            fn drop(&mut self) {
                if std::thread::panicking() {
                    self.0.has_faulted.set(true);
                    self.0.queue.borrow_mut().clear();
                }
                self.0.is_acquired.set(false);
            }
        }
        let _release = Release(self);
        loop {
            let next = self.queue.borrow_mut().pop_front();
            match next {
                Some(work) => work(),
                None => break,
            }
        }
    }

    /// Drops the queued actions and ignores later ones (`Dispose`).
    pub fn dispose(&self) {
        self.queue.borrow_mut().clear();
        self.has_faulted.set(true);
    }
}
