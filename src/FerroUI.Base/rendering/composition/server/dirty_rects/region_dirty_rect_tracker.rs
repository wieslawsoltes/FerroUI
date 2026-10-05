use super::i_dirty_rect_tracker::{
    pixel_rect_from_rect_unscaled, pixel_rect_to_ltrb_rect_unscaled, VisualizeRandom,
};
use super::{IDirtyRectCollector, IDirtyRectTracker};
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::Color;
use crate::platform::{
    IDrawingContextImpl, IPlatformRenderInterface, IPlatformRenderInterfaceRegion, LtrbPixelRect, LtrbRect,
};
use crate::Thickness;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Tracks the dirty area with a platform region holding every added
/// rectangle.
pub struct RegionDirtyRectTracker {
    region: Rc<dyn IPlatformRenderInterfaceRegion>,
    rects: RefCell<Vec<LtrbRect>>,
    random: VisualizeRandom,
    combined_rect: Cell<LtrbRect>,
}

impl RegionDirtyRectTracker {
    pub fn new(platform_render: &dyn IPlatformRenderInterface) -> Self {
        Self {
            region: platform_render.create_region(),
            rects: RefCell::new(Vec::new()),
            random: VisualizeRandom::new(),
            combined_rect: Cell::new(LtrbRect::default()),
        }
    }

    fn get_inflated_pixel_rect(rc: LtrbRect) -> LtrbPixelRect {
        // The inflated rectangle is intersected with the rectangle itself, as
        // in the C# source: the inflation is effectively undone.
        let inflated = rc.inflate(Thickness::uniform(1.0)).intersect_or_empty(rc);
        pixel_rect_from_rect_unscaled(inflated)
    }
}

impl IDirtyRectCollector for RegionDirtyRectTracker {
    fn add_rect(&self, rect: LtrbRect) {
        self.rects.borrow_mut().push(rect);
    }
}

impl IDirtyRectTracker for RegionDirtyRectTracker {
    fn finalize_frame(&self, _bounds: LtrbRect) {
        self.region.reset();
        for rc in self.rects.borrow().iter() {
            self.region.add_rect(Self::get_inflated_pixel_rect(*rc));
        }
        self.combined_rect.set(pixel_rect_to_ltrb_rect_unscaled(self.region.bounds()));
    }

    fn begin_draw(&self, ctx: &mut dyn IDrawingContextImpl) {
        ctx.push_clip_region(&*self.region);
    }

    fn end_draw(&self, ctx: &mut dyn IDrawingContextImpl) {
        ctx.pop_clip();
    }

    fn is_empty(&self) -> bool {
        self.rects.borrow().is_empty()
    }

    fn intersects(&self, rect: LtrbRect) -> bool {
        self.region.intersects(rect)
    }

    fn initialize(&self, _bounds: LtrbRect) {
        self.rects.borrow_mut().clear();
    }

    fn visualize(&self, context: &mut dyn IDrawingContextImpl) {
        let brush = ImmutableSolidColorBrush::new(Color::new(
            150,
            self.random.next(255) as u8,
            self.random.next(255) as u8,
            self.random.next(255) as u8,
        ));
        context.draw_region(Some(&brush), None, &*self.region);
    }

    fn combined_rect(&self) -> LtrbRect {
        self.combined_rect.get()
    }
}

#[cfg(test)]
mod tests {
    use super::super::i_dirty_rect_tracker::tests::{MockDrawingContext, MockRenderInterface};
    use super::*;

    const BOUNDS: LtrbRect = LtrbRect::new(0.0, 0.0, 100.0, 100.0);

    #[test]
    fn collects_rects_into_the_platform_region_on_finalize() {
        let platform = MockRenderInterface::default();
        let tracker = RegionDirtyRectTracker::new(&platform);
        let region = platform.regions.borrow()[0].clone();
        assert!(tracker.is_empty());

        tracker.add_rect(LtrbRect::new(10.2, 10.7, 20.1, 20.0));
        tracker.add_rect(LtrbRect::new(50.0, 60.0, 70.0, 80.0));
        assert!(!tracker.is_empty());
        assert!(region.rects.borrow().is_empty());

        tracker.finalize_frame(BOUNDS);
        // The inflation is undone by the self-intersection; only the pixel
        // snapping (truncate left/top, ceil right/bottom) remains.
        assert_eq!(
            vec![LtrbPixelRect::new(10, 10, 21, 20), LtrbPixelRect::new(50, 60, 70, 80)],
            *region.rects.borrow()
        );
        assert_eq!(LtrbRect::new(10.0, 10.0, 70.0, 80.0), tracker.combined_rect());
        assert!(tracker.intersects(LtrbRect::new(15.0, 15.0, 16.0, 16.0)));
        assert!(!tracker.intersects(LtrbRect::new(30.0, 30.0, 40.0, 40.0)));
    }

    #[test]
    fn finalize_rebuilds_the_region_and_initialize_clears_the_rects() {
        let platform = MockRenderInterface::default();
        let tracker = RegionDirtyRectTracker::new(&platform);
        let region = platform.regions.borrow()[0].clone();
        tracker.add_rect(LtrbRect::new(1.0, 1.0, 2.0, 2.0));
        tracker.finalize_frame(BOUNDS);
        tracker.finalize_frame(BOUNDS);
        assert_eq!(1, region.rects.borrow().len());

        tracker.initialize(BOUNDS);
        assert!(tracker.is_empty());
        // The region keeps the last finalized frame until the next finalize.
        assert_eq!(1, region.rects.borrow().len());
        tracker.finalize_frame(BOUNDS);
        assert!(region.rects.borrow().is_empty());
        assert_eq!(LtrbRect::default(), tracker.combined_rect());
    }

    #[test]
    fn draw_uses_the_region_clip() {
        let platform = MockRenderInterface::default();
        let tracker = RegionDirtyRectTracker::new(&platform);
        tracker.add_rect(LtrbRect::new(1.0, 1.0, 2.0, 2.0));
        tracker.finalize_frame(BOUNDS);
        let mut ctx = MockDrawingContext::default();
        tracker.begin_draw(&mut ctx);
        tracker.end_draw(&mut ctx);
        tracker.visualize(&mut ctx);
        assert_eq!(
            vec![
                "push_clip_region [LtrbPixelRect { left: 1, top: 1, right: 2, bottom: 2 }]".to_string(),
                "pop_clip".to_string(),
                "region alpha 150 rects 1".to_string()
            ],
            ctx.log
        );
    }
}
