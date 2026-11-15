//! The heap of the process, counted by the allocator of the test: with the feature `counters`
//! every block the program allocates and frees is counted, on every thread, so that a start can
//! say how many blocks and bytes each of its phases left allocated, and of which sizes. Without
//! the feature the allocator is the system's and every count is zero.
//!
//! The sizes say how much a phase left and not what it is. `FERROUI_HEAP_SITES=<phase>` (a phase
//! of a start: `build`, `show`, `first`, or `all`) also keeps, for every block allocated in that
//! phase, the frames of the call that allocated it, and the start prints the blocks that are
//! still allocated at its end by those frames (`sites_report`). The frames are addresses of the
//! program; `scripts/perf/heap-sites.py` gives them names and adds them up by function.

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

/// A phase of a start begins (`on`) or ends: the blocks allocated between the two are kept with
/// the frames that allocated them when `FERROUI_HEAP_SITES` names the phase. `all` is from the
/// beginning of `build` to the end of `first`.
pub fn sites(phase: &str, on: bool) {
    let Ok(wanted) = std::env::var("FERROUI_HEAP_SITES") else { return };
    let wanted = wanted.trim();
    if wanted == phase || (wanted == "all" && phase == if on { "build" } else { "first" }) {
        counting::sites(on);
    }
}

/// The blocks of the phases `sites` kept that are still allocated, by the frames that allocated
/// them, the most bytes first: a line `image <program> <address it is loaded at>`, a line of the
/// sums, and a line `site <bytes> <blocks> <allocations> <frames, the innermost first>` for each.
/// Nothing when no phase was kept. It ends the census.
pub fn sites_report() -> Option<String> {
    counting::sites_report()
}

#[cfg(not(feature = "counters"))]
mod counting {
    pub(super) fn snapshot() -> super::Heap {
        super::Heap::default()
    }

    pub(super) fn sites(_on: bool) {}

    pub(super) fn sites_report() -> Option<String> {
        None
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

    pub(super) fn sites(on: bool) {
        sites::keep(on);
    }

    pub(super) fn sites_report() -> Option<String> {
        sites::report()
    }

    struct CountingAllocator;

    // SAFETY: every member forwards to the system allocator with the arguments it was given;
    // the counters are atomics and the tables of the sites are the system allocator's, so
    // neither allocates through this allocator.
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATIONS.fetch_add(1, Relaxed);
            block(layout.size(), 1);
            // SAFETY: the caller upholds the contract of `alloc`.
            let pointer = unsafe { System.alloc(layout) };
            sites::allocated(pointer, layout.size());
            pointer
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            ALLOCATIONS.fetch_add(1, Relaxed);
            block(layout.size(), 1);
            // SAFETY: the caller upholds the contract of `alloc_zeroed`.
            let pointer = unsafe { System.alloc_zeroed(layout) };
            sites::allocated(pointer, layout.size());
            pointer
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            block(layout.size(), -1);
            // Before the block is given back: after it another thread may be given its address.
            sites::freed(pointer);
            // SAFETY: the caller upholds the contract of `dealloc`.
            unsafe { System.dealloc(pointer, layout) }
        }

        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            ALLOCATIONS.fetch_add(1, Relaxed);
            block(layout.size(), -1);
            block(new_size, 1);
            sites::freed(pointer);
            // SAFETY: the caller upholds the contract of `realloc`.
            let pointer = unsafe { System.realloc(pointer, layout, new_size) };
            sites::allocated(pointer, new_size);
            pointer
        }
    }

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;

    /// The blocks allocated while the census is kept, each with the frames of the call that
    /// allocated it. The frames are read from the frame records of the stack, which allocates
    /// nothing and which this allocator can therefore do for every block; the two tables are
    /// made by the system allocator when the census is first kept, so a start that keeps none
    /// has neither. One lock is held for a block, which is slow and which a census can afford.
    mod sites {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::ptr::null_mut;
        use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
        use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize};

        /// The frames kept for a block, the innermost first. The first few are the allocator's
        /// and the collection's that asked; inlined calls have no frame.
        const DEPTH: usize = 16;
        const SITES: usize = 1 << 18;
        const BLOCKS: usize = 1 << 24;
        /// The bits of a block's entry that are the index of its site; the rest is its size.
        const SITE_BITS: usize = 18;

        #[derive(Clone, Copy)]
        #[repr(C)]
        struct Site {
            /// Zero in the first frame is a site nobody has taken.
            frames: [usize; DEPTH],
            allocations: u64,
            blocks: i64,
            bytes: i64,
        }

        #[derive(Clone, Copy)]
        #[repr(C)]
        struct Block {
            /// Zero is an entry without a block.
            pointer: usize,
            size_and_site: usize,
        }

        static KEPT: AtomicBool = AtomicBool::new(false);
        static LOCKED: AtomicBool = AtomicBool::new(false);
        static SITE_TABLE: AtomicPtr<Site> = AtomicPtr::new(null_mut());
        static BLOCK_TABLE: AtomicPtr<Block> = AtomicPtr::new(null_mut());
        /// The blocks in the table, and those that were not kept because a table was full.
        static BLOCK_COUNT: AtomicUsize = AtomicUsize::new(0);
        static NOT_KEPT: AtomicUsize = AtomicUsize::new(0);

        struct Lock;

        fn lock() -> Lock {
            while LOCKED.compare_exchange_weak(false, true, Acquire, Relaxed).is_err() {
                std::hint::spin_loop();
            }
            Lock
        }

        impl Drop for Lock {
            fn drop(&mut self) {
                LOCKED.store(false, Release);
            }
        }

        pub(super) fn keep(on: bool) {
            if on && SITE_TABLE.load(Acquire).is_null() {
                let sites = Layout::array::<Site>(SITES).expect("the table of the sites has a layout");
                let blocks = Layout::array::<Block>(BLOCKS).expect("the table of the blocks has a layout");
                // SAFETY: neither layout is of size zero. Zeroed entries are empty ones.
                let (sites, blocks) = unsafe { (System.alloc_zeroed(sites), System.alloc_zeroed(blocks)) };
                assert!(!sites.is_null() && !blocks.is_null(), "no memory for the census of sites");
                BLOCK_TABLE.store(blocks.cast(), Release);
                SITE_TABLE.store(sites.cast(), Release);
            }
            KEPT.store(on, Release);
        }

        /// The return addresses of the calls that led here, the innermost first, from the frame
        /// records: a record is the address of the caller's record and the return address.
        #[cfg(all(target_arch = "aarch64", target_vendor = "apple"))]
        #[inline(always)]
        fn frames() -> [usize; DEPTH] {
            let mut frames = [0; DEPTH];
            let mut record: *const usize;
            // SAFETY: reads the frame pointer register and nothing else.
            unsafe { std::arch::asm!("mov {}, x29", out(reg) record, options(nomem, nostack, preserves_flags)) };
            for frame in &mut frames {
                if record.is_null() || record as usize % std::mem::align_of::<usize>() != 0 {
                    break;
                }
                // SAFETY: the platform keeps a frame record in every function that calls another
                // one, and the chain of them ends in a null record at the entry of the thread;
                // each is read only after the one that points to it, towards the base of the
                // stack.
                let (caller, address) = unsafe { (*record as *const usize, *record.add(1)) };
                if address == 0 {
                    break;
                }
                *frame = address;
                if caller <= record {
                    break;
                }
                record = caller;
            }
            frames
        }

        /// Where the platform does not promise a frame record in every function, every block is
        /// of one site.
        #[cfg(not(all(target_arch = "aarch64", target_vendor = "apple")))]
        #[inline(always)]
        fn frames() -> [usize; DEPTH] {
            [0; DEPTH]
        }

        #[inline(always)]
        pub(super) fn allocated(pointer: *mut u8, size: usize) {
            if KEPT.load(Relaxed) && !pointer.is_null() {
                let mut frames = frames();
                if frames[0] == 0 {
                    frames[0] = 1;
                }
                insert(pointer as usize, size, &frames);
            }
        }

        #[inline(always)]
        pub(super) fn freed(pointer: *mut u8) {
            if BLOCK_COUNT.load(Relaxed) != 0 {
                remove(pointer as usize);
            }
        }

        fn home_of_block(pointer: usize) -> usize {
            ((pointer as u64 >> 4).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - BLOCKS.trailing_zeros())) as usize
        }

        #[inline(never)]
        fn insert(pointer: usize, size: usize, frames: &[usize; DEPTH]) {
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            for frame in frames {
                hash = (hash ^ *frame as u64).wrapping_mul(0x0100_0000_01b3);
            }
            let _lock = lock();
            let (sites, blocks) = (SITE_TABLE.load(Relaxed), BLOCK_TABLE.load(Relaxed));
            if BLOCK_COUNT.load(Relaxed) >= BLOCKS / 4 * 3 {
                NOT_KEPT.fetch_add(1, Relaxed);
                return;
            }
            let mut index = (hash ^ (hash >> 29)) as usize & (SITES - 1);
            let mut probes = 0;
            // SAFETY: the tables have `SITES` and `BLOCKS` entries, every index is masked to
            // them, and the lock is held for every read and write of an entry.
            unsafe {
                let site = loop {
                    let site = &mut *sites.add(index);
                    if site.frames[0] == 0 {
                        site.frames = *frames;
                        break site;
                    }
                    if site.frames == *frames {
                        break site;
                    }
                    probes += 1;
                    if probes == SITES {
                        NOT_KEPT.fetch_add(1, Relaxed);
                        return;
                    }
                    index = (index + 1) & (SITES - 1);
                };
                site.allocations += 1;
                site.blocks += 1;
                site.bytes += size as i64;
                let mut entry = home_of_block(pointer);
                while (*blocks.add(entry)).pointer != 0 {
                    entry = (entry + 1) & (BLOCKS - 1);
                }
                *blocks.add(entry) = Block { pointer, size_and_site: size << SITE_BITS | index };
            }
            BLOCK_COUNT.fetch_add(1, Relaxed);
        }

        #[inline(never)]
        fn remove(pointer: usize) {
            let _lock = lock();
            let (sites, blocks) = (SITE_TABLE.load(Relaxed), BLOCK_TABLE.load(Relaxed));
            if blocks.is_null() {
                return;
            }
            // SAFETY: as in `insert`.
            unsafe {
                let mut entry = home_of_block(pointer);
                let block = loop {
                    let block = *blocks.add(entry);
                    if block.pointer == 0 {
                        return;
                    }
                    if block.pointer == pointer {
                        break block;
                    }
                    entry = (entry + 1) & (BLOCKS - 1);
                };
                let site = &mut *sites.add(block.size_and_site & (SITES - 1));
                site.blocks -= 1;
                site.bytes -= (block.size_and_site >> SITE_BITS) as i64;
                // The entries after the removed one move back over it, each as far as its home
                // allows, so a search never ends at a gap before the entry it looks for.
                let mut gap = entry;
                let mut next = (entry + 1) & (BLOCKS - 1);
                loop {
                    let moved = *blocks.add(next);
                    if moved.pointer == 0 {
                        break;
                    }
                    let from_home = next.wrapping_sub(home_of_block(moved.pointer)) & (BLOCKS - 1);
                    if from_home >= (next.wrapping_sub(gap) & (BLOCKS - 1)) {
                        *blocks.add(gap) = moved;
                        gap = next;
                    }
                    next = (next + 1) & (BLOCKS - 1);
                }
                (*blocks.add(gap)).pointer = 0;
            }
            BLOCK_COUNT.fetch_sub(1, Relaxed);
        }

        /// The address the program is loaded at, which the addresses of its frames are read
        /// against.
        #[cfg(target_vendor = "apple")]
        fn image_address() -> usize {
            unsafe extern "C" {
                fn _dyld_get_image_header(image_index: u32) -> *const std::ffi::c_void;
            }
            // SAFETY: the function of the loader takes an index and returns an address or null;
            // the first image is the program.
            unsafe { _dyld_get_image_header(0) as usize }
        }

        #[cfg(not(target_vendor = "apple"))]
        fn image_address() -> usize {
            0
        }

        pub(super) fn report() -> Option<String> {
            KEPT.store(false, Release);
            let table = SITE_TABLE.load(Acquire);
            if table.is_null() {
                return None;
            }
            // Allocated before the lock is taken: an allocation inside it would wait for it.
            let mut sites: Vec<Site> = Vec::with_capacity(SITES);
            {
                let _lock = lock();
                for index in 0..SITES {
                    // SAFETY: as in `insert`.
                    let site = unsafe { *table.add(index) };
                    if site.frames[0] != 0 {
                        sites.push(site);
                    }
                }
            }
            sites.sort_by_key(|site| std::cmp::Reverse(site.bytes));
            let program = std::env::current_exe().map(|path| path.display().to_string()).unwrap_or_default();
            let mut text = format!("image {program} {:#x}\n", image_address());
            text.push_str(&format!(
                "sum {} bytes in {} blocks of {} allocations at {} sites; {} blocks not kept\n",
                sites.iter().map(|site| site.bytes).sum::<i64>(),
                sites.iter().map(|site| site.blocks).sum::<i64>(),
                sites.iter().map(|site| site.allocations).sum::<u64>(),
                sites.len(),
                NOT_KEPT.load(Relaxed)
            ));
            for site in sites.iter().filter(|site| site.blocks != 0) {
                text.push_str(&format!("site {} {} {}", site.bytes, site.blocks, site.allocations));
                for frame in site.frames.iter().take_while(|frame| **frame != 0) {
                    text.push_str(&format!(" {frame:#x}"));
                }
                text.push('\n');
            }
            Some(text)
        }
    }
}
