//! Opt-in, allocation-free accounting around the unchanged system allocator.
//! Counts requested Rust heap bytes, not allocator overhead or native SDK heaps.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
struct Tracking {
    bytes: AtomicU64,
    blocks: AtomicU64,
    peak: AtomicU64,
    allocated: AtomicU64,
}
impl Tracking {
    const fn new() -> Self {
        Self {
            bytes: AtomicU64::new(0),
            blocks: AtomicU64::new(0),
            peak: AtomicU64::new(0),
            allocated: AtomicU64::new(0),
        }
    }
    fn add(&self, size: usize) {
        let live = self.bytes.fetch_add(size as u64, Relaxed) + size as u64;
        self.peak.fetch_max(live, Relaxed);
        self.allocated.fetch_add(size as u64, Relaxed);
    }
}
// SAFETY: Every operation delegates the original pointer/layout/alignment to
// System. Counters use only atomics: no locks, allocation, logging or unwinding.
unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            self.add(layout.size());
            self.blocks.fetch_add(1, Relaxed);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            self.add(layout.size());
            self.blocks.fetch_add(1, Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        self.bytes.fetch_sub(layout.size() as u64, Relaxed);
        self.blocks.fetch_sub(1, Relaxed);
        unsafe { System.dealloc(p, layout) };
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(p, layout, size) };
        if !next.is_null() {
            if size >= layout.size() {
                self.add(size - layout.size());
            } else {
                self.bytes.fetch_sub((layout.size() - size) as u64, Relaxed);
            }
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: Tracking = Tracking::new();
pub fn snapshot() -> [u64; 4] {
    // Concurrent allocations can interleave between loads; these are observed
    // counters, not an atomic heap census or a sampling of native allocations.
    [
        ALLOCATOR.bytes.load(Relaxed),
        ALLOCATOR.blocks.load(Relaxed),
        ALLOCATOR.peak.load(Relaxed),
        ALLOCATOR.allocated.load(Relaxed),
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alignment_zeroing_and_resize_preserve_accounting() {
        let a = Tracking::new();
        let layout = Layout::from_size_align(128, 64).unwrap();
        unsafe {
            let p = a.alloc_zeroed(layout);
            assert!(!p.is_null());
            assert_eq!(p as usize % 64, 0);
            assert!(std::slice::from_raw_parts(p, 128).iter().all(|b| *b == 0));
            *p = 19;
            let p = a.realloc(p, layout, 256);
            assert!(!p.is_null());
            assert_eq!(*p, 19);
            assert_eq!(a.bytes.load(Relaxed), 256);
            assert_eq!(a.blocks.load(Relaxed), 1);
            let layout = Layout::from_size_align(256, 64).unwrap();
            let p = a.realloc(p, layout, 64);
            assert!(!p.is_null());
            assert_eq!(*p, 19);
            a.dealloc(p, Layout::from_size_align(64, 64).unwrap());
        }
        assert_eq!(a.bytes.load(Relaxed), 0);
        assert_eq!(a.blocks.load(Relaxed), 0);
        assert_eq!(a.peak.load(Relaxed), 256);
    }
}
