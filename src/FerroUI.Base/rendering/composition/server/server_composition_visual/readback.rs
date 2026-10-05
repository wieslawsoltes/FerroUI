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
//! through an `Arc`; each slot is guarded by its own mutex, which is never
//! contended when the protocol above is followed.

use super::ServerCompositionVisual;
use crate::platform::LtrbRect;
use crate::Matrix;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

/// What the UI thread reads back from a server visual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadbackData {
    pub matrix: Matrix,
    pub revision: u64,
    pub target_id: i64,
    pub visible: bool,
    pub transformed_subtree_bounds: Option<LtrbRect>,
}

struct Slot {
    revision: AtomicU64,
    data: Mutex<ReadbackData>,
}

impl Slot {
    fn new() -> Self {
        Self {
            revision: AtomicU64::new(u64::MAX),
            data: Mutex::new(ReadbackData {
                matrix: Matrix::IDENTITY,
                revision: u64::MAX,
                target_id: 0,
                visible: false,
                transformed_subtree_bounds: None,
            }),
        }
    }

    fn read(&self) -> ReadbackData {
        *self.data.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The two readback slots of a visual.
pub struct VisualReadback {
    readback0: Slot,
    readback1: Slot,
}

impl Default for VisualReadback {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualReadback {
    pub fn new() -> Self {
        Self { readback0: Slot::new(), readback1: Slot::new() }
    }

    pub fn get_readback(&self, reader_revision: u64) -> Option<ReadbackData> {
        let slot0_revision = self.readback0.revision.load(Ordering::SeqCst);
        let slot1_revision = self.readback1.revision.load(Ordering::SeqCst);

        if slot0_revision <= reader_revision && slot1_revision <= reader_revision {
            // Pick the newest one, it's guaranteed to be not touched by the writer
            return Some(if slot1_revision > slot0_revision { self.readback1.read() } else { self.readback0.read() });
        }

        if slot0_revision <= reader_revision {
            return Some(self.readback0.read());
        }

        if slot1_revision <= reader_revision {
            return Some(self.readback1.read());
        }

        // No readback was written for this visual yet
        None
    }

    pub fn update_readback(&self, writer_revision: u64, reader_revision: u64, mut data: ReadbackData) {
        let revision0 = self.readback0.revision.load(Ordering::SeqCst);
        let revision1 = self.readback1.revision.load(Ordering::SeqCst);
        let slot = if revision0 > reader_revision {
            // Future revision is in slot0
            &self.readback0
        } else if revision1 > reader_revision {
            // Future revision is in slot1
            &self.readback1
        } else if revision0 < revision1 {
            // No future revisions, overwrite the oldest one since reader will always pick the newest
            &self.readback0
        } else {
            &self.readback1
        };

        let mut guard = slot.data.lock().unwrap_or_else(PoisonError::into_inner);
        slot.revision.store(writer_revision, Ordering::SeqCst);
        data.revision = writer_revision;
        *guard = data;
    }
}

impl ServerCompositionVisual {
    pub(super) fn enqueue_for_readback_update(&self) {
        if !self.enqueued_for_readback_update.get() {
            if let (Some(compositor), Some(this)) = (self.compositor(), self.rc()) {
                self.enqueued_for_readback_update.set(true);
                compositor.enqueue_visual_for_readback_update_pass(this);
            }
        }
    }

    pub fn get_readback(&self, reader_revision: u64) -> Option<ReadbackData> {
        self.readback.get_readback(reader_revision)
    }

    pub fn update_readback(&self, writer_revision: u64, reader_revision: u64) {
        self.enqueued_for_readback_update.set(false);
        self.readback.update_readback(
            writer_revision,
            reader_revision,
            ReadbackData {
                matrix: self.own_transform.get().unwrap_or(Matrix::IDENTITY),
                revision: writer_revision,
                target_id: self.root().map_or(-1, |root| root.id()),
                transformed_subtree_bounds: self.transformed_sub_tree_bounds.get(),
                visible: self.visible(),
            },
        );
    }
}
