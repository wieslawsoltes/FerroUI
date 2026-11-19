//! The event ring (docs/c-abi.md, section 14.4): events subscribed with
//! [`ferro_subscribe_ring`] append a record (tag, sender, argument) to a ring in memory that the
//! language drains when it wants, with no callback into the language per event.
//!
//! One producer, one consumer: the library writes the records and `head` (the number of records
//! written, with release order after the record) on the UI thread; the language reads `head`
//! with acquire order, the records up to it, and writes `tail` (the number of records it took,
//! with release order). A record is at index `n % capacity` (the capacity is a power of two). A
//! full ring drops the new event and counts it in `dropped`: the language sizes the ring for the
//! events between two of its drains.

use crate::error::{guard, status_or_value, Error, Result};
use crate::thread;
use crate::{FerroHandle, FERRO_INVALID_ARGUMENT, FERRO_INVALID_STATE, FERRO_NOT_FOUND};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::OnceLock;

/// The version of the layout of [`FerroEventRing`].
pub const FERRO_EVENT_RING_LAYOUT: u32 = 1;

/// The event ring of the process: arrays the library allocates once, which stay at their
/// addresses until the process ends. Each record is four words: the tag, the sender (borrowed:
/// valid while the language does not release it, retain it to keep it), the argument, and the
/// number of the record.
#[repr(C)]
pub struct FerroEventRing {
    /// [`FERRO_EVENT_RING_LAYOUT`].
    pub layout: u32,
    /// The number of records, a power of two.
    pub capacity: u32,
    /// The number of records written (by the library).
    pub head: *mut u64,
    /// The number of records taken (by the language).
    pub tail: *mut u64,
    /// The number of events dropped because the ring was full.
    pub dropped: *mut u64,
    /// `capacity` records of four `int64_t`: tag, sender, argument, number.
    pub records: *mut i64,
}

struct Shared {
    ring: FerroEventRing,
    head: &'static AtomicU64,
    tail: &'static AtomicU64,
    dropped: &'static AtomicU64,
    records: &'static [AtomicI64],
    mask: u64,
}

// SAFETY: atomics that live for the process; the raw pointers of `ring` point at them.
unsafe impl Sync for Shared {}
unsafe impl Send for Shared {}

static SHARED: OnceLock<Shared> = OnceLock::new();

/// Creates the event ring of the process with room for `capacity` records (rounded up to a
/// power of two, at most 2^20), or returns the one that exists. `NULL` on a failure. UI thread
/// only.
#[no_mangle]
pub extern "C" fn ferro_event_ring(capacity: i64) -> *const FerroEventRing {
    guard(std::ptr::null(), || {
        let made = (|| -> Result<*const FerroEventRing> {
            thread::check()?;
            if let Some(shared) = SHARED.get() {
                return Ok(&shared.ring);
            }
            if !(1..=1 << 20).contains(&capacity) {
                return Err(Error::new(FERRO_INVALID_ARGUMENT, "the capacity is not between 1 and 2^20"));
            }
            let capacity = (capacity as usize).next_power_of_two();
            let counters: &'static [AtomicU64] =
                Box::leak(Box::new([AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)]));
            let records: &'static mut [AtomicI64] =
                Box::leak((0..capacity * 4).map(|_| AtomicI64::new(0)).collect::<Vec<_>>().into_boxed_slice());
            let ring = FerroEventRing {
                layout: FERRO_EVENT_RING_LAYOUT,
                capacity: capacity as u32,
                head: counters[0].as_ptr(),
                tail: counters[1].as_ptr(),
                dropped: counters[2].as_ptr(),
                records: records.as_mut_ptr().cast(),
            };
            let shared = Shared {
                ring,
                head: &counters[0],
                tail: &counters[1],
                dropped: &counters[2],
                records,
                mask: capacity as u64 - 1,
            };
            let _ = SHARED.set(shared);
            Ok(&SHARED.get().ok_or_else(|| Error::new(FERRO_INVALID_STATE, "no event ring"))?.ring)
        })();
        made.unwrap_or_else(|error| {
            error.record();
            std::ptr::null()
        })
    })
}

/// Whether the ring exists (a subscription to it needs it).
pub(crate) fn exists() -> bool {
    SHARED.get().is_some()
}

/// Appends a record; on the UI thread (an event of a subscription).
#[inline]
pub(crate) fn push(tag: i64, sender: FerroHandle, arg: i64) {
    let Some(shared) = SHARED.get() else {
        return;
    };
    let head = shared.head.load(Ordering::Relaxed);
    if head.wrapping_sub(shared.tail.load(Ordering::Acquire)) > shared.mask {
        shared.dropped.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let at = ((head & shared.mask) * 4) as usize;
    shared.records[at].store(tag, Ordering::Relaxed);
    shared.records[at + 1].store(sender as i64, Ordering::Relaxed);
    shared.records[at + 2].store(arg, Ordering::Relaxed);
    shared.records[at + 3].store(head as i64, Ordering::Relaxed);
    shared.head.store(head.wrapping_add(1), Ordering::Release);
}

ferroui_base::ferro_thread_local! {
    /// The record the last `ferro_event_ring_pop` of the thread took: sender, argument.
    static TAKEN: std::cell::Cell<(FerroHandle, i64)> = const { std::cell::Cell::new((0, 0)) };
}

/// Takes the next record of the ring, for a language that cannot read memory: returns its tag
/// (and keeps its sender and argument for [`ferro_event_ring_sender`] and
/// [`ferro_event_ring_arg`]), or [`FERRO_NOT_FOUND`] when the ring is empty (nothing recorded).
/// The thread that drains the ring.
#[no_mangle]
pub extern "C" fn ferro_event_ring_pop() -> i64 {
    status_or_value(|| {
        let shared =
            SHARED.get().ok_or_else(|| Error::new(FERRO_INVALID_STATE, "no event ring: call ferro_event_ring"))?;
        let tail = shared.tail.load(Ordering::Relaxed);
        if shared.head.load(Ordering::Acquire) == tail {
            return Ok(i64::from(FERRO_NOT_FOUND));
        }
        let at = ((tail & shared.mask) * 4) as usize;
        let tag = shared.records[at].load(Ordering::Relaxed);
        let sender = shared.records[at + 1].load(Ordering::Relaxed) as FerroHandle;
        let arg = shared.records[at + 2].load(Ordering::Relaxed);
        shared.tail.store(tail.wrapping_add(1), Ordering::Release);
        TAKEN.set((sender, arg));
        Ok(tag)
    })
}

/// The sender of the record the last [`ferro_event_ring_pop`] of the thread took.
#[no_mangle]
pub extern "C" fn ferro_event_ring_sender() -> FerroHandle {
    TAKEN.try_with(|taken| taken.get().0).unwrap_or(0)
}

/// The argument of the record the last [`ferro_event_ring_pop`] of the thread took.
#[no_mangle]
pub extern "C" fn ferro_event_ring_arg() -> i64 {
    TAKEN.try_with(|taken| taken.get().1).unwrap_or(0)
}
