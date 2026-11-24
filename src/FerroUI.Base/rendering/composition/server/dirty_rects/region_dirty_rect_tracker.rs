use super::i_dirty_rect_tracker::{
    pixel_rect_from_rect_unscaled, pixel_rect_to_ltrb_rect_unscaled, separate_dirty_rects, VisualizeRandom,
};
use super::{IDirtyRectCollector, IDirtyRectTracker};
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::Color;
use crate::platform::{
    IDrawingContextImpl, IPlatformRenderInterface, IPlatformRenderInterfaceRegion, LtrbPixelRect, LtrbRect,
};
use crate::{PixelRect, Thickness};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Tracks the dirty area with a platform region holding every added
/// rectangle.
pub struct RegionDirtyRectTracker {
    region: Rc<dyn IPlatformRenderInterfaceRegion>,
    rects: RefCell<Vec<LtrbRect>>,
    random: VisualizeRandom,
    combined_rect: Cell<LtrbRect>,
    /// The bounds of the surface of the last finalized frame, which the inflation stays in.
    bounds: Cell<LtrbRect>,
}

impl RegionDirtyRectTracker {
    pub fn new(platform_render: &dyn IPlatformRenderInterface) -> Self {
        Self {
            region: platform_render.create_region(),
            rects: RefCell::new(Vec::new()),
            random: VisualizeRandom::new(),
            combined_rect: Cell::new(LtrbRect::default()),
            bounds: Cell::new(LtrbRect::default()),
        }
    }

    /// Makes the combined rectangle of the region.
    fn publish(&self) {
        self.combined_rect.set(pixel_rect_to_ltrb_rect_unscaled(self.region.bounds()));
    }

    /// The rectangle inflated by a pixel for antialiasing, inside the bounds of the surface
    /// (upstream 070c3be990 fixed the intersection, which was with the rectangle itself and
    /// undid the inflation).
    fn get_inflated_pixel_rect(rc: LtrbRect, bounds: LtrbRect) -> LtrbPixelRect {
        let inflated = rc.inflate(Thickness::uniform(1.0)).intersect_or_empty(bounds);
        pixel_rect_from_rect_unscaled(inflated)
    }
}

impl IDirtyRectCollector for RegionDirtyRectTracker {
    fn add_rect(&self, rect: LtrbRect) {
        self.rects.borrow_mut().push(rect);
    }
}

impl IDirtyRectTracker for RegionDirtyRectTracker {
    fn finalize_frame(&self, bounds: LtrbRect) {
        self.bounds.set(bounds);
        self.region.reset();
        for rc in self.rects.borrow().iter() {
            self.region.add_rect(Self::get_inflated_pixel_rect(*rc, bounds));
        }
        self.publish();
    }

    fn include(&self, rect: LtrbRect, bounds: LtrbRect) {
        self.bounds.set(bounds);
        self.rects.borrow_mut().push(rect);
        self.region.add_rect(Self::get_inflated_pixel_rect(rect, bounds));
        self.publish();
    }

    fn begin_draw(&self, ctx: &mut dyn IDrawingContextImpl) {
        ctx.push_clip_region(&*self.region);
    }

    fn separate_rects(&self, rects: &mut Vec<LtrbRect>) {
        rects.clear();
        let added = self.rects.borrow();
        let bounds = self.bounds.get();
        rects.extend(
            added.iter().map(|rect| pixel_rect_to_ltrb_rect_unscaled(Self::get_inflated_pixel_rect(*rect, bounds))),
        );
        separate_dirty_rects(rects);
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

    fn copy_rects(&self, rects: &mut Vec<PixelRect>) {
        rects.clear();
        rects.extend(self.region.rects().into_iter().map(|rect| rect.to_pixel_rect()));
    }

    fn copy_uninflated_rects(&self, rects: &mut Vec<LtrbRect>) {
        rects.clear();
        rects.extend_from_slice(&self.rects.borrow());
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
        // Each rectangle is inflated by a pixel inside the bounds, then
        // snapped to pixels (truncate left/top, ceil right/bottom).
        assert_eq!(
            vec![LtrbPixelRect::new(9, 9, 22, 21), LtrbPixelRect::new(49, 59, 71, 81)],
            *region.rects.borrow()
        );
        assert_eq!(LtrbRect::new(9.0, 9.0, 71.0, 81.0), tracker.combined_rect());
        assert!(tracker.intersects(LtrbRect::new(15.0, 15.0, 16.0, 16.0)));
        assert!(!tracker.intersects(LtrbRect::new(30.0, 30.0, 40.0, 40.0)));
    }

    #[test]
    fn an_included_rectangle_joins_the_final_region() {
        let platform = MockRenderInterface::default();
        let tracker = RegionDirtyRectTracker::new(&platform);
        let region = platform.regions.borrow()[0].clone();
        tracker.add_rect(LtrbRect::new(10.0, 10.0, 20.0, 20.0));
        tracker.finalize_frame(BOUNDS);
        assert!(!tracker.intersects(LtrbRect::new(30.0, 30.0, 40.0, 40.0)));
        tracker.include(LtrbRect::new(15.0, 15.0, 40.0, 50.0), BOUNDS);
        assert_eq!(
            vec![LtrbPixelRect::new(9, 9, 21, 21), LtrbPixelRect::new(14, 14, 41, 51)],
            *region.rects.borrow()
        );
        assert_eq!(LtrbRect::new(9.0, 9.0, 41.0, 51.0), tracker.combined_rect());
        assert!(tracker.intersects(LtrbRect::new(30.0, 30.0, 40.0, 40.0)));
        // The frame after it starts from the rectangles that were added.
        tracker.finalize_frame(BOUNDS);
        assert_eq!(2, region.rects.borrow().len());
    }

    #[test]
    fn the_rectangles_of_a_frame_drawn_one_at_a_time_are_those_of_its_region_apart() {
        let platform = MockRenderInterface::default();
        let tracker = RegionDirtyRectTracker::new(&platform);
        let region = platform.regions.borrow()[0].clone();
        let mut rects = vec![LtrbRect::new(1.0, 2.0, 3.0, 4.0)];

        tracker.finalize_frame(BOUNDS);
        tracker.separate_rects(&mut rects);
        assert!(rects.is_empty());

        // The rectangles that were added, on the pixels of the region, the
        // larger first.
        tracker.add_rect(LtrbRect::new(10.2, 10.7, 20.1, 20.0));
        tracker.add_rect(LtrbRect::new(50.0, 60.0, 70.0, 80.0));
        tracker.finalize_frame(BOUNDS);
        tracker.separate_rects(&mut rects);
        assert_eq!(vec![LtrbRect::new(49.0, 59.0, 71.0, 81.0), LtrbRect::new(9.0, 9.0, 22.0, 21.0)], rects);

        // One included after the frame was made final is among them, and
        // one it is close to is drawn with it, as the rectangle that
        // holds both.
        tracker.include(LtrbRect::new(60.0, 70.0, 90.0, 90.0), BOUNDS);
        tracker.separate_rects(&mut rects);
        assert_eq!(vec![LtrbRect::new(49.0, 59.0, 91.0, 91.0), LtrbRect::new(9.0, 9.0, 22.0, 21.0)], rects);

        // The frame is what it was: a region, and nothing between its
        // rectangles.
        assert_eq!(3, region.rects.borrow().len());
        assert!(!tracker.intersects(LtrbRect::new(30.0, 30.0, 40.0, 40.0)));
        let mut ctx = MockDrawingContext::default();
        tracker.begin_draw(&mut ctx);
        assert!(ctx.log[0].starts_with("push_clip_region ["), "{:?}", ctx.log);
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
    fn the_copies_are_the_region_in_pixels_and_the_rectangles_as_added() {
        let platform = MockRenderInterface::default();
        let tracker = RegionDirtyRectTracker::new(&platform);
        tracker.add_rect(LtrbRect::new(10.2, 10.7, 20.1, 20.0));
        tracker.finalize_frame(BOUNDS);
        let mut pixels = vec![PixelRect::new(1, 1, 1, 1)];
        tracker.copy_rects(&mut pixels);
        assert_eq!(vec![PixelRect::new(9, 9, 13, 12)], pixels);
        let mut uninflated = Vec::new();
        tracker.copy_uninflated_rects(&mut uninflated);
        assert_eq!(vec![LtrbRect::new(10.2, 10.7, 20.1, 20.0)], uninflated);
        // The inflation stays inside the bounds.
        tracker.initialize(BOUNDS);
        tracker.add_rect(LtrbRect::new(0.0, 0.0, 100.0, 100.0));
        tracker.finalize_frame(BOUNDS);
        tracker.copy_rects(&mut pixels);
        assert_eq!(vec![PixelRect::new(0, 0, 100, 100)], pixels);
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
