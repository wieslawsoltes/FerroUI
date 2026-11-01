//! The guard of a subscription or resource (C3b of #423, docs/idiomatic-rust-disposal.md).

use crate::IDisposable;
use std::fmt;
use std::rc::Rc;

/// A subscription or resource released when the guard is dropped.
///
/// The guard holds the [`IDisposable`] handle an API returned and disposes it
/// once: when it is dropped, or earlier with [`dispose`](Self::dispose). A
/// subscription meant to last as long as its source is kept with
/// [`detach`](Self::detach), which gives the handle back without disposing it
/// (dropping an `Rc<dyn IDisposable>` keeps the subscription alive).
#[must_use = "dropping the guard releases it; call `detach()` to keep it for the life of its source"]
pub struct Subscription(Option<Rc<dyn IDisposable>>);

impl Subscription {
    /// The guard of `handle`.
    pub fn new(handle: Rc<dyn IDisposable>) -> Self {
        Subscription(Some(handle))
    }

    /// Releases the subscription now, the same as dropping the guard.
    pub fn dispose(self) {
        drop(self);
    }

    /// Keeps the subscription for the life of its source and returns its
    /// handle, which can still dispose it.
    pub fn detach(mut self) -> Rc<dyn IDisposable> {
        self.0.take().expect("a guard holds its handle until it is detached or dropped")
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.dispose();
        }
    }
}

impl From<Rc<dyn IDisposable>> for Subscription {
    fn from(handle: Rc<dyn IDisposable>) -> Self {
        Subscription::new(handle)
    }
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Subscription")
    }
}
