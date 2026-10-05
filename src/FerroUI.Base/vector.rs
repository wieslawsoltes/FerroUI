//! Defines a vector.

use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

use crate::utilities::math_utilities::{self, MathUtilities};
use crate::utilities::span_helpers::InvariantF64;
use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::Point;

/// Defines a vector.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vector {
    /// The X component.
    pub x: f64,
    /// The Y component.
    pub y: f64,
}

impl Vector {
    /// The zero vector.
    pub const ZERO: Vector = Vector::new(0.0, 0.0);

    /// The one vector.
    pub const ONE: Vector = Vector::new(1.0, 1.0);

    /// The X unit vector.
    pub const UNIT_X: Vector = Vector::new(1.0, 0.0);

    /// The Y unit vector.
    pub const UNIT_Y: Vector = Vector::new(0.0, 1.0);

    /// Initializes a new instance of the [`Vector`] structure.
    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Parses a [`Vector`] string (`"x, y"` or `"x y"`).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        SpanStringTokenizer::with_message(s, "Invalid Vector.")
            .scope(|t| Ok(Vector::new(t.read_double()?, t.read_double()?)))
    }

    /// Length of the vector.
    #[inline]
    pub fn length(&self) -> f64 {
        self.squared_length().sqrt()
    }

    /// Squared length of the vector.
    #[inline]
    pub fn squared_length(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// Check if two vectors are equal (exact comparison of the components).
    #[inline]
    pub fn equals(&self, other: Vector) -> bool {
        self.x == other.x && self.y == other.y
    }

    /// Check if two vectors are nearly equal (numerically).
    #[inline]
    pub fn nearly_equals(&self, other: Vector) -> bool {
        MathUtilities::are_close(self.x, other.x) && MathUtilities::are_close(self.y, other.y)
    }

    /// Returns a new vector with the specified X component.
    #[inline]
    pub const fn with_x(&self, x: f64) -> Vector {
        Vector::new(x, self.y)
    }

    /// Returns a new vector with the specified Y component.
    #[inline]
    pub const fn with_y(&self, y: f64) -> Vector {
        Vector::new(self.x, y)
    }

    /// Returns a normalized version of this vector. A zero-length vector yields NaN components.
    #[inline]
    pub fn normalize(&self) -> Vector {
        Vector::normalize_vector(*self)
    }

    /// Returns a negated version of this vector.
    #[inline]
    pub fn negate(&self) -> Vector {
        Vector::negate_vector(*self)
    }

    /// Returns the dot product of two vectors.
    #[inline]
    pub fn dot(a: Vector, b: Vector) -> f64 {
        a.x * b.x + a.y * b.y
    }

    /// Returns the cross product of two vectors.
    #[inline]
    pub fn cross(a: Vector, b: Vector) -> f64 {
        a.x * b.y - a.y * b.x
    }

    /// Normalizes the given vector (static form of [`normalize`](Self::normalize)).
    #[inline]
    pub fn normalize_vector(vector: Vector) -> Vector {
        Vector::divide_scalar(vector, vector.length())
    }

    /// Divides the first vector by the second, component-wise.
    #[inline]
    pub fn divide(a: Vector, b: Vector) -> Vector {
        Vector::new(a.x / b.x, a.y / b.y)
    }

    /// Divides the vector by the given scalar.
    #[inline]
    pub fn divide_scalar(vector: Vector, scalar: f64) -> Vector {
        Vector::new(vector.x / scalar, vector.y / scalar)
    }

    /// Multiplies the first vector by the second, component-wise.
    #[inline]
    pub fn multiply(a: Vector, b: Vector) -> Vector {
        Vector::new(a.x * b.x, a.y * b.y)
    }

    /// Multiplies the vector by the given scalar.
    #[inline]
    pub fn multiply_scalar(vector: Vector, scalar: f64) -> Vector {
        Vector::new(vector.x * scalar, vector.y * scalar)
    }

    /// Adds the second to the first vector.
    #[inline]
    #[allow(clippy::should_implement_trait)] // the operator trait is implemented too
    pub fn add(a: Vector, b: Vector) -> Vector {
        Vector::new(a.x + b.x, a.y + b.y)
    }

    /// Subtracts the second from the first vector.
    #[inline]
    pub fn subtract(a: Vector, b: Vector) -> Vector {
        Vector::new(a.x - b.x, a.y - b.y)
    }

    /// Negates the vector (static form of [`negate`](Self::negate)).
    #[inline]
    pub fn negate_vector(vector: Vector) -> Vector {
        Vector::new(-vector.x, -vector.y)
    }

    /// Deconstructs the vector into its X and Y components.
    #[inline]
    pub const fn deconstruct(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    /// Returns a vector whose elements are the absolute values of each of the
    /// specified vector's elements.
    #[inline]
    pub fn abs(&self) -> Vector {
        Vector::new(self.x.abs(), self.y.abs())
    }

    /// Restricts a vector between a minimum and a maximum value.
    #[inline]
    pub fn clamp(value: Vector, min: Vector, max: Vector) -> Vector {
        Vector::min(Vector::max(value, min), max)
    }

    /// Returns a vector whose elements are the maximum of each of the pairs of
    /// elements in two specified vectors.
    #[inline]
    pub fn max(left: Vector, right: Vector) -> Vector {
        Vector::new(
            math_utilities::max(left.x, right.x),
            math_utilities::max(left.y, right.y),
        )
    }

    /// Returns a vector whose elements are the minimum of each of the pairs of
    /// elements in two specified vectors.
    #[inline]
    pub fn min(left: Vector, right: Vector) -> Vector {
        Vector::new(
            math_utilities::min(left.x, right.x),
            math_utilities::min(left.y, right.y),
        )
    }

    /// Computes the Euclidean distance between the two given points.
    #[inline]
    pub fn distance(value1: Vector, value2: Vector) -> f64 {
        Vector::distance_squared(value1, value2).sqrt()
    }

    /// Returns the Euclidean distance squared between two specified points.
    #[inline]
    pub fn distance_squared(value1: Vector, value2: Vector) -> f64 {
        let difference = value1 - value2;
        Vector::dot(difference, difference)
    }
}

/// Converts the [`Vector`] to a [`Point`] (an explicit conversion in the reference API).
impl From<Vector> for Point {
    #[inline]
    fn from(a: Vector) -> Point {
        Point::new(a.x, a.y)
    }
}

impl From<Vector> for (f64, f64) {
    #[inline]
    fn from(v: Vector) -> Self {
        (v.x, v.y)
    }
}

/// Calculates the dot product of two vectors.
impl Mul for Vector {
    type Output = f64;
    #[inline]
    fn mul(self, b: Vector) -> f64 {
        Vector::dot(self, b)
    }
}

impl Mul<f64> for Vector {
    type Output = Vector;
    #[inline]
    fn mul(self, scale: f64) -> Vector {
        Vector::multiply_scalar(self, scale)
    }
}

impl Mul<Vector> for f64 {
    type Output = Vector;
    #[inline]
    fn mul(self, vector: Vector) -> Vector {
        Vector::multiply_scalar(vector, self)
    }
}

impl Div<f64> for Vector {
    type Output = Vector;
    #[inline]
    fn div(self, scale: f64) -> Vector {
        Vector::divide_scalar(self, scale)
    }
}

impl Neg for Vector {
    type Output = Vector;
    #[inline]
    fn neg(self) -> Vector {
        Vector::negate_vector(self)
    }
}

impl Add for Vector {
    type Output = Vector;
    #[inline]
    fn add(self, b: Vector) -> Vector {
        Vector::add(self, b)
    }
}

impl Sub for Vector {
    type Output = Vector;
    #[inline]
    fn sub(self, b: Vector) -> Vector {
        Vector::subtract(self, b)
    }
}

impl AddAssign for Vector {
    #[inline]
    fn add_assign(&mut self, b: Vector) {
        *self = *self + b;
    }
}

impl SubAssign for Vector {
    #[inline]
    fn sub_assign(&mut self, b: Vector) {
        *self = *self - b;
    }
}

impl MulAssign<f64> for Vector {
    #[inline]
    fn mul_assign(&mut self, scale: f64) {
        *self = *self * scale;
    }
}

impl DivAssign<f64> for Vector {
    #[inline]
    fn div_assign(&mut self, scale: f64) {
        *self = *self / scale;
    }
}

impl FromStr for Vector {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        Vector::parse(s)
    }
}

impl fmt::Display for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", InvariantF64(self.x), InvariantF64(self.y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_should_return_correct_length_of_vector() {
        let vector = Vector::new(2.0, 4.0);
        let length = ((2 * 2 + 4 * 4) as f64).sqrt();

        assert_eq!(length, vector.length());
    }

    #[test]
    fn length_squared_should_return_correct_length_of_vector() {
        let vector_a = Vector::new(2.0, 4.0);
        let squared_length_a = (2 * 2 + 4 * 4) as f64;

        assert_eq!(squared_length_a, vector_a.squared_length());
    }

    #[test]
    fn normalize_should_return_normalized_vector() {
        // the length of a normalized vector must be 1

        let vector_a = Vector::new(13.0, 84.0);
        let vector_b = Vector::new(-34.0, 345.0);
        let vector_c = Vector::new(-34.0, -84.0);

        assert_eq!(1.0, vector_a.normalize().length());
        assert_eq!(1.0, vector_b.normalize().length());
        assert_eq!(1.0, vector_c.normalize().length());
    }

    #[test]
    fn negate_should_return_negated_vector() {
        let vector = Vector::new(2.0, 4.0);
        let negated = Vector::new(-2.0, -4.0);

        assert_eq!(negated, vector.negate());
    }

    #[test]
    fn dot_should_return_correct_value() {
        let a = Vector::new(-6.0, 8.0);
        let b = Vector::new(5.0, 12.0);

        assert_eq!(66.0, Vector::dot(a, b));
    }

    #[test]
    fn cross_should_return_correct_value() {
        let a = Vector::new(-6.0, 8.0);
        let b = Vector::new(5.0, 12.0);

        assert_eq!(-112.0, Vector::cross(a, b));
    }

    #[test]
    fn divied_by_vector_should_return_correct_value() {
        let a = Vector::new(10.0, 2.0);
        let b = Vector::new(5.0, 2.0);

        let expected = Vector::new(2.0, 1.0);

        assert_eq!(expected, Vector::divide(a, b));
    }

    #[test]
    fn divied_should_return_correct_value() {
        let vector = Vector::new(10.0, 2.0);
        let expected = Vector::new(5.0, 1.0);

        assert_eq!(expected, Vector::divide_scalar(vector, 2.0));
    }

    #[test]
    fn multiply_by_vector_should_return_correct_value() {
        let a = Vector::new(10.0, 2.0);
        let b = Vector::new(2.0, 2.0);

        let expected = Vector::new(20.0, 4.0);

        assert_eq!(expected, Vector::multiply(a, b));
    }

    #[test]
    fn multiply_should_return_correct_value() {
        let vector = Vector::new(10.0, 2.0);

        let expected = Vector::new(20.0, 4.0);

        assert_eq!(expected, Vector::multiply_scalar(vector, 2.0));
    }

    #[test]
    fn scale_vector_should_be_commutative() {
        let vector = Vector::new(10.0, 2.0);

        let expected = vector * 2.0;

        assert_eq!(expected, 2.0 * vector);
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn parse_display_and_helpers() {
        assert_eq!(Vector::parse("1,2").unwrap(), Vector::new(1.0, 2.0));
        assert_eq!(Vector::parse("x").unwrap_err().message(), "Invalid Vector.");
        assert_eq!(Vector::new(1.0, 2.5).to_string(), "1, 2.5");
        assert_eq!(Vector::new(-1.0, 2.0).abs(), Vector::new(1.0, 2.0));
        assert_eq!(
            Vector::clamp(Vector::new(-5.0, 5.0), Vector::ZERO, Vector::ONE),
            Vector::new(0.0, 1.0)
        );
        assert_eq!(Vector::distance(Vector::ZERO, Vector::new(3.0, 4.0)), 5.0);
        assert_eq!(Vector::new(1.0, 2.0) * Vector::new(3.0, 4.0), 11.0);
        assert_eq!(Point::from(Vector::UNIT_X), Point::new(1.0, 0.0));
    }
}
