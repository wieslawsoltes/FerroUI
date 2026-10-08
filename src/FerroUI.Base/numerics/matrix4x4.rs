//! A 4x4 matrix (a three-dimensional transform).

use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

use super::single::InvariantF32;
use super::{Matrix3x2, Quaternion, Vector3};

/// A 4x4 matrix (a three-dimensional transform).
///
/// The layout is the one of the reference runtime and is relied on when the
/// value is passed to a graphics API by address: sixteen consecutive singles
/// in row order (`m11` to `m14`, then the second, third and fourth row).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[repr(C)]
pub struct Matrix4x4 {
    /// The first element of the first row.
    pub m11: f32,
    /// The second element of the first row.
    pub m12: f32,
    /// The third element of the first row.
    pub m13: f32,
    /// The fourth element of the first row.
    pub m14: f32,
    /// The first element of the second row.
    pub m21: f32,
    /// The second element of the second row.
    pub m22: f32,
    /// The third element of the second row.
    pub m23: f32,
    /// The fourth element of the second row.
    pub m24: f32,
    /// The first element of the third row.
    pub m31: f32,
    /// The second element of the third row.
    pub m32: f32,
    /// The third element of the third row.
    pub m33: f32,
    /// The fourth element of the third row.
    pub m34: f32,
    /// The first element of the fourth row.
    pub m41: f32,
    /// The second element of the fourth row.
    pub m42: f32,
    /// The third element of the fourth row.
    pub m43: f32,
    /// The fourth element of the fourth row.
    pub m44: f32,
}

impl Matrix4x4 {
    /// The multiplicative identity matrix.
    pub const IDENTITY: Matrix4x4 = Matrix4x4::new(
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    );

    /// Creates a 4x4 matrix from the specified components, given in row order.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        m11: f32,
        m12: f32,
        m13: f32,
        m14: f32,
        m21: f32,
        m22: f32,
        m23: f32,
        m24: f32,
        m31: f32,
        m32: f32,
        m33: f32,
        m34: f32,
        m41: f32,
        m42: f32,
        m43: f32,
        m44: f32,
    ) -> Self {
        Self {
            m11,
            m12,
            m13,
            m14,
            m21,
            m22,
            m23,
            m24,
            m31,
            m32,
            m33,
            m34,
            m41,
            m42,
            m43,
            m44,
        }
    }

    /// Creates a 4x4 matrix from the specified 3x2 matrix.
    #[inline]
    pub const fn from_matrix3x2(value: Matrix3x2) -> Self {
        Self::new(
            value.m11, value.m12, 0.0, 0.0, value.m21, value.m22, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, value.m31,
            value.m32, 0.0, 1.0,
        )
    }

    /// Indicates whether the current matrix is the identity matrix.
    #[inline]
    pub fn is_identity(&self) -> bool {
        *self == Matrix4x4::IDENTITY
    }

    /// Gets the translation component of this matrix.
    #[inline]
    pub fn translation(&self) -> Vector3 {
        Vector3::new(self.m41, self.m42, self.m43)
    }

    /// Sets the translation component of this matrix.
    #[inline]
    pub fn set_translation(&mut self, value: Vector3) {
        self.m41 = value.x;
        self.m42 = value.y;
        self.m43 = value.z;
    }

    /// Creates a matrix that rotates around an arbitrary (unit) vector.
    ///
    /// As in the reference runtime, the matrix is built from the quaternion
    /// of the rotation.
    pub fn create_from_axis_angle(axis: Vector3, angle: f32) -> Matrix4x4 {
        let q = Quaternion::create_from_axis_angle(axis, angle);
        Matrix4x4::create_from_quaternion(q)
    }

    /// Creates a rotation matrix from the specified quaternion rotation value.
    pub fn create_from_quaternion(quaternion: Quaternion) -> Matrix4x4 {
        let mut result = Matrix4x4::IDENTITY;

        let xx = quaternion.x * quaternion.x;
        let yy = quaternion.y * quaternion.y;
        let zz = quaternion.z * quaternion.z;

        let xy = quaternion.x * quaternion.y;
        let wz = quaternion.z * quaternion.w;
        let xz = quaternion.z * quaternion.x;
        let wy = quaternion.y * quaternion.w;
        let yz = quaternion.y * quaternion.z;
        let wx = quaternion.x * quaternion.w;

        result.m11 = 1.0 - 2.0 * (yy + zz);
        result.m12 = 2.0 * (xy + wz);
        result.m13 = 2.0 * (xz - wy);

        result.m21 = 2.0 * (xy - wz);
        result.m22 = 1.0 - 2.0 * (zz + xx);
        result.m23 = 2.0 * (yz + wx);

        result.m31 = 2.0 * (xz + wy);
        result.m32 = 2.0 * (yz - wx);
        result.m33 = 1.0 - 2.0 * (yy + xx);

        result
    }

    /// Creates a rotation matrix from the specified yaw, pitch, and roll.
    #[inline]
    pub fn create_from_yaw_pitch_roll(yaw: f32, pitch: f32, roll: f32) -> Matrix4x4 {
        let q = Quaternion::create_from_yaw_pitch_roll(yaw, pitch, roll);
        Matrix4x4::create_from_quaternion(q)
    }

    /// Creates a view matrix: a camera at `camera_position` that looks at
    /// `camera_target`, with `camera_up_vector` as the direction that is up
    /// from the camera's point of view (right-handed).
    ///
    /// As in the reference runtime, nothing is validated: a target at the
    /// position of the camera, or an up vector parallel to the direction of
    /// view, gives NaN components.
    pub fn create_look_at(camera_position: Vector3, camera_target: Vector3, camera_up_vector: Vector3) -> Matrix4x4 {
        let zaxis = Vector3::normalize(camera_position - camera_target);
        let xaxis = Vector3::normalize(Vector3::cross(camera_up_vector, zaxis));
        let yaxis = Vector3::cross(zaxis, xaxis);

        // [  xaxis.x  yaxis.x  zaxis.x  0 ]
        // [  xaxis.y  yaxis.y  zaxis.y  0 ]
        // [  xaxis.z  yaxis.z  zaxis.z  0 ]
        // [ -xaxis.p -yaxis.p -zaxis.p  1 ]   (`.p`: the dot product with the position)
        Matrix4x4::new(
            xaxis.x,
            yaxis.x,
            zaxis.x,
            0.0,
            xaxis.y,
            yaxis.y,
            zaxis.y,
            0.0,
            xaxis.z,
            yaxis.z,
            zaxis.z,
            0.0,
            -Vector3::dot(xaxis, camera_position),
            -Vector3::dot(yaxis, camera_position),
            -Vector3::dot(zaxis, camera_position),
            1.0,
        )
    }

    /// Creates a perspective projection matrix based on a field of view,
    /// aspect ratio, and near and far view plane distances (right-handed,
    /// with a depth range of zero to one).
    ///
    /// `field_of_view` is the field of view in the y direction, in radians.
    /// A far plane at positive infinity is accepted and gives the limit of
    /// the matrix (`m33` is -1 and `m43` is `-near_plane_distance`).
    ///
    /// # Panics
    /// Panics where the reference runtime throws an
    /// `ArgumentOutOfRangeException`: when `field_of_view` is less than or
    /// equal to zero or greater than or equal to pi, when
    /// `near_plane_distance` or `far_plane_distance` is less than or equal to
    /// zero, and when `near_plane_distance` is greater than or equal to
    /// `far_plane_distance`.
    pub fn create_perspective_field_of_view(
        field_of_view: f32,
        aspect_ratio: f32,
        near_plane_distance: f32,
        far_plane_distance: f32,
    ) -> Matrix4x4 {
        // The comparisons are the ones of the reference runtime: a NaN
        // argument passes every one of them.
        if field_of_view <= 0.0 {
            panic!("field_of_view ('{field_of_view}') must be greater than '0'.");
        }
        if field_of_view >= std::f32::consts::PI {
            panic!("field_of_view ('{field_of_view}') must be less than '{}'.", std::f32::consts::PI);
        }
        if near_plane_distance <= 0.0 {
            panic!("near_plane_distance ('{near_plane_distance}') must be greater than '0'.");
        }
        if far_plane_distance <= 0.0 {
            panic!("far_plane_distance ('{far_plane_distance}') must be greater than '0'.");
        }
        if near_plane_distance >= far_plane_distance {
            panic!("near_plane_distance ('{near_plane_distance}') must be less than '{far_plane_distance}'.");
        }

        let height = 1.0 / (field_of_view * 0.5).tan();
        let width = height / aspect_ratio;
        let range = if far_plane_distance == f32::INFINITY {
            -1.0
        } else {
            far_plane_distance / (near_plane_distance - far_plane_distance)
        };

        // [  w  0  0   0 ]
        // [  0  h  0   0 ]
        // [  0  0  r  -1 ]
        // [  0  0  rn  0 ]   (`rn`: the range times the near plane distance)
        Matrix4x4::new(
            width,
            0.0,
            0.0,
            0.0,
            0.0,
            height,
            0.0,
            0.0,
            0.0,
            0.0,
            range,
            -1.0,
            0.0,
            0.0,
            range * near_plane_distance,
            0.0,
        )
    }

    /// Creates a matrix for rotating points around the X axis.
    pub fn create_rotation_x(radians: f32) -> Matrix4x4 {
        let c = radians.cos();
        let s = radians.sin();

        // [  1  0  0  0 ]
        // [  0  c  s  0 ]
        // [  0 -s  c  0 ]
        // [  0  0  0  1 ]
        let mut result = Matrix4x4::IDENTITY;
        result.m22 = c;
        result.m23 = s;
        result.m32 = -s;
        result.m33 = c;
        result
    }

    /// Creates a matrix for rotating points around the X axis from a center point.
    pub fn create_rotation_x_at(radians: f32, center_point: Vector3) -> Matrix4x4 {
        let c = radians.cos();
        let s = radians.sin();

        let y = center_point.y * (1.0 - c) + center_point.z * s;
        let z = center_point.z * (1.0 - c) - center_point.y * s;

        // [  1  0  0  0 ]
        // [  0  c  s  0 ]
        // [  0 -s  c  0 ]
        // [  0  y  z  1 ]
        let mut result = Matrix4x4::IDENTITY;
        result.m22 = c;
        result.m23 = s;
        result.m32 = -s;
        result.m33 = c;
        result.m42 = y;
        result.m43 = z;
        result
    }

    /// Creates a matrix for rotating points around the Y axis.
    pub fn create_rotation_y(radians: f32) -> Matrix4x4 {
        let c = radians.cos();
        let s = radians.sin();

        // [  c  0 -s  0 ]
        // [  0  1  0  0 ]
        // [  s  0  c  0 ]
        // [  0  0  0  1 ]
        let mut result = Matrix4x4::IDENTITY;
        result.m11 = c;
        result.m13 = -s;
        result.m31 = s;
        result.m33 = c;
        result
    }

    /// Creates a matrix for rotating points around the Y axis from a center point.
    pub fn create_rotation_y_at(radians: f32, center_point: Vector3) -> Matrix4x4 {
        let c = radians.cos();
        let s = radians.sin();

        let x = center_point.x * (1.0 - c) - center_point.z * s;
        let z = center_point.z * (1.0 - c) + center_point.x * s;

        // [  c  0 -s  0 ]
        // [  0  1  0  0 ]
        // [  s  0  c  0 ]
        // [  x  0  z  1 ]
        let mut result = Matrix4x4::IDENTITY;
        result.m11 = c;
        result.m13 = -s;
        result.m31 = s;
        result.m33 = c;
        result.m41 = x;
        result.m43 = z;
        result
    }

    /// Creates a matrix for rotating points around the Z axis.
    pub fn create_rotation_z(radians: f32) -> Matrix4x4 {
        let c = radians.cos();
        let s = radians.sin();

        // [  c  s  0  0 ]
        // [ -s  c  0  0 ]
        // [  0  0  1  0 ]
        // [  0  0  0  1 ]
        let mut result = Matrix4x4::IDENTITY;
        result.m11 = c;
        result.m12 = s;
        result.m21 = -s;
        result.m22 = c;
        result
    }

    /// Creates a matrix for rotating points around the Z axis from a center point.
    pub fn create_rotation_z_at(radians: f32, center_point: Vector3) -> Matrix4x4 {
        let c = radians.cos();
        let s = radians.sin();

        let x = center_point.x * (1.0 - c) + center_point.y * s;
        let y = center_point.y * (1.0 - c) - center_point.x * s;

        // [  c  s  0  0 ]
        // [ -s  c  0  0 ]
        // [  0  0  1  0 ]
        // [  x  y  0  1 ]
        let mut result = Matrix4x4::IDENTITY;
        result.m11 = c;
        result.m12 = s;
        result.m21 = -s;
        result.m22 = c;
        result.m41 = x;
        result.m42 = y;
        result
    }

    /// Creates a scaling matrix from the specified vector scale.
    #[inline]
    pub fn create_scale(scales: Vector3) -> Matrix4x4 {
        Matrix4x4::create_scale_xyz(scales.x, scales.y, scales.z)
    }

    /// Creates a scaling matrix from the specified X, Y, and Z components.
    #[inline]
    pub fn create_scale_xyz(x_scale: f32, y_scale: f32, z_scale: f32) -> Matrix4x4 {
        let mut result = Matrix4x4::IDENTITY;
        result.m11 = x_scale;
        result.m22 = y_scale;
        result.m33 = z_scale;
        result
    }

    /// Creates a uniform scaling matrix that scales equally on each axis.
    #[inline]
    pub fn create_scale_uniform(scale: f32) -> Matrix4x4 {
        Matrix4x4::create_scale_xyz(scale, scale, scale)
    }

    /// Creates a scaling matrix with a center point.
    pub fn create_scale_at(scales: Vector3, center_point: Vector3) -> Matrix4x4 {
        let tx = center_point.x * (1.0 - scales.x);
        let ty = center_point.y * (1.0 - scales.y);
        let tz = center_point.z * (1.0 - scales.z);

        let mut result = Matrix4x4::IDENTITY;
        result.m11 = scales.x;
        result.m22 = scales.y;
        result.m33 = scales.z;
        result.m41 = tx;
        result.m42 = ty;
        result.m43 = tz;
        result
    }

    /// Creates a translation matrix from the specified 3-dimensional vector.
    #[inline]
    pub fn create_translation(position: Vector3) -> Matrix4x4 {
        Matrix4x4::create_translation_xyz(position.x, position.y, position.z)
    }

    /// Creates a translation matrix from the specified X, Y, and Z components.
    #[inline]
    pub fn create_translation_xyz(x_position: f32, y_position: f32, z_position: f32) -> Matrix4x4 {
        let mut result = Matrix4x4::IDENTITY;
        result.m41 = x_position;
        result.m42 = y_position;
        result.m43 = z_position;
        result
    }

    /// Calculates the determinant of the current 4x4 matrix.
    pub fn get_determinant(&self) -> f32 {
        // | a b c d |     | f g h |     | e g h |     | e f h |     | e f g |
        // | e f g h | = a | j k l | - b | i k l | + c | i j l | - d | i j k |
        // | i j k l |     | n o p |     | m o p |     | m n p |     | m n o |
        // | m n o p |
        let a = self.m11;
        let b = self.m12;
        let c = self.m13;
        let d = self.m14;
        let e = self.m21;
        let f = self.m22;
        let g = self.m23;
        let h = self.m24;
        let i = self.m31;
        let j = self.m32;
        let k = self.m33;
        let l = self.m34;
        let m = self.m41;
        let n = self.m42;
        let o = self.m43;
        let p = self.m44;

        let kp_lo = k * p - l * o;
        let jp_ln = j * p - l * n;
        let jo_kn = j * o - k * n;
        let ip_lm = i * p - l * m;
        let io_km = i * o - k * m;
        let in_jm = i * n - j * m;

        a * (f * kp_lo - g * jp_ln + h * jo_kn) - b * (e * kp_lo - g * ip_lm + h * io_km)
            + c * (e * jp_ln - f * ip_lm + h * in_jm)
            - d * (e * jo_kn - f * io_km + g * in_jm)
    }

    /// Tries to invert the specified matrix. Returns `None` when the matrix is
    /// singular; see [`Matrix4x4::invert_or_nan`] for the value the reference
    /// runtime stores in that case.
    #[allow(clippy::field_reassign_with_default)]
    pub fn invert(matrix: Matrix4x4) -> Option<Matrix4x4> {
        let a = matrix.m11;
        let b = matrix.m12;
        let c = matrix.m13;
        let d = matrix.m14;
        let e = matrix.m21;
        let f = matrix.m22;
        let g = matrix.m23;
        let h = matrix.m24;
        let i = matrix.m31;
        let j = matrix.m32;
        let k = matrix.m33;
        let l = matrix.m34;
        let m = matrix.m41;
        let n = matrix.m42;
        let o = matrix.m43;
        let p = matrix.m44;

        let kp_lo = k * p - l * o;
        let jp_ln = j * p - l * n;
        let jo_kn = j * o - k * n;
        let ip_lm = i * p - l * m;
        let io_km = i * o - k * m;
        let in_jm = i * n - j * m;

        let a11 = f * kp_lo - g * jp_ln + h * jo_kn;
        let a12 = -(e * kp_lo - g * ip_lm + h * io_km);
        let a13 = e * jp_ln - f * ip_lm + h * in_jm;
        let a14 = -(e * jo_kn - f * io_km + g * in_jm);

        let det = a * a11 + b * a12 + c * a13 + d * a14;

        // The reference runtime compares against the smallest positive
        // (subnormal) single.
        if det.abs() < f32::from_bits(1) {
            return None;
        }

        let inv_det = 1.0 / det;

        let mut result = Matrix4x4::default();

        result.m11 = a11 * inv_det;
        result.m21 = a12 * inv_det;
        result.m31 = a13 * inv_det;
        result.m41 = a14 * inv_det;

        result.m12 = -(b * kp_lo - c * jp_ln + d * jo_kn) * inv_det;
        result.m22 = (a * kp_lo - c * ip_lm + d * io_km) * inv_det;
        result.m32 = -(a * jp_ln - b * ip_lm + d * in_jm) * inv_det;
        result.m42 = (a * jo_kn - b * io_km + c * in_jm) * inv_det;

        let gp_ho = g * p - h * o;
        let fp_hn = f * p - h * n;
        let fo_gn = f * o - g * n;
        let ep_hm = e * p - h * m;
        let eo_gm = e * o - g * m;
        let en_fm = e * n - f * m;

        result.m13 = (b * gp_ho - c * fp_hn + d * fo_gn) * inv_det;
        result.m23 = -(a * gp_ho - c * ep_hm + d * eo_gm) * inv_det;
        result.m33 = (a * fp_hn - b * ep_hm + d * en_fm) * inv_det;
        result.m43 = -(a * fo_gn - b * eo_gm + c * en_fm) * inv_det;

        let gl_hk = g * l - h * k;
        let fl_hj = f * l - h * j;
        let fk_gj = f * k - g * j;
        let el_hi = e * l - h * i;
        let ek_gi = e * k - g * i;
        let ej_fi = e * j - f * i;

        result.m14 = -(b * gl_hk - c * fl_hj + d * fk_gj) * inv_det;
        result.m24 = (a * gl_hk - c * el_hi + d * ek_gi) * inv_det;
        result.m34 = -(a * fl_hj - b * el_hi + d * ej_fi) * inv_det;
        result.m44 = (a * fk_gj - b * ek_gi + c * ej_fi) * inv_det;

        Some(result)
    }

    /// Inverts the specified matrix; a singular matrix gives a matrix whose
    /// components are all NaN (the `out` value of the reference runtime).
    pub fn invert_or_nan(matrix: Matrix4x4) -> Matrix4x4 {
        const NAN: f32 = f32::NAN;
        Matrix4x4::invert(matrix).unwrap_or(Matrix4x4::new(
            NAN, NAN, NAN, NAN, NAN, NAN, NAN, NAN, NAN, NAN, NAN, NAN, NAN, NAN, NAN, NAN,
        ))
    }

    /// Performs a linear interpolation from one matrix to a second matrix
    /// based on a value that specifies the weighting of the second matrix.
    pub fn lerp(matrix1: Matrix4x4, matrix2: Matrix4x4, amount: f32) -> Matrix4x4 {
        Matrix4x4::new(
            matrix1.m11 + (matrix2.m11 - matrix1.m11) * amount,
            matrix1.m12 + (matrix2.m12 - matrix1.m12) * amount,
            matrix1.m13 + (matrix2.m13 - matrix1.m13) * amount,
            matrix1.m14 + (matrix2.m14 - matrix1.m14) * amount,
            matrix1.m21 + (matrix2.m21 - matrix1.m21) * amount,
            matrix1.m22 + (matrix2.m22 - matrix1.m22) * amount,
            matrix1.m23 + (matrix2.m23 - matrix1.m23) * amount,
            matrix1.m24 + (matrix2.m24 - matrix1.m24) * amount,
            matrix1.m31 + (matrix2.m31 - matrix1.m31) * amount,
            matrix1.m32 + (matrix2.m32 - matrix1.m32) * amount,
            matrix1.m33 + (matrix2.m33 - matrix1.m33) * amount,
            matrix1.m34 + (matrix2.m34 - matrix1.m34) * amount,
            matrix1.m41 + (matrix2.m41 - matrix1.m41) * amount,
            matrix1.m42 + (matrix2.m42 - matrix1.m42) * amount,
            matrix1.m43 + (matrix2.m43 - matrix1.m43) * amount,
            matrix1.m44 + (matrix2.m44 - matrix1.m44) * amount,
        )
    }

    /// Transposes the rows and columns of a matrix.
    pub fn transpose(matrix: Matrix4x4) -> Matrix4x4 {
        Matrix4x4::new(
            matrix.m11, matrix.m21, matrix.m31, matrix.m41, matrix.m12, matrix.m22, matrix.m32, matrix.m42,
            matrix.m13, matrix.m23, matrix.m33, matrix.m43, matrix.m14, matrix.m24, matrix.m34, matrix.m44,
        )
    }

    fn to_array(self) -> [f32; 16] {
        [
            self.m11, self.m12, self.m13, self.m14, self.m21, self.m22, self.m23, self.m24, self.m31, self.m32,
            self.m33, self.m34, self.m41, self.m42, self.m43, self.m44,
        ]
    }

    fn from_array(v: [f32; 16]) -> Matrix4x4 {
        Matrix4x4::new(
            v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8], v[9], v[10], v[11], v[12], v[13], v[14],
            v[15],
        )
    }
}

impl Add for Matrix4x4 {
    type Output = Matrix4x4;
    fn add(self, value2: Matrix4x4) -> Matrix4x4 {
        let (a, b) = (self.to_array(), value2.to_array());
        Matrix4x4::from_array(std::array::from_fn(|i| a[i] + b[i]))
    }
}

impl Sub for Matrix4x4 {
    type Output = Matrix4x4;
    fn sub(self, value2: Matrix4x4) -> Matrix4x4 {
        let (a, b) = (self.to_array(), value2.to_array());
        Matrix4x4::from_array(std::array::from_fn(|i| a[i] - b[i]))
    }
}

impl Neg for Matrix4x4 {
    type Output = Matrix4x4;
    fn neg(self) -> Matrix4x4 {
        let a = self.to_array();
        Matrix4x4::from_array(std::array::from_fn(|i| -a[i]))
    }
}

impl Mul for Matrix4x4 {
    type Output = Matrix4x4;
    fn mul(self, value2: Matrix4x4) -> Matrix4x4 {
        let value1 = self;
        Matrix4x4::new(
            // First row
            value1.m11 * value2.m11 + value1.m12 * value2.m21 + value1.m13 * value2.m31 + value1.m14 * value2.m41,
            value1.m11 * value2.m12 + value1.m12 * value2.m22 + value1.m13 * value2.m32 + value1.m14 * value2.m42,
            value1.m11 * value2.m13 + value1.m12 * value2.m23 + value1.m13 * value2.m33 + value1.m14 * value2.m43,
            value1.m11 * value2.m14 + value1.m12 * value2.m24 + value1.m13 * value2.m34 + value1.m14 * value2.m44,
            // Second row
            value1.m21 * value2.m11 + value1.m22 * value2.m21 + value1.m23 * value2.m31 + value1.m24 * value2.m41,
            value1.m21 * value2.m12 + value1.m22 * value2.m22 + value1.m23 * value2.m32 + value1.m24 * value2.m42,
            value1.m21 * value2.m13 + value1.m22 * value2.m23 + value1.m23 * value2.m33 + value1.m24 * value2.m43,
            value1.m21 * value2.m14 + value1.m22 * value2.m24 + value1.m23 * value2.m34 + value1.m24 * value2.m44,
            // Third row
            value1.m31 * value2.m11 + value1.m32 * value2.m21 + value1.m33 * value2.m31 + value1.m34 * value2.m41,
            value1.m31 * value2.m12 + value1.m32 * value2.m22 + value1.m33 * value2.m32 + value1.m34 * value2.m42,
            value1.m31 * value2.m13 + value1.m32 * value2.m23 + value1.m33 * value2.m33 + value1.m34 * value2.m43,
            value1.m31 * value2.m14 + value1.m32 * value2.m24 + value1.m33 * value2.m34 + value1.m34 * value2.m44,
            // Fourth row
            value1.m41 * value2.m11 + value1.m42 * value2.m21 + value1.m43 * value2.m31 + value1.m44 * value2.m41,
            value1.m41 * value2.m12 + value1.m42 * value2.m22 + value1.m43 * value2.m32 + value1.m44 * value2.m42,
            value1.m41 * value2.m13 + value1.m42 * value2.m23 + value1.m43 * value2.m33 + value1.m44 * value2.m43,
            value1.m41 * value2.m14 + value1.m42 * value2.m24 + value1.m43 * value2.m34 + value1.m44 * value2.m44,
        )
    }
}

/// Multiplies every component by a scalar.
impl Mul<f32> for Matrix4x4 {
    type Output = Matrix4x4;
    fn mul(self, value2: f32) -> Matrix4x4 {
        let a = self.to_array();
        Matrix4x4::from_array(std::array::from_fn(|i| a[i] * value2))
    }
}

impl fmt::Display for Matrix4x4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ {{M11:{} M12:{} M13:{} M14:{}}} {{M21:{} M22:{} M23:{} M24:{}}} {{M31:{} M32:{} M33:{} M34:{}}} {{M41:{} M42:{} M43:{} M44:{}}} }}",
            InvariantF32(self.m11),
            InvariantF32(self.m12),
            InvariantF32(self.m13),
            InvariantF32(self.m14),
            InvariantF32(self.m21),
            InvariantF32(self.m22),
            InvariantF32(self.m23),
            InvariantF32(self.m24),
            InvariantF32(self.m31),
            InvariantF32(self.m32),
            InvariantF32(self.m33),
            InvariantF32(self.m34),
            InvariantF32(self.m41),
            InvariantF32(self.m42),
            InvariantF32(self.m43),
            InvariantF32(self.m44)
        )
    }
}
