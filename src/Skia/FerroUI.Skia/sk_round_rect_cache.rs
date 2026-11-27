use ferroui_base::RoundedRect;
use skia_safe::{RRect, Rect, Vector};

/// Rounded rectangles for the drawing context's call sites.
///
/// A Skia rounded rectangle is a plain value in Rust, so there is nothing to
/// pool: every function makes or takes a value on the stack. The pool this
/// type had (a thread-local vector of values) only added a thread-local
/// access, a borrow flag and vector traffic to every rounded draw; the drawing
/// context builds its values directly and does not use this type.
pub struct SkRoundRectCache;

impl SkRoundRectCache {
    /// An empty rounded rectangle.
    pub fn get() -> RRect {
        RRect::default()
    }

    /// A rounded rectangle with the bounds `rectangle` and the radii of
    /// `rounded_rect`.
    pub fn get_and_set_radii(rectangle: &Rect, rounded_rect: &RoundedRect) -> RRect {
        let radii = [
            Vector::new(rounded_rect.radii_top_left.x as f32, rounded_rect.radii_top_left.y as f32),
            Vector::new(rounded_rect.radii_top_right.x as f32, rounded_rect.radii_top_right.y as f32),
            Vector::new(rounded_rect.radii_bottom_right.x as f32, rounded_rect.radii_bottom_right.y as f32),
            Vector::new(rounded_rect.radii_bottom_left.x as f32, rounded_rect.radii_bottom_left.y as f32),
        ];
        Self::get_and_set_radii_points(rectangle, &radii)
    }

    /// A rounded rectangle with the specified radii (top-left, top-right,
    /// bottom-right, bottom-left).
    pub fn get_and_set_radii_points(rectangle: &Rect, radii: &[Vector; 4]) -> RRect {
        RRect::new_rect_radii(rectangle, radii)
    }

    /// Takes a rounded rectangle back: nothing to do for a value.
    pub fn return_item(rect: RRect) {
        let _ = rect;
    }

    /// Takes a rounded rectangle back: nothing to do for a value.
    pub fn return_reset(rect: RRect) {
        let _ = rect;
    }

    /// Nothing is cached.
    pub fn clear() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rounded_rectangle_is_made_with_the_radii_of_its_corners() {
        let rounded = RoundedRect::new(
            ferroui_base::Rect::new(0.0, 0.0, 40.0, 30.0),
            ferroui_base::Vector::new(1.0, 2.0),
            ferroui_base::Vector::new(3.0, 4.0),
            ferroui_base::Vector::new(5.0, 6.0),
            ferroui_base::Vector::new(7.0, 8.0),
        );
        let made = SkRoundRectCache::get_and_set_radii(&Rect::from_wh(40.0, 30.0), &rounded);
        let radii = [Vector::new(1.0, 2.0), Vector::new(3.0, 4.0), Vector::new(5.0, 6.0), Vector::new(7.0, 8.0)];
        assert_eq!(made.radii_ref(), &radii);
        assert!(SkRoundRectCache::get().is_empty());
    }
}
