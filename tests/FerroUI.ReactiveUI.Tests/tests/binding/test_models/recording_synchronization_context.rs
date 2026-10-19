//! `RecordingSynchronizationContext` (`TestModels/RecordingSynchronizationContext.cs`).

use ferroui_system::threading::SynchronizationContext;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A context that counts what is posted to it and runs it at once
/// (`RecordingSynchronizationContext`).
#[derive(Default)]
pub struct RecordingSynchronizationContext {
    post_count: AtomicUsize,
}

#[allow(dead_code)]
impl RecordingSynchronizationContext {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// The number of posts (`PostCount`).
    pub fn post_count(&self) -> usize {
        self.post_count.load(Ordering::SeqCst)
    }
}

impl SynchronizationContext for RecordingSynchronizationContext {
    fn post(&self, work: Box<dyn FnOnce() + Send>) {
        self.post_count.fetch_add(1, Ordering::SeqCst);
        work();
    }

    fn send(&self, work: Box<dyn FnOnce() + Send>) {
        work();
    }

    fn post_local(&self, work: Box<dyn FnOnce()>) {
        self.post_count.fetch_add(1, Ordering::SeqCst);
        work();
    }

    fn send_local(&self, work: Box<dyn FnOnce()>) {
        work();
    }
}
