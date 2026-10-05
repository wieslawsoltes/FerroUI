//! A vector with two single-precision components.

use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use super::single::{self, InvariantF32};
use super::{Matrix3x2, Matrix4x4, Quaternion};
use crate::Vector;

/// A vector with two single-precision components.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vector2 {
    /// The X component of the vector.
    pub x: f32,
    /// The Y component of the vector.
    pub y: f32,
}

impl Vector2 {
    /// The vector `(0, 0)`.
    pub const ZERO: Vector2 = Vector2::new(0.0, 0.0);
    /// The vector `(1, 1)`.
    pub const ONE: Vector2 = Vector2::new(1.0, 1.0);
    /// The vector `(1, 0)`.
    pub const UNIT_X: Vector2 = Vector2::new(1.0, 0.0);
    /// The vector `(0, 1)`.
    pub const UNIT_Y: Vector2 = Vector2::new(0.0, 1.0);

    /// Creates a vector from its components.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Creates a vector whose components all have the same value.
    #[inline]
    pub const fn from_value(value: f32) -> Self {
        Self::new(value, value)
    }

    /// Returns a vector whose elements are the absolute values of each of the
    /// specified vector's elements.
    #[inline]
    pub fn abs(value: Vector2) -> Vector2 {
        Vector2::new(value.x.abs(), value.y.abs())
    }

    /// Restricts a vector between a minimum and a maximum value
    /// (`min(max(value, min), max)`, so `max` wins when the bounds are inverted).
    #[inline]
    pub fn clamp(value1: Vector2, min: Vector2, max: Vector2) -> Vector2 {
        Vector2::min(Vector2::max(value1, min), max)
    }

    /// Computes the Euclidean distance between the two given points.
    #[inline]
    pub fn distance(value1: Vector2, value2: Vector2) -> f32 {
        Vector2::distance_squared(value1, value2).sqrt()
    }

    /// Returns the Euclidean distance squared between two specified points.
    #[inline]
    pub fn distance_squared(value1: Vector2, value2: Vector2) -> f32 {
        (value1 - value2).length_squared()
    }

    /// Returns the dot product of two vectors.
    #[inline]
    pub fn dot(value1: Vector2, value2: Vector2) -> f32 {
        (value1.x * value2.x) + (value1.y * value2.y)
    }

    /// Returns the length of the vector.
    #[inline]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns the length of the vector squared.
    #[inline]
    pub fn length_squared(&self) -> f32 {
        Vector2::dot(*self, *self)
    }

    /// Performs a linear interpolation between two vectors based on the given weighting.
    #[inline]
    pub fn lerp(value1: Vector2, value2: Vector2, amount: f32) -> Vector2 {
        (value1 * (1.0 - amount)) + (value2 * amount)
    }

    /// Returns a vector whose elements are the maximum of each of the pairs of
    /// elements in two specified vectors.
    #[inline]
    pub fn max(value1: Vector2, value2: Vector2) -> Vector2 {
        Vector2::new(single::max(value1.x, value2.x), single::max(value1.y, value2.y))
    }

    /// Returns a vector whose elements are the minimum of each of the pairs of
    /// elements in two specified vectors.
    #[inline]
    pub fn min(value1: Vector2, value2: Vector2) -> Vector2 {
        Vector2::new(single::min(value1.x, value2.x), single::min(value1.y, value2.y))
    }

    /// Returns a vector with the same direction as the specified vector, but
    /// with a length of one (NaN components for a zero-length vector).
    #[inline]
    pub fn normalize(value: Vector2) -> Vector2 {
        value / value.length()
    }

    /// Returns a vector whose elements are the square root of each of a
    /// specified vector's elements.
    #[inline]
    pub fn square_root(value: Vector2) -> Vector2 {
        Vector2::new(value.x.sqrt(), value.y.sqrt())
    }

    /// Transforms a vector by a specified 3x2 matrix.
    #[inline]
    pub fn transform(position: Vector2, matrix: Matrix3x2) -> Vector2 {
        Vector2::new(
            (position.x * matrix.m11) + (position.y * matrix.m21) + matrix.m31,
            (position.x * matrix.m12) + (position.y * matrix.m22) + matrix.m32,
        )
    }

    /// Transforms a vector by a specified 4x4 matrix.
    #[inline]
    pub fn transform_matrix4x4(position: Vector2, matrix: Matrix4x4) -> Vector2 {
        Vector2::new(
            (position.x * matrix.m11) + (position.y * matrix.m21) + matrix.m41,
            (position.x * matrix.m12) + (position.y * matrix.m22) + matrix.m42,
        )
    }

    /// Transforms a vector by the specified rotation.
    pub fn transform_quaternion(value: Vector2, rotation: Quaternion) -> Vector2 {
        let x2 = rotation.x + rotation.x;
        let y2 = rotation.y + rotation.y;
        let z2 = rotation.z + rotation.z;

        let wz2 = rotation.w * z2;
        let xx2 = rotation.x * x2;
        let xy2 = rotation.x * y2;
        let yy2 = rotation.y * y2;
        let zz2 = rotation.z * z2;

        Vector2::new(
            value.x * (1.0 - yy2 - zz2) + value.y * (xy2 - wz2),
            value.x * (xy2 + wz2) + value.y * (1.0 - xx2 - zz2),
        )
    }

    /// Transforms a vector normal by the given 3x2 matrix.
    #[inline]
    pub fn transform_normal(normal: Vector2, matrix: Matrix3x2) -> Vector2 {
        Vector2::new(
            (normal.x * matrix.m11) + (normal.y * matrix.m21),
            (normal.x * matrix.m12) + (normal.y * matrix.m22),
        )
    }
}

/// The narrowing conversion of the double-precision vector.
impl From<Vector> for Vector2 {
    #[inline]
    fn from(value: Vector) -> Vector2 {
        Vector2::new(value.x as f32, value.y as f32)
    }
}

/// The widening conversion to the double-precision vector.
impl From<Vector2> for Vector {
    #[inline]
    fn from(value: Vector2) -> Vector {
        Vector::new(value.x as f64, value.y as f64)
    }
}

impl Add for Vector2 {
    type Output = Vector2;
    #[inline]
    fn add(self, right: Vector2) -> Vector2 {
        Vector2::new(self.x + right.x, self.y + right.y)
    }
}

impl Sub for Vector2 {
    type Output = Vector2;
    #[inline]
    fn sub(self, right: Vector2) -> Vector2 {
        Vector2::new(self.x - right.x, self.y - right.y)
    }
}

impl Mul for Vector2 {
    type Output = Vector2;
    #[inline]
    fn mul(self, right: Vector2) -> Vector2 {
        Vector2::new(self.x * right.x, self.y * right.y)
    }
}

impl Mul<f32> for Vector2 {
    type Output = Vector2;
    #[inline]
    fn mul(self, right: f32) -> Vector2 {
        Vector2::new(self.x * right, self.y * right)
    }
}

impl Mul<Vector2> for f32 {
    type Output = Vector2;
    #[inline]
    fn mul(self, right: Vector2) -> Vector2 {
        right * self
    }
}

impl Div for Vector2 {
    type Output = Vector2;
    #[inline]
    fn div(self, right: Vector2) -> Vector2 {
        Vector2::new(self.x / right.x, self.y / right.y)
    }
}

impl Div<f32> for Vector2 {
    type Output = Vector2;
    #[inline]
    fn div(self, right: f32) -> Vector2 {
        Vector2::new(self.x / right, self.y / right)
    }
}

impl Neg for Vector2 {
    type Output = Vector2;
    #[inline]
    fn neg(self) -> Vector2 {
        Vector2::new(-self.x, -self.y)
    }
}

impl AddAssign for Vector2 {
    #[inline]
    fn add_assign(&mut self, right: Vector2) {
        *self = *self + right;
    }
}

impl SubAssign for Vector2 {
    #[inline]
    fn sub_assign(&mut self, right: Vector2) {
        *self = *self - right;
    }
}

impl MulAssign for Vector2 {
    #[inline]
    fn mul_assign(&mut self, right: Vector2) {
        *self = *self * right;
    }
}

impl MulAssign<f32> for Vector2 {
    #[inline]
    fn mul_assign(&mut self, right: f32) {
        *self = *self * right;
    }
}

impl DivAssign for Vector2 {
    #[inline]
    fn div_assign(&mut self, right: Vector2) {
        *self = *self / right;
    }
}

impl DivAssign<f32> for Vector2 {
    #[inline]
    fn div_assign(&mut self, right: f32) {
        *self = *self / right;
    }
}

impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}, {}>", InvariantF32(self.x), InvariantF32(self.y))
    }
}
