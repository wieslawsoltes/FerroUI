use crate::utilities::span_helpers::InvariantF64;
use crate::{Matrix, PixelRect, Point, Rect, Thickness};
use std::fmt;

/// A rectangle stored as left/top/right/bottom edges.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LtrbRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl LtrbRect {
    pub const INFINITE: LtrbRect =
        LtrbRect { left: f64::NEG_INFINITY, top: f64::NEG_INFINITY, right: f64::INFINITY, bottom: f64::INFINITY };

    pub const fn new(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self { left, top, right, bottom }
    }

    pub fn from_rect(rc: Rect) -> Self {
        let rc = rc.normalize();
        Self { left: rc.x, top: rc.y, right: rc.right(), bottom: rc.bottom() }
    }

    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }

    pub fn is_well_ordered(&self) -> bool {
        self.left <= self.right && self.top <= self.bottom
    }

    pub fn is_zero_size(&self) -> bool {
        self.left == self.right || self.top == self.bottom
    }

    pub fn is_empty(&self) -> bool {
        self.is_zero_size()
    }

    pub fn null_if_zero_size(self) -> Option<LtrbRect> {
        (!self.is_zero_size()).then_some(self)
    }

    pub fn intersect_or_null(&self, rect: LtrbRect) -> Option<LtrbRect> {
        let left = if rect.left > self.left { rect.left } else { self.left };
        let top = if rect.top > self.top { rect.top } else { self.top };
        let right = if rect.right < self.right { rect.right } else { self.right };
        let bottom = if rect.bottom < self.bottom { rect.bottom } else { self.bottom };
        (right > left && bottom > top).then_some(LtrbRect::new(left, top, right, bottom))
    }

    pub fn intersect_or_empty(&self, rect: LtrbRect) -> LtrbRect {
        self.intersect_or_null(rect).unwrap_or_default()
    }

    pub fn intersects(&self, rect: LtrbRect) -> bool {
        rect.left < self.right && self.left < rect.right && rect.top < self.bottom && self.top < rect.bottom
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }

    pub fn to_rect(&self) -> Rect {
        Rect::new(self.left, self.top, self.right - self.left, self.bottom - self.top)
    }

    pub fn inflate(&self, thickness: Thickness) -> LtrbRect {
        LtrbRect::new(
            self.left - thickness.left,
            self.top - thickness.top,
            self.right + thickness.right,
            self.bottom + thickness.bottom,
        )
    }

    /// The axis-aligned bounding box of the rectangle transformed by
    /// `matrix`.
    pub fn top_left(&self) -> Point {
        Point::new(self.left, self.top)
    }

    pub fn top_right(&self) -> Point {
        Point::new(self.right, self.top)
    }

    pub fn bottom_left(&self) -> Point {
        Point::new(self.left, self.bottom)
    }

    pub fn bottom_right(&self) -> Point {
        Point::new(self.right, self.bottom)
    }

    pub fn transform_to_aabb(&self, matrix: Matrix) -> LtrbRect {
        let points = [
            self.top_left().transform(matrix),
            self.top_right().transform(matrix),
            self.bottom_right().transform(matrix),
            self.bottom_left().transform(matrix),
        ];

        let mut left = f64::MAX;
        let mut right = f64::MIN;
        let mut top = f64::MAX;
        let mut bottom = f64::MIN;

        for p in points {
            if p.x < left {
                left = p.x;
            }
            if p.x > right {
                right = p.x;
            }
            if p.y < top {
                top = p.y;
            }
            if p.y > bottom {
                bottom = p.y;
            }
        }

        LtrbRect::new(left, top, right, bottom)
    }

    /// The union of two optional rectangles, where `None` is the empty
    /// set.
    pub fn full_union(left: Option<LtrbRect>, right: Option<LtrbRect>) -> Option<LtrbRect> {
        match (left, right) {
            (None, right) => right,
            (left, None) => left,
            (Some(left), Some(right)) => Some(right.union(left)),
        }
    }

    /// The union of an optional rectangle with an optional `Rect`, where
    /// `None` is the empty set.
    pub fn full_union_rect(left: Option<LtrbRect>, right: Option<Rect>) -> Option<LtrbRect> {
        match (left, right) {
            (left, None) => left,
            (None, Some(right)) => Some(LtrbRect::from_rect(right)),
            (Some(left), Some(right)) => Some(left.union(LtrbRect::from_rect(right))),
        }
    }

    pub fn union(&self, rect: LtrbRect) -> LtrbRect {
        LtrbRect::new(
            self.left.min(rect.left),
            self.top.min(rect.top),
            self.right.max(rect.right),
            self.bottom.max(rect.bottom),
        )
    }

    /// Whether the rectangle contains another one, edges included.
    pub fn contains_rect(&self, rect: LtrbRect) -> bool {
        rect.left >= self.left && rect.right <= self.right && rect.top >= self.top && rect.bottom <= self.bottom
    }
}

impl fmt::Display for LtrbRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}-{}:{} ({}x{})",
            InvariantF64(self.left),
            InvariantF64(self.top),
            InvariantF64(self.right),
            InvariantF64(self.bottom),
            InvariantF64(self.width()),
            InvariantF64(self.height())
        )
    }
}

/// A pixel rectangle stored as left/top/right/bottom edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LtrbPixelRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl LtrbPixelRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }

    pub fn from_pixel_rect(rc: PixelRect) -> Self {
        Self { left: rc.x, top: rc.y, right: rc.right(), bottom: rc.bottom() }
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn is_empty(&self) -> bool {
        self.left == self.right && self.top == self.bottom
    }

    pub fn to_pixel_rect(&self) -> PixelRect {
        PixelRect::new(self.left, self.top, self.right - self.left, self.bottom - self.top)
    }

    pub fn union(&self, rect: LtrbPixelRect) -> LtrbPixelRect {
        LtrbPixelRect::new(
            self.left.min(rect.left),
            self.top.min(rect.top),
            self.right.max(rect.right),
            self.bottom.max(rect.bottom),
        )
    }

    /// Converts a logical rectangle to the smallest pixel rectangle that
    /// contains it at the given scaling.
    pub fn from_rect_with_dpi(rect: LtrbRect, scaling: f64) -> Self {
        Self {
            left: (rect.left * scaling).floor() as i32,
            top: (rect.top * scaling).floor() as i32,
            right: (rect.right * scaling).ceil() as i32,
            bottom: (rect.bottom * scaling).ceil() as i32,
        }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }

    pub fn to_rect_unscaled(&self) -> Rect {
        Rect::new(
            f64::from(self.left),
            f64::from(self.top),
            f64::from(self.right - self.left),
            f64::from(self.bottom - self.top),
        )
    }

    pub fn to_ltrb_rect_unscaled(&self) -> LtrbRect {
        LtrbRect::new(f64::from(self.left), f64::from(self.top), f64::from(self.right), f64::from(self.bottom))
    }

    pub fn from_rect_unscaled(rect: LtrbRect) -> Self {
        Self {
            left: rect.left as i32,
            top: rect.top as i32,
            right: rect.right.ceil() as i32,
            bottom: rect.bottom.ceil() as i32,
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of these types.
    use super::*;

    #[test]
    fn corners_and_containment() {
        let rect = LtrbRect::new(1.0, 2.0, 5.0, 8.0);

        assert_eq!(Point::new(1.0, 2.0), rect.top_left());
        assert_eq!(Point::new(5.0, 2.0), rect.top_right());
        assert_eq!(Point::new(1.0, 8.0), rect.bottom_left());
        assert_eq!(Point::new(5.0, 8.0), rect.bottom_right());

        assert!(rect.contains_rect(rect));
        assert!(rect.contains_rect(LtrbRect::new(2.0, 3.0, 4.0, 4.0)));
        assert!(!rect.contains_rect(LtrbRect::new(0.0, 3.0, 4.0, 4.0)));
        assert_eq!("1:2-5:8 (4x6)", rect.to_string());
    }

    #[test]
    fn full_union_with_a_rect() {
        let left = LtrbRect::new(0.0, 0.0, 1.0, 1.0);
        let right = Rect::new(2.0, 2.0, 2.0, 2.0);

        assert_eq!(None, LtrbRect::full_union_rect(None, None));
        assert_eq!(Some(left), LtrbRect::full_union_rect(Some(left), None));
        assert_eq!(Some(LtrbRect::new(2.0, 2.0, 4.0, 4.0)), LtrbRect::full_union_rect(None, Some(right)));
        assert_eq!(Some(LtrbRect::new(0.0, 0.0, 4.0, 4.0)), LtrbRect::full_union_rect(Some(left), Some(right)));
    }

    #[test]
    fn pixel_rect_conversions_without_scaling() {
        let rect = LtrbPixelRect::new(1, 2, 5, 8);

        assert!(rect.contains(1, 2));
        assert!(rect.contains(5, 8));
        assert!(!rect.contains(6, 8));
        assert_eq!(Rect::new(1.0, 2.0, 4.0, 6.0), rect.to_rect_unscaled());
        assert_eq!(LtrbRect::new(1.0, 2.0, 5.0, 8.0), rect.to_ltrb_rect_unscaled());

        let rounded = LtrbPixelRect::from_rect_unscaled(LtrbRect::new(1.7, 2.2, 4.1, 7.5));
        assert_eq!(LtrbPixelRect::new(1, 2, 5, 8), rounded);
    }
}
