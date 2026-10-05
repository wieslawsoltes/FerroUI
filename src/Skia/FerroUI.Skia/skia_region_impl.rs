use crate::skia_sharp_extensions::to_ltrb_pixel_rect;
use ferroui_base::platform::{IPlatformRenderInterfaceRegion, LtrbPixelRect, LtrbRect};
use ferroui_base::Point;
use skia_safe::region::RegionOp;
use skia_safe::{IPoint, IRect, Region};
use std::any::Any;
use std::cell::{Ref, RefCell};

/// A Skia implementation of a platform region.
pub struct SkiaRegionImpl {
    region: RefCell<Option<Region>>,
    rects: RefCell<Option<Vec<LtrbPixelRect>>>,
}

impl SkiaRegionImpl {
    /// Creates an empty region.
    pub fn new() -> Self {
        Self { region: RefCell::new(Some(Region::new())), rects: RefCell::new(None) }
    }

    /// The Skia region.
    ///
    /// # Panics
    /// Panics when the region has been disposed.
    pub fn region(&self) -> Ref<'_, Region> {
        Ref::map(self.region.borrow(), |region| region.as_ref().expect("SkiaRegionImpl has been disposed"))
    }

    fn with_region_mut<R>(&self, f: impl FnOnce(&mut Region) -> R) -> R {
        let mut region = self.region.borrow_mut();
        f(region.as_mut().expect("SkiaRegionImpl has been disposed"))
    }
}

impl Default for SkiaRegionImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl IPlatformRenderInterfaceRegion for SkiaRegionImpl {
    fn add_rect(&self, rect: LtrbPixelRect) {
        *self.rects.borrow_mut() = None;
        self.with_region_mut(|region| {
            region.op_rect(IRect::new(rect.left, rect.top, rect.right, rect.bottom), RegionOp::Union)
        });
    }

    fn reset(&self) {
        *self.rects.borrow_mut() = None;
        self.with_region_mut(|region| region.set_empty());
    }

    fn is_empty(&self) -> bool {
        self.region().is_empty()
    }

    fn bounds(&self) -> LtrbPixelRect {
        to_ltrb_pixel_rect(*self.region().bounds())
    }

    fn rects(&self) -> Vec<LtrbPixelRect> {
        let mut rects = self.rects.borrow_mut();
        rects
            .get_or_insert_with(|| {
                let region = self.region();
                let mut list = Vec::new();
                let mut iter = skia_safe::region::Iterator::new(&region);
                while !iter.is_done() {
                    list.push(to_ltrb_pixel_rect(*iter.rect()));
                    iter.next();
                }
                list
            })
            .clone()
    }

    fn intersects(&self, rect: LtrbRect) -> bool {
        self.region().intersects_rect(IRect::new(
            rect.left as i32,
            rect.top as i32,
            rect.right.ceil() as i32,
            rect.bottom.ceil() as i32,
        ))
    }

    fn contains(&self, pt: Point) -> bool {
        self.region().contains_point(IPoint::new(pt.x as i32, pt.y as i32))
    }

    fn dispose(&self) {
        *self.region.borrow_mut() = None;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
