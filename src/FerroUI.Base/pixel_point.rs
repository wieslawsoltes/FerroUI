//! Represents a point in device pixels.

use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::str::FromStr;

use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::{PixelVector, Point, Vector};

/// Represents a point in device pixels.
///
/// Integer arithmetic wraps on overflow. Conversions from floating point truncate
/// toward zero and saturate at the `i32` range (NaN becomes 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PixelPoint {
    /// The X co-ordinate.
    pub x: i32,
    /// The Y co-ordinate.
    pub y: i32,
}

impl PixelPoint {
    /// A point representing 0,0.
    pub const ORIGIN: PixelPoint = PixelPoint::new(0, 0);

    /// Initializes a new instance of the [`PixelPoint`] structure.
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Parses a [`PixelPoint`] string (`"x, y"`).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        SpanStringTokenizer::with_message(s, "Invalid PixelPoint.")
            .scope(|t| Ok(PixelPoint::new(t.read_int32()?, t.read_int32()?)))
    }

    /// Returns a boolean indicating whether the point is equal to the other given point.
    #[inline]
    pub const fn equals(&self, other: PixelPoint) -> bool {
        self.x == other.x && self.y == other.y
    }

    /// Returns a new [`PixelPoint`] with the same Y co-ordinate and the specified X co-ordinate.
    #[inline]
    pub const fn with_x(&self, x: i32) -> PixelPoint {
        PixelPoint::new(x, self.y)
    }

    /// Returns a new [`PixelPoint`] with the same X co-ordinate and the specified Y co-ordinate.
    #[inline]
    pub const fn with_y(&self, y: i32) -> PixelPoint {
        PixelPoint::new(self.x, y)
    }

    /// Converts the [`PixelPoint`] to a device-independent [`Point`] using the
    /// specified scaling factor.
    #[inline]
    pub fn to_point(&self, scale: f64) -> Point {
        Point::new(self.x as f64 / scale, self.y as f64 / scale)
    }

    /// Converts the [`PixelPoint`] to a device-independent [`Point`] using the
    /// specified per-axis scaling factor.
    #[inline]
    pub fn to_point_vector(&self, scale: Vector) -> Point {
        Point::new(self.x as f64 / scale.x, self.y as f64 / scale.y)
    }

    /// Converts the [`PixelPoint`] to a device-independent [`Point`] using the
    /// specified dots per inch (DPI).
    #[inline]
    pub fn to_point_with_dpi(&self, dpi: f64) -> Point {
        self.to_point(dpi / 96.0)
    }

    /// Converts the [`PixelPoint`] to a device-independent [`Point`] using the
    /// specified per-axis dots per inch (DPI).
    #[inline]
    pub fn to_point_with_dpi_vector(&self, dpi: Vector) -> Point {
        self.to_point_vector(Vector::new(dpi.x / 96.0, dpi.y / 96.0))
    }

    /// Converts a [`Point`] to device pixels using the specified scaling factor.
    #[inline]
    pub fn from_point(point: Point, scale: f64) -> PixelPoint {
        PixelPoint::new((point.x * scale) as i32, (point.y * scale) as i32)
    }

    /// Converts a [`Point`] to device pixels using the specified per-axis scaling factor.
    #[inline]
    pub fn from_point_vector(point: Point, scale: Vector) -> PixelPoint {
        PixelPoint::new((point.x * scale.x) as i32, (point.y * scale.y) as i32)
    }

    /// Converts a [`Point`] to device pixels using the specified dots per inch (DPI).
    #[inline]
    pub fn from_point_with_dpi(point: Point, dpi: f64) -> PixelPoint {
        PixelPoint::from_point(point, dpi / 96.0)
    }

    /// Converts a [`Point`] to device pixels using the specified per-axis dots per inch (DPI).
    #[inline]
    pub fn from_point_with_dpi_vector(point: Point, dpi: Vector) -> PixelPoint {
        PixelPoint::from_point_vector(point, Vector::new(dpi.x / 96.0, dpi.y / 96.0))
    }
}

/// Converts the [`PixelPoint`] to a [`PixelVector`].
impl From<PixelPoint> for PixelVector {
    #[inline]
    fn from(p: PixelPoint) -> PixelVector {
        PixelVector::new(p.x, p.y)
    }
}

impl Add for PixelPoint {
    type Output = PixelPoint;
    #[inline]
    fn add(self, b: PixelPoint) -> PixelPoint {
        PixelPoint::new(self.x.wrapping_add(b.x), self.y.wrapping_add(b.y))
    }
}

impl Add<PixelVector> for PixelPoint {
    type Output = PixelPoint;
    #[inline]
    fn add(self, b: PixelVector) -> PixelPoint {
        PixelPoint::new(self.x.wrapping_add(b.x), self.y.wrapping_add(b.y))
    }
}

impl Sub for PixelPoint {
    type Output = PixelPoint;
    #[inline]
    fn sub(self, b: PixelPoint) -> PixelPoint {
        PixelPoint::new(self.x.wrapping_sub(b.x), self.y.wrapping_sub(b.y))
    }
}

impl Sub<PixelVector> for PixelPoint {
    type Output = PixelPoint;
    #[inline]
    fn sub(self, b: PixelVector) -> PixelPoint {
        PixelPoint::new(self.x.wrapping_sub(b.x), self.y.wrapping_sub(b.y))
    }
}

impl AddAssign for PixelPoint {
    #[inline]
    fn add_assign(&mut self, b: PixelPoint) {
        *self = *self + b;
    }
}

impl AddAssign<PixelVector> for PixelPoint {
    #[inline]
    fn add_assign(&mut self, b: PixelVector) {
        *self = *self + b;
    }
}

impl SubAssign for PixelPoint {
    #[inline]
    fn sub_assign(&mut self, b: PixelPoint) {
        *self = *self - b;
    }
}

impl SubAssign<PixelVector> for PixelPoint {
    #[inline]
    fn sub_assign(&mut self, b: PixelVector) {
        *self = *self - b;
    }
}

impl FromStr for PixelPoint {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        PixelPoint::parse(s)
    }
}

impl fmt::Display for PixelPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    // The reference test-suite has no tests for this type.
    use super::*;

    #[test]
    fn parse_display_and_operators() {
        assert_eq!(
            PixelPoint::parse("10, -20").unwrap(),
            PixelPoint::new(10, -20)
        );
        assert_eq!(
            PixelPoint::parse("1.5, 2").unwrap_err().message(),
            "Invalid PixelPoint."
        );
        assert_eq!(PixelPoint::new(10, -20).to_string(), "10, -20");
        let p = PixelPoint::new(1, 2);
        assert_eq!(p + PixelPoint::new(1, 1), PixelPoint::new(2, 3));
        assert_eq!(p + PixelVector::new(1, 1), PixelPoint::new(2, 3));
        assert_eq!(p - PixelPoint::new(1, 1), PixelPoint::new(0, 1));
        assert_eq!(p - PixelVector::new(1, 1), PixelPoint::new(0, 1));
        assert_eq!(PixelVector::from(p), PixelVector::new(1, 2));
        assert_eq!(
            PixelPoint::new(i32::MAX, 0) + PixelPoint::new(1, 0),
            PixelPoint::new(i32::MIN, 0)
        );
    }

    #[test]
    fn conversions_truncate_toward_zero() {
        assert_eq!(
            PixelPoint::from_point(Point::new(1.9, -1.9), 1.0),
            PixelPoint::new(1, -1)
        );
        assert_eq!(
            PixelPoint::from_point_with_dpi(Point::new(10.0, 10.0), 144.0),
            PixelPoint::new(15, 15)
        );
        assert_eq!(
            PixelPoint::from_point_vector(Point::new(10.0, 10.0), Vector::new(2.0, 3.0)),
            PixelPoint::new(20, 30)
        );
        assert_eq!(
            PixelPoint::new(15, 30).to_point(1.5),
            Point::new(10.0, 20.0)
        );
        assert_eq!(
            PixelPoint::new(15, 30).to_point_with_dpi_vector(Vector::new(144.0, 192.0)),
            Point::new(10.0, 15.0)
        );
    }
}
