use super::i_dirty_rect_tracker::{
    full_union, pixel_rect_from_rect_unscaled, simplify_dirty_rects, unite_dirty_rects, VisualizeRandom,
};
use super::multi_dirty_rect_tracker_c_dirty_region::CDirtyRegion2;
use super::{IDirtyRectCollector, IDirtyRectTracker};
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::Color;
use crate::platform::{IDrawingContextImpl, IPlatformRenderInterface, IPlatformRenderInterfaceRegion, LtrbRect};
use crate::Thickness;
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

/// Tracks the dirty area as a bounded set of rectangles, merging the ones
/// that waste the least area when the set is full.
pub struct MultiDirtyRectTracker {
    max_overhead: f64,
    regions: RefCell<CDirtyRegion2>,
    clip_region: Rc<dyn IPlatformRenderInterfaceRegion>,
    inflated_rects: RefCell<Vec<LtrbRect>>,
    random: VisualizeRandom,
    combined_rect: Cell<LtrbRect>,
    /// Whether a frame draws the one rectangle that holds its dirty
    /// rectangles (`draw_as_one_rectangle`).
    one_rectangle: Cell<bool>,
}

impl MultiDirtyRectTracker {
    pub fn new(platform_render: &dyn IPlatformRenderInterface, max_dirty_rects: i32, max_overhead: f64) -> Self {
        Self {
            max_overhead,
            regions: RefCell::new(CDirtyRegion2::new(max_dirty_rects)),
            clip_region: platform_render.create_region(),
            inflated_rects: RefCell::new(Vec::new()),
            random: VisualizeRandom::new(),
            combined_rect: Cell::new(LtrbRect::default()),
            one_rectangle: Cell::new(false),
        }
    }

    /// Makes the clip and the combined rectangle of the final rectangles,
    /// which are first made as simple as the area they cover allows, and
    /// one rectangle where the frame is to draw one.
    fn publish(&self, bounds: LtrbRect) {
        let mut inflated_rects = self.inflated_rects.borrow_mut();
        simplify_dirty_rects(&mut inflated_rects, bounds);
        if self.one_rectangle.get() {
            unite_dirty_rects(&mut inflated_rects, bounds);
        }
        self.clip_region.reset();
        let mut combined: Option<LtrbRect> = None;
        for rect in inflated_rects.iter() {
            self.clip_region.add_rect(pixel_rect_from_rect_unscaled(*rect));
            combined = full_union(combined, Some(*rect));
        }
        self.combined_rect.set(combined.unwrap_or_default());
    }

    /// The dirty rectangles of the last finalized frame, inflated and
    /// clipped to the frame bounds.
    pub fn inflated_rects(&self) -> Ref<'_, [LtrbRect]> {
        Ref::map(self.inflated_rects.borrow(), |rects| rects.as_slice())
    }
}

impl IDirtyRectCollector for MultiDirtyRectTracker {
    fn add_rect(&self, rect: LtrbRect) {
        self.regions.borrow_mut().add(rect);
    }
}

impl IDirtyRectTracker for MultiDirtyRectTracker {
    fn finalize_frame(&self, bounds: LtrbRect) {
        {
            let mut inflated_rects = self.inflated_rects.borrow_mut();
            inflated_rects.clear();
            let mut regions = self.regions.borrow_mut();
            for rect in regions.get_uninflated_dirty_regions() {
                inflated_rects.push(rect.inflate(Thickness::uniform(1.0)).intersect_or_empty(bounds));
            }
        }
        self.publish(bounds);
    }

    fn include(&self, rect: LtrbRect, bounds: LtrbRect) {
        self.inflated_rects.borrow_mut().push(rect.inflate(Thickness::uniform(1.0)).intersect_or_empty(bounds));
        self.publish(bounds);
    }

    fn begin_draw(&self, ctx: &mut dyn IDrawingContextImpl) {
        if self.one_rectangle.get() {
            ctx.push_clip(self.combined_rect.get().to_rect());
        } else {
            ctx.push_clip_region(&*self.clip_region);
        }
    }

    fn draw_as_one_rectangle(&self, one: bool) {
        self.one_rectangle.set(one);
    }

    fn end_draw(&self, ctx: &mut dyn IDrawingContextImpl) {
        ctx.pop_clip();
    }

    fn is_empty(&self) -> bool {
        self.regions.borrow().is_empty()
    }

    fn intersects(&self, rect: LtrbRect) -> bool {
        self.inflated_rects.borrow().iter().any(|r| r.intersects(rect))
    }

    fn initialize(&self, bounds: LtrbRect) {
        self.regions.borrow_mut().initialize(bounds, self.max_overhead);
        self.inflated_rects.borrow_mut().clear();
        self.clip_region.reset();
        self.combined_rect.set(LtrbRect::default());
    }

    fn visualize(&self, context: &mut dyn IDrawingContextImpl) {
        let brush = ImmutableSolidColorBrush::new(Color::new(
            150,
            self.random.next(255) as u8,
            self.random.next(255) as u8,
            self.random.next(255) as u8,
        ));
        context.draw_region(Some(&brush), None, &*self.clip_region);
    }

    fn combined_rect(&self) -> LtrbRect {
        self.combined_rect.get()
    }
}

#[cfg(test)]
mod tests {
    use super::super::i_dirty_rect_tracker::tests::{MockDrawingContext, MockRenderInterface};
    use super::*;
    use crate::platform::LtrbPixelRect;

    const BOUNDS: LtrbRect = LtrbRect::new(0.0, 0.0, 1000.0, 1000.0);

    #[test]
    fn finalize_inflates_by_one_and_clips_to_bounds() {
        let platform = MockRenderInterface::default();
        let tracker = MultiDirtyRectTracker::new(&platform, 4, 0.0);
        let region = platform.regions.borrow()[0].clone();
        tracker.initialize(BOUNDS);
        assert!(tracker.is_empty());

        tracker.add_rect(LtrbRect::new(0.0, 0.0, 10.0, 10.0));
        tracker.add_rect(LtrbRect::new(500.5, 500.5, 510.2, 520.0));
        assert!(!tracker.is_empty());
        assert!(tracker.inflated_rects().is_empty());

        tracker.finalize_frame(BOUNDS);
        // With no allowed overhead the newest rectangle ends up in the first slot.
        assert_eq!(
            vec![LtrbRect::new(499.0, 499.0, 512.0, 521.0), LtrbRect::new(0.0, 0.0, 11.0, 11.0)],
            tracker.inflated_rects().to_vec()
        );
        assert_eq!(
            vec![LtrbPixelRect::new(499, 499, 512, 521), LtrbPixelRect::new(0, 0, 11, 11)],
            *region.rects.borrow()
        );
        assert_eq!(LtrbRect::new(0.0, 0.0, 512.0, 521.0), tracker.combined_rect());

        assert!(tracker.intersects(LtrbRect::new(10.5, 10.5, 20.0, 20.0)));
        assert!(!tracker.intersects(LtrbRect::new(11.0, 11.0, 499.0, 499.0)));
        assert!(tracker.intersects(LtrbRect::new(100.0, 100.0, 499.5, 499.5)));
    }

    #[test]
    fn initialize_resets_everything() {
        let platform = MockRenderInterface::default();
        let tracker = MultiDirtyRectTracker::new(&platform, 4, 0.0);
        let region = platform.regions.borrow()[0].clone();
        tracker.initialize(BOUNDS);
        tracker.add_rect(LtrbRect::new(5.0, 5.0, 10.0, 10.0));
        tracker.finalize_frame(BOUNDS);
        assert_eq!(1, region.rects.borrow().len());

        tracker.initialize(BOUNDS);
        assert!(tracker.is_empty());
        assert!(tracker.inflated_rects().is_empty());
        assert!(region.rects.borrow().is_empty());
        assert_eq!(LtrbRect::default(), tracker.combined_rect());
        tracker.finalize_frame(BOUNDS);
        assert_eq!(LtrbRect::default(), tracker.combined_rect());
    }

    #[test]
    fn rects_added_before_initialize_are_dropped() {
        // The surface bounds are empty until `initialize` is called.
        let platform = MockRenderInterface::default();
        let tracker = MultiDirtyRectTracker::new(&platform, 4, 0.0);
        tracker.add_rect(LtrbRect::new(5.0, 5.0, 10.0, 10.0));
        assert!(tracker.is_empty());
    }

    #[test]
    fn invalid_rect_makes_the_whole_surface_dirty() {
        let platform = MockRenderInterface::default();
        let tracker = MultiDirtyRectTracker::new(&platform, 4, 0.0);
        tracker.initialize(BOUNDS);
        tracker.add_rect(LtrbRect::new(5.0, 5.0, 10.0, 10.0));
        tracker.add_rect(LtrbRect::new(f64::NAN, 5.0, 10.0, 10.0));
        tracker.finalize_frame(BOUNDS);
        // The surface bounds, inflated and clipped back to the bounds.
        assert_eq!(vec![BOUNDS], tracker.inflated_rects().to_vec());
        assert_eq!(BOUNDS, tracker.combined_rect());
    }

    #[test]
    fn rectangles_that_fill_their_union_are_drawn_as_one_rectangle() {
        let platform = MockRenderInterface::default();
        let tracker = MultiDirtyRectTracker::new(&platform, 4, 0.0);
        let region = platform.regions.borrow()[0].clone();
        tracker.initialize(BOUNDS);
        tracker.add_rect(LtrbRect::new(100.0, 100.0, 500.0, 280.0));
        tracker.add_rect(LtrbRect::new(100.0, 320.0, 500.0, 500.0));
        tracker.finalize_frame(BOUNDS);
        assert_eq!(vec![LtrbRect::new(99.0, 99.0, 501.0, 501.0)], tracker.inflated_rects().to_vec());
        assert_eq!(vec![LtrbPixelRect::new(99, 99, 501, 501)], *region.rects.borrow());
        assert_eq!(LtrbRect::new(99.0, 99.0, 501.0, 501.0), tracker.combined_rect());
        // What lies between the two is drawn as well.
        assert!(tracker.intersects(LtrbRect::new(200.0, 290.0, 300.0, 310.0)));
    }

    #[test]
    fn rectangles_that_are_all_but_the_surface_are_drawn_as_the_surface() {
        let platform = MockRenderInterface::default();
        let tracker = MultiDirtyRectTracker::new(&platform, 4, 0.0);
        let region = platform.regions.borrow()[0].clone();
        tracker.initialize(BOUNDS);
        tracker.add_rect(LtrbRect::new(5.0, 0.0, 995.0, 480.0));
        tracker.add_rect(LtrbRect::new(5.0, 500.0, 995.0, 1000.0));
        tracker.finalize_frame(BOUNDS);
        assert_eq!(vec![BOUNDS], tracker.inflated_rects().to_vec());
        assert_eq!(vec![LtrbPixelRect::new(0, 0, 1000, 1000)], *region.rects.borrow());
        assert_eq!(BOUNDS, tracker.combined_rect());
    }

    #[test]
    fn an_included_rectangle_joins_the_final_rectangles() {
        let platform = MockRenderInterface::default();
        let tracker = MultiDirtyRectTracker::new(&platform, 4, 0.0);
        let region = platform.regions.borrow()[0].clone();
        tracker.initialize(BOUNDS);
        tracker.add_rect(LtrbRect::new(0.0, 0.0, 10.0, 10.0));
        // Nothing is final before the frame is: the tracker answers for the
        // rectangles it draws.
        assert!(!tracker.intersects(LtrbRect::new(0.0, 0.0, 10.0, 10.0)));
        tracker.finalize_frame(BOUNDS);
        assert!(tracker.intersects(LtrbRect::new(5.0, 5.0, 600.0, 20.0)));

        tracker.include(LtrbRect::new(5.0, 5.0, 600.0, 20.0), BOUNDS);
        assert_eq!(
            vec![LtrbRect::new(0.0, 0.0, 11.0, 11.0), LtrbRect::new(4.0, 4.0, 601.0, 21.0)],
            tracker.inflated_rects().to_vec()
        );
        assert_eq!(
            vec![LtrbPixelRect::new(0, 0, 11, 11), LtrbPixelRect::new(4, 4, 601, 21)],
            *region.rects.borrow()
        );
        assert_eq!(LtrbRect::new(0.0, 0.0, 601.0, 21.0), tracker.combined_rect());
        assert!(tracker.intersects(LtrbRect::new(550.0, 15.0, 700.0, 400.0)));

        // An included rectangle that fills the union with the others is
        // drawn with them as one.
        tracker.include(LtrbRect::new(0.0, 0.0, 600.0, 20.0), BOUNDS);
        assert_eq!(vec![LtrbRect::new(0.0, 0.0, 601.0, 21.0)], tracker.inflated_rects().to_vec());
        assert_eq!(vec![LtrbPixelRect::new(0, 0, 601, 21)], *region.rects.borrow());
    }

    #[test]
    fn a_frame_asked_for_one_rectangle_draws_the_rectangle_that_holds_its_dirty_rectangles() {
        let platform = MockRenderInterface::default();
        let tracker = MultiDirtyRectTracker::new(&platform, 4, 0.0);
        let region = platform.regions.borrow()[0].clone();
        let between = LtrbRect::new(200.0, 200.0, 210.0, 210.0);
        let frame = |tracker: &MultiDirtyRectTracker| {
            tracker.initialize(BOUNDS);
            tracker.add_rect(LtrbRect::new(10.0, 10.0, 20.0, 20.0));
            tracker.add_rect(LtrbRect::new(500.5, 500.5, 510.2, 520.0));
            tracker.finalize_frame(BOUNDS);
        };
        let clip_of = |tracker: &MultiDirtyRectTracker| {
            let mut ctx = MockDrawingContext::default();
            tracker.begin_draw(&mut ctx);
            tracker.end_draw(&mut ctx);
            ctx.log
        };

        // As the tracker makes it: two rectangles far apart, a region, and
        // nothing of what lies between them.
        frame(&tracker);
        assert_eq!(2, tracker.inflated_rects().len());
        assert!(!tracker.intersects(between));
        assert!(clip_of(&tracker)[0].starts_with("push_clip_region ["), "{:?}", clip_of(&tracker));

        // Asked for one rectangle: the one that holds both, on whole pixels,
        // is what the frame draws, what it tests the visuals against and
        // what it clips to, with a clip of a rectangle.
        tracker.draw_as_one_rectangle(true);
        frame(&tracker);
        let one = LtrbRect::new(9.0, 9.0, 512.0, 521.0);
        assert_eq!(vec![one], tracker.inflated_rects().to_vec());
        assert_eq!(one, tracker.combined_rect());
        assert_eq!(vec![LtrbPixelRect::new(9, 9, 512, 521)], *region.rects.borrow());
        assert!(tracker.intersects(between));
        assert!(!tracker.intersects(LtrbRect::new(600.0, 30.0, 700.0, 40.0)));
        assert_eq!(vec!["push_clip 9, 9, 503, 512".to_string(), "pop_clip".to_string()], clip_of(&tracker));

        // A rectangle included after the frame was made final joins it.
        tracker.include(LtrbRect::new(600.0, 30.0, 700.0, 40.0), BOUNDS);
        let wider = LtrbRect::new(9.0, 9.0, 701.0, 521.0);
        assert_eq!(vec![wider], tracker.inflated_rects().to_vec());
        assert_eq!(wider, tracker.combined_rect());
        assert_eq!(vec![LtrbPixelRect::new(9, 9, 701, 521)], *region.rects.borrow());
        assert_eq!("push_clip 9, 9, 692, 512", clip_of(&tracker)[0]);

        // One dirty rectangle is drawn on whole pixels as well, and two that
        // span the surface are the surface.
        tracker.initialize(BOUNDS);
        tracker.add_rect(LtrbRect::new(500.5, 500.5, 510.2, 520.0));
        tracker.finalize_frame(BOUNDS);
        assert_eq!(vec![LtrbRect::new(499.0, 499.0, 512.0, 521.0)], tracker.inflated_rects().to_vec());
        tracker.initialize(BOUNDS);
        tracker.add_rect(LtrbRect::new(0.0, 0.0, 10.0, 10.0));
        tracker.add_rect(LtrbRect::new(990.0, 990.0, 1000.0, 1000.0));
        tracker.finalize_frame(BOUNDS);
        assert_eq!(vec![BOUNDS], tracker.inflated_rects().to_vec());
        assert_eq!("push_clip 0, 0, 1000, 1000", clip_of(&tracker)[0]);

        // A frame with nothing to draw has an empty clip.
        tracker.initialize(BOUNDS);
        tracker.finalize_frame(BOUNDS);
        assert!(tracker.inflated_rects().is_empty());
        assert_eq!(LtrbRect::default(), tracker.combined_rect());

        // Not asked any more: the dirty area as the tracker makes it.
        tracker.draw_as_one_rectangle(false);
        frame(&tracker);
        assert_eq!(2, tracker.inflated_rects().len());
        assert_eq!(2, region.rects.borrow().len());
        assert!(!tracker.intersects(between));
        assert!(clip_of(&tracker)[0].starts_with("push_clip_region ["));
    }

    #[test]
    fn draw_uses_the_region_clip() {
        let platform = MockRenderInterface::default();
        let tracker = MultiDirtyRectTracker::new(&platform, 4, 0.0);
        tracker.initialize(BOUNDS);
        tracker.add_rect(LtrbRect::new(5.0, 5.0, 10.0, 10.0));
        tracker.finalize_frame(BOUNDS);
        let mut ctx = MockDrawingContext::default();
        tracker.begin_draw(&mut ctx);
        tracker.end_draw(&mut ctx);
        tracker.visualize(&mut ctx);
        assert_eq!(
            vec![
                "push_clip_region [LtrbPixelRect { left: 4, top: 4, right: 11, bottom: 11 }]".to_string(),
                "pop_clip".to_string(),
                "region alpha 150 rects 1".to_string()
            ],
            ctx.log
        );
    }
}
