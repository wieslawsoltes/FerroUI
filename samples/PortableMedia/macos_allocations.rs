//! Development-only live Rust allocation accounting, separate from native malloc.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
static BYTES: AtomicU64 = AtomicU64::new(0);
static BLOCKS: AtomicU64 = AtomicU64::new(0);
struct Tracked;
#[global_allocator]
static ALLOCATOR: Tracked = Tracked;
unsafe impl GlobalAlloc for Tracked {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            BLOCKS.fetch_add(1, Ordering::Relaxed);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(layout);
        if !p.is_null() {
            BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            BLOCKS.fetch_add(1, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        BYTES.fetch_sub(layout.size() as u64, Ordering::Relaxed);
        BLOCKS.fetch_sub(1, Ordering::Relaxed);
        System.dealloc(p, layout)
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = System.realloc(p, layout, size);
        if !next.is_null() {
            if size >= layout.size() {
                BYTES.fetch_add((size - layout.size()) as u64, Ordering::Relaxed);
            } else {
                BYTES.fetch_sub((layout.size() - size) as u64, Ordering::Relaxed);
            }
        }
        next
    }
}
pub fn live() -> (u64, u64) {
    (
        BYTES.load(Ordering::Relaxed),
        BLOCKS.load(Ordering::Relaxed),
    )
}
