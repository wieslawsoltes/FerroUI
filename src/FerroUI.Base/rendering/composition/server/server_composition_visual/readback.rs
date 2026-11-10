//! Here we are using a simplified version Multi-Version Concurrency Control
//! with only one reader and only one writer.
//!
//! The goal is to provide un-teared view of a particular revision for the
//! UI thread.
//!
//! We are taking a shared lock before switching reader's revision and are
//! using the same lock to produce a new revision, so we know for sure that
//! reader can't switch to a newer revision while we are writing.
//!
//! Reader's behavior:
//! 1) reader will only pick slots with revision <= its current revision
//! 2) reader will pick the newest revision among slots from (1)
//!
//! There are two scenarios that can be encountered by the writer:
//! 1) both slots contain data for revisions older than the reader's current
//!    revision, in that case we pick the slot with the oldest revision and
//!    update it.
//!    1.1) if reader comes before update it will pick the newer one
//!    1.2) if reader comes after update, the overwritten slot would have a
//!         revision that's higher than the reader's one, so it will still
//!         pick the same slot
//! 2) one of the slots contains data for a revision newer than the reader's
//!    current revision. In that case we simply pick the slot with revision
//!    the reader isn't allowed to touch anyway. Both before and after
//!    update the reader will see only one (same) slot it's allowed to touch
//!
//! The slots are shared between the UI-thread visual and its server visual
//! through an `Arc`. The values of a slot are atomics (not from upstream,
//! which writes plain fields): the protocol above keeps the reader away from
//! the slot the writer is writing, and the lock of the readback indices, held
//! for the whole write and taken by the reader to move to a newer revision,
//! orders the write of a revision before every read of it
//! (desktop-performance.md, 9.30).

use super::ServerCompositionVisual;
use crate::platform::LtrbRect;
use crate::rendering::composition::server::ReadbackQueueEntry;
use crate::Matrix;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

/// What the UI thread reads back from a server visual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadbackData {
    pub matrix: Matrix,
    pub revision: u64,
    pub target_id: i64,
    pub visible: bool,
    pub transformed_subtree_bounds: Option<LtrbRect>,
    /// The bounds of the subtree in the coordinates of the visual, before its
    /// own transform (not from upstream, #580): the area the renderer drew the
    /// effect of the visual over, its padding included, which the interactive
    /// effects normalize the pointer to.
    pub subtree_bounds: Option<LtrbRect>,
}

/// The visual is visible.
const FLAG_VISIBLE: u64 = 1;
/// The slot has transformed subtree bounds.
const FLAG_TRANSFORMED_SUBTREE_BOUNDS: u64 = 1 << 1;
/// The slot has subtree bounds.
const FLAG_SUBTREE_BOUNDS: u64 = 1 << 2;

/// The values of one revision. The revision itself is kept beside the one of
/// the other slot, in [`VisualReadback`].
struct Slot {
    matrix: [AtomicU64; 9],
    target_id: AtomicI64,
    flags: AtomicU64,
    transformed_subtree_bounds: [AtomicU64; 4],
    subtree_bounds: [AtomicU64; 4],
}

fn store_rect(slot: &[AtomicU64; 4], rect: Option<LtrbRect>) {
    if let Some(rect) = rect {
        for (value, part) in slot.iter().zip([rect.left, rect.top, rect.right, rect.bottom]) {
            value.store(part.to_bits(), Ordering::Relaxed);
        }
    }
}

fn load_rect(slot: &[AtomicU64; 4], present: bool) -> Option<LtrbRect> {
    present.then(|| {
        let [left, top, right, bottom] = slot.each_ref().map(|value| f64::from_bits(value.load(Ordering::Relaxed)));
        LtrbRect { left, top, right, bottom }
    })
}

impl Slot {
    fn new() -> Self {
        Self {
            matrix: Default::default(),
            target_id: AtomicI64::new(0),
            flags: AtomicU64::new(0),
            transformed_subtree_bounds: Default::default(),
            subtree_bounds: Default::default(),
        }
    }

    fn read(&self, revision: u64) -> ReadbackData {
        let [m11, m12, m13, m21, m22, m23, m31, m32, m33] =
            self.matrix.each_ref().map(|value| f64::from_bits(value.load(Ordering::Relaxed)));
        let flags = self.flags.load(Ordering::Relaxed);
        ReadbackData {
            matrix: Matrix::new_3x3(m11, m12, m13, m21, m22, m23, m31, m32, m33),
            revision,
            target_id: self.target_id.load(Ordering::Relaxed),
            visible: flags & FLAG_VISIBLE != 0,
            transformed_subtree_bounds: load_rect(
                &self.transformed_subtree_bounds,
                flags & FLAG_TRANSFORMED_SUBTREE_BOUNDS != 0,
            ),
            subtree_bounds: load_rect(&self.subtree_bounds, flags & FLAG_SUBTREE_BOUNDS != 0),
        }
    }

    fn write(&self, data: &ReadbackData) {
        let m = &data.matrix;
        for (value, part) in self.matrix.iter().zip([m.m11, m.m12, m.m13, m.m21, m.m22, m.m23, m.m31, m.m32, m.m33]) {
            value.store(part.to_bits(), Ordering::Relaxed);
        }
        self.target_id.store(data.target_id, Ordering::Relaxed);
        let mut flags = 0;
        if data.visible {
            flags |= FLAG_VISIBLE;
        }
        if data.transformed_subtree_bounds.is_some() {
            flags |= FLAG_TRANSFORMED_SUBTREE_BOUNDS;
        }
        if data.subtree_bounds.is_some() {
            flags |= FLAG_SUBTREE_BOUNDS;
        }
        self.flags.store(flags, Ordering::Relaxed);
        store_rect(&self.transformed_subtree_bounds, data.transformed_subtree_bounds);
        store_rect(&self.subtree_bounds, data.subtree_bounds);
    }
}

/// The two readback slots of a visual. The revisions of both come first, so
/// that choosing a slot reads one place.
pub struct VisualReadback {
    revisions: [AtomicU64; 2],
    slots: [Slot; 2],
}

impl Default for VisualReadback {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualReadback {
    pub fn new() -> Self {
        Self { revisions: [AtomicU64::new(u64::MAX), AtomicU64::new(u64::MAX)], slots: [Slot::new(), Slot::new()] }
    }

    /// The slot a reader at `reader_revision` reads, with its revision.
    fn slot_for_read(&self, reader_revision: u64) -> Option<(usize, u64)> {
        let slot0_revision = self.revisions[0].load(Ordering::SeqCst);
        let slot1_revision = self.revisions[1].load(Ordering::SeqCst);
        match (slot0_revision <= reader_revision, slot1_revision <= reader_revision) {
            // Pick the newest one, it's guaranteed to be not touched by the writer
            (true, true) if slot1_revision > slot0_revision => Some((1, slot1_revision)),
            (true, _) => Some((0, slot0_revision)),
            (false, true) => Some((1, slot1_revision)),
            // No readback was written for this visual yet
            (false, false) => None,
        }
    }

    pub fn get_readback(&self, reader_revision: u64) -> Option<ReadbackData> {
        let (slot, revision) = self.slot_for_read(reader_revision)?;
        Some(self.slots[slot].read(revision))
    }

    /// The revision of the readback [`get_readback`](Self::get_readback) would return for
    /// `reader_revision`, read without copying its data.
    pub fn readback_revision(&self, reader_revision: u64) -> Option<u64> {
        self.slot_for_read(reader_revision).map(|(_, revision)| revision)
    }

    /// Writes `data` as the revision `writer_revision`; its own `revision` is
    /// ignored.
    pub fn update_readback(&self, writer_revision: u64, reader_revision: u64, data: ReadbackData) {
        let revision0 = self.revisions[0].load(Ordering::SeqCst);
        let revision1 = self.revisions[1].load(Ordering::SeqCst);
        let slot = if revision0 > reader_revision {
            // Future revision is in slot0
            0
        } else if revision1 > reader_revision {
            // Future revision is in slot1
            1
        } else if revision0 < revision1 {
            // No future revisions, overwrite the oldest one since reader will always pick the newest
            0
        } else {
            1
        };

        // The revision first: from here on the reader leaves the slot alone.
        self.revisions[slot].store(writer_revision, Ordering::SeqCst);
        self.slots[slot].write(&data);
    }
}

impl ServerCompositionVisual {
    /// Has the readback pass write the visual, with the values it has when
    /// the pass runs.
    pub(super) fn enqueue_for_readback_update(&self) {
        let Some(compositor) = self.compositor() else { return };
        let pass = compositor.readback_pass();
        if self.readback_enqueued_pass.get() == pass {
            if self.readback_snapshot.get() {
                // The values changed after the update walk copied them.
                let Some(this) = self.rc() else { return };
                compositor.set_readback_entry(self.readback_queue_index.get(), ReadbackQueueEntry::Visual(this));
                self.readback_snapshot.set(false);
            }
            return;
        }

        let Some(this) = self.rc() else { return };
        self.readback_queue_index.set(compositor.push_readback_entry(ReadbackQueueEntry::Visual(this)));
        self.readback_enqueued_pass.set(pass);
        self.readback_snapshot.set(false);
    }

    /// Has the readback pass write the values the visual has now (not from
    /// upstream, where the pass reads every visual): the update walk calls it
    /// with the visual at hand, and the pass, which runs after the frame is
    /// drawn, does not have to bring the visual back from memory
    /// (desktop-performance.md, 9.30). A change announced later in the frame
    /// turns the entry back into a read of the visual.
    pub(super) fn snapshot_for_readback_update(&self) {
        let Some(compositor) = self.compositor() else { return };
        let pass = compositor.readback_pass();
        let data = self.readback_data(0);
        if self.readback_enqueued_pass.get() == pass {
            compositor.set_readback_snapshot(self.readback_queue_index.get(), &self.readback, data);
        } else {
            let entry = ReadbackQueueEntry::Snapshot(self.readback.clone(), data);
            self.readback_queue_index.set(compositor.push_readback_entry(entry));
            self.readback_enqueued_pass.set(pass);
        }
        self.readback_snapshot.set(true);
    }

    /// Sets the own transform of the visual, one of the values the readback
    /// copies.
    pub(super) fn set_own_transform(&self, transform: Option<Matrix>) {
        self.own_transform.set(transform);
        // `true` here may be left from an earlier frame: the enqueueing
        // sorts that out.
        if self.readback_snapshot.get() {
            self.enqueue_for_readback_update();
        }
    }

    pub fn get_readback(&self, reader_revision: u64) -> Option<ReadbackData> {
        self.readback.get_readback(reader_revision)
    }

    fn readback_data(&self, revision: u64) -> ReadbackData {
        ReadbackData {
            matrix: self.own_transform.get().unwrap_or(Matrix::IDENTITY),
            revision,
            target_id: self.root().map_or(-1, |root| root.id()),
            transformed_subtree_bounds: self.transformed_sub_tree_bounds.get(),
            subtree_bounds: self.sub_tree_bounds.get(),
            visible: self.visible(),
        }
    }

    pub fn update_readback(&self, writer_revision: u64, reader_revision: u64) {
        self.readback.update_readback(writer_revision, reader_revision, self.readback_data(writer_revision));
    }
}
