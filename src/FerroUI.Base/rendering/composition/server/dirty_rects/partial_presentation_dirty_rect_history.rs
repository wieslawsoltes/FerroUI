//! The dirty rectangles of the frames before the current one, for partial presentation
//! (upstream `PartialPresentationDirtyRectHistory`, 070c3be990).
//!
//! An output buffer can hold an older revision of the scene. The history supplies the scene
//! changes that bring that buffer up to date, besides those of the current frame. The repairs
//! of a buffer stay apart from the scene changes, so a repair does not spread to the other
//! buffers. The dirty rectangle tracker still merges, inflates for antialiasing, clips and
//! culls.

use super::IDirtyRectTracker;
use crate::platform::{IPartialPresentationBuffer, IPartialPresentationFeature, LtrbRect};
use crate::{Matrix, PixelRect};
use std::collections::VecDeque;
use std::rc::Rc;

/// The number of frames kept: a buffer older than that is repainted whole.
const MAX_HISTORY: usize = 16;

#[derive(Default)]
pub(crate) struct PartialPresentationDirtyRectHistory {
    /// The scene revision of each frame with its dirty rectangles before inflation, oldest
    /// first. The vector of a frame that falls out is reused for the next one.
    history: VecDeque<(u64, Vec<LtrbRect>)>,
    /// Each buffer with the last scene revision drawn into it. The order of the buffers
    /// varies; there are a few.
    buffers: Vec<(Rc<dyn IPartialPresentationBuffer>, u64)>,
    /// What [`before_draw`](Self::before_draw) captured of the pending frame.
    frame_uninflated_dirty_rects: Vec<LtrbRect>,
    frame_dirty_rects: Vec<PixelRect>,
    buffer_dirty_rects: Vec<PixelRect>,
    /// The feature of the drawing context of the frame, until
    /// [`after_successful_draw`](Self::after_successful_draw).
    partial_presentation: Option<Rc<dyn IPartialPresentationFeature>>,
    buffer: Option<Rc<dyn IPartialPresentationBuffer>>,
    revision: u64,
}

fn same_buffer(left: &Rc<dyn IPartialPresentationBuffer>, right: &Rc<dyn IPartialPresentationBuffer>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(left), Rc::as_ptr(right))
}

impl PartialPresentationDirtyRectHistory {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The finalized scene changes since the previous frame, in the coordinates of the output
    /// (rotated), once [`before_target_draw`](Self::before_target_draw) ran.
    pub(crate) fn frame_dirty_rects(&self) -> &[PixelRect] {
        &self.frame_dirty_rects
    }

    /// Every pixel the frame repaints in the output buffer, repairs included, in the
    /// coordinates of the output (rotated).
    pub(crate) fn buffer_dirty_rects(&self) -> &[PixelRect] {
        &self.buffer_dirty_rects
    }

    /// Called before anything of the frame is drawn, offscreen too: captures the frame.
    pub(crate) fn before_draw(
        &mut self,
        partial_presentation: Rc<dyn IPartialPresentationFeature>,
        tracker: &dyn IDirtyRectTracker,
    ) {
        // A buffer needs every revision after its last draw. Buffers whose contents are no
        // longer valid or whose history is gone are forgotten, so the target draw repaints
        // them whole.
        let oldest = self.revision.saturating_sub(self.history.len() as u64);
        self.buffers.retain(|(buffer, revision)| buffer.is_valid() && *revision >= oldest);

        self.buffer = partial_presentation.buffer();
        self.partial_presentation = Some(partial_presentation);
        // The replay uses the rectangles before inflation, so none is inflated twice; the
        // finalized pixel rectangles describe this frame to the platform.
        tracker.copy_uninflated_rects(&mut self.frame_uninflated_dirty_rects);
        tracker.copy_rects(&mut self.frame_dirty_rects);
    }

    /// Called before the output buffer is drawn to: makes the tracker hold everything the
    /// buffer needs repainted and declares the rectangles to the feature. `false` when the
    /// platform refused them (the frame is not presented).
    ///
    /// # Panics
    /// Panics when [`before_draw`](Self::before_draw) was not called for the frame.
    pub(crate) fn before_target_draw(
        &mut self,
        tracker: &dyn IDirtyRectTracker,
        bounds: LtrbRect,
        output_transform: Matrix,
    ) -> bool {
        let partial_presentation =
            self.partial_presentation.clone().expect("before_draw must be called before before_target_draw");

        // A tracker can cache its finalized region: it is rebuilt from the saved scene
        // rectangles, so finalization covers those of this frame and of the history.
        tracker.initialize(bounds);

        // A buffer the history does not know, or one that is no longer valid, is repainted
        // whole. Otherwise its contents hold its recorded revision: only the later scene
        // changes are replayed.
        let known = self.buffer.as_ref().filter(|buffer| buffer.is_valid()).and_then(|buffer| {
            self.buffers.iter().find(|(known, _)| same_buffer(known, buffer)).map(|(_, revision)| *revision)
        });
        match known {
            None => tracker.add_rect(bounds),
            Some(revision) => {
                for rect in &self.frame_uninflated_dirty_rects {
                    tracker.add_rect(*rect);
                }
                for (frame_revision, rects) in &self.history {
                    if *frame_revision > revision {
                        for rect in rects {
                            tracker.add_rect(*rect);
                        }
                    }
                }
            }
        }

        // Finalization merges the scene changes and the repairs by the policy of the tracker.
        tracker.finalize_frame(bounds);
        tracker.copy_rects(&mut self.buffer_dirty_rects);
        // The history and the culling of the scene stay in scene coordinates; only the
        // rectangles of the output are rotated.
        if !output_transform.is_identity() {
            transform_rects(&mut self.frame_dirty_rects, output_transform);
            transform_rects(&mut self.buffer_dirty_rects, output_transform);
        }
        // The platform is told before the first clear or draw to the output buffer.
        partial_presentation.set_dirty_rects(&self.frame_dirty_rects, &self.buffer_dirty_rects)
    }

    /// Called after a frame that was presented: records it.
    pub(crate) fn after_successful_draw(&mut self) {
        self.partial_presentation = None;
        self.revision += 1;
        let mut rects = if self.history.len() >= MAX_HISTORY {
            self.history.pop_front().map(|(_, rects)| rects).unwrap_or_default()
        } else {
            Vec::new()
        };
        rects.clear();
        rects.extend_from_slice(&self.frame_uninflated_dirty_rects);
        self.history.push_back((self.revision, rects));

        // The disposal of the drawing context can invalidate the buffer: only a valid buffer
        // holds this revision.
        if let Some(buffer) = self.buffer.take().filter(|buffer| buffer.is_valid()) {
            match self.buffers.iter_mut().find(|(known, _)| same_buffer(known, &buffer)) {
                Some(entry) => entry.1 = self.revision,
                None => self.buffers.push((buffer, self.revision)),
            }
        }
    }
}

fn transform_rects(rects: &mut [PixelRect], transform: Matrix) {
    for rect in rects {
        *rect = PixelRect::from_rect(rect.to_rect(1.0).transform_to_aabb(transform), 1.0);
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream, which tests the history in `PartialPresentationDirtyRectHistoryTests`;
    // these follow its cases with the single rectangle tracker.
    use super::*;
    use crate::rendering::composition::server::dirty_rects::{IDirtyRectCollector, SingleDirtyRectTracker};
    use std::cell::{Cell, RefCell};

    struct Buffer(Cell<bool>);

    impl IPartialPresentationBuffer for Buffer {
        fn is_valid(&self) -> bool {
            self.0.get()
        }
    }

    #[derive(Default)]
    struct Feature {
        buffer: RefCell<Option<Rc<dyn IPartialPresentationBuffer>>>,
        declared: RefCell<Vec<(Vec<PixelRect>, Vec<PixelRect>)>>,
    }

    impl IPartialPresentationFeature for Feature {
        fn buffer(&self) -> Option<Rc<dyn IPartialPresentationBuffer>> {
            self.buffer.borrow().clone()
        }

        fn set_dirty_rects(&self, frame: &[PixelRect], buffer: &[PixelRect]) -> bool {
            self.declared.borrow_mut().push((frame.to_vec(), buffer.to_vec()));
            true
        }

        fn succeeded(&self) {}
    }

    const BOUNDS: LtrbRect = LtrbRect::new(0.0, 0.0, 100.0, 100.0);

    /// Draws a frame with `dirty` into `buffer` and returns what was declared.
    fn frame(
        history: &mut PartialPresentationDirtyRectHistory,
        tracker: &SingleDirtyRectTracker,
        buffer: &Rc<Buffer>,
        dirty: LtrbRect,
    ) -> (Vec<PixelRect>, Vec<PixelRect>) {
        tracker.initialize(BOUNDS);
        tracker.add_rect(dirty);
        tracker.finalize_frame(BOUNDS);
        let feature = Rc::new(Feature::default());
        *feature.buffer.borrow_mut() = Some(buffer.clone());
        history.before_draw(feature.clone(), tracker);
        assert!(history.before_target_draw(tracker, BOUNDS, Matrix::IDENTITY));
        history.after_successful_draw();
        let declared = feature.declared.borrow()[0].clone();
        declared
    }

    fn full() -> Vec<PixelRect> {
        vec![PixelRect::new(0, 0, 100, 100)]
    }

    #[test]
    fn an_unknown_buffer_is_repainted_whole_and_a_known_one_by_its_changes() {
        let mut history = PartialPresentationDirtyRectHistory::new();
        let tracker = SingleDirtyRectTracker::new();
        let a = Rc::new(Buffer(Cell::new(true)));
        let b = Rc::new(Buffer(Cell::new(true)));

        let (frame_rects, buffer_rects) = frame(&mut history, &tracker, &a, LtrbRect::new(10.0, 10.0, 20.0, 20.0));
        assert_eq!(frame_rects, vec![PixelRect::new(9, 9, 12, 12)]);
        assert_eq!(buffer_rects, full());

        let (_, buffer_rects) = frame(&mut history, &tracker, &b, LtrbRect::new(30.0, 30.0, 40.0, 40.0));
        assert_eq!(buffer_rects, full());

        // Buffer A holds revision 1: it gets the change of revision 2 and its own.
        let (frame_rects, buffer_rects) = frame(&mut history, &tracker, &a, LtrbRect::new(50.0, 50.0, 60.0, 60.0));
        assert_eq!(frame_rects, vec![PixelRect::new(49, 49, 12, 12)]);
        assert_eq!(buffer_rects, vec![PixelRect::new(29, 29, 32, 32)]);

        // Buffer B holds revision 2: revision 3 and the frame.
        let (_, buffer_rects) = frame(&mut history, &tracker, &b, LtrbRect::new(50.0, 50.0, 55.0, 55.0));
        assert_eq!(buffer_rects, vec![PixelRect::new(49, 49, 12, 12)]);
    }

    #[test]
    fn an_invalid_buffer_is_repainted_whole() {
        let mut history = PartialPresentationDirtyRectHistory::new();
        let tracker = SingleDirtyRectTracker::new();
        let a = Rc::new(Buffer(Cell::new(true)));
        frame(&mut history, &tracker, &a, LtrbRect::new(10.0, 10.0, 20.0, 20.0));
        a.0.set(false);
        let (_, buffer_rects) = frame(&mut history, &tracker, &a, LtrbRect::new(10.0, 10.0, 20.0, 20.0));
        assert_eq!(buffer_rects, full());
    }

    #[test]
    fn a_buffer_older_than_the_history_is_repainted_whole() {
        let mut history = PartialPresentationDirtyRectHistory::new();
        let tracker = SingleDirtyRectTracker::new();
        let old = Rc::new(Buffer(Cell::new(true)));
        let other = Rc::new(Buffer(Cell::new(true)));
        frame(&mut history, &tracker, &old, LtrbRect::new(1.0, 1.0, 2.0, 2.0));
        for _ in 0..=MAX_HISTORY {
            frame(&mut history, &tracker, &other, LtrbRect::new(1.0, 1.0, 2.0, 2.0));
        }
        let (_, buffer_rects) = frame(&mut history, &tracker, &old, LtrbRect::new(1.0, 1.0, 2.0, 2.0));
        assert_eq!(buffer_rects, full());
        assert_eq!(history.history.len(), MAX_HISTORY);
    }

    #[test]
    fn the_rectangles_of_the_output_are_rotated() {
        let mut history = PartialPresentationDirtyRectHistory::new();
        let tracker = SingleDirtyRectTracker::new();
        let a = Rc::new(Buffer(Cell::new(true)));
        tracker.add_rect(LtrbRect::new(10.0, 20.0, 30.0, 40.0));
        tracker.finalize_frame(BOUNDS);
        let feature = Rc::new(Feature::default());
        *feature.buffer.borrow_mut() = Some(a.clone());
        history.before_draw(feature.clone(), &tracker);
        // A quarter turn of a 100x100 scene: (x, y) to (100 - y, x).
        let rotation = Matrix::new(0.0, 1.0, -1.0, 0.0, 100.0, 0.0);
        assert!(history.before_target_draw(&tracker, BOUNDS, rotation));
        assert_eq!(history.frame_dirty_rects(), &[PixelRect::new(59, 9, 22, 22)]);
        assert_eq!(history.buffer_dirty_rects(), &full()[..]);
    }
}
