//! The allocations of a measured loop: with the feature `count-allocations`
//! the global allocator of the test build of the sample is the system
//! allocator behind a count of the allocations and bytes of each thread.
//! Without the feature (the default) there is no allocator here, the test
//! build allocates as every other build does, and [`snapshot`] reads zeros.
//!
//! The allocator also counts what is alive in the process, blocks and bytes
//! ([`live`]), and tells the recorder of `allocation_trace.rs` of every
//! block. The tours of `catalog_tour.rs` read both.
//!
//! Not a port: the upstream sample has no tests. The recycling benchmark of
//! `frame_benchmark.rs` reads the counts around its measured steps:
//!
//! ```sh
//! cargo test -p control-catalog --release --features count-allocations --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
//! ```

/// Whether allocations are counted (the feature `count-allocations`).
pub const ENABLED: bool = cfg!(feature = "count-allocations");

/// What a thread has allocated so far, or between two moments
/// ([`since`](Self::since)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocationCounts {
    /// The blocks allocated (`alloc` and `alloc_zeroed`).
    pub allocations: u64,
    /// The blocks resized (`realloc`).
    pub reallocations: u64,
    /// The bytes asked for: the size of each allocated block and what each
    /// resized block grew by.
    pub bytes: u64,
}

impl AllocationCounts {
    /// What was allocated between `earlier` and these counts.
    pub fn since(&self, earlier: &AllocationCounts) -> AllocationCounts {
        AllocationCounts {
            allocations: self.allocations - earlier.allocations,
            reallocations: self.reallocations - earlier.reallocations,
            bytes: self.bytes - earlier.bytes,
        }
    }

    /// Adds `other` to these counts.
    pub fn add(&mut self, other: &AllocationCounts) {
        self.allocations += other.allocations;
        self.reallocations += other.reallocations;
        self.bytes += other.bytes;
    }
}

/// What is alive in the process: the blocks that were allocated and not
/// freed, and their bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LiveCounts {
    pub allocations: i64,
    pub bytes: i64,
}

/// What is alive in the process, on every thread; zeros without the feature
/// `count-allocations`. The records of an allocation trace are not counted.
pub fn live() -> LiveCounts {
    #[cfg(feature = "count-allocations")]
    {
        counting::live()
    }
    #[cfg(not(feature = "count-allocations"))]
    {
        LiveCounts::default()
    }
}

/// What this thread has allocated so far; zeros without the feature
/// `count-allocations`.
pub fn snapshot() -> AllocationCounts {
    #[cfg(feature = "count-allocations")]
    {
        counting::snapshot()
    }
    #[cfg(not(feature = "count-allocations"))]
    {
        AllocationCounts::default()
    }
}

#[cfg(feature = "count-allocations")]
mod counting {
    use super::AllocationCounts;
    use std::alloc::{GlobalAlloc, Layout, System};
    use super::super::allocation_trace as trace;
    use std::cell::Cell;
    use std::sync::atomic::{AtomicI64, Ordering};

    thread_local! {
        // Constants without destructors: reading them allocates nothing, so
        // the allocator may use them.
        static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
        static REALLOCATIONS: Cell<u64> = const { Cell::new(0) };
        static BYTES: Cell<u64> = const { Cell::new(0) };
    }

    // What is alive, in the process: a block is freed by any thread.
    static LIVE_ALLOCATIONS: AtomicI64 = AtomicI64::new(0);
    static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);

    fn add_live(allocations: i64, bytes: i64) {
        if trace::is_inside() {
            return;
        }
        LIVE_ALLOCATIONS.fetch_add(allocations, Ordering::Relaxed);
        LIVE_BYTES.fetch_add(bytes, Ordering::Relaxed);
    }

    pub(super) fn live() -> super::LiveCounts {
        super::LiveCounts {
            allocations: LIVE_ALLOCATIONS.load(Ordering::Relaxed),
            bytes: LIVE_BYTES.load(Ordering::Relaxed),
        }
    }

    fn add(counter: &'static std::thread::LocalKey<Cell<u64>>, amount: u64) {
        // What the recorder of the allocation trace allocates is its own.
        if trace::is_inside() {
            return;
        }
        // A thread that is ending has no counts left: nothing is counted.
        let _ = counter.try_with(|cell| cell.set(cell.get() + amount));
    }

    pub(super) fn snapshot() -> AllocationCounts {
        AllocationCounts {
            allocations: ALLOCATIONS.with(Cell::get),
            reallocations: REALLOCATIONS.with(Cell::get),
            bytes: BYTES.with(Cell::get),
        }
    }

    /// The system allocator, counting what each thread asks of it.
    struct CountingAllocator;

    // SAFETY: every request is passed to the system allocator unchanged; the
    // counting touches only thread-local cells.
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            add(&ALLOCATIONS, 1);
            add(&BYTES, layout.size() as u64);
            add_live(1, layout.size() as i64);
            // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc`.
            let ptr = unsafe { System.alloc(layout) };
            trace::allocated(ptr, layout.size());
            ptr
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            add(&ALLOCATIONS, 1);
            add(&BYTES, layout.size() as u64);
            add_live(1, layout.size() as i64);
            // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc_zeroed`.
            let ptr = unsafe { System.alloc_zeroed(layout) };
            trace::allocated(ptr, layout.size());
            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            add_live(-1, -(layout.size() as i64));
            trace::freed(ptr);
            // SAFETY: the caller upholds the contract of `GlobalAlloc::dealloc`.
            unsafe { System.dealloc(ptr, layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            add(&REALLOCATIONS, 1);
            add(&BYTES, new_size.saturating_sub(layout.size()) as u64);
            add_live(0, new_size as i64 - layout.size() as i64);
            trace::freed(ptr);
            // SAFETY: the caller upholds the contract of `GlobalAlloc::realloc`.
            let ptr = unsafe { System.realloc(ptr, layout, new_size) };
            trace::allocated(ptr, new_size);
            ptr
        }
    }

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;
}

#[test]
fn live_counts_are_read_with_the_feature_and_read_zero_without_it() {
    let block: Vec<u8> = std::hint::black_box(Vec::with_capacity(std::hint::black_box(8192)));
    let with_block = live();
    drop(block);
    if ENABLED {
        assert!(with_block.bytes >= 8192);
        assert!(with_block.allocations >= 1);
    } else {
        assert_eq!(LiveCounts::default(), with_block);
    }
}

#[test]
fn allocations_are_counted_with_the_feature_and_read_zero_without_it() {
    let before = snapshot();
    // Through `black_box`, so that an optimised build makes the allocation.
    let block: Vec<u8> = std::hint::black_box(Vec::with_capacity(std::hint::black_box(4096)));
    let counted = snapshot().since(&before);
    drop(block);
    if ENABLED {
        assert_eq!(1, counted.allocations);
        assert_eq!(4096, counted.bytes);
    } else {
        assert_eq!(AllocationCounts::default(), counted);
    }
}
