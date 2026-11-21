use super::{LtrbPixelRect, LtrbRect};
use crate::Point;

/// A platform region: a set of pixel rectangles.
pub trait IPlatformRenderInterfaceRegion: std::any::Any {
    fn add_rect(&self, rect: LtrbPixelRect);
    fn reset(&self);
    fn is_empty(&self) -> bool;
    fn bounds(&self) -> LtrbPixelRect;
    fn rects(&self) -> Vec<LtrbPixelRect>;
    fn intersects(&self, rect: LtrbRect) -> bool;
    fn contains(&self, pt: Point) -> bool;
    fn dispose(&self);
}

impl dyn IPlatformRenderInterfaceRegion {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
