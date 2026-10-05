//! Represents a rectangle in device pixels.

use std::fmt;
use std::str::FromStr;

use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::{PixelPoint, PixelSize, PixelVector, Point, Rect, Vector};

/// Represents a rectangle in device pixels. Integer arithmetic wraps on overflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PixelRect {
    /// The X position.
    pub x: i32,
    /// The Y position.
    pub y: i32,
    /// The width.
    pub width: i32,
    /// The height.
    pub height: i32,
}

impl PixelRect {
    /// Initializes a new instance of the [`PixelRect`] structure.
    #[inline]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Initializes a [`PixelRect`] at the origin with the given size.
    #[inline]
    pub const fn from_size(size: PixelSize) -> Self {
        Self::new(0, 0, size.width, size.height)
    }

    /// Initializes a [`PixelRect`] from its position and size.
    #[inline]
    pub const fn from_position_size(position: PixelPoint, size: PixelSize) -> Self {
        Self::new(position.x, position.y, size.width, size.height)
    }

    /// Initializes a [`PixelRect`] from its top left and bottom right corners.
    #[inline]
    pub const fn from_points(top_left: PixelPoint, bottom_right: PixelPoint) -> Self {
        Self::new(
            top_left.x,
            top_left.y,
            bottom_right.x.wrapping_sub(top_left.x),
            bottom_right.y.wrapping_sub(top_left.y),
        )
    }

    /// Gets the position of the rectangle.
    #[inline]
    pub const fn position(&self) -> PixelPoint {
        PixelPoint::new(self.x, self.y)
    }

    /// Gets the size of the rectangle.
    #[inline]
    pub const fn size(&self) -> PixelSize {
        PixelSize::new(self.width, self.height)
    }

    /// Gets the right position of the rectangle.
    #[inline]
    pub const fn right(&self) -> i32 {
        self.x.wrapping_add(self.width)
    }

    /// Gets the bottom position of the rectangle.
    #[inline]
    pub const fn bottom(&self) -> i32 {
        self.y.wrapping_add(self.height)
    }

    /// Gets the top left point of the rectangle.
    #[inline]
    pub const fn top_left(&self) -> PixelPoint {
        PixelPoint::new(self.x, self.y)
    }

    /// Gets the top right point of the rectangle.
    #[inline]
    pub const fn top_right(&self) -> PixelPoint {
        PixelPoint::new(self.right(), self.y)
    }

    /// Gets the bottom left point of the rectangle.
    #[inline]
    pub const fn bottom_left(&self) -> PixelPoint {
        PixelPoint::new(self.x, self.bottom())
    }

    /// Gets the bottom right point of the rectangle.
    #[inline]
    pub const fn bottom_right(&self) -> PixelPoint {
        PixelPoint::new(self.right(), self.bottom())
    }

    /// Gets the center point of the rectangle.
    #[inline]
    pub const fn center(&self) -> PixelPoint {
        PixelPoint::new(
            self.x.wrapping_add(self.width / 2),
            self.y.wrapping_add(self.height / 2),
        )
    }

    /// Determines whether a point is in the bounds of the rectangle (edges inclusive).
    #[inline]
    pub const fn contains(&self, p: PixelPoint) -> bool {
        p.x >= self.x && p.x <= self.right() && p.y >= self.y && p.y <= self.bottom()
    }

    /// Determines whether a point is in the bounds of the rectangle, exclusive of
    /// the rectangle's bottom/right edge.
    #[inline]
    pub const fn contains_exclusive(&self, p: PixelPoint) -> bool {
        p.x >= self.x
            && p.x < self.x.wrapping_add(self.width)
            && p.y >= self.y
            && p.y < self.y.wrapping_add(self.height)
    }

    /// Determines whether the rectangle fully contains another rectangle.
    #[inline]
    pub const fn contains_rect(&self, r: PixelRect) -> bool {
        self.contains(r.top_left()) && self.contains(r.bottom_right())
    }

    /// Centers another rectangle in this rectangle.
    #[inline]
    pub const fn center_rect(&self, rect: PixelRect) -> PixelRect {
        PixelRect::new(
            self.x.wrapping_add(self.width.wrapping_sub(rect.width) / 2),
            self.y
                .wrapping_add(self.height.wrapping_sub(rect.height) / 2),
            rect.width,
            rect.height,
        )
    }

    /// Returns a boolean indicating whether the rect is equal to the other given rect.
    #[inline]
    pub const fn equals(&self, other: PixelRect) -> bool {
        self.x == other.x
            && self.y == other.y
            && self.width == other.width
            && self.height == other.height
    }

    /// Gets the intersection of two rectangles, or an empty (default) rectangle if
    /// they do not overlap.
    pub const fn intersect(&self, rect: PixelRect) -> PixelRect {
        let new_left = if rect.x > self.x { rect.x } else { self.x };
        let new_top = if rect.y > self.y { rect.y } else { self.y };
        let new_right = if rect.right() < self.right() {
            rect.right()
        } else {
            self.right()
        };
        let new_bottom = if rect.bottom() < self.bottom() {
            rect.bottom()
        } else {
            self.bottom()
        };

        if (new_right > new_left) && (new_bottom > new_top) {
            PixelRect::new(
                new_left,
                new_top,
                new_right.wrapping_sub(new_left),
                new_bottom.wrapping_sub(new_top),
            )
        } else {
            PixelRect::new(0, 0, 0, 0)
        }
    }

    /// Determines whether a rectangle intersects with this rectangle.
    #[inline]
    pub const fn intersects(&self, rect: PixelRect) -> bool {
        (rect.x < self.right())
            && (self.x < rect.right())
            && (rect.y < self.bottom())
            && (self.y < rect.bottom())
    }

    /// Translates the rectangle by an offset.
    #[inline]
    pub fn translate(&self, offset: PixelVector) -> PixelRect {
        PixelRect::from_position_size(self.position() + offset, self.size())
    }

    /// Gets the union of two rectangles. A rectangle whose width and height are
    /// both zero is ignored.
    pub fn union(&self, rect: PixelRect) -> PixelRect {
        if self.width == 0 && self.height == 0 {
            rect
        } else if rect.width == 0 && rect.height == 0 {
            *self
        } else {
            let x1 = self.x.min(rect.x);
            let x2 = self.right().max(rect.right());
            let y1 = self.y.min(rect.y);
            let y2 = self.bottom().max(rect.bottom());

            PixelRect::from_points(PixelPoint::new(x1, y1), PixelPoint::new(x2, y2))
        }
    }

    /// Returns a new [`PixelRect`] with the specified X position.
    #[inline]
    pub const fn with_x(&self, x: i32) -> PixelRect {
        PixelRect::new(x, self.y, self.width, self.height)
    }

    /// Returns a new [`PixelRect`] with the specified Y position.
    #[inline]
    pub const fn with_y(&self, y: i32) -> PixelRect {
        PixelRect::new(self.x, y, self.width, self.height)
    }

    /// Returns a new [`PixelRect`] with the specified width.
    #[inline]
    pub const fn with_width(&self, width: i32) -> PixelRect {
        PixelRect::new(self.x, self.y, width, self.height)
    }

    /// Returns a new [`PixelRect`] with the specified height.
    #[inline]
    pub const fn with_height(&self, height: i32) -> PixelRect {
        PixelRect::new(self.x, self.y, self.width, height)
    }

    /// Converts the [`PixelRect`] to a device-independent [`Rect`] using the
    /// specified scaling factor.
    #[inline]
    pub fn to_rect(&self, scale: f64) -> Rect {
        Rect::from_position_size(self.position().to_point(scale), self.size().to_size(scale))
    }

    /// Converts the [`PixelRect`] to a device-independent [`Rect`] using the
    /// specified per-axis scaling factor.
    #[inline]
    pub fn to_rect_vector(&self, scale: Vector) -> Rect {
        Rect::from_position_size(
            self.position().to_point_vector(scale),
            self.size().to_size_vector(scale),
        )
    }

    /// Converts the [`PixelRect`] to a device-independent [`Rect`] using the
    /// specified dots per inch (DPI).
    #[inline]
    pub fn to_rect_with_dpi(&self, dpi: f64) -> Rect {
        Rect::from_position_size(
            self.position().to_point_with_dpi(dpi),
            self.size().to_size_with_dpi(dpi),
        )
    }

    /// Converts the [`PixelRect`] to a device-independent [`Rect`] using the
    /// specified per-axis dots per inch (DPI).
    #[inline]
    pub fn to_rect_with_dpi_vector(&self, dpi: Vector) -> Rect {
        Rect::from_position_size(
            self.position().to_point_with_dpi_vector(dpi),
            self.size().to_size_with_dpi_vector(dpi),
        )
    }

    /// Converts a [`Rect`] to device pixels using the specified scaling factor.
    /// The top left is truncated and the bottom right is rounded up.
    #[inline]
    pub fn from_rect(rect: Rect, scale: f64) -> PixelRect {
        PixelRect::from_points(
            PixelPoint::from_point(rect.position(), scale),
            Self::from_point_ceiling(rect.bottom_right(), Vector::new(scale, scale)),
        )
    }

    /// Converts a [`Rect`] to device pixels using the specified per-axis scaling factor.
    #[inline]
    pub fn from_rect_vector(rect: Rect, scale: Vector) -> PixelRect {
        PixelRect::from_points(
            PixelPoint::from_point_vector(rect.position(), scale),
            Self::from_point_ceiling(rect.bottom_right(), scale),
        )
    }

    /// Converts a [`Rect`] to device pixels using the specified dots per inch (DPI).
    #[inline]
    pub fn from_rect_with_dpi(rect: Rect, dpi: f64) -> PixelRect {
        PixelRect::from_points(
            PixelPoint::from_point_with_dpi(rect.position(), dpi),
            Self::from_point_ceiling(rect.bottom_right(), Vector::new(dpi / 96.0, dpi / 96.0)),
        )
    }

    /// Converts a [`Rect`] to device pixels using the specified per-axis dots per inch (DPI).
    #[inline]
    pub fn from_rect_with_dpi_vector(rect: Rect, dpi: Vector) -> PixelRect {
        PixelRect::from_points(
            PixelPoint::from_point_with_dpi_vector(rect.position(), dpi),
            Self::from_point_ceiling(rect.bottom_right(), dpi / 96.0),
        )
    }

    /// Parses a [`PixelRect`] string (`"x, y, width, height"`).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        SpanStringTokenizer::with_message(s, "Invalid PixelRect.").scope(|t| {
            Ok(PixelRect::new(
                t.read_int32()?,
                t.read_int32()?,
                t.read_int32()?,
                t.read_int32()?,
            ))
        })
    }

    #[inline]
    fn from_point_ceiling(point: Point, scale: Vector) -> PixelPoint {
        PixelPoint::new(
            (point.x * scale.x).ceil() as i32,
            (point.y * scale.y).ceil() as i32,
        )
    }
}

impl FromStr for PixelRect {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        PixelRect::parse(s)
    }
}

impl fmt::Display for PixelRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}, {}, {}", self.x, self.y, self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_rect_snaps_to_device_pixels() {
        let rect = Rect::new(189.0, 189.0, 26.0, 164.0);
        let result = PixelRect::from_rect(rect, 1.5);

        assert_eq!(PixelRect::new(283, 283, 40, 247), result);
    }

    #[test]
    fn from_rect_vector_snaps_to_device_pixels() {
        let rect = Rect::new(189.0, 189.0, 26.0, 164.0);
        let result = PixelRect::from_rect_vector(rect, Vector::new(1.5, 1.5));

        assert_eq!(PixelRect::new(283, 283, 40, 247), result);
    }

    #[test]
    fn from_rect_with_dpi_snaps_to_device_pixels() {
        let rect = Rect::new(189.0, 189.0, 26.0, 164.0);
        let result = PixelRect::from_rect_with_dpi(rect, 144.0);

        assert_eq!(PixelRect::new(283, 283, 40, 247), result);
    }

    #[test]
    fn from_rect_with_dpi_vector_snaps_to_device_pixels() {
        let rect = Rect::new(189.0, 189.0, 26.0, 164.0);
        let result = PixelRect::from_rect_with_dpi_vector(rect, Vector::new(144.0, 144.0));

        assert_eq!(PixelRect::new(283, 283, 40, 247), result);
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn geometry() {
        let r = PixelRect::new(10, 20, 30, 41);
        assert_eq!(r.right(), 40);
        assert_eq!(r.bottom(), 61);
        assert_eq!(r.center(), PixelPoint::new(25, 40));
        assert_eq!(r.top_right(), PixelPoint::new(40, 20));
        assert_eq!(r.bottom_left(), PixelPoint::new(10, 61));
        assert!(r.contains(PixelPoint::new(40, 61)));
        assert!(!r.contains_exclusive(PixelPoint::new(40, 61)));
        assert!(r.contains_rect(PixelRect::new(11, 21, 5, 5)));
        assert!(r.intersects(PixelRect::new(39, 60, 5, 5)));
        assert!(!r.intersects(PixelRect::new(40, 61, 5, 5)));
        assert_eq!(
            r.intersect(PixelRect::new(20, 30, 100, 100)),
            PixelRect::new(20, 30, 20, 31)
        );
        assert_eq!(
            r.intersect(PixelRect::new(100, 100, 1, 1)),
            PixelRect::default()
        );
        assert_eq!(
            r.union(PixelRect::new(0, 0, 5, 5)),
            PixelRect::new(0, 0, 40, 61)
        );
        assert_eq!(r.union(PixelRect::default()), r);
        assert_eq!(
            r.translate(PixelVector::new(1, 1)),
            PixelRect::new(11, 21, 30, 41)
        );
        assert_eq!(
            PixelRect::new(0, 0, 10, 10).center_rect(PixelRect::new(0, 0, 4, 4)),
            PixelRect::new(3, 3, 4, 4)
        );
    }

    #[test]
    fn parse_display_and_to_rect() {
        assert_eq!(
            PixelRect::parse("1,2,3,4").unwrap(),
            PixelRect::new(1, 2, 3, 4)
        );
        assert_eq!(
            PixelRect::parse("1,2,3").unwrap_err().message(),
            "Invalid PixelRect."
        );
        assert_eq!(PixelRect::new(1, 2, 3, 4).to_string(), "1, 2, 3, 4");
        assert_eq!(
            PixelRect::new(3, 6, 9, 12).to_rect(1.5),
            Rect::new(2.0, 4.0, 6.0, 8.0)
        );
        assert_eq!(
            PixelRect::new(3, 6, 9, 12).to_rect_with_dpi(144.0),
            Rect::new(2.0, 4.0, 6.0, 8.0)
        );
    }
}
