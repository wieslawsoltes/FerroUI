use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic identity of an owned native model, independent of collection index.
pub(crate) fn next_entity_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
