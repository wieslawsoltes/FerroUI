//! A vector with three single-precision components.

use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use super::single::{self, InvariantF32};
use super::{Matrix4x4, Quaternion, Vector2};
use crate::Vector3D;

/// A vector with three single-precision components.
///
/// The layout is the one of the reference runtime and is relied on when the
/// value is passed to a graphics API by address (as a vertex attribute, for
/// example): three consecutive singles, `x`, `y`, `z`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[repr(C)]
pub struct Vector3 {
    /// The X component of the vector.
    pub x: f32,
    /// The Y component of the vector.
    pub y: f32,
    /// The Z component of the vector.
    pub z: f32,
}

impl Vector3 {
    /// The vector `(0, 0, 0)`.
    pub const ZERO: Vector3 = Vector3::new(0.0, 0.0, 0.0);
    /// The vector `(1, 1, 1)`.
    pub const ONE: Vector3 = Vector3::new(1.0, 1.0, 1.0);
    /// The vector `(1, 0, 0)`.
    pub const UNIT_X: Vector3 = Vector3::new(1.0, 0.0, 0.0);
    /// The vector `(0, 1, 0)`.
    pub const UNIT_Y: Vector3 = Vector3::new(0.0, 1.0, 0.0);
    /// The vector `(0, 0, 1)`.
    pub const UNIT_Z: Vector3 = Vector3::new(0.0, 0.0, 1.0);

    /// Creates a vector from its components.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Creates a vector whose components all have the same value.
    #[inline]
    pub const fn from_value(value: f32) -> Self {
        Self::new(value, value, value)
    }

    /// Creates a vector from a [`Vector2`] and a Z component.
    #[inline]
    pub const fn from_vector2(value: Vector2, z: f32) -> Self {
        Self::new(value.x, value.y, z)
    }

    /// Returns a vector whose elements are the absolute values of each of the
    /// specified vector's elements.
    #[inline]
    pub fn abs(value: Vector3) -> Vector3 {
        Vector3::new(value.x.abs(), value.y.abs(), value.z.abs())
    }

    /// Restricts a vector between a minimum and a maximum value
    /// (`min(max(value, min), max)`, so `max` wins when the bounds are inverted).
    #[inline]
    pub fn clamp(value1: Vector3, min: Vector3, max: Vector3) -> Vector3 {
        Vector3::min(Vector3::max(value1, min), max)
    }

    /// Computes the cross product of two vectors.
    #[inline]
    pub fn cross(vector1: Vector3, vector2: Vector3) -> Vector3 {
        Vector3::new(
            (vector1.y * vector2.z) - (vector1.z * vector2.y),
            (vector1.z * vector2.x) - (vector1.x * vector2.z),
            (vector1.x * vector2.y) - (vector1.y * vector2.x),
        )
    }

    /// Computes the Euclidean distance between the two given points.
    #[inline]
    pub fn distance(value1: Vector3, value2: Vector3) -> f32 {
        Vector3::distance_squared(value1, value2).sqrt()
    }

    /// Returns the Euclidean distance squared between two specified points.
    #[inline]
    pub fn distance_squared(value1: Vector3, value2: Vector3) -> f32 {
        (value1 - value2).length_squared()
    }

    /// Returns the dot product of two vectors.
    #[inline]
    pub fn dot(vector1: Vector3, vector2: Vector3) -> f32 {
        (vector1.x * vector2.x) + (vector1.y * vector2.y) + (vector1.z * vector2.z)
    }

    /// Returns the length of the vector.
    #[inline]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns the length of the vector squared.
    #[inline]
    pub fn length_squared(&self) -> f32 {
        Vector3::dot(*self, *self)
    }

    /// Performs a linear interpolation between two vectors based on the given weighting.
    #[inline]
    pub fn lerp(value1: Vector3, value2: Vector3, amount: f32) -> Vector3 {
        (value1 * (1.0 - amount)) + (value2 * amount)
    }

    /// Returns a vector whose elements are the maximum of each of the pairs of
    /// elements in two specified vectors.
    #[inline]
    pub fn max(value1: Vector3, value2: Vector3) -> Vector3 {
        Vector3::new(
            single::max(value1.x, value2.x),
            single::max(value1.y, value2.y),
            single::max(value1.z, value2.z),
        )
    }

    /// Returns a vector whose elements are the minimum of each of the pairs of
    /// elements in two specified vectors.
    #[inline]
    pub fn min(value1: Vector3, value2: Vector3) -> Vector3 {
        Vector3::new(
            single::min(value1.x, value2.x),
            single::min(value1.y, value2.y),
            single::min(value1.z, value2.z),
        )
    }

    /// Returns a vector with the same direction as the specified vector, but
    /// with a length of one (NaN components for a zero-length vector).
    #[inline]
    pub fn normalize(value: Vector3) -> Vector3 {
        value / value.length()
    }

    /// Returns a vector whose elements are the square root of each of a
    /// specified vector's elements.
    #[inline]
    pub fn square_root(value: Vector3) -> Vector3 {
        Vector3::new(value.x.sqrt(), value.y.sqrt(), value.z.sqrt())
    }

    /// Transforms a vector by a specified 4x4 matrix.
    #[inline]
    pub fn transform(position: Vector3, matrix: Matrix4x4) -> Vector3 {
        Vector3::new(
            (position.x * matrix.m11) + (position.y * matrix.m21) + (position.z * matrix.m31) + matrix.m41,
            (position.x * matrix.m12) + (position.y * matrix.m22) + (position.z * matrix.m32) + matrix.m42,
            (position.x * matrix.m13) + (position.y * matrix.m23) + (position.z * matrix.m33) + matrix.m43,
        )
    }

    /// Transforms a vector by the specified rotation.
    pub fn transform_quaternion(value: Vector3, rotation: Quaternion) -> Vector3 {
        let x2 = rotation.x + rotation.x;
        let y2 = rotation.y + rotation.y;
        let z2 = rotation.z + rotation.z;

        let wx2 = rotation.w * x2;
        let wy2 = rotation.w * y2;
        let wz2 = rotation.w * z2;
        let xx2 = rotation.x * x2;
        let xy2 = rotation.x * y2;
        let xz2 = rotation.x * z2;
        let yy2 = rotation.y * y2;
        let yz2 = rotation.y * z2;
        let zz2 = rotation.z * z2;

        Vector3::new(
            value.x * (1.0 - yy2 - zz2) + value.y * (xy2 - wz2) + value.z * (xz2 + wy2),
            value.x * (xy2 + wz2) + value.y * (1.0 - xx2 - zz2) + value.z * (yz2 - wx2),
            value.x * (xz2 - wy2) + value.y * (yz2 + wx2) + value.z * (1.0 - xx2 - yy2),
        )
    }

    /// Transforms a vector normal by the given 4x4 matrix.
    #[inline]
    pub fn transform_normal(normal: Vector3, matrix: Matrix4x4) -> Vector3 {
        Vector3::new(
            (normal.x * matrix.m11) + (normal.y * matrix.m21) + (normal.z * matrix.m31),
            (normal.x * matrix.m12) + (normal.y * matrix.m22) + (normal.z * matrix.m32),
            (normal.x * matrix.m13) + (normal.y * matrix.m23) + (normal.z * matrix.m33),
        )
    }
}

/// The narrowing conversion of the double-precision vector.
impl From<Vector3D> for Vector3 {
    #[inline]
    fn from(value: Vector3D) -> Vector3 {
        Vector3::new(value.x as f32, value.y as f32, value.z as f32)
    }
}

/// The widening conversion to the double-precision vector.
impl From<Vector3> for Vector3D {
    #[inline]
    fn from(value: Vector3) -> Vector3D {
        Vector3D::new(value.x as f64, value.y as f64, value.z as f64)
    }
}

impl Add for Vector3 {
    type Output = Vector3;
    #[inline]
    fn add(self, right: Vector3) -> Vector3 {
        Vector3::new(self.x + right.x, self.y + right.y, self.z + right.z)
    }
}

impl Sub for Vector3 {
    type Output = Vector3;
    #[inline]
    fn sub(self, right: Vector3) -> Vector3 {
        Vector3::new(self.x - right.x, self.y - right.y, self.z - right.z)
    }
}

impl Mul for Vector3 {
    type Output = Vector3;
    #[inline]
    fn mul(self, right: Vector3) -> Vector3 {
        Vector3::new(self.x * right.x, self.y * right.y, self.z * right.z)
    }
}

impl Mul<f32> for Vector3 {
    type Output = Vector3;
    #[inline]
    fn mul(self, right: f32) -> Vector3 {
        Vector3::new(self.x * right, self.y * right, self.z * right)
    }
}

impl Mul<Vector3> for f32 {
    type Output = Vector3;
    #[inline]
    fn mul(self, right: Vector3) -> Vector3 {
        right * self
    }
}

impl Div for Vector3 {
    type Output = Vector3;
    #[inline]
    fn div(self, right: Vector3) -> Vector3 {
        Vector3::new(self.x / right.x, self.y / right.y, self.z / right.z)
    }
}

impl Div<f32> for Vector3 {
    type Output = Vector3;
    #[inline]
    fn div(self, right: f32) -> Vector3 {
        Vector3::new(self.x / right, self.y / right, self.z / right)
    }
}

impl Neg for Vector3 {
    type Output = Vector3;
    #[inline]
    fn neg(self) -> Vector3 {
        Vector3::new(-self.x, -self.y, -self.z)
    }
}

impl AddAssign for Vector3 {
    #[inline]
    fn add_assign(&mut self, right: Vector3) {
        *self = *self + right;
    }
}

impl SubAssign for Vector3 {
    #[inline]
    fn sub_assign(&mut self, right: Vector3) {
        *self = *self - right;
    }
}

impl MulAssign for Vector3 {
    #[inline]
    fn mul_assign(&mut self, right: Vector3) {
        *self = *self * right;
    }
}

impl MulAssign<f32> for Vector3 {
    #[inline]
    fn mul_assign(&mut self, right: f32) {
        *self = *self * right;
    }
}

impl DivAssign for Vector3 {
    #[inline]
    fn div_assign(&mut self, right: Vector3) {
        *self = *self / right;
    }
}

impl DivAssign<f32> for Vector3 {
    #[inline]
    fn div_assign(&mut self, right: f32) {
        *self = *self / right;
    }
}

impl fmt::Display for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<{}, {}, {}>",
            InvariantF32(self.x),
            InvariantF32(self.y),
            InvariantF32(self.z)
        )
    }
}
