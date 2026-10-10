//! A signal to the worker thread that is posted at most once until it ran
//! (the port of `ServerSignaler.cs`).

use std::sync::{Arc, Mutex, PoisonError};

/// A job for the queue of the worker.
pub type SignalJob = Box<dyn FnOnce() + Send>;

/// Posts a callback to the worker, once however often it is signalled before
/// the callback runs.
///
/// The reference holds the worker and calls its `PostOob`; this type holds
/// the posting as a function, which is what lets it be made before the
/// worker and tested without one.
pub struct ServerSignaler {
    inner: Arc<Inner>,
}

struct Inner {
    signaled: Mutex<bool>,
    post: Box<dyn Fn(SignalJob) + Send + Sync>,
    callback: Box<dyn Fn() + Send + Sync>,
}

impl ServerSignaler {
    pub fn new(post: impl Fn(SignalJob) + Send + Sync + 'static, callback: impl Fn() + Send + Sync + 'static) -> Self {
        Self { inner: Arc::new(Inner { signaled: Mutex::new(false), post: Box::new(post), callback: Box::new(callback) }) }
    }

    pub fn signal(&self) {
        let mut signaled = self.inner.signaled.lock().unwrap_or_else(PoisonError::into_inner);
        if *signaled {
            return;
        }
        *signaled = true;
        let inner = self.inner.clone();
        (self.inner.post)(Box::new(move || inner.callback()));
    }
}

impl Inner {
    fn callback(&self) {
        *self.signaled.lock().unwrap_or_else(PoisonError::into_inner) = false;
        (self.callback)();
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn a_signal_is_posted_once_until_it_ran() {
        let queue: Arc<Mutex<Vec<SignalJob>>> = Arc::new(Mutex::new(Vec::new()));
        let runs = Arc::new(AtomicUsize::new(0));
        let signaler = {
            let queue = queue.clone();
            let runs = runs.clone();
            ServerSignaler::new(
                move |job| queue.lock().unwrap().push(job),
                move || {
                    runs.fetch_add(1, Ordering::SeqCst);
                },
            )
        };

        signaler.signal();
        signaler.signal();
        signaler.signal();
        assert_eq!(queue.lock().unwrap().len(), 1);

        let job = queue.lock().unwrap().pop().unwrap();
        job();
        assert_eq!(runs.load(Ordering::SeqCst), 1);

        signaler.signal();
        assert_eq!(queue.lock().unwrap().len(), 1);
    }
}
