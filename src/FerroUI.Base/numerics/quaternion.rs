//! A vector that is used to encode three-dimensional physical rotations.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

use super::single::InvariantF32;
use super::{Matrix4x4, Vector3};

/// The dot product above which a spherical interpolation falls back to a
/// linear one.
const SLERP_EPSILON: f32 = 1e-6;

/// A vector that is used to encode three-dimensional physical rotations.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Quaternion {
    /// The X value of the vector component of the quaternion.
    pub x: f32,
    /// The Y value of the vector component of the quaternion.
    pub y: f32,
    /// The Z value of the vector component of the quaternion.
    pub z: f32,
    /// The rotation component of the quaternion.
    pub w: f32,
}

impl Quaternion {
    /// A quaternion that represents no rotation.
    pub const IDENTITY: Quaternion = Quaternion::new(0.0, 0.0, 0.0, 1.0);

    /// A quaternion whose components are all zero.
    pub const ZERO: Quaternion = Quaternion::new(0.0, 0.0, 0.0, 0.0);

    /// Constructs a quaternion from the specified components.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Creates a quaternion from the specified vector and rotation parts.
    #[inline]
    pub const fn from_vector3(vector_part: Vector3, scalar_part: f32) -> Self {
        Self::new(vector_part.x, vector_part.y, vector_part.z, scalar_part)
    }

    /// Gets a value that indicates whether the current instance is the identity quaternion.
    #[inline]
    pub fn is_identity(&self) -> bool {
        *self == Quaternion::IDENTITY
    }

    /// Concatenates two quaternions: the result represents the `value1`
    /// rotation followed by the `value2` rotation.
    pub fn concatenate(value1: Quaternion, value2: Quaternion) -> Quaternion {
        // Concatenate rotation is actually q2 * q1 instead of q1 * q2.
        // So that's why value2 goes q1 and value1 goes q2.
        value2 * value1
    }

    /// Returns the conjugate of a specified quaternion.
    #[inline]
    pub fn conjugate(value: Quaternion) -> Quaternion {
        Quaternion::new(-value.x, -value.y, -value.z, value.w)
    }

    /// Creates a quaternion from a unit vector and an angle to rotate around the vector.
    pub fn create_from_axis_angle(axis: Vector3, angle: f32) -> Quaternion {
        let half_angle = angle * 0.5;
        let s = half_angle.sin();
        let c = half_angle.cos();

        Quaternion::new(axis.x * s, axis.y * s, axis.z * s, c)
    }

    /// Creates a quaternion from the specified rotation matrix.
    pub fn create_from_rotation_matrix(matrix: Matrix4x4) -> Quaternion {
        let trace = matrix.m11 + matrix.m22 + matrix.m33;

        let mut q = Quaternion::default();

        if trace > 0.0 {
            let mut s = (trace + 1.0).sqrt();
            q.w = s * 0.5;
            s = 0.5 / s;
            q.x = (matrix.m23 - matrix.m32) * s;
            q.y = (matrix.m31 - matrix.m13) * s;
            q.z = (matrix.m12 - matrix.m21) * s;
        } else if matrix.m11 >= matrix.m22 && matrix.m11 >= matrix.m33 {
            let s = (1.0 + matrix.m11 - matrix.m22 - matrix.m33).sqrt();
            let inv_s = 0.5 / s;
            q.x = 0.5 * s;
            q.y = (matrix.m12 + matrix.m21) * inv_s;
            q.z = (matrix.m13 + matrix.m31) * inv_s;
            q.w = (matrix.m23 - matrix.m32) * inv_s;
        } else if matrix.m22 > matrix.m33 {
            let s = (1.0 + matrix.m22 - matrix.m11 - matrix.m33).sqrt();
            let inv_s = 0.5 / s;
            q.x = (matrix.m21 + matrix.m12) * inv_s;
            q.y = 0.5 * s;
            q.z = (matrix.m32 + matrix.m23) * inv_s;
            q.w = (matrix.m31 - matrix.m13) * inv_s;
        } else {
            let s = (1.0 + matrix.m33 - matrix.m11 - matrix.m22).sqrt();
            let inv_s = 0.5 / s;
            q.x = (matrix.m31 + matrix.m13) * inv_s;
            q.y = (matrix.m32 + matrix.m23) * inv_s;
            q.z = 0.5 * s;
            q.w = (matrix.m12 - matrix.m21) * inv_s;
        }

        q
    }

    /// Creates a new quaternion from the given yaw, pitch, and roll.
    pub fn create_from_yaw_pitch_roll(yaw: f32, pitch: f32, roll: f32) -> Quaternion {
        // Roll first, about axis the object is facing, then
        // pitch upward, then yaw to face into the new heading
        let half_roll = roll * 0.5;
        let sr = half_roll.sin();
        let cr = half_roll.cos();

        let half_pitch = pitch * 0.5;
        let sp = half_pitch.sin();
        let cp = half_pitch.cos();

        let half_yaw = yaw * 0.5;
        let sy = half_yaw.sin();
        let cy = half_yaw.cos();

        Quaternion::new(
            cy * sp * cr + sy * cp * sr,
            sy * cp * cr - cy * sp * sr,
            cy * cp * sr - sy * sp * cr,
            cy * cp * cr + sy * sp * sr,
        )
    }

    /// Calculates the dot product of two quaternions.
    #[inline]
    pub fn dot(quaternion1: Quaternion, quaternion2: Quaternion) -> f32 {
        (quaternion1.x * quaternion2.x)
            + (quaternion1.y * quaternion2.y)
            + (quaternion1.z * quaternion2.z)
            + (quaternion1.w * quaternion2.w)
    }

    /// Returns the inverse of a quaternion.
    pub fn inverse(value: Quaternion) -> Quaternion {
        //  -1   (       a              -v       )
        // q   = ( -------------   ------------- )
        //       (  a^2 + |v|^2  ,  a^2 + |v|^2  )
        let inv_norm = 1.0 / value.length_squared();
        Quaternion::new(
            -value.x * inv_norm,
            -value.y * inv_norm,
            -value.z * inv_norm,
            value.w * inv_norm,
        )
    }

    /// Calculates the length of the quaternion.
    #[inline]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Calculates the squared length of the quaternion.
    #[inline]
    pub fn length_squared(&self) -> f32 {
        Quaternion::dot(*self, *self)
    }

    /// Performs a linear interpolation between two quaternions based on a
    /// value that specifies the weighting of the second quaternion. The
    /// result is normalized.
    pub fn lerp(quaternion1: Quaternion, quaternion2: Quaternion, amount: f32) -> Quaternion {
        let t = amount;
        let t1 = 1.0 - t;

        let dot = Quaternion::dot(quaternion1, quaternion2);

        let r = if dot >= 0.0 {
            Quaternion::new(
                t1 * quaternion1.x + t * quaternion2.x,
                t1 * quaternion1.y + t * quaternion2.y,
                t1 * quaternion1.z + t * quaternion2.z,
                t1 * quaternion1.w + t * quaternion2.w,
            )
        } else {
            Quaternion::new(
                t1 * quaternion1.x - t * quaternion2.x,
                t1 * quaternion1.y - t * quaternion2.y,
                t1 * quaternion1.z - t * quaternion2.z,
                t1 * quaternion1.w - t * quaternion2.w,
            )
        };

        Quaternion::normalize(r)
    }

    /// Divides each component of a specified quaternion by its length.
    pub fn normalize(value: Quaternion) -> Quaternion {
        let length = value.length();
        Quaternion::new(
            value.x / length,
            value.y / length,
            value.z / length,
            value.w / length,
        )
    }

    /// Interpolates between two quaternions, using spherical linear interpolation.
    pub fn slerp(quaternion1: Quaternion, quaternion2: Quaternion, amount: f32) -> Quaternion {
        let t = amount;

        let mut cos_omega = Quaternion::dot(quaternion1, quaternion2);

        let mut flip = false;

        if cos_omega < 0.0 {
            flip = true;
            cos_omega = -cos_omega;
        }

        let s1;
        let s2;

        if cos_omega > (1.0 - SLERP_EPSILON) {
            // Too close, do straight linear interpolation.
            s1 = 1.0 - t;
            s2 = if flip { -t } else { t };
        } else {
            let omega = cos_omega.acos();
            let inv_sin_omega = 1.0 / omega.sin();

            s1 = ((1.0 - t) * omega).sin() * inv_sin_omega;
            s2 = if flip {
                -(t * omega).sin() * inv_sin_omega
            } else {
                (t * omega).sin() * inv_sin_omega
            };
        }

        Quaternion::new(
            s1 * quaternion1.x + s2 * quaternion2.x,
            s1 * quaternion1.y + s2 * quaternion2.y,
            s1 * quaternion1.z + s2 * quaternion2.z,
            s1 * quaternion1.w + s2 * quaternion2.w,
        )
    }
}

impl Add for Quaternion {
    type Output = Quaternion;
    #[inline]
    fn add(self, value2: Quaternion) -> Quaternion {
        Quaternion::new(
            self.x + value2.x,
            self.y + value2.y,
            self.z + value2.z,
            self.w + value2.w,
        )
    }
}

impl Sub for Quaternion {
    type Output = Quaternion;
    #[inline]
    fn sub(self, value2: Quaternion) -> Quaternion {
        Quaternion::new(
            self.x - value2.x,
            self.y - value2.y,
            self.z - value2.z,
            self.w - value2.w,
        )
    }
}

impl Neg for Quaternion {
    type Output = Quaternion;
    #[inline]
    fn neg(self) -> Quaternion {
        Quaternion::new(-self.x, -self.y, -self.z, -self.w)
    }
}

/// The quaternion product.
impl Mul for Quaternion {
    type Output = Quaternion;
    fn mul(self, value2: Quaternion) -> Quaternion {
        let q1x = self.x;
        let q1y = self.y;
        let q1z = self.z;
        let q1w = self.w;

        let q2x = value2.x;
        let q2y = value2.y;
        let q2z = value2.z;
        let q2w = value2.w;

        // cross(av, bv)
        let cx = q1y * q2z - q1z * q2y;
        let cy = q1z * q2x - q1x * q2z;
        let cz = q1x * q2y - q1y * q2x;

        let dot = q1x * q2x + q1y * q2y + q1z * q2z;

        Quaternion::new(
            q1x * q2w + q2x * q1w + cx,
            q1y * q2w + q2y * q1w + cy,
            q1z * q2w + q2z * q1w + cz,
            q1w * q2w - dot,
        )
    }
}

/// Multiplies every component by a scalar.
impl Mul<f32> for Quaternion {
    type Output = Quaternion;
    #[inline]
    fn mul(self, value2: f32) -> Quaternion {
        Quaternion::new(self.x * value2, self.y * value2, self.z * value2, self.w * value2)
    }
}

/// Multiplies by the inverse of the right-hand quaternion.
impl Div for Quaternion {
    type Output = Quaternion;
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, value2: Quaternion) -> Quaternion {
        self * Quaternion::inverse(value2)
    }
}

impl fmt::Display for Quaternion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{X:{} Y:{} Z:{} W:{}}}",
            InvariantF32(self.x),
            InvariantF32(self.y),
            InvariantF32(self.z),
            InvariantF32(self.w)
        )
    }
}
