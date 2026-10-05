use crate::sk_cache_base::SkCacheBase;
use ferroui_base::RoundedRect;
use skia_safe::{RRect, Rect, Vector};

thread_local! {
    static SHARED: SkCacheBase<RRect> = const { SkCacheBase::new() };
}

/// Cache of rounded rectangles that can be shared.
///
/// A Skia rounded rectangle is a plain value here, so the cache hands out
/// values rather than pooled heap objects; the type exists to keep the call
/// sites of the drawing context shaped like the paint cache's.
pub struct SkRoundRectCache;

impl SkRoundRectCache {
    /// Gets a cached, empty rounded rectangle.
    pub fn get() -> RRect {
        SHARED.with(|cache| cache.get())
    }

    /// Gets a cached round rect and sets it with the radii of `rounded_rect`.
    pub fn get_and_set_radii(rectangle: &Rect, rounded_rect: &RoundedRect) -> RRect {
        let radii = [
            Vector::new(rounded_rect.radii_top_left.x as f32, rounded_rect.radii_top_left.y as f32),
            Vector::new(rounded_rect.radii_top_right.x as f32, rounded_rect.radii_top_right.y as f32),
            Vector::new(rounded_rect.radii_bottom_right.x as f32, rounded_rect.radii_bottom_right.y as f32),
            Vector::new(rounded_rect.radii_bottom_left.x as f32, rounded_rect.radii_bottom_left.y as f32),
        ];
        Self::get_and_set_radii_points(rectangle, &radii)
    }

    /// Gets a cached round rect and sets it with the specified radii
    /// (top-left, top-right, bottom-right, bottom-left).
    pub fn get_and_set_radii_points(rectangle: &Rect, radii: &[Vector; 4]) -> RRect {
        let mut item = Self::get();
        item.set_rect_radii(rectangle, radii);
        item
    }

    /// Returns a round rect for reuse later, without resetting it.
    pub fn return_item(rect: RRect) {
        SHARED.with(|cache| cache.return_item(rect));
    }

    /// Returns a round rect and resets it for reuse later.
    pub fn return_reset(mut rect: RRect) {
        rect.set_empty();
        SHARED.with(|cache| cache.return_item(rect));
    }

    /// Clears all cached round rects.
    pub fn clear() {
        SHARED.with(|cache| cache.clear());
    }
}
