use super::i_dirty_rect_tracker::{full_union, pixel_rect_from_rect_unscaled, VisualizeRandom};
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
        }
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
        let mut inflated_rects = self.inflated_rects.borrow_mut();
        inflated_rects.clear();
        self.clip_region.reset();

        let mut regions = self.regions.borrow_mut();
        let dirty_regions = regions.get_uninflated_dirty_regions();

        let mut combined: Option<LtrbRect> = None;
        for rect in dirty_regions {
            let inflated = rect.inflate(Thickness::uniform(1.0)).intersect_or_empty(bounds);
            inflated_rects.push(inflated);
            self.clip_region.add_rect(pixel_rect_from_rect_unscaled(inflated));
            combined = full_union(combined, Some(inflated));
        }

        self.combined_rect.set(combined.unwrap_or_default());
    }

    fn begin_draw(&self, ctx: &mut dyn IDrawingContextImpl) {
        ctx.push_clip_region(&*self.clip_region);
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
