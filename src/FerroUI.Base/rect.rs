//! Defines a rectangle.

use std::fmt;
use std::ops::{Div, DivAssign, Mul, MulAssign};
use std::str::FromStr;

use crate::utilities::math_utilities;
use crate::utilities::span_helpers::InvariantF64;
use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::{Matrix, Point, Size, Thickness, Vector};

/// Defines a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    /// The X position.
    pub x: f64,
    /// The Y position.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

impl Rect {
    /// Initializes a new instance of the [`Rect`] structure.
    #[inline]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Initializes a [`Rect`] at the origin with the given size.
    #[inline]
    pub const fn from_size(size: Size) -> Self {
        Self::new(0.0, 0.0, size.width, size.height)
    }

    /// Initializes a [`Rect`] from its position and size.
    #[inline]
    pub const fn from_position_size(position: Point, size: Size) -> Self {
        Self::new(position.x, position.y, size.width, size.height)
    }

    /// Initializes a [`Rect`] from its top left and bottom right corners.
    #[inline]
    pub fn from_points(top_left: Point, bottom_right: Point) -> Self {
        Self::new(
            top_left.x,
            top_left.y,
            bottom_right.x - top_left.x,
            bottom_right.y - top_left.y,
        )
    }

    /// Gets the position of the rectangle.
    #[inline]
    pub const fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Gets the size of the rectangle.
    #[inline]
    pub const fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Gets the right position of the rectangle.
    #[inline]
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Gets the bottom position of the rectangle.
    #[inline]
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Gets the left position.
    #[inline]
    pub const fn left(&self) -> f64 {
        self.x
    }

    /// Gets the top position.
    #[inline]
    pub const fn top(&self) -> f64 {
        self.y
    }

    /// Gets the top left point of the rectangle.
    #[inline]
    pub const fn top_left(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Gets the top right point of the rectangle.
    #[inline]
    pub fn top_right(&self) -> Point {
        Point::new(self.right(), self.y)
    }

    /// Gets the bottom left point of the rectangle.
    #[inline]
    pub fn bottom_left(&self) -> Point {
        Point::new(self.x, self.bottom())
    }

    /// Gets the bottom right point of the rectangle.
    #[inline]
    pub fn bottom_right(&self) -> Point {
        Point::new(self.right(), self.bottom())
    }

    /// Gets the center point of the rectangle.
    #[inline]
    pub fn center(&self) -> Point {
        Point::new(self.x + (self.width / 2.0), self.y + (self.height / 2.0))
    }

    /// Determines whether a point is in the bounds of the rectangle (edges inclusive).
    #[inline]
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.x + self.width && p.y >= self.y && p.y <= self.y + self.height
    }

    /// Determines whether a point is in the bounds of the rectangle, exclusive of
    /// the rectangle's bottom/right edge.
    #[inline]
    pub fn contains_exclusive(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.x + self.width && p.y >= self.y && p.y < self.y + self.height
    }

    /// Determines whether the rectangle fully contains another rectangle.
    #[inline]
    pub fn contains_rect(&self, r: Rect) -> bool {
        self.contains(r.top_left()) && self.contains(r.bottom_right())
    }

    /// Centers another rectangle in this rectangle.
    #[inline]
    pub fn center_rect(&self, rect: Rect) -> Rect {
        Rect::new(
            self.x + ((self.width - rect.width) / 2.0),
            self.y + ((self.height - rect.height) / 2.0),
            rect.width,
            rect.height,
        )
    }

    /// Inflates the rectangle by the same amount on every side.
    #[inline]
    pub fn inflate(&self, thickness: f64) -> Rect {
        self.inflate_thickness(Thickness::uniform(thickness))
    }

    /// Inflates the rectangle.
    #[inline]
    pub fn inflate_thickness(&self, thickness: Thickness) -> Rect {
        Rect::from_position_size(
            Point::new(self.x - thickness.left, self.y - thickness.top),
            self.size().inflate(thickness),
        )
    }

    /// Deflates the rectangle by the same amount on every side.
    #[inline]
    pub fn deflate(&self, thickness: f64) -> Rect {
        self.deflate_thickness(Thickness::uniform(thickness))
    }

    /// Deflates the rectangle by a [`Thickness`]. The deflated rectangle size
    /// cannot be less than 0.
    #[inline]
    pub fn deflate_thickness(&self, thickness: Thickness) -> Rect {
        Rect::from_position_size(
            Point::new(self.x + thickness.left, self.y + thickness.top),
            self.size().deflate(thickness),
        )
    }

    /// Returns a boolean indicating whether the rect is equal to the other given
    /// rect (exact comparison).
    #[inline]
    pub fn equals(&self, other: Rect) -> bool {
        self.x == other.x
            && self.y == other.y
            && self.width == other.width
            && self.height == other.height
    }

    /// Gets the intersection of two rectangles, or an empty (default) rectangle
    /// if they do not overlap.
    pub fn intersect(&self, rect: Rect) -> Rect {
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
            Rect::new(
                new_left,
                new_top,
                new_right - new_left,
                new_bottom - new_top,
            )
        } else {
            Rect::default()
        }
    }

    /// Determines whether a rectangle intersects with this rectangle.
    #[inline]
    pub fn intersects(&self, rect: Rect) -> bool {
        (rect.x < self.right())
            && (self.x < rect.right())
            && (rect.y < self.bottom())
            && (self.y < rect.bottom())
    }

    /// Returns the axis-aligned bounding box of a transformed rectangle.
    pub fn transform_to_aabb(&self, matrix: Matrix) -> Rect {
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

        Rect::from_points(Point::new(left, top), Point::new(right, bottom))
    }

    /// Translates the rectangle by an offset.
    #[inline]
    pub fn translate(&self, offset: Vector) -> Rect {
        Rect::from_position_size(self.position() + offset, self.size())
    }

    /// Normalizes the rectangle so both the width and height are positive, without
    /// changing its location: positive-size rects are returned unchanged, negative
    /// sizes are flipped. A rect with any NaN edge becomes the default rect.
    pub fn normalize(&self) -> Rect {
        let mut rect = *self;

        if rect.right().is_nan()
            || rect.bottom().is_nan()
            || rect.x.is_nan()
            || rect.y.is_nan()
            || self.height.is_nan()
            || self.width.is_nan()
        {
            return Rect::default();
        }

        if rect.width < 0.0 {
            let x = self.x + self.width;
            let width = self.x - x;

            rect = rect.with_x(x).with_width(width);
        }

        if rect.height < 0.0 {
            let y = self.y + self.height;
            let height = self.y - y;

            rect = rect.with_y(y).with_height(height);
        }

        rect
    }

    /// Gets the union of two rectangles. A rectangle whose width and height are
    /// both zero is ignored.
    pub fn union(&self, rect: Rect) -> Rect {
        if self.width == 0.0 && self.height == 0.0 {
            rect
        } else if rect.width == 0.0 && rect.height == 0.0 {
            *self
        } else {
            let x1 = math_utilities::min(self.x, rect.x);
            let x2 = math_utilities::max(self.right(), rect.right());
            let y1 = math_utilities::min(self.y, rect.y);
            let y2 = math_utilities::max(self.bottom(), rect.bottom());

            Rect::from_points(Point::new(x1, y1), Point::new(x2, y2))
        }
    }

    /// Union of two optional rectangles; a missing side yields the other side.
    #[inline]
    pub fn union_optional(left: Option<Rect>, right: Option<Rect>) -> Option<Rect> {
        match (left, right) {
            (None, right) => right,
            (left, None) => left,
            (Some(left), Some(right)) => Some(left.union(right)),
        }
    }

    /// Returns a new [`Rect`] with the specified X position.
    #[inline]
    pub const fn with_x(&self, x: f64) -> Rect {
        Rect::new(x, self.y, self.width, self.height)
    }

    /// Returns a new [`Rect`] with the specified Y position.
    #[inline]
    pub const fn with_y(&self, y: f64) -> Rect {
        Rect::new(self.x, y, self.width, self.height)
    }

    /// Returns a new [`Rect`] with the specified width.
    #[inline]
    pub const fn with_width(&self, width: f64) -> Rect {
        Rect::new(self.x, self.y, width, self.height)
    }

    /// Returns a new [`Rect`] with the specified height.
    #[inline]
    pub const fn with_height(&self, height: f64) -> Rect {
        Rect::new(self.x, self.y, self.width, height)
    }

    /// Parses a [`Rect`] string (`"x, y, width, height"`).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        SpanStringTokenizer::with_message(s, "Invalid Rect.").scope(|t| {
            Ok(Rect::new(
                t.read_double()?,
                t.read_double()?,
                t.read_double()?,
                t.read_double()?,
            ))
        })
    }

    /// This method should be used internally to check for the rect emptiness.
    /// Once a dedicated "empty" value exists this is the single place to update.
    #[inline]
    pub fn is_empty(&self) -> bool {
        *self == Rect::default()
    }
}

/// Multiplies a rectangle by a scaling vector.
impl Mul<Vector> for Rect {
    type Output = Rect;
    #[inline]
    fn mul(self, scale: Vector) -> Rect {
        Rect::new(
            self.x * scale.x,
            self.y * scale.y,
            self.width * scale.x,
            self.height * scale.y,
        )
    }
}

/// Multiplies a rectangle by a scale.
impl Mul<f64> for Rect {
    type Output = Rect;
    #[inline]
    fn mul(self, scale: f64) -> Rect {
        Rect::new(
            self.x * scale,
            self.y * scale,
            self.width * scale,
            self.height * scale,
        )
    }
}

/// Divides a rectangle by a vector.
impl Div<Vector> for Rect {
    type Output = Rect;
    #[inline]
    fn div(self, scale: Vector) -> Rect {
        Rect::new(
            self.x / scale.x,
            self.y / scale.y,
            self.width / scale.x,
            self.height / scale.y,
        )
    }
}

impl MulAssign<Vector> for Rect {
    #[inline]
    fn mul_assign(&mut self, scale: Vector) {
        *self = *self * scale;
    }
}

impl MulAssign<f64> for Rect {
    #[inline]
    fn mul_assign(&mut self, scale: f64) {
        *self = *self * scale;
    }
}

impl DivAssign<Vector> for Rect {
    #[inline]
    fn div_assign(&mut self, scale: Vector) {
        *self = *self / scale;
    }
}

impl FromStr for Rect {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        Rect::parse(s)
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}, {}, {}, {}",
            InvariantF64(self.x),
            InvariantF64(self.y),
            InvariantF64(self.width),
            InvariantF64(self.height)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_should_return_correct_value_for_intersecting_rects() {
        let result = Rect::new(0.0, 0.0, 100.0, 100.0).union(Rect::new(50.0, 50.0, 100.0, 100.0));

        assert_eq!(Rect::new(0.0, 0.0, 150.0, 150.0), result);
    }

    #[test]
    fn union_should_return_correct_value_for_non_intersecting_rects() {
        let result = Rect::new(0.0, 0.0, 100.0, 100.0).union(Rect::new(150.0, 150.0, 100.0, 100.0));

        assert_eq!(Rect::new(0.0, 0.0, 250.0, 250.0), result);
    }

    #[test]
    fn union_should_ignore_empty_this_rect() {
        let result = Rect::new(0.0, 0.0, 0.0, 0.0).union(Rect::new(150.0, 150.0, 100.0, 100.0));

        assert_eq!(Rect::new(150.0, 150.0, 100.0, 100.0), result);
    }

    #[test]
    fn union_should_ignore_empty_other_rect() {
        let result = Rect::new(0.0, 0.0, 100.0, 100.0).union(Rect::new(150.0, 150.0, 0.0, 0.0));

        assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), result);
    }

    #[test]
    fn normalize_should_reverse_negative_size() {
        let result = Rect::from_points(Point::new(100.0, 100.0), Point::new(0.0, 0.0)).normalize();

        assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), result);
    }

    #[test]
    fn normalize_should_make_invalid_rects_empty() {
        let result = Rect::new(
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
        )
        .normalize();

        assert_eq!(Rect::default(), result);
    }

    #[test]
    fn parse_parses() {
        let rect = Rect::parse("1,2 3,-4").unwrap();
        let expected = Rect::new(1.0, 2.0, 3.0, -4.0);
        assert_eq!(expected, rect);
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn edges_and_corners() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(r.right(), 40.0);
        assert_eq!(r.bottom(), 60.0);
        assert_eq!(r.left(), 10.0);
        assert_eq!(r.top(), 20.0);
        assert_eq!(r.top_right(), Point::new(40.0, 20.0));
        assert_eq!(r.bottom_left(), Point::new(10.0, 60.0));
        assert_eq!(r.bottom_right(), Point::new(40.0, 60.0));
        assert_eq!(r.center(), Point::new(25.0, 40.0));
        assert_eq!(r.position(), Point::new(10.0, 20.0));
        assert_eq!(r.size(), Size::new(30.0, 40.0));
        assert_eq!(
            Rect::from_size(Size::new(1.0, 2.0)),
            Rect::new(0.0, 0.0, 1.0, 2.0)
        );
    }

    #[test]
    fn contains_intersects_and_intersect() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains(Point::new(10.0, 10.0)));
        assert!(!r.contains_exclusive(Point::new(10.0, 10.0)));
        assert!(r.contains_exclusive(Point::new(0.0, 0.0)));
        assert!(r.contains_rect(Rect::new(1.0, 1.0, 9.0, 9.0)));
        assert!(!r.contains_rect(Rect::new(1.0, 1.0, 10.0, 9.0)));
        assert!(r.intersects(Rect::new(5.0, 5.0, 10.0, 10.0)));
        assert!(!r.intersects(Rect::new(10.0, 0.0, 10.0, 10.0)));
        assert_eq!(
            r.intersect(Rect::new(5.0, 5.0, 10.0, 10.0)),
            Rect::new(5.0, 5.0, 5.0, 5.0)
        );
        assert_eq!(
            r.intersect(Rect::new(10.0, 0.0, 10.0, 10.0)),
            Rect::default()
        );
        assert!(Rect::default().is_empty());
    }

    #[test]
    fn inflate_deflate_translate_center_rect() {
        let r = Rect::new(10.0, 10.0, 20.0, 20.0);
        assert_eq!(r.inflate(5.0), Rect::new(5.0, 5.0, 30.0, 30.0));
        assert_eq!(r.deflate(5.0), Rect::new(15.0, 15.0, 10.0, 10.0));
        assert_eq!(r.deflate(15.0), Rect::new(25.0, 25.0, 0.0, 0.0));
        assert_eq!(
            r.inflate_thickness(Thickness::new(1.0, 2.0, 3.0, 4.0)),
            Rect::new(9.0, 8.0, 24.0, 26.0)
        );
        assert_eq!(
            r.translate(Vector::new(1.0, -1.0)),
            Rect::new(11.0, 9.0, 20.0, 20.0)
        );
        assert_eq!(
            r.center_rect(Rect::new(0.0, 0.0, 10.0, 10.0)),
            Rect::new(15.0, 15.0, 10.0, 10.0)
        );
    }

    #[test]
    fn transform_to_aabb() {
        let r = Rect::new(0.0, 0.0, 10.0, 20.0);
        assert_eq!(
            r.transform_to_aabb(Matrix::create_translation(5.0, 5.0)),
            Rect::new(5.0, 5.0, 10.0, 20.0)
        );
        let rotated = r.transform_to_aabb(Matrix::create_rotation(std::f64::consts::FRAC_PI_2));
        assert!((rotated.x - -20.0).abs() < 1e-9);
        assert!(rotated.y.abs() < 1e-9);
        assert!((rotated.width - 20.0).abs() < 1e-9);
        assert!((rotated.height - 10.0).abs() < 1e-9);
    }

    #[test]
    fn operators_display_and_union_optional() {
        let r = Rect::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(r * 2.0, Rect::new(2.0, 4.0, 6.0, 8.0));
        assert_eq!(r * Vector::new(2.0, 3.0), Rect::new(2.0, 6.0, 6.0, 12.0));
        assert_eq!(r / Vector::new(2.0, 2.0), Rect::new(0.5, 1.0, 1.5, 2.0));
        assert_eq!(r.to_string(), "1, 2, 3, 4");
        assert_eq!(Rect::parse("1,2,3").unwrap_err().message(), "Invalid Rect.");
        assert_eq!(Rect::union_optional(None, Some(r)), Some(r));
        assert_eq!(Rect::union_optional(Some(r), None), Some(r));
        assert_eq!(Rect::union_optional(None, None), None);
    }
}
