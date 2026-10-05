//! Defines a point.

use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

use crate::utilities::math_utilities::MathUtilities;
use crate::utilities::span_helpers::InvariantF64;
use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::{Matrix, Vector};

/// Defines a point.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
    /// The X position.
    pub x: f64,
    /// The Y position.
    pub y: f64,
}

impl Point {
    /// Initializes a new instance of the [`Point`] structure.
    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Computes the Euclidean distance between the two given points.
    #[inline]
    pub fn distance(value1: Point, value2: Point) -> f64 {
        let distance_squared = ((value2.x - value1.x) * (value2.x - value1.x))
            + ((value2.y - value1.y) * (value2.y - value1.y));
        distance_squared.sqrt()
    }

    /// Parses a [`Point`] string (`"x, y"` or `"x y"`).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        SpanStringTokenizer::with_message(s, "Invalid Point.")
            .scope(|t| Ok(Point::new(t.read_double()?, t.read_double()?)))
    }

    /// Returns a boolean indicating whether the point is equal to the other given
    /// point (bitwise equality of the coordinates is *not* required: this is `==`).
    #[inline]
    pub fn equals(&self, other: Point) -> bool {
        self.x == other.x && self.y == other.y
    }

    /// Returns a boolean indicating whether the point is equal to the other given
    /// point (numerically, within a relative epsilon).
    #[inline]
    pub fn nearly_equals(&self, other: Point) -> bool {
        MathUtilities::are_close(self.x, other.x) && MathUtilities::are_close(self.y, other.y)
    }

    /// Transforms the point by a matrix.
    #[inline]
    pub fn transform(&self, transform: Matrix) -> Point {
        transform.transform(*self)
    }

    /// Returns a new point with the specified X coordinate.
    #[inline]
    pub const fn with_x(&self, x: f64) -> Point {
        Point::new(x, self.y)
    }

    /// Returns a new point with the specified Y coordinate.
    #[inline]
    pub const fn with_y(&self, y: f64) -> Point {
        Point::new(self.x, y)
    }

    /// Deconstructs the point into its X and Y coordinates.
    #[inline]
    pub const fn deconstruct(&self) -> (f64, f64) {
        (self.x, self.y)
    }
}

/// Converts the [`Point`] to a [`Vector`].
impl From<Point> for Vector {
    #[inline]
    fn from(p: Point) -> Vector {
        Vector::new(p.x, p.y)
    }
}

impl From<Point> for (f64, f64) {
    #[inline]
    fn from(p: Point) -> Self {
        (p.x, p.y)
    }
}

impl Neg for Point {
    type Output = Point;
    #[inline]
    fn neg(self) -> Point {
        Point::new(-self.x, -self.y)
    }
}

impl Add for Point {
    type Output = Point;
    #[inline]
    fn add(self, b: Point) -> Point {
        Point::new(self.x + b.x, self.y + b.y)
    }
}

impl Add<Vector> for Point {
    type Output = Point;
    #[inline]
    fn add(self, b: Vector) -> Point {
        Point::new(self.x + b.x, self.y + b.y)
    }
}

impl Sub for Point {
    type Output = Point;
    #[inline]
    fn sub(self, b: Point) -> Point {
        Point::new(self.x - b.x, self.y - b.y)
    }
}

impl Sub<Vector> for Point {
    type Output = Point;
    #[inline]
    fn sub(self, b: Vector) -> Point {
        Point::new(self.x - b.x, self.y - b.y)
    }
}

impl Mul<f64> for Point {
    type Output = Point;
    #[inline]
    fn mul(self, k: f64) -> Point {
        Point::new(self.x * k, self.y * k)
    }
}

impl Mul<Point> for f64 {
    type Output = Point;
    #[inline]
    fn mul(self, p: Point) -> Point {
        Point::new(p.x * self, p.y * self)
    }
}

impl Div<f64> for Point {
    type Output = Point;
    #[inline]
    fn div(self, k: f64) -> Point {
        Point::new(self.x / k, self.y / k)
    }
}

/// Applies a matrix to a point.
impl Mul<Matrix> for Point {
    type Output = Point;
    #[inline]
    fn mul(self, matrix: Matrix) -> Point {
        matrix.transform(self)
    }
}

impl AddAssign for Point {
    #[inline]
    fn add_assign(&mut self, b: Point) {
        *self = *self + b;
    }
}

impl AddAssign<Vector> for Point {
    #[inline]
    fn add_assign(&mut self, b: Vector) {
        *self = *self + b;
    }
}

impl SubAssign for Point {
    #[inline]
    fn sub_assign(&mut self, b: Point) {
        *self = *self - b;
    }
}

impl SubAssign<Vector> for Point {
    #[inline]
    fn sub_assign(&mut self, b: Vector) {
        *self = *self - b;
    }
}

impl MulAssign<f64> for Point {
    #[inline]
    fn mul_assign(&mut self, k: f64) {
        *self = *self * k;
    }
}

impl DivAssign<f64> for Point {
    #[inline]
    fn div_assign(&mut self, k: f64) {
        *self = *self / k;
    }
}

impl MulAssign<Matrix> for Point {
    #[inline]
    fn mul_assign(&mut self, matrix: Matrix) {
        *self = *self * matrix;
    }
}

impl FromStr for Point {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        Point::parse(s)
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", InvariantF64(self.x), InvariantF64(self.y))
    }
}

#[cfg(test)]
mod tests {
    // The reference test-suite has no dedicated tests for this type; these cover
    // the members that the other suites do not exercise.
    use super::*;

    #[test]
    fn parse_and_display_round_trip() {
        assert_eq!(Point::parse("1.5, -2").unwrap(), Point::new(1.5, -2.0));
        assert_eq!("3 4".parse::<Point>().unwrap(), Point::new(3.0, 4.0));
        assert_eq!(Point::new(1.5, -2.0).to_string(), "1.5, -2");
        assert_eq!(Point::parse("1").unwrap_err().message(), "Invalid Point.");
        assert!(Point::parse("1,2,3").is_err());
        assert!(Point::parse("1,,2").is_err());
        assert!(Point::parse("").is_err());
    }

    #[test]
    fn operators() {
        let p = Point::new(1.0, 2.0);
        assert_eq!(-p, Point::new(-1.0, -2.0));
        assert_eq!(p + Point::new(1.0, 1.0), Point::new(2.0, 3.0));
        assert_eq!(p + Vector::new(1.0, 1.0), Point::new(2.0, 3.0));
        assert_eq!(p - Point::new(1.0, 1.0), Point::new(0.0, 1.0));
        assert_eq!(p - Vector::new(1.0, 1.0), Point::new(0.0, 1.0));
        assert_eq!(p * 2.0, Point::new(2.0, 4.0));
        assert_eq!(2.0 * p, Point::new(2.0, 4.0));
        assert_eq!(p / 2.0, Point::new(0.5, 1.0));
        assert_eq!(
            p * Matrix::create_translation(1.0, 1.0),
            Point::new(2.0, 3.0)
        );
        assert_eq!(Vector::from(p), Vector::new(1.0, 2.0));
        assert_eq!(
            Point::distance(Point::new(0.0, 0.0), Point::new(3.0, 4.0)),
            5.0
        );
    }

    #[test]
    fn equality_is_exact_and_nearly_equals_is_tolerant() {
        let a = Point::new(0.1 + 0.2, 1.0);
        let b = Point::new(0.3, 1.0);
        assert_ne!(a, b);
        assert!(a.nearly_equals(b));
        assert_ne!(Point::new(f64::NAN, 0.0), Point::new(f64::NAN, 0.0));
    }
}
