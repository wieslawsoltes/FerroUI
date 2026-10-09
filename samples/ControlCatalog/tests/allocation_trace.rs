//! Where the blocks of a stretch of work were allocated: a recorder behind
//! the counting allocator of `allocations.rs` (the feature
//! `count-allocations`).
//!
//! Not a port: the upstream sample has no tests, and its runtime has a
//! collector and profilers for the question this answers. Between [`start`]
//! and [`stop`] every block that is allocated, on any thread, is recorded
//! with the call stack of its allocation and the epoch it was allocated in
//! ([`next_epoch`] starts one); a block that is freed, then or later, is
//! forgotten. [`alive`] returns what is still alive of an epoch, grouped by
//! call stack: what the work of that epoch left behind. A block that is
//! resized while recording counts as allocated there, with its whole size,
//! so a list that only grows shows as the stack that grew it.
//!
//! [`holders`] answers who keeps those blocks: it reads the recorded blocks
//! for words that point into a block of the epoch, as a conservative
//! collector would. What it finds is every reference into what the work
//! left: the strong one that keeps it alive among them, beside the weak
//! ones (a weak reference points at the same block) and the words a list
//! leaves in the part of its buffer it no longer uses. No holder from an
//! earlier epoch means a cycle of the blocks among themselves, or a holder
//! that is not a recorded block (a static, a block allocated before the
//! recording or outside the allocator of Rust).
//!
//! The call stack is read from the frame pointers, which the targets of
//! Apple on ARM always keep; on other targets, and without the feature,
//! nothing is recorded and [`take`] returns nothing.

/// The frames kept of a call stack, from the allocation outwards.
pub const FRAMES: usize = 48;

/// The blocks still alive that one call stack allocated.
#[derive(Clone, Debug)]
pub struct Site {
    pub allocations: u64,
    pub bytes: u64,
    /// The return addresses, from the allocation outwards.
    pub frames: Vec<usize>,
}

/// References from the blocks one call stack allocated in one epoch into
/// the blocks another allocated.
#[derive(Clone, Debug)]
pub struct Holder {
    /// The number of pointers.
    pub references: u64,
    /// The epoch of the blocks that hold the pointers.
    pub epoch: u32,
    /// The size of one of the blocks that hold the pointers.
    pub bytes: usize,
    /// The call stack that allocated the blocks that hold the pointers.
    pub holder: Vec<usize>,
    /// The call stack that allocated the blocks they point into.
    pub target: Vec<usize>,
}

/// Whether call stacks are recorded in this build.
pub const ENABLED: bool = cfg!(all(feature = "count-allocations", target_os = "macos", target_arch = "aarch64"));

#[cfg(all(feature = "count-allocations", target_os = "macos", target_arch = "aarch64"))]
mod recording {
    use super::{Holder, Site, FRAMES};
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::ffi::{c_char, c_int, c_void, CStr};
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Mutex;

    struct Record {
        size: usize,
        epoch: u32,
        length: u8,
        frames: [usize; FRAMES],
    }

    static RECORDING: AtomicBool = AtomicBool::new(false);
    static EPOCH: AtomicU32 = AtomicU32::new(0);
    // Whether there may be records: a freed block is looked up.
    static TRACKING: AtomicBool = AtomicBool::new(false);
    static RECORDS: Mutex<Option<HashMap<usize, Record>>> = Mutex::new(None);

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

    /// Whether the thread is inside the recorder: what it allocates and
    /// frees there is the recorder's own and is not counted.
    pub fn is_inside() -> bool {
        INSIDE.try_with(Cell::get).unwrap_or(false)
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

    pub fn allocated(ptr: *mut u8, size: usize) {
        if !RECORDING.load(Ordering::Relaxed) || ptr.is_null() || !enter() {
            return;
        }
        let (frames, length) = call_stack();
        if let Ok(mut records) = RECORDS.lock() {
            let epoch = EPOCH.load(Ordering::Relaxed);
            records.get_or_insert_with(HashMap::new).insert(ptr as usize, Record { size, epoch, length, frames });
        }
        leave();
    }

    pub fn freed(ptr: *mut u8) {
        if !TRACKING.load(Ordering::Relaxed) || !enter() {
            return;
        }
        if let Ok(mut records) = RECORDS.lock() {
            if let Some(records) = records.as_mut() {
                records.remove(&(ptr as usize));
            }
        }
        leave();
    }

    pub fn start() {
        TRACKING.store(true, Ordering::SeqCst);
        RECORDING.store(true, Ordering::SeqCst);
    }

    pub fn stop() {
        RECORDING.store(false, Ordering::SeqCst);
    }

    pub fn next_epoch() -> u32 {
        EPOCH.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn clear() {
        RECORDING.store(false, Ordering::SeqCst);
        TRACKING.store(false, Ordering::SeqCst);
        let entered = enter();
        let records = RECORDS.lock().ok().and_then(|mut records| records.take());
        drop(records);
        if entered {
            leave();
        }
    }

    pub fn alive(epoch: u32) -> Vec<Site> {
        let entered = enter();
        let mut sites: HashMap<Vec<usize>, Site> = HashMap::new();
        if let Ok(records) = RECORDS.lock() {
            for record in records.iter().flat_map(|records| records.values()).filter(|record| record.epoch == epoch) {
                let frames = &record.frames[..record.length as usize];
                let site = match sites.get_mut(frames) {
                    Some(site) => site,
                    None => sites.entry(frames.to_vec()).or_insert(Site { allocations: 0, bytes: 0, frames: frames.to_vec() }),
                };
                site.allocations += 1;
                site.bytes += record.size as u64;
            }
        }
        let mut sites: Vec<Site> = sites.into_values().collect();
        sites.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.frames.cmp(&b.frames)));
        if entered {
            leave();
        }
        sites
    }

    pub fn counts(epoch: u32, is_target: &dyn Fn(&[usize]) -> bool) -> Vec<(Vec<usize>, usize, usize)> {
        let entered = enter();
        let mut counts = Vec::new();
        if let Ok(records) = RECORDS.lock() {
            for (address, record) in records.iter().flat_map(|records| records.iter()) {
                if record.epoch == epoch
                    && record.size >= 2 * std::mem::size_of::<usize>()
                    && address % std::mem::align_of::<usize>() == 0
                    && is_target(&record.frames[..record.length as usize])
                {
                    // SAFETY: the first two words of a block that is alive.
                    let words = unsafe { std::ptr::read_volatile(*address as *const [usize; 2]) };
                    counts.push((record.frames[..record.length as usize].to_vec(), words[0], words[1]));
                }
            }
        }
        if entered {
            leave();
        }
        counts
    }

    pub fn holders(epoch: u32, every_epoch: bool, is_target: &dyn Fn(&[usize]) -> bool) -> Vec<Holder> {
        let entered = enter();
        let mut holders: HashMap<(Vec<usize>, Vec<usize>, u32), (u64, usize)> = HashMap::new();
        if let Ok(records) = RECORDS.lock() {
            if let Some(records) = records.as_ref() {
                // The blocks of the epoch, by address.
                let mut targets: Vec<(usize, usize, &Record)> = records
                    .iter()
                    .filter(|(_, record)| record.epoch == epoch && is_target(&record.frames[..record.length as usize]))
                    .map(|(address, record)| (*address, *address + record.size.max(1), record))
                    .collect();
                targets.sort_by_key(|target| target.0);
                let (lowest, highest) = match (targets.first(), targets.last()) {
                    (Some(first), Some(last)) => (first.0, last.1),
                    _ => (0, 0),
                };
                for (address, record) in records.iter().filter(|(_, record)| every_epoch || record.epoch < epoch) {
                    let words = record.size / std::mem::size_of::<usize>();
                    if address % std::mem::align_of::<usize>() != 0 {
                        continue;
                    }
                    for index in 0..words {
                        // SAFETY: a word of a block that is alive (it is
                        // recorded, and a freed block is forgotten before it
                        // is returned to the system), read as plain bytes.
                        let word = unsafe { std::ptr::read_volatile((*address as *const usize).add(index)) };
                        if word < lowest || word >= highest {
                            continue;
                        }
                        let after = targets.partition_point(|target| target.0 <= word);
                        let Some(target) = after.checked_sub(1).map(|index| &targets[index]) else { continue };
                        if word >= target.1 {
                            continue;
                        }
                        let key = (
                            record.frames[..record.length as usize].to_vec(),
                            target.2.frames[..target.2.length as usize].to_vec(),
                            record.epoch,
                        );
                        let entry = holders.entry(key).or_default();
                        entry.0 += 1;
                        entry.1 = record.size;
                    }
                }
            }
        }
        let mut holders: Vec<Holder> = holders
            .into_iter()
            .map(|((holder, target, epoch), (references, bytes))| Holder { references, epoch, bytes, holder, target })
            .collect();
        holders.sort_by(|a, b| b.references.cmp(&a.references).then_with(|| a.holder.cmp(&b.holder)).then_with(|| a.target.cmp(&b.target)));
        if entered {
            leave();
        }
        holders
    }

    pub fn symbol(address: usize) -> String {
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
        super::demangle(&name)
    }
}

#[cfg(not(all(feature = "count-allocations", target_os = "macos", target_arch = "aarch64")))]
mod recording {
    use super::{Holder, Site};

    #[cfg(feature = "count-allocations")]
    pub fn allocated(_ptr: *mut u8, _size: usize) {}
    #[cfg(feature = "count-allocations")]
    pub fn freed(_ptr: *mut u8) {}
    #[cfg(feature = "count-allocations")]
    pub fn is_inside() -> bool {
        false
    }
    pub fn start() {}
    pub fn stop() {}
    pub fn next_epoch() -> u32 {
        0
    }
    pub fn clear() {}
    pub fn alive(_epoch: u32) -> Vec<Site> {
        Vec::new()
    }
    pub fn holders(_epoch: u32, _every_epoch: bool, _is_target: &dyn Fn(&[usize]) -> bool) -> Vec<Holder> {
        Vec::new()
    }
    pub fn counts(_epoch: u32, _is_target: &dyn Fn(&[usize]) -> bool) -> Vec<(Vec<usize>, usize, usize)> {
        Vec::new()
    }
    pub fn symbol(address: usize) -> String {
        format!("{address:#x}")
    }
}

#[cfg(feature = "count-allocations")]
pub(super) use recording::{allocated, freed, is_inside};

/// Starts recording: the records of an earlier recording are kept.
pub fn start() {
    recording::start()
}

/// Stops recording. The blocks recorded so far are still forgotten when
/// they are freed.
pub fn stop() {
    recording::stop()
}

/// Starts the next epoch and returns it: the blocks recorded from now on
/// belong to it. The first epoch is 0.
pub fn next_epoch() -> u32 {
    recording::next_epoch()
}

/// Stops recording and forgets every record.
pub fn clear() {
    recording::clear()
}

/// The recorded blocks of `epoch` that are still alive, by call stack, the
/// largest first.
pub fn alive(epoch: u32) -> Vec<Site> {
    recording::alive(epoch)
}

/// The call stack and the first two words of the blocks of `epoch` whose
/// call stack `is_target` accepts: of the block of an `Rc`, the number of
/// strong references and the number of weak ones (plus one while there is a
/// strong one). A block without strong references is an object that was
/// dropped and whose memory weak references still hold.
pub fn counts(epoch: u32, is_target: &dyn Fn(&[usize]) -> bool) -> Vec<(Vec<usize>, usize, usize)> {
    recording::counts(epoch, is_target)
}

/// The references from the recorded blocks (of the epochs before `epoch`,
/// or of every epoch) into the blocks of `epoch` whose call stack
/// `is_target` accepts, by the call stacks of both, the most frequent first.
pub fn holders(epoch: u32, every_epoch: bool, is_target: &dyn Fn(&[usize]) -> bool) -> Vec<Holder> {
    recording::holders(epoch, every_epoch, is_target)
}

/// The name of the function a return address lies in, or the address.
pub fn symbol(address: usize) -> String {
    recording::symbol(address)
}

/// The path of a symbol of the legacy mangling (`_ZN..E`) without its hash;
/// any other name unchanged.
pub fn demangle(name: &str) -> String {
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

#[test]
fn a_legacy_symbol_is_demangled_to_its_path() {
    let implementation = "_$LT$alloc..vec..Vec$LT$T$GT$$u20$as$u20$core..ops..Drop$GT$";
    assert_eq!(
        "<alloc::vec::Vec<T> as core::ops::Drop>::drop",
        demangle(&format!("__ZN{}{implementation}4drop17h0123456789abcdefE", implementation.len()))
    );
    assert_eq!("core::ptr::drop_in_place", demangle("_ZN4core3ptr13drop_in_place17h0123456789abcdefE"));
    assert_eq!("_malloc", demangle("_malloc"));
}

#[test]
fn a_block_alive_after_the_recording_is_reported_with_its_call_stack() {
    if !ENABLED {
        assert!(alive(0).is_empty());
        return;
    }
    // The measurements, which are the other users of the recorder, run
    // alone; here the epochs are this test's.
    start();
    let before = next_epoch();
    let mut holder: Vec<usize> = std::hint::black_box(Vec::with_capacity(std::hint::black_box(321)));
    let epoch = next_epoch();
    let kept: Vec<u8> = std::hint::black_box(Vec::with_capacity(std::hint::black_box(12_345)));
    let freed: Vec<u8> = std::hint::black_box(Vec::with_capacity(std::hint::black_box(23_456)));
    drop(freed);
    holder.push(kept.as_ptr() as usize);
    stop();
    let sites = alive(epoch);
    let holders = holders(epoch, false, &|_| true);
    clear();
    assert!(before < epoch);
    assert!(sites.iter().any(|site| site.bytes == 12_345 && !site.frames.is_empty()));
    assert!(sites.iter().all(|site| site.bytes != 23_456));
    assert!(holders.iter().any(|holder| holder.references == 1 && !holder.holder.is_empty()));
    drop(kept);
    drop(holder);
}
