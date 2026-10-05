//! A minimal cooperative cancellation primitive.
//!
//! The dispatcher API takes cancellation tokens in the same places the
//! reference implementation does (run loops, queued operations, frames).
//! Tokens are cheap handles that can be shared across threads.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

type Callback = Box<dyn FnOnce() + Send>;

struct Inner {
    canceled: AtomicBool,
    callbacks: Mutex<Callbacks>,
}

#[derive(Default)]
struct Callbacks {
    next_id: u64,
    entries: Vec<(u64, Callback)>,
}

/// Signals to one or more [`CancellationToken`]s that they should be
/// canceled.
#[derive(Clone)]
pub struct CancellationTokenSource {
    inner: Arc<Inner>,
}

impl Default for CancellationTokenSource {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationTokenSource {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner { canceled: AtomicBool::new(false), callbacks: Mutex::new(Callbacks::default()) }),
        }
    }

    /// The token observing this source.
    pub fn token(&self) -> CancellationToken {
        CancellationToken { inner: Some(self.inner.clone()) }
    }

    pub fn is_cancellation_requested(&self) -> bool {
        self.inner.canceled.load(Ordering::SeqCst)
    }

    /// Requests cancellation and runs the registered callbacks on the
    /// calling thread. Subsequent calls do nothing.
    pub fn cancel(&self) {
        if self.inner.canceled.swap(true, Ordering::SeqCst) {
            return;
        }
        let entries = {
            let mut callbacks = self.inner.callbacks.lock().unwrap_or_else(PoisonError::into_inner);
            std::mem::take(&mut callbacks.entries)
        };
        for (_, callback) in entries {
            callback();
        }
    }
}

/// Propagates notification that operations should be canceled.
#[derive(Clone, Default)]
pub struct CancellationToken {
    inner: Option<Arc<Inner>>,
}

impl CancellationToken {
    /// A token that can never be canceled.
    pub const fn none() -> Self {
        Self { inner: None }
    }

    /// Whether this token is capable of being in the canceled state.
    pub fn can_be_canceled(&self) -> bool {
        self.inner.is_some()
    }

    pub fn is_cancellation_requested(&self) -> bool {
        self.inner.as_ref().is_some_and(|inner| inner.canceled.load(Ordering::SeqCst))
    }

    /// Registers a callback that runs when the token is canceled. When the
    /// token is already canceled the callback runs immediately.
    pub fn register(&self, callback: impl FnOnce() + Send + 'static) -> CancellationTokenRegistration {
        let Some(inner) = &self.inner else {
            return CancellationTokenRegistration { inner: None, id: 0 };
        };
        {
            let mut callbacks = inner.callbacks.lock().unwrap_or_else(PoisonError::into_inner);
            // Checked under the lock so that a concurrent `cancel` either
            // sees the new entry or has already flipped the flag.
            if !inner.canceled.load(Ordering::SeqCst) {
                callbacks.next_id += 1;
                let id = callbacks.next_id;
                callbacks.entries.push((id, Box::new(callback)));
                return CancellationTokenRegistration { inner: Some(inner.clone()), id };
            }
        }
        callback();
        CancellationTokenRegistration { inner: None, id: 0 }
    }
}

/// A callback registered with a [`CancellationToken`].
pub struct CancellationTokenRegistration {
    inner: Option<Arc<Inner>>,
    id: u64,
}

impl CancellationTokenRegistration {
    /// Removes the callback so that it no longer runs on cancellation.
    pub fn dispose(&self) {
        if let Some(inner) = &self.inner {
            let removed = {
                let mut callbacks = inner.callbacks.lock().unwrap_or_else(PoisonError::into_inner);
                callbacks.entries.iter().position(|(id, _)| *id == self.id).map(|i| callbacks.entries.remove(i))
            };
            drop(removed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn none_cannot_be_canceled() {
        let token = CancellationToken::none();
        assert!(!token.can_be_canceled());
        assert!(!token.is_cancellation_requested());
    }

    #[test]
    fn cancel_runs_callbacks_once() {
        let count = Arc::new(AtomicUsize::new(0));
        let source = CancellationTokenSource::new();
        let token = source.token();
        let c = count.clone();
        token.register(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        let c = count.clone();
        let removed = token.register(move || {
            c.fetch_add(10, Ordering::SeqCst);
        });
        removed.dispose();
        source.cancel();
        source.cancel();
        assert!(token.is_cancellation_requested());
        assert_eq!(count.load(Ordering::SeqCst), 1);

        let c = count.clone();
        token.register(move || {
            c.fetch_add(100, Ordering::SeqCst);
        });
        assert_eq!(count.load(Ordering::SeqCst), 101);
    }
}
