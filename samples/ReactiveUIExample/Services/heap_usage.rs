//! The heap of the process as its allocator counts it: the counterpart of the managed heap of the
//! reference (`GC.GetTotalMemory(false)`, `GC.CollectionCount(0)`).
//!
//! Not an upstream file. Rust has no garbage collector whose heap the program can ask for; the
//! example installs an allocator that forwards to the allocator of the system and counts the bytes
//! it holds and the allocations it made.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// The bytes the allocator holds.
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);

/// The allocations the allocator made.
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

/// The allocator of the system, counting what it holds.
struct CountingAllocator;

// SAFETY: every call is forwarded to the allocator of the system with the same arguments; the
// counters do not affect the memory that is returned.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc`.
        let block = unsafe { System.alloc(layout) };
        if !block.is_null() {
            ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        block
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc_zeroed`.
        let block = unsafe { System.alloc_zeroed(layout) };
        if !block.is_null() {
            ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        block
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds the contract of `GlobalAlloc::dealloc`.
        unsafe { System.dealloc(block, layout) };
        ALLOCATED_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller upholds the contract of `GlobalAlloc::realloc`.
        let moved = unsafe { System.realloc(block, layout, new_size) };
        if !moved.is_null() {
            ALLOCATED_BYTES.fetch_add(new_size, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        moved
    }
}

// Deviation (DEVIATIONS.md, samples): the managed heap of the reference is the heap the allocator of
// the example counts.
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// The bytes the heap of the process holds (`GC.GetTotalMemory(false)`).
pub fn heap_bytes() -> usize {
    ALLOCATED_BYTES.load(Ordering::Relaxed)
}

/// The allocations made since the process started (the browser service reports it where the
/// reference reports `GC.CollectionCount(0)`).
pub fn heap_allocation_count() -> u64 {
    ALLOCATIONS.load(Ordering::Relaxed)
}
