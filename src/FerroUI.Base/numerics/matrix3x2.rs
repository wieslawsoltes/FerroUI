//! A 3x2 matrix (a two-dimensional affine transform).

use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

use super::single::{ieee_remainder, InvariantF32};
use super::Vector2;

/// Rotations within this distance (0.1% of a degree) of a multiple of a
/// quarter turn use exact sine and cosine values.
const ROTATION_EPSILON: f32 = 0.001 * std::f32::consts::PI / 180.0;

/// A 3x2 matrix (a two-dimensional affine transform).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Matrix3x2 {
    /// The first element of the first row.
    pub m11: f32,
    /// The second element of the first row.
    pub m12: f32,
    /// The first element of the second row.
    pub m21: f32,
    /// The second element of the second row.
    pub m22: f32,
    /// The first element of the third row.
    pub m31: f32,
    /// The second element of the third row.
    pub m32: f32,
}

impl Matrix3x2 {
    /// The multiplicative identity matrix.
    pub const IDENTITY: Matrix3x2 = Matrix3x2::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    /// Creates a 3x2 matrix from the specified components.
    #[inline]
    pub const fn new(m11: f32, m12: f32, m21: f32, m22: f32, m31: f32, m32: f32) -> Self {
        Self {
            m11,
            m12,
            m21,
            m22,
            m31,
            m32,
        }
    }

    /// Gets a value that indicates whether the current matrix is the identity matrix.
    #[inline]
    pub fn is_identity(&self) -> bool {
        *self == Matrix3x2::IDENTITY
    }

    /// Gets the translation component of this matrix.
    #[inline]
    pub fn translation(&self) -> Vector2 {
        Vector2::new(self.m31, self.m32)
    }

    /// Sets the translation component of this matrix.
    #[inline]
    pub fn set_translation(&mut self, value: Vector2) {
        self.m31 = value.x;
        self.m32 = value.y;
    }

    /// Creates a rotation matrix using the given rotation in radians.
    pub fn create_rotation(radians: f32) -> Matrix3x2 {
        let (s, c) = rotation_sin_cos(radians);

        // [  c  s ]
        // [ -s  c ]
        // [  0  0 ]
        Matrix3x2::new(c, s, -s, c, 0.0, 0.0)
    }

    /// Creates a rotation matrix using the specified rotation in radians and a center point.
    pub fn create_rotation_at(radians: f32, center_point: Vector2) -> Matrix3x2 {
        let (s, c) = rotation_sin_cos(radians);

        let x = center_point.x * (1.0 - c) + center_point.y * s;
        let y = center_point.y * (1.0 - c) - center_point.x * s;

        // [  c  s ]
        // [ -s  c ]
        // [  x  y ]
        Matrix3x2::new(c, s, -s, c, x, y)
    }

    /// Creates a scaling matrix from the specified vector scale.
    #[inline]
    pub fn create_scale(scales: Vector2) -> Matrix3x2 {
        Matrix3x2::create_scale_xy(scales.x, scales.y)
    }

    /// Creates a scaling matrix from the specified X and Y components.
    #[inline]
    pub fn create_scale_xy(x_scale: f32, y_scale: f32) -> Matrix3x2 {
        Matrix3x2::new(x_scale, 0.0, 0.0, y_scale, 0.0, 0.0)
    }

    /// Creates a scaling matrix that scales uniformly with the given scale.
    #[inline]
    pub fn create_scale_uniform(scale: f32) -> Matrix3x2 {
        Matrix3x2::create_scale_xy(scale, scale)
    }

    /// Creates a scaling matrix from the specified vector scale with an offset
    /// from the specified center point.
    #[inline]
    pub fn create_scale_at(scales: Vector2, center_point: Vector2) -> Matrix3x2 {
        let tx = center_point.x * (1.0 - scales.x);
        let ty = center_point.y * (1.0 - scales.y);
        Matrix3x2::new(scales.x, 0.0, 0.0, scales.y, tx, ty)
    }

    /// Creates a skew matrix from the specified angles in radians.
    pub fn create_skew(radians_x: f32, radians_y: f32) -> Matrix3x2 {
        let x_tan = radians_x.tan();
        let y_tan = radians_y.tan();

        Matrix3x2::new(1.0, y_tan, x_tan, 1.0, 0.0, 0.0)
    }

    /// Creates a skew matrix from the specified angles in radians and a center point.
    pub fn create_skew_at(radians_x: f32, radians_y: f32, center_point: Vector2) -> Matrix3x2 {
        let x_tan = radians_x.tan();
        let y_tan = radians_y.tan();

        let tx = -center_point.y * x_tan;
        let ty = -center_point.x * y_tan;

        Matrix3x2::new(1.0, y_tan, x_tan, 1.0, tx, ty)
    }

    /// Creates a translation matrix from the specified 2-dimensional vector.
    #[inline]
    pub fn create_translation(position: Vector2) -> Matrix3x2 {
        Matrix3x2::create_translation_xy(position.x, position.y)
    }

    /// Creates a translation matrix from the specified X and Y components.
    #[inline]
    pub fn create_translation_xy(x_position: f32, y_position: f32) -> Matrix3x2 {
        Matrix3x2::new(1.0, 0.0, 0.0, 1.0, x_position, y_position)
    }

    /// Calculates the determinant for this matrix.
    #[inline]
    pub fn get_determinant(&self) -> f32 {
        // There isn't actually any such thing as a determinant for a non-square matrix,
        // but this 3x2 type is really just an optimization of a 3x3 where we happen to
        // know the rightmost column is always (0, 0, 1). So we expand to 3x3 format:
        //
        //  [ M11, M12, 0 ]
        //  [ M21, M22, 0 ]
        //  [ M31, M32, 1 ]
        //
        // Sum the diagonal products:
        //  (M11 * M22 * 1) + (M12 * 0 * M31) + (0 * M21 * M32)
        //
        // Subtract the opposite diagonal products:
        //  (M31 * M22 * 0) + (M32 * 0 * M11) + (1 * M21 * M12)
        //
        // Collapse out the constants and oh look, this is just a 2x2 determinant!
        (self.m11 * self.m22) - (self.m21 * self.m12)
    }

    /// Tries to invert the specified matrix. Returns `None` when the matrix is
    /// singular; see [`Matrix3x2::invert_or_nan`] for the value the reference
    /// runtime stores in that case.
    pub fn invert(matrix: Matrix3x2) -> Option<Matrix3x2> {
        let det = (matrix.m11 * matrix.m22) - (matrix.m21 * matrix.m12);

        // The reference runtime compares against the smallest positive
        // (subnormal) single.
        if det.abs() < f32::from_bits(1) {
            return None;
        }

        let inv_det = 1.0 / det;

        Some(Matrix3x2::new(
            matrix.m22 * inv_det,
            -matrix.m12 * inv_det,
            -matrix.m21 * inv_det,
            matrix.m11 * inv_det,
            (matrix.m21 * matrix.m32 - matrix.m31 * matrix.m22) * inv_det,
            (matrix.m31 * matrix.m12 - matrix.m11 * matrix.m32) * inv_det,
        ))
    }

    /// Inverts the specified matrix; a singular matrix gives a matrix whose
    /// components are all NaN (the `out` value of the reference runtime).
    pub fn invert_or_nan(matrix: Matrix3x2) -> Matrix3x2 {
        Matrix3x2::invert(matrix).unwrap_or(Matrix3x2::new(
            f32::NAN,
            f32::NAN,
            f32::NAN,
            f32::NAN,
            f32::NAN,
            f32::NAN,
        ))
    }

    /// Performs a linear interpolation from one matrix to a second matrix
    /// based on a value that specifies the weighting of the second matrix.
    pub fn lerp(matrix1: Matrix3x2, matrix2: Matrix3x2, amount: f32) -> Matrix3x2 {
        Matrix3x2::new(
            matrix1.m11 + (matrix2.m11 - matrix1.m11) * amount,
            matrix1.m12 + (matrix2.m12 - matrix1.m12) * amount,
            matrix1.m21 + (matrix2.m21 - matrix1.m21) * amount,
            matrix1.m22 + (matrix2.m22 - matrix1.m22) * amount,
            matrix1.m31 + (matrix2.m31 - matrix1.m31) * amount,
            matrix1.m32 + (matrix2.m32 - matrix1.m32) * amount,
        )
    }
}

/// The sine and cosine of a 2D rotation, exact near multiples of a quarter turn.
#[allow(clippy::manual_range_contains)]
fn rotation_sin_cos(radians: f32) -> (f32, f32) {
    use std::f32::consts::PI;

    let radians = ieee_remainder(radians, PI * 2.0);

    if radians > -ROTATION_EPSILON && radians < ROTATION_EPSILON {
        // Exact case for zero rotation.
        (0.0, 1.0)
    } else if radians > PI / 2.0 - ROTATION_EPSILON && radians < PI / 2.0 + ROTATION_EPSILON {
        // Exact case for 90 degree rotation.
        (1.0, 0.0)
    } else if radians < -PI + ROTATION_EPSILON || radians > PI - ROTATION_EPSILON {
        // Exact case for 180 degree rotation.
        (0.0, -1.0)
    } else if radians > -PI / 2.0 - ROTATION_EPSILON && radians < -PI / 2.0 + ROTATION_EPSILON {
        // Exact case for 270 degree rotation.
        (-1.0, 0.0)
    } else {
        // Arbitrary rotation.
        (radians.sin(), radians.cos())
    }
}

impl Add for Matrix3x2 {
    type Output = Matrix3x2;
    #[inline]
    fn add(self, value2: Matrix3x2) -> Matrix3x2 {
        Matrix3x2::new(
            self.m11 + value2.m11,
            self.m12 + value2.m12,
            self.m21 + value2.m21,
            self.m22 + value2.m22,
            self.m31 + value2.m31,
            self.m32 + value2.m32,
        )
    }
}

impl Sub for Matrix3x2 {
    type Output = Matrix3x2;
    #[inline]
    fn sub(self, value2: Matrix3x2) -> Matrix3x2 {
        Matrix3x2::new(
            self.m11 - value2.m11,
            self.m12 - value2.m12,
            self.m21 - value2.m21,
            self.m22 - value2.m22,
            self.m31 - value2.m31,
            self.m32 - value2.m32,
        )
    }
}

impl Neg for Matrix3x2 {
    type Output = Matrix3x2;
    #[inline]
    fn neg(self) -> Matrix3x2 {
        Matrix3x2::new(-self.m11, -self.m12, -self.m21, -self.m22, -self.m31, -self.m32)
    }
}

impl Mul for Matrix3x2 {
    type Output = Matrix3x2;
    #[inline]
    fn mul(self, value2: Matrix3x2) -> Matrix3x2 {
        let value1 = self;
        Matrix3x2::new(
            value1.m11 * value2.m11 + value1.m12 * value2.m21,
            value1.m11 * value2.m12 + value1.m12 * value2.m22,
            value1.m21 * value2.m11 + value1.m22 * value2.m21,
            value1.m21 * value2.m12 + value1.m22 * value2.m22,
            value1.m31 * value2.m11 + value1.m32 * value2.m21 + value2.m31,
            value1.m31 * value2.m12 + value1.m32 * value2.m22 + value2.m32,
        )
    }
}

/// Multiplies every component by a scalar.
impl Mul<f32> for Matrix3x2 {
    type Output = Matrix3x2;
    #[inline]
    fn mul(self, value2: f32) -> Matrix3x2 {
        Matrix3x2::new(
            self.m11 * value2,
            self.m12 * value2,
            self.m21 * value2,
            self.m22 * value2,
            self.m31 * value2,
            self.m32 * value2,
        )
    }
}

impl fmt::Display for Matrix3x2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ {{M11:{} M12:{}}} {{M21:{} M22:{}}} {{M31:{} M32:{}}} }}",
            InvariantF32(self.m11),
            InvariantF32(self.m12),
            InvariantF32(self.m21),
            InvariantF32(self.m22),
            InvariantF32(self.m31),
            InvariantF32(self.m32)
        )
    }
}
