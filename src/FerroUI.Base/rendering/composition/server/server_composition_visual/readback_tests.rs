//! Tests of the readback slots and of the queue the readback pass writes
//! them from (desktop-performance.md, 9.30).

use super::{ReadbackData, ServerCompositionContainerVisual, ServerCompositionVisual, VisualReadback};
use crate::platform::LtrbRect;
use crate::rendering::composition::server::{BatchQueue, ReadbackIndices, ServerCompositor};
use crate::rendering::composition::CompositionOptions;
use crate::Matrix;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

fn data(offset: f64) -> ReadbackData {
    ReadbackData {
        matrix: Matrix::create_translation(offset, -offset),
        revision: 0,
        target_id: 7,
        visible: true,
        transformed_subtree_bounds: Some(LtrbRect::new(offset, 0.0, offset + 10.0, 5.0)),
        subtree_bounds: Some(LtrbRect::new(0.0, 0.0, 10.0, 5.0)),
    }
}

fn with_revision(mut data: ReadbackData, revision: u64) -> ReadbackData {
    data.revision = revision;
    data
}

// --- the slots -------------------------------------------------------------------------

#[test]
fn nothing_is_read_before_the_first_write() {
    let readback = VisualReadback::new();
    assert_eq!(None, readback.get_readback(0));
    assert_eq!(None, readback.get_readback(u64::MAX - 1));
    assert_eq!(None, readback.readback_revision(5));
}

#[test]
fn every_value_survives_a_slot() {
    let readback = VisualReadback::new();
    let written = ReadbackData {
        matrix: Matrix::new_3x3(1.5, -2.0, 0.25, 3.0, 4.5, -0.5, 100.0, -200.0, 0.75),
        revision: 0,
        target_id: -1,
        visible: false,
        transformed_subtree_bounds: None,
        subtree_bounds: Some(LtrbRect::new(-1.0, f64::NEG_INFINITY, f64::INFINITY, 4.0)),
    };
    readback.update_readback(1, 0, written);
    assert_eq!(Some(with_revision(written, 1)), readback.get_readback(1));

    // The other slot, with the opposite flags.
    let written = ReadbackData {
        visible: true,
        target_id: i64::MAX,
        transformed_subtree_bounds: Some(LtrbRect::new(1.0, 2.0, 3.0, 4.0)),
        subtree_bounds: None,
        ..written
    };
    readback.update_readback(2, 1, written);
    assert_eq!(Some(with_revision(written, 2)), readback.get_readback(2));
    assert_eq!(-1, readback.get_readback(1).unwrap().target_id);
}

#[test]
fn a_reader_keeps_its_revision_while_newer_ones_are_written() {
    let readback = VisualReadback::new();
    readback.update_readback(1, 0, data(1.0));
    // The reader is still at the revision before the write.
    assert_eq!(None, readback.get_readback(0));
    assert_eq!(Some(with_revision(data(1.0), 1)), readback.get_readback(1));

    // Two newer revisions while the reader stays at the first: both go to
    // the slot the reader does not read.
    readback.update_readback(2, 1, data(2.0));
    assert_eq!(Some(with_revision(data(1.0), 1)), readback.get_readback(1));
    readback.update_readback(3, 1, data(3.0));
    assert_eq!(Some(with_revision(data(1.0), 1)), readback.get_readback(1));
    assert_eq!(Some(1), readback.readback_revision(1));

    // The reader moves on and the writer takes the slot it left.
    assert_eq!(Some(with_revision(data(3.0), 3)), readback.get_readback(3));
    readback.update_readback(4, 3, data(4.0));
    assert_eq!(Some(with_revision(data(3.0), 3)), readback.get_readback(3));
    assert_eq!(Some(with_revision(data(4.0), 4)), readback.get_readback(4));
    assert_eq!(Some(4), readback.readback_revision(9));
}

#[test]
fn two_writes_of_one_revision_leave_the_last() {
    let readback = VisualReadback::new();
    readback.update_readback(1, 0, data(1.0));
    readback.update_readback(2, 1, data(2.0));
    readback.update_readback(2, 1, data(5.0));
    assert_eq!(Some(with_revision(data(1.0), 1)), readback.get_readback(1));
    assert_eq!(Some(with_revision(data(5.0), 2)), readback.get_readback(2));
}

// --- the queue of the pass -------------------------------------------------------------

struct Server {
    compositor: Rc<ServerCompositor>,
    visual: Rc<ServerCompositionVisual>,
    readback: Arc<VisualReadback>,
}

impl Server {
    fn new() -> Self {
        let compositor = ServerCompositor::new(
            None,
            CompositionOptions::default(),
            Arc::new(BatchQueue::default()),
            Arc::new(ReadbackIndices::new()),
            Rc::new(|| Duration::ZERO),
        );
        let readback = Arc::new(VisualReadback::new());
        let visual = Self::visual(&compositor, readback.clone());
        Self { compositor, visual, readback }
    }

    fn visual(compositor: &Rc<ServerCompositor>, readback: Arc<VisualReadback>) -> Rc<ServerCompositionVisual> {
        ServerCompositionVisual::new(compositor, None, readback, Box::new(ServerCompositionContainerVisual))
    }

    /// What the update walk leaves a visual with.
    fn finalize(&self, offset: f64) {
        self.visual.set_own_transform(Some(Matrix::create_translation(offset, 0.0)));
        self.visual.sub_tree_bounds.set(Some(LtrbRect::new(0.0, 0.0, 10.0, 5.0)));
        self.visual.transformed_sub_tree_bounds.set(Some(LtrbRect::new(offset, 0.0, offset + 10.0, 5.0)));
        self.visual.snapshot_for_readback_update();
    }

    /// Runs the pass and reads what it wrote, as the UI thread does.
    fn pass(&self) -> ReadbackData {
        self.compositor.visual_readback_update_pass();
        let indices = self.compositor.readback();
        indices.next_read();
        self.readback.get_readback(indices.read_revision()).expect("the pass wrote the visual")
    }
}

#[test]
fn the_pass_writes_what_the_update_walk_left() {
    let s = Server::new();
    s.finalize(30.0);
    assert_eq!(1, s.compositor.readback_queue_len());
    // The copy does not hold the visual.
    assert_eq!(1, Rc::strong_count(&s.visual));

    let read = s.pass();
    assert_eq!(0, s.compositor.readback_queue_len());
    assert_eq!(Matrix::create_translation(30.0, 0.0), read.matrix);
    assert_eq!(Some(LtrbRect::new(30.0, 0.0, 40.0, 5.0)), read.transformed_subtree_bounds);
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 10.0, 5.0)), read.subtree_bounds);
    assert_eq!(-1, read.target_id);
    assert_eq!(1, read.revision);
}

#[test]
fn a_change_announced_after_the_copy_is_read_by_the_pass() {
    let s = Server::new();
    s.finalize(30.0);
    // The transform changes after the walk copied it.
    s.visual.set_own_transform(Some(Matrix::create_translation(70.0, 0.0)));
    assert_eq!(1, s.compositor.readback_queue_len());
    assert_eq!(2, Rc::strong_count(&s.visual));
    assert_eq!(Matrix::create_translation(70.0, 0.0), s.pass().matrix);
    assert_eq!(1, Rc::strong_count(&s.visual));

    // So does a change of the visibility or of the other values of the readback.
    s.finalize(10.0);
    s.visual.trigger_visible_dirty();
    s.visual.sub_tree_bounds.set(None);
    assert_eq!(None, s.pass().subtree_bounds);

    // And the walk copies again when it visits the visual once more.
    s.finalize(10.0);
    s.visual.trigger_combined_transform_dirty();
    s.finalize(20.0);
    assert_eq!(1, s.compositor.readback_queue_len());
    assert_eq!(Matrix::create_translation(20.0, 0.0), s.pass().matrix);
}

#[test]
fn a_visual_is_in_the_queue_once_per_pass() {
    let s = Server::new();
    s.visual.enqueue_for_readback_update();
    s.visual.enqueue_for_readback_update();
    s.finalize(1.0);
    s.finalize(2.0);
    s.visual.enqueue_for_readback_update();
    s.visual.enqueue_for_readback_update();
    assert_eq!(1, s.compositor.readback_queue_len());
    let first = s.pass();
    assert_eq!(Matrix::create_translation(2.0, 0.0), first.matrix);

    // Nothing was enqueued: the pass does not start a revision.
    s.compositor.visual_readback_update_pass();
    assert_eq!(first.revision, s.compositor.readback().write_revision());

    // The next pass takes the visual again, in either form.
    s.visual.enqueue_for_readback_update();
    assert_eq!(1, s.compositor.readback_queue_len());
    assert_eq!(first.revision + 1, s.pass().revision);
    s.finalize(3.0);
    assert_eq!(1, s.compositor.readback_queue_len());
    let third = s.pass();
    assert_eq!(first.revision + 2, third.revision);
    assert_eq!(Matrix::create_translation(3.0, 0.0), third.matrix);
}

#[test]
fn the_visuals_of_a_pass_share_its_revision() {
    let s = Server::new();
    let other_readback = Arc::new(VisualReadback::new());
    let other = Server::visual(&s.compositor, other_readback.clone());
    let third_readback = Arc::new(VisualReadback::new());
    let third = Server::visual(&s.compositor, third_readback.clone());

    // A copy, a visual the pass reads, and a copy turned back into a read.
    s.finalize(1.0);
    other.set_own_transform(Some(Matrix::create_translation(2.0, 0.0)));
    other.enqueue_for_readback_update();
    third.snapshot_for_readback_update();
    third.set_own_transform(Some(Matrix::create_translation(3.0, 0.0)));
    assert_eq!(3, s.compositor.readback_queue_len());

    let first = s.pass();
    let revision = s.compositor.readback().read_revision();
    assert_eq!(revision, first.revision);
    let second = other_readback.get_readback(revision).unwrap();
    let last = third_readback.get_readback(revision).unwrap();
    assert_eq!((revision, revision), (second.revision, last.revision));
    assert_eq!(Matrix::create_translation(2.0, 0.0), second.matrix);
    assert_eq!(Matrix::create_translation(3.0, 0.0), last.matrix);

    // A visual released while it waits in the queue is written all the same.
    other.enqueue_for_readback_update();
    drop(other);
    s.compositor.visual_readback_update_pass();
    s.compositor.readback().next_read();
    assert_eq!(Some(revision + 1), other_readback.readback_revision(revision + 1));
}
