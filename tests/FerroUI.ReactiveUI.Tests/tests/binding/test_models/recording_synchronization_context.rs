//! `RecordingSynchronizationContext` (`TestModels/RecordingSynchronizationContext.cs`).
//!
//! The context is a schedule of `ferroui-system`, given with `BindingSchedulers::use_schedule`
//! (`UseSynchronizationContext`).

use ferroui_system::task::Schedule;
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

impl Schedule for RecordingSynchronizationContext {
    fn schedule(&self, work: Box<dyn FnOnce() + Send>) {
        self.post_count.fetch_add(1, Ordering::SeqCst);
        work();
    }

    fn schedule_local(&self, work: Box<dyn FnOnce()>) {
        self.post_count.fetch_add(1, Ordering::SeqCst);
        work();
    }
}
