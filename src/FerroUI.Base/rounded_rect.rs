//! Represents a rectangle with (possibly elliptical) rounded corners.

use crate::utilities::math_utilities;
use crate::{CornerRadius, Point, Rect, Thickness, Vector};

/// Represents a rectangle with per-corner radii.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct RoundedRect {
    pub rect: Rect,
    pub radii_top_left: Vector,
    pub radii_top_right: Vector,
    pub radii_bottom_left: Vector,
    pub radii_bottom_right: Vector,
}

impl RoundedRect {
    /// Initializes a [`RoundedRect`] from elliptical corner radii, clockwise from the top left.
    #[inline]
    pub const fn new(
        rect: Rect,
        radii_top_left: Vector,
        radii_top_right: Vector,
        radii_bottom_right: Vector,
        radii_bottom_left: Vector,
    ) -> Self {
        Self {
            rect,
            radii_top_left,
            radii_top_right,
            radii_bottom_left,
            radii_bottom_right,
        }
    }

    /// Initializes a [`RoundedRect`] from circular corner radii, clockwise from the top left.
    #[inline]
    pub const fn from_radii(
        rect: Rect,
        radius_top_left: f64,
        radius_top_right: f64,
        radius_bottom_right: f64,
        radius_bottom_left: f64,
    ) -> Self {
        Self::new(
            rect,
            Vector::new(radius_top_left, radius_top_left),
            Vector::new(radius_top_right, radius_top_right),
            Vector::new(radius_bottom_right, radius_bottom_right),
            Vector::new(radius_bottom_left, radius_bottom_left),
        )
    }

    /// Initializes a [`RoundedRect`] with the same elliptical radii on every corner.
    #[inline]
    pub const fn from_uniform_radii(rect: Rect, radii: Vector) -> Self {
        Self::new(rect, radii, radii, radii, radii)
    }

    /// Initializes a [`RoundedRect`] with the same X and Y radius on every corner.
    #[inline]
    pub const fn from_radius_xy(rect: Rect, radius_x: f64, radius_y: f64) -> Self {
        Self::from_uniform_radii(rect, Vector::new(radius_x, radius_y))
    }

    /// Initializes a [`RoundedRect`] with the same circular radius on every corner.
    #[inline]
    pub const fn from_radius(rect: Rect, radius: f64) -> Self {
        Self::from_radius_xy(rect, radius, radius)
    }

    /// Initializes a [`RoundedRect`] without rounded corners.
    #[inline]
    pub const fn from_rect(rect: Rect) -> Self {
        Self::from_radius(rect, 0.0)
    }

    /// Initializes a [`RoundedRect`] from bounds and a [`CornerRadius`].
    #[inline]
    pub const fn from_corner_radius(bounds: Rect, radius: CornerRadius) -> Self {
        Self::from_radii(
            bounds,
            radius.top_left,
            radius.top_right,
            radius.bottom_right,
            radius.bottom_left,
        )
    }

    #[inline]
    pub fn equals(&self, other: RoundedRect) -> bool {
        *self == other
    }

    /// Gets a value indicating whether any corner has a non-zero radius.
    #[inline]
    pub fn is_rounded(&self) -> bool {
        self.radii_top_left != Vector::ZERO
            || self.radii_top_right != Vector::ZERO
            || self.radii_bottom_right != Vector::ZERO
            || self.radii_bottom_left != Vector::ZERO
    }

    /// Gets a value indicating whether all corners have the same radii.
    #[inline]
    pub fn is_uniform(&self) -> bool {
        self.radii_top_left.equals(self.radii_top_right)
            && self.radii_top_left.equals(self.radii_bottom_right)
            && self.radii_top_left.equals(self.radii_bottom_left)
    }

    #[inline]
    pub fn inflate(&self, dx: f64, dy: f64) -> RoundedRect {
        self.deflate(-dx, -dy)
    }

    pub fn deflate(&self, dx: f64, dy: f64) -> RoundedRect {
        if !self.is_rounded() {
            return RoundedRect::from_rect(
                self.rect.deflate_thickness(Thickness::symmetric(dx, dy)),
            );
        }

        // Same algorithm as Skia's SkRRect inset.
        let mut left = self.rect.x + dx;
        let mut top = self.rect.y + dy;
        let mut right = left + self.rect.width - dx * 2.0;
        let mut bottom = top + self.rect.height - dy * 2.0;
        let mut radii = [
            self.radii_top_left,
            self.radii_top_right,
            self.radii_bottom_right,
            self.radii_bottom_left,
        ];

        let mut degenerate = false;
        if right <= left {
            degenerate = true;
            left = (left + right) * 0.5;
            right = left;
        }
        if bottom <= top {
            degenerate = true;
            top = (top + bottom) * 0.5;
            bottom = top;
        }
        if degenerate {
            return RoundedRect::from_rect(Rect::new(left, top, right - left, bottom - top));
        }

        for radius in &mut radii {
            let rx = math_utilities::max(0.0, radius.x - dx);
            let ry = math_utilities::max(0.0, radius.y - dy);
            *radius = if rx == 0.0 || ry == 0.0 {
                Vector::default()
            } else {
                Vector::new(rx, ry)
            };
        }

        RoundedRect::new(
            Rect::new(left, top, right - left, bottom - top),
            radii[0],
            radii[1],
            radii[2],
            radii[3],
        )
    }

    /// This method should be used internally to check for the rect emptiness.
    /// Once a dedicated "empty" value exists this is the single place to update.
    #[inline]
    pub fn is_empty(&self) -> bool {
        *self == RoundedRect::default()
    }

    #[inline]
    fn is_outside_corner(dx: f64, dy: f64, radius: f64) -> bool {
        (dx < 0.0) && (dy < 0.0) && (dx * dx + dy * dy > radius * radius)
    }

    /// Determines whether a point is inside the rounded rectangle, exclusive of the
    /// bottom/right edge. Elliptical corners are approximated by circles using the
    /// smaller of the two radii.
    pub fn contains_exclusive(&self, p: Point) -> bool {
        // Do a simple rectangular bounds check first
        if !self.rect.contains_exclusive(p) {
            return false;
        }

        let rect = self.rect;

        // If any radii totals exceed available bounds, determine a scale factor that needs to be applied
        let mut scale_factor = 1.0;
        if rect.width > 0.0 {
            let radii_width = math_utilities::max(
                self.radii_top_left.x + self.radii_top_right.x,
                self.radii_bottom_left.x + self.radii_bottom_right.x,
            );
            if radii_width > rect.width {
                scale_factor = math_utilities::min(scale_factor, rect.width / radii_width);
            }
        }
        if rect.height > 0.0 {
            let radii_height = math_utilities::max(
                self.radii_top_left.y + self.radii_bottom_left.y,
                self.radii_top_right.y + self.radii_bottom_right.y,
            );
            if radii_height > rect.height {
                scale_factor = math_utilities::min(scale_factor, rect.height / radii_height);
            }
        }

        // Before corner hit-testing, make the point relative to the bounds' upper-left
        let p = Point::new(p.x - rect.x, p.y - rect.y);

        // Top-left corner
        let radius =
            math_utilities::min(self.radii_top_left.x, self.radii_top_left.y) * scale_factor;
        if Self::is_outside_corner(p.x - radius, p.y - radius, radius) {
            return false;
        }

        // Top-right corner
        let radius =
            math_utilities::min(self.radii_top_right.x, self.radii_top_right.y) * scale_factor;
        if Self::is_outside_corner(rect.width - radius - p.x, p.y - radius, radius) {
            return false;
        }

        // Bottom-right corner
        let radius = math_utilities::min(self.radii_bottom_right.x, self.radii_bottom_right.y)
            * scale_factor;
        if Self::is_outside_corner(
            rect.width - radius - p.x,
            rect.height - radius - p.y,
            radius,
        ) {
            return false;
        }

        // Bottom-left corner
        let radius =
            math_utilities::min(self.radii_bottom_left.x, self.radii_bottom_left.y) * scale_factor;
        if Self::is_outside_corner(p.x - radius, rect.height - radius - p.y, radius) {
            return false;
        }

        true
    }
}

impl From<Rect> for RoundedRect {
    #[inline]
    fn from(r: Rect) -> RoundedRect {
        RoundedRect::from_rect(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_exclusive_should_return_expected_result_for_point() {
        let cases: [(f64, f64, bool); 13] = [
            // Corners
            (0.0, 0.0, false),
            (100.0, 0.0, false),
            (100.0, 100.0, false),
            (0.0, 100.0, false),
            // Indent 10px
            (10.0, 10.0, false),
            (90.0, 10.0, true),
            (90.0, 90.0, false),
            (10.0, 90.0, true),
            // Indent 17px
            (17.0, 17.0, false),
            (83.0, 17.0, true),
            (83.0, 83.0, true),
            (17.0, 83.0, true),
            // Center
            (50.0, 50.0, true),
        ];

        for (x, y, expected_result) in cases {
            let rrect = RoundedRect::from_corner_radius(
                Rect::new(0.0, 0.0, 100.0, 100.0),
                CornerRadius::new(60.0, 10.0, 50.0, 30.0),
            );

            assert_eq!(
                expected_result,
                rrect.contains_exclusive(Point::new(x, y)),
                "point ({x}, {y})"
            );
        }
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn rounded_uniform_and_deflate() {
        let plain: RoundedRect = Rect::new(0.0, 0.0, 10.0, 10.0).into();
        assert!(!plain.is_rounded());
        assert!(plain.is_uniform());
        assert!(RoundedRect::default().is_empty());
        assert_eq!(
            plain.deflate(1.0, 2.0),
            RoundedRect::from_rect(Rect::new(1.0, 2.0, 8.0, 6.0))
        );

        let rr = RoundedRect::from_radius(Rect::new(0.0, 0.0, 100.0, 100.0), 10.0);
        assert!(rr.is_rounded());
        assert_eq!(
            rr.deflate(4.0, 4.0),
            RoundedRect::from_radius(Rect::new(4.0, 4.0, 92.0, 92.0), 6.0)
        );
        // Radii that shrink to zero collapse the corner.
        assert_eq!(
            rr.deflate(10.0, 10.0),
            RoundedRect::from_rect(Rect::new(10.0, 10.0, 80.0, 80.0))
        );
        // Degenerate result collapses to the centre line.
        assert_eq!(
            rr.deflate(60.0, 10.0),
            RoundedRect::from_rect(Rect::new(50.0, 10.0, 0.0, 80.0))
        );
        assert_eq!(
            rr.inflate(5.0, 5.0),
            RoundedRect::from_radius(Rect::new(-5.0, -5.0, 110.0, 110.0), 15.0)
        );
    }
}
