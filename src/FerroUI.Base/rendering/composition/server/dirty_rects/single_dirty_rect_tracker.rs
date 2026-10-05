use super::i_dirty_rect_tracker::{
    full_union, pixel_rect_from_rect_unscaled, pixel_rect_to_ltrb_rect_unscaled, VisualizeRandom,
};
use super::{IDirtyRectCollector, IDirtyRectTracker};
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{BoxShadows, Color};
use crate::platform::{IDrawingContextImpl, LtrbRect};
use crate::Thickness;
use std::cell::Cell;

/// Tracks the dirty area as a single bounding rectangle.
pub struct SingleDirtyRectTracker {
    rect: Cell<Option<LtrbRect>>,
    extended_rect: Cell<LtrbRect>,
    random: VisualizeRandom,
}

impl SingleDirtyRectTracker {
    pub fn new() -> Self {
        Self { rect: Cell::new(None), extended_rect: Cell::new(LtrbRect::default()), random: VisualizeRandom::new() }
    }
}

impl Default for SingleDirtyRectTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl IDirtyRectCollector for SingleDirtyRectTracker {
    fn add_rect(&self, rect: LtrbRect) {
        self.rect.set(full_union(self.rect.get(), Some(rect)));
    }
}

impl IDirtyRectTracker for SingleDirtyRectTracker {
    fn finalize_frame(&self, bounds: LtrbRect) {
        self.extended_rect.set(match self.rect.get() {
            Some(rect) => pixel_rect_to_ltrb_rect_unscaled(pixel_rect_from_rect_unscaled(
                rect.inflate(Thickness::uniform(1.0)).intersect_or_empty(bounds),
            )),
            None => LtrbRect::default(),
        });
    }

    fn begin_draw(&self, ctx: &mut dyn IDrawingContextImpl) {
        ctx.push_clip(self.extended_rect.get().to_rect());
    }

    fn end_draw(&self, ctx: &mut dyn IDrawingContextImpl) {
        ctx.pop_clip();
    }

    fn is_empty(&self) -> bool {
        self.rect.get().map(|r| r.is_zero_size()).unwrap_or(true)
    }

    fn intersects(&self, rect: LtrbRect) -> bool {
        self.extended_rect.get().intersects(rect)
    }

    fn initialize(&self, _bounds: LtrbRect) {
        self.rect.set(None);
    }

    fn visualize(&self, context: &mut dyn IDrawingContextImpl) {
        let brush = ImmutableSolidColorBrush::new(Color::new(
            30,
            self.random.next(255) as u8,
            self.random.next(255) as u8,
            self.random.next(255) as u8,
        ));
        context.draw_rectangle(Some(&brush), None, self.extended_rect.get().to_rect().into(), &BoxShadows::default());
    }

    fn is_single_dirty_rect_tracker(&self) -> bool {
        true
    }

    fn combined_rect(&self) -> LtrbRect {
        self.extended_rect.get()
    }
}

#[cfg(test)]
mod tests {
    use super::super::i_dirty_rect_tracker::tests::MockDrawingContext;
    use super::*;

    const BOUNDS: LtrbRect = LtrbRect::new(0.0, 0.0, 100.0, 100.0);

    #[test]
    fn starts_empty() {
        let tracker = SingleDirtyRectTracker::new();
        assert!(tracker.is_empty());
        tracker.finalize_frame(BOUNDS);
        assert_eq!(LtrbRect::default(), tracker.combined_rect());
        assert!(!tracker.intersects(BOUNDS));
    }

    #[test]
    fn unions_rects_and_inflates_on_finalize() {
        let tracker = SingleDirtyRectTracker::new();
        tracker.initialize(BOUNDS);
        tracker.add_rect(LtrbRect::new(10.0, 10.0, 20.0, 20.0));
        tracker.add_rect(LtrbRect::new(40.5, 30.0, 50.5, 35.2));
        assert!(!tracker.is_empty());
        // Nothing is visible before the frame is finalized.
        assert_eq!(LtrbRect::default(), tracker.combined_rect());

        tracker.finalize_frame(BOUNDS);
        // Union (10,10)-(50.5,35.2), inflated by 1, right/bottom rounded up.
        assert_eq!(LtrbRect::new(9.0, 9.0, 52.0, 37.0), tracker.combined_rect());
        assert!(tracker.intersects(LtrbRect::new(51.0, 36.0, 60.0, 60.0)));
        assert!(!tracker.intersects(LtrbRect::new(52.0, 37.0, 60.0, 60.0)));
    }

    #[test]
    fn finalize_clips_to_bounds() {
        let tracker = SingleDirtyRectTracker::new();
        tracker.add_rect(LtrbRect::new(-20.0, 90.0, 300.0, 400.0));
        tracker.finalize_frame(BOUNDS);
        assert_eq!(LtrbRect::new(0.0, 89.0, 100.0, 100.0), tracker.combined_rect());

        // Entirely outside: the intersection is empty.
        let tracker = SingleDirtyRectTracker::new();
        tracker.add_rect(LtrbRect::new(200.0, 200.0, 300.0, 300.0));
        tracker.finalize_frame(BOUNDS);
        assert_eq!(LtrbRect::default(), tracker.combined_rect());
        assert!(!tracker.is_empty());
    }

    #[test]
    fn zero_size_rect_counts_as_empty() {
        let tracker = SingleDirtyRectTracker::new();
        tracker.add_rect(LtrbRect::new(5.0, 5.0, 5.0, 50.0));
        assert!(tracker.is_empty());
    }

    #[test]
    fn initialize_resets_the_accumulated_rect_but_not_the_finalized_one() {
        let tracker = SingleDirtyRectTracker::new();
        tracker.add_rect(LtrbRect::new(10.0, 10.0, 20.0, 20.0));
        tracker.finalize_frame(BOUNDS);
        tracker.initialize(BOUNDS);
        assert!(tracker.is_empty());
        assert_eq!(LtrbRect::new(9.0, 9.0, 21.0, 21.0), tracker.combined_rect());
        tracker.finalize_frame(BOUNDS);
        assert_eq!(LtrbRect::default(), tracker.combined_rect());
    }

    #[test]
    fn begin_and_end_draw_push_and_pop_the_clip() {
        let tracker = SingleDirtyRectTracker::new();
        tracker.add_rect(LtrbRect::new(10.0, 10.0, 20.0, 20.0));
        tracker.finalize_frame(BOUNDS);
        let mut ctx = MockDrawingContext::default();
        tracker.begin_draw(&mut ctx);
        tracker.end_draw(&mut ctx);
        tracker.visualize(&mut ctx);
        assert_eq!(3, ctx.log.len());
        assert_eq!("push_clip 9, 9, 12, 12", ctx.log[0]);
        assert_eq!("pop_clip", ctx.log[1]);
        assert!(ctx.log[2].starts_with("rect #1e"), "{}", ctx.log[2]);
        assert!(ctx.log[2].ends_with(" 9, 9, 12, 12"), "{}", ctx.log[2]);
    }
}
