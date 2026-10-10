//! Who holds a survivor: with the feature `trace-holders` the global
//! allocator of the test build is the system allocator behind a recorder of
//! every block that is alive, with the call stack that allocated it.
//!
//! Not a port: the runtime of the reference has a collector and profilers
//! for the question. [`report`] reads the recorded blocks for words that
//! point into the block of an object, as a conservative collector would, and
//! describes them by the call stack that allocated the block they are in:
//! the strong reference that keeps the object alive is among them, beside
//! the weak ones (a weak reference points at the same block; the weak
//! references of the object model are marked in a build with the feature,
//! `ferroui-base/tagged-weak-references`, and counted apart) and the words a
//! list leaves in the part of its buffer it no longer uses. A reference from
//! the block of the object itself is the short form of a cycle. A reference
//! that is found nowhere is held outside the heap of Rust: a static, a
//! thread-local value, the stack of the test.
//!
//! The call stack is read from the frame pointers, which the targets of
//! Apple on ARM always keep; on other targets and without the feature
//! nothing is recorded and [`report`] says so.

/// Whether blocks are recorded in this build.
pub const ENABLED: bool = cfg!(all(feature = "trace-holders", target_os = "macos", target_arch = "aarch64"));

/// What is known about the references to the block that holds `address`, as
/// the lines of a failure message.
pub fn report(address: usize) -> String {
    recording::report(address)
}

#[cfg(not(all(feature = "trace-holders", target_os = "macos", target_arch = "aarch64")))]
mod recording {
    pub fn report(_address: usize) -> String {
        "  who holds it is not recorded in this build: run the test with `--features trace-holders -- --test-threads=1` \
         (macOS on ARM) for the call stacks of the blocks that point at it\n"
            .to_string()
    }
}

#[cfg(all(feature = "trace-holders", target_os = "macos", target_arch = "aarch64"))]
mod recording {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::collections::{BTreeMap, HashMap};
    use std::ffi::{c_char, c_int, c_void, CStr};
    use std::fmt::Write;
    use std::sync::Mutex;

    /// The frames kept of a call stack, from the allocation outwards.
    const FRAMES: usize = 32;
    /// The frames printed of a call stack.
    const PRINTED_FRAMES: usize = 18;
    /// The groups of holders printed.
    const PRINTED_HOLDERS: usize = 12;

    #[derive(Clone, Copy)]
    struct Record {
        size: usize,
        length: u8,
        frames: [usize; FRAMES],
    }

    impl Record {
        fn frames(&self) -> &[usize] {
            &self.frames[..self.length as usize]
        }
    }

    // The blocks that are alive, by address.
    static RECORDS: Mutex<BTreeMap<usize, Record>> = Mutex::new(BTreeMap::new());

    thread_local! {
        // The thread is inside the recorder: what the recorder itself
        // allocates and frees is not recorded.
        static INSIDE: Cell<bool> = const { Cell::new(false) };
    }

    extern "C" {
        fn pthread_self() -> *mut c_void;
        fn pthread_get_stackaddr_np(thread: *mut c_void) -> *mut c_void;
        fn dladdr(address: *const c_void, info: *mut DlInfo) -> c_int;
    }

    #[repr(C)]
    struct DlInfo {
        file_name: *const c_char,
        file_base: *mut c_void,
        symbol_name: *const c_char,
        symbol_address: *mut c_void,
    }

    fn enter() -> bool {
        INSIDE.try_with(|inside| !inside.replace(true)).unwrap_or(false)
    }

    fn leave() {
        let _ = INSIDE.try_with(|inside| inside.set(false));
    }

    fn call_stack() -> ([usize; FRAMES], u8) {
        let mut frames = [0usize; FRAMES];
        let mut length = 0;
        let mut frame: *const usize;
        // SAFETY: reads the frame pointer register.
        unsafe { std::arch::asm!("mov {}, x29", out(reg) frame, options(nomem, nostack, preserves_flags)) };
        // SAFETY: the calls have no preconditions.
        let top = unsafe { pthread_get_stackaddr_np(pthread_self()) } as usize;
        while length < FRAMES {
            let address = frame as usize;
            if address == 0 || address % 16 != 0 || address + 16 > top {
                break;
            }
            // SAFETY: a frame record of the stack of this thread, below its
            // top: the saved frame pointer and the return address.
            let (next, return_address) = unsafe { (*frame as *const usize, *frame.add(1)) };
            if return_address == 0 {
                break;
            }
            frames[length] = return_address;
            length += 1;
            if next as usize <= address {
                break;
            }
            frame = next;
        }
        (frames, length as u8)
    }

    fn allocated(ptr: *mut u8, size: usize) {
        if ptr.is_null() || !enter() {
            return;
        }
        let (frames, length) = call_stack();
        if let Ok(mut records) = RECORDS.lock() {
            records.insert(ptr as usize, Record { size, length, frames });
        }
        leave();
    }

    fn freed(ptr: *mut u8) {
        if !enter() {
            return;
        }
        if let Ok(mut records) = RECORDS.lock() {
            records.remove(&(ptr as usize));
        }
        leave();
    }

    /// The system allocator, telling the recorder of every block.
    struct RecordingAllocator;

    // SAFETY: every request is passed to the system allocator unchanged. A
    // block is forgotten before it is returned to the system and recorded
    // after the system handed it out, both under the lock of the records, so
    // a recorded block is alive while the lock is held.
    unsafe impl GlobalAlloc for RecordingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc`.
            let ptr = unsafe { System.alloc(layout) };
            allocated(ptr, layout.size());
            ptr
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc_zeroed`.
            let ptr = unsafe { System.alloc_zeroed(layout) };
            allocated(ptr, layout.size());
            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            freed(ptr);
            // SAFETY: the caller upholds the contract of `GlobalAlloc::dealloc`.
            unsafe { System.dealloc(ptr, layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            freed(ptr);
            // SAFETY: the caller upholds the contract of `GlobalAlloc::realloc`.
            let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
            if new_ptr.is_null() {
                // The block was not moved and is still alive.
                allocated(ptr, layout.size());
            } else {
                allocated(new_ptr, new_size);
            }
            new_ptr
        }
    }

    #[global_allocator]
    static ALLOCATOR: RecordingAllocator = RecordingAllocator;

    /// The references from the blocks one call stack allocated.
    struct Holder {
        references: u64,
        weak: u64,
        bytes: usize,
        is_target: bool,
        frames: Vec<usize>,
    }

    /// The block that holds `address` (its start, its size and the call
    /// stack that allocated it) and the references into it.
    fn holders(address: usize) -> Option<(usize, usize, Vec<usize>, Vec<Holder>)> {
        let entered = enter();
        let mut result = None;
        if let Ok(records) = RECORDS.lock() {
            let target = records
                .range(..=address)
                .next_back()
                .filter(|(start, record)| address < **start + record.size.max(1))
                .map(|(start, record)| (*start, *record));
            if let Some((start, target)) = target {
                let end = start + target.size.max(1);
                let word_size = std::mem::size_of::<usize>();
                let mut groups: HashMap<(Vec<usize>, bool), (u64, u64, usize)> = HashMap::new();
                for (block, record) in records.iter() {
                    if block % std::mem::align_of::<usize>() != 0 {
                        continue;
                    }
                    let words = record.size / word_size;
                    for index in 0..words {
                        // SAFETY: a word of a block that is alive (it is
                        // recorded, and a freed block is forgotten under the
                        // lock before it is returned to the system), read as
                        // plain bytes.
                        let word = unsafe { std::ptr::read_volatile((*block as *const usize).add(index)) };
                        if word < start || word >= end {
                            continue;
                        }
                        let entry = groups.entry((record.frames().to_vec(), *block == start)).or_default();
                        entry.0 += 1;
                        entry.2 = record.size;
                        // A weak reference of the object model is the
                        // pointer, the table of its type and the mark.
                        if index + 2 < words {
                            // SAFETY: a word of the same block.
                            let mark = unsafe { std::ptr::read_volatile((*block as *const usize).add(index + 2)) };
                            if mark == ferroui_base::WEAK_REFERENCE_TAG {
                                entry.1 += 1;
                            }
                        }
                    }
                }
                let mut holders: Vec<Holder> = groups
                    .into_iter()
                    .map(|((frames, is_target), (references, weak, bytes))| Holder { references, weak, bytes, is_target, frames })
                    .collect();
                holders.sort_by(|a, b| {
                    (b.references - b.weak)
                        .cmp(&(a.references - a.weak))
                        .then_with(|| b.references.cmp(&a.references))
                        .then_with(|| a.frames.cmp(&b.frames))
                });
                result = Some((start, target.size, target.frames().to_vec(), holders));
            }
        }
        if entered {
            leave();
        }
        result
    }

    fn symbol(address: usize) -> String {
        let mut info = DlInfo {
            file_name: std::ptr::null(),
            file_base: std::ptr::null_mut(),
            symbol_name: std::ptr::null(),
            symbol_address: std::ptr::null_mut(),
        };
        // SAFETY: `info` is a valid place for the result; the address is
        // only looked up.
        let found = unsafe { dladdr(address as *const c_void, &mut info) };
        if found == 0 || info.symbol_name.is_null() {
            return format!("{address:#x}");
        }
        // SAFETY: a symbol name of the loader is a C string that lives as
        // long as its image.
        let name = unsafe { CStr::from_ptr(info.symbol_name) }.to_string_lossy();
        demangle(&name)
    }

    /// The path of a symbol of the legacy mangling (`_ZN..E`) without its
    /// hash; any other name unchanged.
    fn demangle(name: &str) -> String {
        let unprefixed = name.strip_prefix('_').filter(|rest| rest.starts_with("_ZN")).unwrap_or(name);
        let Some(mut rest) = unprefixed.strip_prefix("_ZN").and_then(|rest| rest.strip_suffix('E')) else {
            return name.to_string();
        };
        let mut parts = Vec::new();
        while !rest.is_empty() {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            let Ok(length) = rest[..digits].parse::<usize>() else { return name.to_string() };
            let Some(part) = rest.get(digits..digits + length) else { return name.to_string() };
            parts.push(part);
            rest = &rest[digits + length..];
        }
        let is_hash =
            |part: &str| part.len() == 17 && part.starts_with('h') && part[1..].bytes().all(|b| b.is_ascii_hexdigit());
        if parts.last().is_some_and(|part| is_hash(part)) {
            parts.pop();
        }
        let parts: Vec<String> = parts
            .iter()
            .map(|part| {
                let mut part = part.to_string();
                for (escape, text) in [
                    ("$LT$", "<"),
                    ("$GT$", ">"),
                    ("$u20$", " "),
                    ("$C$", ","),
                    ("$RF$", "&"),
                    ("$BP$", "*"),
                    ("$LP$", "("),
                    ("$RP$", ")"),
                    ("$u5b$", "["),
                    ("$u5d$", "]"),
                    ("$u7b$", "{"),
                    ("$u7d$", "}"),
                    ("$u27$", "'"),
                    ("..", "::"),
                ] {
                    part = part.replace(escape, text);
                }
                match part.strip_prefix('_') {
                    Some(stripped) if stripped.starts_with('<') => stripped.to_string(),
                    _ => part,
                }
            })
            .collect();
        parts.join("::")
    }

    /// The frames of a call stack as lines, without the frames of the
    /// allocator and of the recorder.
    fn write_frames(out: &mut String, frames: &[usize], symbols: &mut HashMap<usize, String>) {
        let names: Vec<String> =
            frames.iter().map(|address| symbols.entry(*address).or_insert_with(|| symbol(*address)).clone()).collect();
        let is_allocator = |name: &str| {
            name.contains("holder_trace")
                || name.contains("__rust_alloc")
                || name.contains("__rust_realloc")
                || name.contains("__rg_")
                || name.contains("__rustc")
                || name.starts_with("alloc::alloc::")
                || name.starts_with("alloc::raw_vec::")
        };
        let first = names.iter().position(|name| !is_allocator(name)).unwrap_or(0);
        for name in names.iter().skip(first).take(PRINTED_FRAMES) {
            let _ = writeln!(out, "        {name}");
        }
        if names.len() > first + PRINTED_FRAMES {
            let _ = writeln!(out, "        ... ({} more frames)", names.len() - first - PRINTED_FRAMES);
        }
    }

    pub fn report(address: usize) -> String {
        let mut out = String::new();
        let Some((start, size, frames, holders)) = holders(address) else {
            let _ = writeln!(out, "  the block at {address:#x} is not recorded: it was not allocated by the allocator of Rust");
            return out;
        };
        let mut symbols = HashMap::new();
        let _ = writeln!(out, "  its block: {size} bytes at {start:#x}, allocated at");
        write_frames(&mut out, &frames, &mut symbols);
        let references: u64 = holders.iter().map(|holder| holder.references).sum();
        let weak: u64 = holders.iter().map(|holder| holder.weak).sum();
        let _ = writeln!(
            out,
            "  words of the heap that point into it: {references}, of which {weak} are weak references of the object \
             model (the others: a strong reference, a weak one of the standard library, a word of a buffer that is no \
             longer in use); a strong reference that is not among them is held outside the heap (a static, a \
             thread-local value, the stack)"
        );
        for holder in holders.iter().take(PRINTED_HOLDERS) {
            let _ = writeln!(
                out,
                "  - {} ({} weak) from {} of {} bytes allocated at",
                holder.references,
                holder.weak,
                if holder.is_target { "the block of the object itself (a cycle through its own fields)" } else { "a block" },
                holder.bytes
            );
            write_frames(&mut out, &holder.frames, &mut symbols);
        }
        if holders.len() > PRINTED_HOLDERS {
            let _ = writeln!(out, "  ... ({} more groups of holders)", holders.len() - PRINTED_HOLDERS);
        }
        out
    }

    #[test]
    fn a_reference_from_a_block_of_the_heap_is_reported_with_its_call_stack() {
        let target: Box<[usize; 4]> = std::hint::black_box(Box::new([1, 2, 3, 4]));
        let address = &*target as *const [usize; 4] as usize;
        let holder: Vec<usize> = std::hint::black_box(vec![address, 0, 0, 0]);
        let (start, size, frames, holders) = holders(address).expect("the block is recorded");
        assert_eq!(address, start);
        assert_eq!(4 * std::mem::size_of::<usize>(), size);
        assert!(!frames.is_empty());
        assert!(holders.iter().any(|holder| holder.references >= 1 && holder.weak == 0 && !holder.is_target));
        assert!(report(address).contains("words of the heap that point into it"));
        drop(holder);
        drop(target);
    }

    #[test]
    fn a_legacy_symbol_is_demangled_to_its_path() {
        assert_eq!("core::ptr::drop_in_place", demangle("_ZN4core3ptr13drop_in_place17h0123456789abcdefE"));
        assert_eq!("_malloc", demangle("_malloc"));
    }
}

#[test]
fn a_build_without_the_recorder_says_so() {
    if !ENABLED {
        assert!(report(0).contains("trace-holders"));
    }
}
