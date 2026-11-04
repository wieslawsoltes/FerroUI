//! Count-only allocator for the public property companion. Counts are calls
//! requesting allocation and cumulative requested bytes, not live memory.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static COUNTING: AtomicBool = AtomicBool::new(false);
static REQUESTS: AtomicU64 = AtomicU64::new(0);
static REQUESTED_BYTES: AtomicU64 = AtomicU64::new(0);

pub(crate) struct Counter;

fn record(bytes: usize) {
    if COUNTING.load(Ordering::Relaxed) {
        REQUESTS.fetch_add(1, Ordering::Relaxed);
        REQUESTED_BYTES.fetch_add(bytes as u64, Ordering::Relaxed);
    }
}

// SAFETY: every operation forwards its original pointer/layout to System;
// recording uses only atomics and cannot allocate or access the allocation.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        unsafe { System.realloc(pointer, layout, size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

pub(crate) struct Counts {
    pub(crate) requests: u64,
    pub(crate) requested_bytes: u64,
}

pub(crate) fn begin() {
    assert!(!COUNTING.load(Ordering::Relaxed), "nested allocation count window");
    REQUESTS.store(0, Ordering::Relaxed);
    REQUESTED_BYTES.store(0, Ordering::Relaxed);
    COUNTING.store(true, Ordering::Relaxed);
}

pub(crate) fn end() -> Counts {
    assert!(COUNTING.swap(false, Ordering::Relaxed), "allocation count window was not active");
    Counts {
        requests: REQUESTS.load(Ordering::Relaxed),
        requested_bytes: REQUESTED_BYTES.load(Ordering::Relaxed),
    }
}
