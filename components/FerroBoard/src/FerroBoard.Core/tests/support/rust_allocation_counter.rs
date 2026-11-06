//! Test/probe-only, current-thread Rust allocation requests; excludes native allocator calls.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Allocations {
    pub(crate) calls: usize,
    pub(crate) requested_bytes: usize,
}
thread_local! { static COUNTER: Cell<Option<Allocations>> = const { Cell::new(None) }; }
struct CountingAllocator;
fn record(bytes: usize) {
    let _ = COUNTER.try_with(|counter| {
        if let Some(mut value) = counter.get() {
            value.calls = value.calls.saturating_add(1);
            value.requested_bytes = value.requested_bytes.saturating_add(bytes);
            counter.set(Some(value));
        }
    });
}
// SAFETY: all operations forward their original pointer/layout contract unchanged to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: forward the caller's valid layout to the backing allocator.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: forward the caller's valid layout to the backing allocator.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        // SAFETY: forward the caller's live allocation and requested size unchanged.
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: forward the caller's matching live pointer/layout unchanged.
        unsafe { System.dealloc(pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

pub(crate) fn counted<T>(operation: impl FnOnce() -> T) -> (T, Allocations) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            COUNTER.with(|counter| counter.set(None));
        }
    }
    COUNTER.with(|counter| {
        assert!(counter.get().is_none());
        counter.set(Some(Allocations::default()));
    });
    let reset = Reset;
    let value = black_box(operation());
    let allocations = COUNTER.with(|counter| counter.take().unwrap());
    drop(reset);
    (value, allocations)
}
