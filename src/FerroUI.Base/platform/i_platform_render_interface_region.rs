use super::{LtrbPixelRect, LtrbRect};
use crate::Point;

/// A platform region: a set of pixel rectangles.
pub trait IPlatformRenderInterfaceRegion {
    fn add_rect(&self, rect: LtrbPixelRect);
    fn reset(&self);
    fn is_empty(&self) -> bool;
    fn bounds(&self) -> LtrbPixelRect;
    fn rects(&self) -> Vec<LtrbPixelRect>;
    fn intersects(&self, rect: LtrbRect) -> bool;
    fn contains(&self, pt: Point) -> bool;
    fn dispose(&self);
    /// Lets the backend recover its concrete type.
    fn as_any(&self) -> &dyn std::any::Any;
}
