//! The heap of the process, counted by the allocator of the test: with the feature `counters`
//! every block the program allocates and frees is counted, on every thread, so that a start can
//! say how many blocks and bytes each of its phases left allocated, and of which sizes. Without
//! the feature the allocator is the system's and every count is zero.

/// The upper size, in bytes, of each class the live blocks are counted in. The last class has
/// every larger block.
pub const CLASSES: [usize; 12] = [16, 32, 48, 64, 96, 128, 192, 256, 512, 1024, 4096, usize::MAX];

/// The heap at one moment, or the difference of two such moments.
#[derive(Clone, Copy, Default)]
pub struct Heap {
    /// The blocks allocated so far (a block that grew in place or moved counts once more).
    pub allocations: u64,
    /// The bytes of the blocks that are allocated and not freed.
    pub live_bytes: i64,
    /// The blocks that are allocated and not freed, and their bytes, by the class of their size.
    pub live_blocks_by_class: [i64; CLASSES.len()],
    pub live_bytes_by_class: [i64; CLASSES.len()],
}

impl Heap {
    /// What changed since `earlier`.
    pub fn since(&self, earlier: &Heap) -> Heap {
        let mut difference = Heap {
            allocations: self.allocations - earlier.allocations,
            live_bytes: self.live_bytes - earlier.live_bytes,
            ..Heap::default()
        };
        for class in 0..CLASSES.len() {
            difference.live_blocks_by_class[class] = self.live_blocks_by_class[class] - earlier.live_blocks_by_class[class];
            difference.live_bytes_by_class[class] = self.live_bytes_by_class[class] - earlier.live_bytes_by_class[class];
        }
        difference
    }

    /// The blocks that are allocated and not freed.
    pub fn live_blocks(&self) -> i64 {
        self.live_blocks_by_class.iter().sum()
    }

    /// One line: the allocations, the live blocks and bytes, and the classes that hold a block.
    pub fn report(&self) -> String {
        let megabytes = |bytes: i64| bytes as f64 / (1024.0 * 1024.0);
        let mut text = format!(
            "{} allocations; {} blocks, {:.1} MB left allocated",
            self.allocations,
            self.live_blocks(),
            megabytes(self.live_bytes)
        );
        let mut lower = 0;
        for (class, upper) in CLASSES.iter().enumerate() {
            let (blocks, bytes) = (self.live_blocks_by_class[class], self.live_bytes_by_class[class]);
            if blocks != 0 {
                let range = if *upper == usize::MAX { format!("over {lower}") } else { format!("{}..={upper}", lower + 1) };
                text.push_str(&format!("\n    blocks of {range} bytes: {blocks} ({:.1} MB)", megabytes(bytes)));
            }
            lower = *upper;
        }
        text
    }
}

/// The heap now: zero without the feature `counters`.
pub fn snapshot() -> Heap {
    counting::snapshot()
}

#[cfg(not(feature = "counters"))]
mod counting {
    pub(super) fn snapshot() -> super::Heap {
        super::Heap::default()
    }
}

#[cfg(feature = "counters")]
mod counting {
    use super::{Heap, CLASSES};
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicI64, AtomicU64, Ordering::Relaxed};

    static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
    static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
    static LIVE_BLOCKS_BY_CLASS: [AtomicI64; CLASSES.len()] = [const { AtomicI64::new(0) }; CLASSES.len()];
    static LIVE_BYTES_BY_CLASS: [AtomicI64; CLASSES.len()] = [const { AtomicI64::new(0) }; CLASSES.len()];

    fn class_of(size: usize) -> usize {
        CLASSES.iter().position(|upper| size <= *upper).unwrap_or(CLASSES.len() - 1)
    }

    fn block(size: usize, blocks: i64) {
        let bytes = blocks * size as i64;
        let class = class_of(size);
        LIVE_BYTES.fetch_add(bytes, Relaxed);
        LIVE_BLOCKS_BY_CLASS[class].fetch_add(blocks, Relaxed);
        LIVE_BYTES_BY_CLASS[class].fetch_add(bytes, Relaxed);
    }

    pub(super) fn snapshot() -> Heap {
        let mut heap = Heap {
            allocations: ALLOCATIONS.load(Relaxed),
            live_bytes: LIVE_BYTES.load(Relaxed),
            ..Heap::default()
        };
        for class in 0..CLASSES.len() {
            heap.live_blocks_by_class[class] = LIVE_BLOCKS_BY_CLASS[class].load(Relaxed);
            heap.live_bytes_by_class[class] = LIVE_BYTES_BY_CLASS[class].load(Relaxed);
        }
        heap
    }

    struct CountingAllocator;

    // SAFETY: every member forwards to the system allocator with the arguments it was given;
    // the counters are atomics, which do not allocate.
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATIONS.fetch_add(1, Relaxed);
            block(layout.size(), 1);
            // SAFETY: the caller upholds the contract of `alloc`.
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            ALLOCATIONS.fetch_add(1, Relaxed);
            block(layout.size(), 1);
            // SAFETY: the caller upholds the contract of `alloc_zeroed`.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            block(layout.size(), -1);
            // SAFETY: the caller upholds the contract of `dealloc`.
            unsafe { System.dealloc(pointer, layout) }
        }

        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            ALLOCATIONS.fetch_add(1, Relaxed);
            block(layout.size(), -1);
            block(new_size, 1);
            // SAFETY: the caller upholds the contract of `realloc`.
            unsafe { System.realloc(pointer, layout, new_size) }
        }
    }

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;
}
