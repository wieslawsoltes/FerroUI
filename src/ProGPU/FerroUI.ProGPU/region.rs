//! The region of the backend: a list of pixel rectangles. The backend does not draw with regions
//! (`supports_regions` is false); the type serves the callers that ask for one regardless.

use ferroui_base::platform::{IPlatformRenderInterfaceRegion, LtrbPixelRect, LtrbRect};
use ferroui_base::Point;
use std::any::Any;
use std::cell::RefCell;

/// A list of pixel rectangles.
#[derive(Default)]
pub struct ProGpuRegion {
    rects: RefCell<Vec<LtrbPixelRect>>,
}

impl IPlatformRenderInterfaceRegion for ProGpuRegion {
    fn add_rect(&self, rect: LtrbPixelRect) {
        if rect.right > rect.left && rect.bottom > rect.top {
            self.rects.borrow_mut().push(rect);
        }
    }

    fn reset(&self) {
        self.rects.borrow_mut().clear();
    }

    fn is_empty(&self) -> bool {
        self.rects.borrow().is_empty()
    }

    fn bounds(&self) -> LtrbPixelRect {
        let rects = self.rects.borrow();
        let Some(first) = rects.first() else { return LtrbPixelRect::new(0, 0, 0, 0) };
        rects.iter().skip(1).fold(*first, |bounds, rect| {
            LtrbPixelRect::new(bounds.left.min(rect.left), bounds.top.min(rect.top), bounds.right.max(rect.right), bounds.bottom.max(rect.bottom))
        })
    }

    fn rects(&self) -> Vec<LtrbPixelRect> {
        self.rects.borrow().clone()
    }

    fn intersects(&self, rect: LtrbRect) -> bool {
        self.rects.borrow().iter().any(|own| {
            (own.left as f64) < rect.right && rect.left < own.right as f64 && (own.top as f64) < rect.bottom && rect.top < own.bottom as f64
        })
    }

    fn contains(&self, point: Point) -> bool {
        self.rects.borrow().iter().any(|own| {
            point.x >= own.left as f64 && point.x < own.right as f64 && point.y >= own.top as f64 && point.y < own.bottom as f64
        })
    }

    fn dispose(&self) {
        self.reset();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
