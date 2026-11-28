//! The glyph outlines the engines draw by key (docs/porting/progpu-backend.md, section 2.3.8).
//!
//! A frame does not carry the outlines of its glyphs: a run names each with the key of the
//! outline, a raster scale and a subpixel phase, and the engine holds the outline. This log is
//! what the engines hold them from: every outline a run named, in the order they were first
//! named, and the release of every face that was dropped. An engine reads the entries it has
//! not seen before it takes a scene, so a scene never names an outline its engine does not
//! hold; an engine made later reads the log from its start.
//!
//! The log is one for the process: outlines belong to faces, faces are shared between threads,
//! and an engine may be made after the outlines it draws.

use crate::font::KeptOutline;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// An entry of the log.
#[derive(Clone)]
pub(crate) enum Entry {
    /// An outline a run named.
    Outline(Arc<KeptOutline>),
    /// A variation instance of a face that was dropped: its outlines left the log, and an
    /// engine that holds them forgets them.
    Release(u64),
}

struct Log {
    /// The entries with their sequence numbers, ascending.
    entries: Vec<(u64, Entry)>,
    /// The instances that have outlines in the log.
    instances: BTreeSet<u64>,
}

static LOG: Mutex<Log> = Mutex::new(Log { entries: Vec::new(), instances: BTreeSet::new() });
/// The sequence number of the last entry (zero for an empty log): what an engine compares its
/// own with, without taking the lock.
static LAST: AtomicU64 = AtomicU64::new(0);

fn lock() -> MutexGuard<'static, Log> {
    LOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Puts `outline` in the log, once: a run is about to name it.
pub(crate) fn register(outline: &Arc<KeptOutline>) {
    if outline.logged().load(Ordering::Acquire) {
        return;
    }
    let mut log = lock();
    if outline.logged().load(Ordering::Relaxed) {
        return;
    }
    let sequence = LAST.load(Ordering::Relaxed) + 1;
    log.entries.push((sequence, Entry::Outline(outline.clone())));
    log.instances.insert(outline.key().face);
    LAST.store(sequence, Ordering::Release);
    // Said last, inside the lock: whoever reads that the outline is logged finds its entry.
    outline.logged().store(true, Ordering::Release);
}

/// Takes the outlines of the variation `instances` of a dropped face out of the log and tells
/// the engines to forget them.
pub(crate) fn release(instances: &[u64]) {
    if instances.is_empty() {
        return;
    }
    let mut log = lock();
    let held: Vec<u64> = instances.iter().copied().filter(|instance| log.instances.remove(instance)).collect();
    if held.is_empty() {
        return;
    }
    log.entries.retain(|(_, entry)| !matches!(entry, Entry::Outline(outline) if held.contains(&outline.key().face)));
    let mut sequence = LAST.load(Ordering::Relaxed);
    for instance in held {
        sequence += 1;
        log.entries.push((sequence, Entry::Release(instance)));
    }
    LAST.store(sequence, Ordering::Release);
}

/// The sequence number of the last entry.
pub(crate) fn last() -> u64 {
    LAST.load(Ordering::Acquire)
}

/// The entries after the entry `sequence`, in order, and the sequence number of the last.
pub(crate) fn after(sequence: u64) -> (Vec<Entry>, u64) {
    let log = lock();
    let start = log.entries.partition_point(|(number, _)| *number <= sequence);
    (log.entries[start..].iter().map(|(_, entry)| entry.clone()).collect(), LAST.load(Ordering::Relaxed))
}

/// Whether the log has outlines of the variation `instance` (for the tests of the release).
#[cfg(test)]
pub(crate) fn holds(instance: u64) -> bool {
    lock().instances.contains(&instance)
}
