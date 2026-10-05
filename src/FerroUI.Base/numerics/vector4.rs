//! A vector with four single-precision components.

use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use super::single::{self, InvariantF32};
use super::{Matrix4x4, Vector2, Vector3};

/// A vector with four single-precision components.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vector4 {
    /// The X component of the vector.
    pub x: f32,
    /// The Y component of the vector.
    pub y: f32,
    /// The Z component of the vector.
    pub z: f32,
    /// The W component of the vector.
    pub w: f32,
}

impl Vector4 {
    /// The vector `(0, 0, 0, 0)`.
    pub const ZERO: Vector4 = Vector4::new(0.0, 0.0, 0.0, 0.0);
    /// The vector `(1, 1, 1, 1)`.
    pub const ONE: Vector4 = Vector4::new(1.0, 1.0, 1.0, 1.0);
    /// The vector `(1, 0, 0, 0)`.
    pub const UNIT_X: Vector4 = Vector4::new(1.0, 0.0, 0.0, 0.0);
    /// The vector `(0, 1, 0, 0)`.
    pub const UNIT_Y: Vector4 = Vector4::new(0.0, 1.0, 0.0, 0.0);
    /// The vector `(0, 0, 1, 0)`.
    pub const UNIT_Z: Vector4 = Vector4::new(0.0, 0.0, 1.0, 0.0);
    /// The vector `(0, 0, 0, 1)`.
    pub const UNIT_W: Vector4 = Vector4::new(0.0, 0.0, 0.0, 1.0);

    /// Creates a vector from its components.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Creates a vector whose components all have the same value.
    #[inline]
    pub const fn from_value(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    /// Creates a vector from a [`Vector2`] and the Z and W components.
    #[inline]
    pub const fn from_vector2(value: Vector2, z: f32, w: f32) -> Self {
        Self::new(value.x, value.y, z, w)
    }

    /// Creates a vector from a [`Vector3`] and a W component.
    #[inline]
    pub const fn from_vector3(value: Vector3, w: f32) -> Self {
        Self::new(value.x, value.y, value.z, w)
    }

    /// Returns a vector whose elements are the absolute values of each of the
    /// specified vector's elements.
    #[inline]
    pub fn abs(value: Vector4) -> Vector4 {
        Vector4::new(value.x.abs(), value.y.abs(), value.z.abs(), value.w.abs())
    }

    /// Restricts a vector between a minimum and a maximum value
    /// (`min(max(value, min), max)`, so `max` wins when the bounds are inverted).
    #[inline]
    pub fn clamp(value1: Vector4, min: Vector4, max: Vector4) -> Vector4 {
        Vector4::min(Vector4::max(value1, min), max)
    }

    /// Computes the Euclidean distance between the two given points.
    #[inline]
    pub fn distance(value1: Vector4, value2: Vector4) -> f32 {
        Vector4::distance_squared(value1, value2).sqrt()
    }

    /// Returns the Euclidean distance squared between two specified points.
    #[inline]
    pub fn distance_squared(value1: Vector4, value2: Vector4) -> f32 {
        (value1 - value2).length_squared()
    }

    /// Returns the dot product of two vectors.
    #[inline]
    pub fn dot(vector1: Vector4, vector2: Vector4) -> f32 {
        (vector1.x * vector2.x) + (vector1.y * vector2.y) + (vector1.z * vector2.z) + (vector1.w * vector2.w)
    }

    /// Returns the length of the vector.
    #[inline]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns the length of the vector squared.
    #[inline]
    pub fn length_squared(&self) -> f32 {
        Vector4::dot(*self, *self)
    }

    /// Performs a linear interpolation between two vectors based on the given weighting.
    #[inline]
    pub fn lerp(value1: Vector4, value2: Vector4, amount: f32) -> Vector4 {
        (value1 * (1.0 - amount)) + (value2 * amount)
    }

    /// Returns a vector whose elements are the maximum of each of the pairs of
    /// elements in two specified vectors.
    #[inline]
    pub fn max(value1: Vector4, value2: Vector4) -> Vector4 {
        Vector4::new(
            single::max(value1.x, value2.x),
            single::max(value1.y, value2.y),
            single::max(value1.z, value2.z),
            single::max(value1.w, value2.w),
        )
    }

    /// Returns a vector whose elements are the minimum of each of the pairs of
    /// elements in two specified vectors.
    #[inline]
    pub fn min(value1: Vector4, value2: Vector4) -> Vector4 {
        Vector4::new(
            single::min(value1.x, value2.x),
            single::min(value1.y, value2.y),
            single::min(value1.z, value2.z),
            single::min(value1.w, value2.w),
        )
    }

    /// Returns a vector with the same direction as the specified vector, but
    /// with a length of one (NaN components for a zero-length vector).
    #[inline]
    pub fn normalize(vector: Vector4) -> Vector4 {
        vector / vector.length()
    }

    /// Returns a vector whose elements are the square root of each of a
    /// specified vector's elements.
    #[inline]
    pub fn square_root(value: Vector4) -> Vector4 {
        Vector4::new(value.x.sqrt(), value.y.sqrt(), value.z.sqrt(), value.w.sqrt())
    }

    /// Transforms a four-dimensional vector by a specified 4x4 matrix.
    #[inline]
    pub fn transform(vector: Vector4, matrix: Matrix4x4) -> Vector4 {
        Vector4::new(
            (vector.x * matrix.m11) + (vector.y * matrix.m21) + (vector.z * matrix.m31) + (vector.w * matrix.m41),
            (vector.x * matrix.m12) + (vector.y * matrix.m22) + (vector.z * matrix.m32) + (vector.w * matrix.m42),
            (vector.x * matrix.m13) + (vector.y * matrix.m23) + (vector.z * matrix.m33) + (vector.w * matrix.m43),
            (vector.x * matrix.m14) + (vector.y * matrix.m24) + (vector.z * matrix.m34) + (vector.w * matrix.m44),
        )
    }

    /// Transforms a three-dimensional vector (with an implied `w` of one) by
    /// a specified 4x4 matrix.
    #[inline]
    pub fn transform_vector3(position: Vector3, matrix: Matrix4x4) -> Vector4 {
        Vector4::transform(Vector4::from_vector3(position, 1.0), matrix)
    }

    /// Transforms a two-dimensional vector (with an implied `z` of zero and
    /// `w` of one) by a specified 4x4 matrix.
    #[inline]
    pub fn transform_vector2(position: Vector2, matrix: Matrix4x4) -> Vector4 {
        Vector4::transform(Vector4::from_vector2(position, 0.0, 1.0), matrix)
    }
}

impl Add for Vector4 {
    type Output = Vector4;
    #[inline]
    fn add(self, right: Vector4) -> Vector4 {
        Vector4::new(self.x + right.x, self.y + right.y, self.z + right.z, self.w + right.w)
    }
}

impl Sub for Vector4 {
    type Output = Vector4;
    #[inline]
    fn sub(self, right: Vector4) -> Vector4 {
        Vector4::new(self.x - right.x, self.y - right.y, self.z - right.z, self.w - right.w)
    }
}

impl Mul for Vector4 {
    type Output = Vector4;
    #[inline]
    fn mul(self, right: Vector4) -> Vector4 {
        Vector4::new(self.x * right.x, self.y * right.y, self.z * right.z, self.w * right.w)
    }
}

impl Mul<f32> for Vector4 {
    type Output = Vector4;
    #[inline]
    fn mul(self, right: f32) -> Vector4 {
        Vector4::new(self.x * right, self.y * right, self.z * right, self.w * right)
    }
}

impl Mul<Vector4> for f32 {
    type Output = Vector4;
    #[inline]
    fn mul(self, right: Vector4) -> Vector4 {
        right * self
    }
}

impl Div for Vector4 {
    type Output = Vector4;
    #[inline]
    fn div(self, right: Vector4) -> Vector4 {
        Vector4::new(self.x / right.x, self.y / right.y, self.z / right.z, self.w / right.w)
    }
}

impl Div<f32> for Vector4 {
    type Output = Vector4;
    #[inline]
    fn div(self, right: f32) -> Vector4 {
        Vector4::new(self.x / right, self.y / right, self.z / right, self.w / right)
    }
}

impl Neg for Vector4 {
    type Output = Vector4;
    #[inline]
    fn neg(self) -> Vector4 {
        Vector4::new(-self.x, -self.y, -self.z, -self.w)
    }
}

impl AddAssign for Vector4 {
    #[inline]
    fn add_assign(&mut self, right: Vector4) {
        *self = *self + right;
    }
}

impl SubAssign for Vector4 {
    #[inline]
    fn sub_assign(&mut self, right: Vector4) {
        *self = *self - right;
    }
}

impl MulAssign for Vector4 {
    #[inline]
    fn mul_assign(&mut self, right: Vector4) {
        *self = *self * right;
    }
}

impl MulAssign<f32> for Vector4 {
    #[inline]
    fn mul_assign(&mut self, right: f32) {
        *self = *self * right;
    }
}

impl DivAssign for Vector4 {
    #[inline]
    fn div_assign(&mut self, right: Vector4) {
        *self = *self / right;
    }
}

impl DivAssign<f32> for Vector4 {
    #[inline]
    fn div_assign(&mut self, right: f32) {
        *self = *self / right;
    }
}

impl fmt::Display for Vector4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<{}, {}, {}, {}>",
            InvariantF32(self.x),
            InvariantF32(self.y),
            InvariantF32(self.z),
            InvariantF32(self.w)
        )
    }
}
