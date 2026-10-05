//! 3D vector with double-precision components.

use std::fmt;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

use crate::utilities::math_utilities;
use crate::utilities::span_helpers::InvariantF64;
use crate::utilities::{FormatError, SpanStringTokenizer};

/// 3D vector with double-precision components.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vector3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3D {
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Parses a [`Vector3D`] string (`"x, y, z"`).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        SpanStringTokenizer::with_message(s, "Invalid Vector.").scope(|t| {
            Ok(Vector3D::new(
                t.read_double()?,
                t.read_double()?,
                t.read_double()?,
            ))
        })
    }

    /// Calculates the dot product of two vectors.
    #[inline]
    pub fn dot(vector1: Vector3D, vector2: Vector3D) -> f64 {
        (vector1.x * vector2.x) + (vector1.y * vector2.y) + (vector1.z * vector2.z)
    }

    /// Adds the second to the first vector.
    #[inline]
    #[allow(clippy::should_implement_trait)] // the operator trait is implemented too
    pub fn add(left: Vector3D, right: Vector3D) -> Vector3D {
        Vector3D::new(left.x + right.x, left.y + right.y, left.z + right.z)
    }

    /// Subtracts the second from the first vector.
    #[inline]
    pub fn substract(left: Vector3D, right: Vector3D) -> Vector3D {
        Vector3D::new(left.x - right.x, left.y - right.y, left.z - right.z)
    }

    /// Multiplies the first vector by the second, component-wise.
    #[inline]
    pub fn multiply(left: Vector3D, right: Vector3D) -> Vector3D {
        Vector3D::new(left.x * right.x, left.y * right.y, left.z * right.z)
    }

    /// Multiplies the vector by the given scalar.
    #[inline]
    pub fn multiply_scalar(left: Vector3D, right: f64) -> Vector3D {
        Vector3D::new(left.x * right, left.y * right, left.z * right)
    }

    /// Divides the first vector by the second, component-wise.
    #[inline]
    pub fn divide(left: Vector3D, right: Vector3D) -> Vector3D {
        Vector3D::new(left.x / right.x, left.y / right.y, left.z / right.z)
    }

    /// Divides the vector by the given scalar.
    #[inline]
    pub fn divide_scalar(left: Vector3D, right: f64) -> Vector3D {
        Vector3D::new(left.x / right, left.y / right, left.z / right)
    }

    /// Returns a vector whose elements are the absolute values of each of the
    /// specified vector's elements.
    #[inline]
    pub fn abs(&self) -> Vector3D {
        Vector3D::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    /// Restricts a vector between a minimum and a maximum value.
    #[inline]
    pub fn clamp(value: Vector3D, min: Vector3D, max: Vector3D) -> Vector3D {
        Vector3D::min(Vector3D::max(value, min), max)
    }

    /// Returns a vector whose elements are the maximum of each of the pairs of
    /// elements in two specified vectors.
    #[inline]
    pub fn max(left: Vector3D, right: Vector3D) -> Vector3D {
        Vector3D::new(
            math_utilities::max(left.x, right.x),
            math_utilities::max(left.y, right.y),
            math_utilities::max(left.z, right.z),
        )
    }

    /// Returns a vector whose elements are the minimum of each of the pairs of
    /// elements in two specified vectors.
    #[inline]
    pub fn min(left: Vector3D, right: Vector3D) -> Vector3D {
        Vector3D::new(
            math_utilities::min(left.x, right.x),
            math_utilities::min(left.y, right.y),
            math_utilities::min(left.z, right.z),
        )
    }

    /// Length of the vector.
    #[inline]
    pub fn length(&self) -> f64 {
        Vector3D::dot(*self, *self).sqrt()
    }

    /// Returns a normalized version of this vector.
    #[inline]
    pub fn normalize(value: Vector3D) -> Vector3D {
        Vector3D::divide_scalar(value, value.length())
    }

    /// Computes the squared Euclidean distance between the two given points.
    #[inline]
    pub fn distance_squared(value1: Vector3D, value2: Vector3D) -> f64 {
        let difference = Vector3D::substract(value1, value2);
        Vector3D::dot(difference, difference)
    }

    /// Computes the Euclidean distance between the two given points.
    #[inline]
    pub fn distance(value1: Vector3D, value2: Vector3D) -> f64 {
        Vector3D::distance_squared(value1, value2).sqrt()
    }

    /// Deconstructs the vector into its components.
    #[inline]
    pub const fn deconstruct(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }
}

impl From<Vector3D> for (f64, f64, f64) {
    #[inline]
    fn from(v: Vector3D) -> Self {
        (v.x, v.y, v.z)
    }
}

impl Add for Vector3D {
    type Output = Vector3D;
    #[inline]
    fn add(self, right: Vector3D) -> Vector3D {
        Vector3D::add(self, right)
    }
}

impl Sub for Vector3D {
    type Output = Vector3D;
    #[inline]
    fn sub(self, right: Vector3D) -> Vector3D {
        Vector3D::substract(self, right)
    }
}

impl Neg for Vector3D {
    type Output = Vector3D;
    #[inline]
    fn neg(self) -> Vector3D {
        Vector3D::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<f64> for Vector3D {
    type Output = Vector3D;
    #[inline]
    fn mul(self, right: f64) -> Vector3D {
        Vector3D::multiply_scalar(self, right)
    }
}

impl AddAssign for Vector3D {
    #[inline]
    fn add_assign(&mut self, right: Vector3D) {
        *self = *self + right;
    }
}

impl SubAssign for Vector3D {
    #[inline]
    fn sub_assign(&mut self, right: Vector3D) {
        *self = *self - right;
    }
}

impl MulAssign<f64> for Vector3D {
    #[inline]
    fn mul_assign(&mut self, right: f64) {
        *self = *self * right;
    }
}

impl FromStr for Vector3D {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        Vector3D::parse(s)
    }
}

impl fmt::Display for Vector3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Vector3D {{ X = {}, Y = {}, Z = {} }}",
            InvariantF64(self.x),
            InvariantF64(self.y),
            InvariantF64(self.z)
        )
    }
}

#[cfg(test)]
mod tests {
    // The reference test-suite has no tests for this type.
    use super::*;

    #[test]
    fn parse_and_display() {
        assert_eq!(
            Vector3D::parse("1, 2.5 -3").unwrap(),
            Vector3D::new(1.0, 2.5, -3.0)
        );
        assert_eq!(
            Vector3D::parse("1,2").unwrap_err().message(),
            "Invalid Vector."
        );
        assert_eq!(
            Vector3D::new(1.0, 2.5, -3.0).to_string(),
            "Vector3D { X = 1, Y = 2.5, Z = -3 }"
        );
    }

    #[test]
    fn arithmetic() {
        let a = Vector3D::new(1.0, 2.0, 3.0);
        let b = Vector3D::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vector3D::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vector3D::new(3.0, 3.0, 3.0));
        assert_eq!(-a, Vector3D::new(-1.0, -2.0, -3.0));
        assert_eq!(a * 2.0, Vector3D::new(2.0, 4.0, 6.0));
        assert_eq!(Vector3D::dot(a, b), 32.0);
        assert_eq!(Vector3D::multiply(a, b), Vector3D::new(4.0, 10.0, 18.0));
        assert_eq!(Vector3D::divide(b, a), Vector3D::new(4.0, 2.5, 2.0));
        assert_eq!(Vector3D::new(2.0, 3.0, 6.0).length(), 7.0);
        assert_eq!(
            Vector3D::normalize(Vector3D::new(0.0, 0.0, 5.0)),
            Vector3D::new(0.0, 0.0, 1.0)
        );
        assert_eq!(Vector3D::distance(a, Vector3D::new(1.0, 2.0, 5.0)), 2.0);
        assert_eq!(
            Vector3D::clamp(
                Vector3D::new(-1.0, 0.5, 9.0),
                Vector3D::default(),
                Vector3D::new(1.0, 1.0, 1.0)
            ),
            Vector3D::new(0.0, 0.5, 1.0)
        );
        assert_eq!(Vector3D::new(-1.0, 2.0, -3.0).abs(), a);
    }
}
