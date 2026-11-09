//! `Matrix3D`: a 4x4 transformation of 3D space.

use std::fmt;
use std::ops::{Mul, MulAssign};
use std::str::FromStr;

use ferroui_base::utilities::FormatError;

use crate::text::{is_keyword, is_zero, parse_numbers, write_numbers};
use crate::{Matrix3DError, Point3D, Point4D, Quaternion, Vector3D};

/// A 4x4 transformation of 3D space, with double-precision elements (WPF's
/// `System.Windows.Media.Media3D.Matrix3D`).
///
/// The convention is WPF's: points are row vectors multiplied on the left, `p' = p * M`, so the
/// translation is the fourth row (`offset_x`, `offset_y`, `offset_z`) and the projective terms are
/// the fourth column (`m14`, `m24`, `m34`, `m44`). `a * b` is `a` followed by `b`;
/// [`append`](Matrix3D::append) multiplies on the right and [`prepend`](Matrix3D::prepend) on the
/// left. The default value is [`Matrix3D::IDENTITY`], as in WPF.
///
/// The methods that change the matrix take `&mut self`, as WPF's change the structure in place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix3D {
    /// Row 1, column 1.
    pub m11: f64,
    /// Row 1, column 2.
    pub m12: f64,
    /// Row 1, column 3.
    pub m13: f64,
    /// Row 1, column 4 (projective).
    pub m14: f64,
    /// Row 2, column 1.
    pub m21: f64,
    /// Row 2, column 2.
    pub m22: f64,
    /// Row 2, column 3.
    pub m23: f64,
    /// Row 2, column 4 (projective).
    pub m24: f64,
    /// Row 3, column 1.
    pub m31: f64,
    /// Row 3, column 2.
    pub m32: f64,
    /// Row 3, column 3.
    pub m33: f64,
    /// Row 3, column 4 (projective).
    pub m34: f64,
    /// Row 4, column 1: the translation along X.
    pub offset_x: f64,
    /// Row 4, column 2: the translation along Y.
    pub offset_y: f64,
    /// Row 4, column 3: the translation along Z.
    pub offset_z: f64,
    /// Row 4, column 4 (projective).
    pub m44: f64,
}

impl Matrix3D {
    /// The identity transformation (WPF's `Matrix3D.Identity`).
    pub const IDENTITY: Matrix3D = Matrix3D::new(
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    );

    /// A matrix with the given elements, row by row.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        m11: f64,
        m12: f64,
        m13: f64,
        m14: f64,
        m21: f64,
        m22: f64,
        m23: f64,
        m24: f64,
        m31: f64,
        m32: f64,
        m33: f64,
        m34: f64,
        offset_x: f64,
        offset_y: f64,
        offset_z: f64,
        m44: f64,
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
            offset_x,
            offset_y,
            offset_z,
            m44,
        }
    }

    /// Makes this the identity.
    #[inline]
    pub fn set_identity(&mut self) {
        *self = Matrix3D::IDENTITY;
    }

    /// Whether this is the identity, element by element.
    #[inline]
    pub fn is_identity(&self) -> bool {
        *self == Matrix3D::IDENTITY
    }

    /// Whether the matrix is affine: no projective terms (`m14`, `m24`, `m34` zero and `m44` one).
    #[inline]
    pub fn is_affine(&self) -> bool {
        self.m14 == 0.0 && self.m24 == 0.0 && self.m34 == 0.0 && self.m44 == 1.0
    }

    /// The determinant: of the upper 3x3 block for an affine matrix, of the whole matrix
    /// otherwise.
    pub fn determinant(&self) -> f64 {
        if self.is_affine() {
            return self.m11 * (self.m22 * self.m33 - self.m23 * self.m32)
                - self.m12 * (self.m21 * self.m33 - self.m23 * self.m31)
                + self.m13 * (self.m21 * self.m32 - self.m22 * self.m31);
        }
        let minors = Minors::of(&self.rows());
        minors.determinant()
    }

    /// Whether the matrix has an inverse: its determinant is not zero within ten machine epsilons.
    #[inline]
    pub fn has_inverse(&self) -> bool {
        !is_zero(self.determinant())
    }

    /// Replaces the matrix by its inverse; [`Matrix3DError::NotInvertible`] (and the matrix
    /// unchanged) when it has none ([`has_inverse`](Matrix3D::has_inverse) is false).
    pub fn invert(&mut self) -> Result<(), Matrix3DError> {
        if self.is_identity() {
            return Ok(());
        }
        let determinant = self.determinant();
        if is_zero(determinant) {
            return Err(Matrix3DError::NotInvertible);
        }
        let a = self.rows();
        let m = Minors::of(&a);
        let (s, c) = (m.s, m.c);
        let inverse = [
            [
                a[1][1] * c[5] - a[1][2] * c[4] + a[1][3] * c[3],
                -a[0][1] * c[5] + a[0][2] * c[4] - a[0][3] * c[3],
                a[3][1] * s[5] - a[3][2] * s[4] + a[3][3] * s[3],
                -a[2][1] * s[5] + a[2][2] * s[4] - a[2][3] * s[3],
            ],
            [
                -a[1][0] * c[5] + a[1][2] * c[2] - a[1][3] * c[1],
                a[0][0] * c[5] - a[0][2] * c[2] + a[0][3] * c[1],
                -a[3][0] * s[5] + a[3][2] * s[2] - a[3][3] * s[1],
                a[2][0] * s[5] - a[2][2] * s[2] + a[2][3] * s[1],
            ],
            [
                a[1][0] * c[4] - a[1][1] * c[2] + a[1][3] * c[0],
                -a[0][0] * c[4] + a[0][1] * c[2] - a[0][3] * c[0],
                a[3][0] * s[4] - a[3][1] * s[2] + a[3][3] * s[0],
                -a[2][0] * s[4] + a[2][1] * s[2] - a[2][3] * s[0],
            ],
            [
                -a[1][0] * c[3] + a[1][1] * c[1] - a[1][2] * c[0],
                a[0][0] * c[3] - a[0][1] * c[1] + a[0][2] * c[0],
                -a[3][0] * s[3] + a[3][1] * s[1] - a[3][2] * s[0],
                a[2][0] * s[3] - a[2][1] * s[1] + a[2][2] * s[0],
            ],
        ];
        // The adjugate over the determinant (for an affine matrix, the determinant of its 3x3
        // block, which is the determinant of the whole matrix).
        let mut rows = inverse;
        for row in &mut rows {
            for element in row.iter_mut() {
                *element /= determinant;
            }
        }
        *self = Matrix3D::from_rows(rows);
        Ok(())
    }

    /// Multiplies the matrix by `matrix` on the right: this transformation, then `matrix`.
    #[inline]
    pub fn append(&mut self, matrix: Matrix3D) {
        *self = *self * matrix;
    }

    /// Multiplies the matrix by `matrix` on the left: `matrix`, then this transformation.
    #[inline]
    pub fn prepend(&mut self, matrix: Matrix3D) {
        *self = matrix * *self;
    }

    /// Appends the rotation of the quaternion about the origin.
    #[inline]
    pub fn rotate(&mut self, quaternion: Quaternion) {
        self.append(Matrix3D::rotation(quaternion, Point3D::default()));
    }

    /// Prepends the rotation of the quaternion about the origin.
    #[inline]
    pub fn rotate_prepend(&mut self, quaternion: Quaternion) {
        self.prepend(Matrix3D::rotation(quaternion, Point3D::default()));
    }

    /// Appends the rotation of the quaternion about `center`.
    #[inline]
    pub fn rotate_at(&mut self, quaternion: Quaternion, center: Point3D) {
        self.append(Matrix3D::rotation(quaternion, center));
    }

    /// Prepends the rotation of the quaternion about `center`.
    #[inline]
    pub fn rotate_at_prepend(&mut self, quaternion: Quaternion, center: Point3D) {
        self.prepend(Matrix3D::rotation(quaternion, center));
    }

    /// Appends a scale by the components of the vector, about the origin.
    #[inline]
    pub fn scale(&mut self, scale: Vector3D) {
        self.append(Matrix3D::scaling(scale, Point3D::default()));
    }

    /// Prepends a scale by the components of the vector, about the origin.
    #[inline]
    pub fn scale_prepend(&mut self, scale: Vector3D) {
        self.prepend(Matrix3D::scaling(scale, Point3D::default()));
    }

    /// Appends a scale by the components of the vector, about `center`.
    #[inline]
    pub fn scale_at(&mut self, scale: Vector3D, center: Point3D) {
        self.append(Matrix3D::scaling(scale, center));
    }

    /// Prepends a scale by the components of the vector, about `center`.
    #[inline]
    pub fn scale_at_prepend(&mut self, scale: Vector3D, center: Point3D) {
        self.prepend(Matrix3D::scaling(scale, center));
    }

    /// Appends a translation by the vector.
    #[inline]
    pub fn translate(&mut self, offset: Vector3D) {
        self.append(Matrix3D::translation(offset));
    }

    /// Prepends a translation by the vector.
    #[inline]
    pub fn translate_prepend(&mut self, offset: Vector3D) {
        self.prepend(Matrix3D::translation(offset));
    }

    /// The point transformed by the matrix, divided by its `w` when the matrix is not affine.
    pub fn transform(&self, point: Point3D) -> Point3D {
        let Point3D { x, y, z } = point;
        let mut result = Point3D::new(
            x * self.m11 + y * self.m21 + z * self.m31 + self.offset_x,
            x * self.m12 + y * self.m22 + z * self.m32 + self.offset_y,
            x * self.m13 + y * self.m23 + z * self.m33 + self.offset_z,
        );
        if !self.is_affine() {
            let w = x * self.m14 + y * self.m24 + z * self.m34 + self.m44;
            result.x /= w;
            result.y /= w;
            result.z /= w;
        }
        result
    }

    /// Transforms the points in place, as [`transform`](Matrix3D::transform).
    pub fn transform_points(&self, points: &mut [Point3D]) {
        for point in points {
            *point = self.transform(*point);
        }
    }

    /// The vector transformed by the matrix: by its upper 3x3 block, so the translation and the
    /// projective terms do not apply.
    pub fn transform_vector(&self, vector: Vector3D) -> Vector3D {
        let Vector3D { x, y, z } = vector;
        Vector3D::new(
            x * self.m11 + y * self.m21 + z * self.m31,
            x * self.m12 + y * self.m22 + z * self.m32,
            x * self.m13 + y * self.m23 + z * self.m33,
        )
    }

    /// Transforms the vectors in place, as [`transform_vector`](Matrix3D::transform_vector).
    pub fn transform_vectors(&self, vectors: &mut [Vector3D]) {
        for vector in vectors {
            *vector = self.transform_vector(*vector);
        }
    }

    /// The homogeneous point transformed by the whole matrix, without a divide.
    pub fn transform_point4d(&self, point: Point4D) -> Point4D {
        let Point4D { x, y, z, w } = point;
        Point4D::new(
            x * self.m11 + y * self.m21 + z * self.m31 + w * self.offset_x,
            x * self.m12 + y * self.m22 + z * self.m32 + w * self.offset_y,
            x * self.m13 + y * self.m23 + z * self.m33 + w * self.offset_z,
            x * self.m14 + y * self.m24 + z * self.m34 + w * self.m44,
        )
    }

    /// Transforms the homogeneous points in place, as
    /// [`transform_point4d`](Matrix3D::transform_point4d).
    pub fn transform_points4d(&self, points: &mut [Point4D]) {
        for point in points {
            *point = self.transform_point4d(*point);
        }
    }

    /// Parses the text form: `"Identity"`, or the 16 elements row by row (commas, white space or
    /// both between the numbers) in the invariant culture.
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        if is_keyword(text, "Identity") {
            return Ok(Matrix3D::IDENTITY);
        }
        let values = parse_numbers::<16>(text, "Invalid Matrix3D.")?;
        Ok(Matrix3D::from_rows([
            [values[0], values[1], values[2], values[3]],
            [values[4], values[5], values[6], values[7]],
            [values[8], values[9], values[10], values[11]],
            [values[12], values[13], values[14], values[15]],
        ]))
    }

    /// The rotation of the quaternion about `center`. The quaternion is used as it is, not
    /// normalized, as in WPF.
    fn rotation(quaternion: Quaternion, center: Point3D) -> Matrix3D {
        let Quaternion { x, y, z, w } = quaternion;
        let (x2, y2, z2) = (x + x, y + y, z + z);
        let (xx, xy, xz) = (x * x2, x * y2, x * z2);
        let (yy, yz, zz) = (y * y2, y * z2, z * z2);
        let (wx, wy, wz) = (w * x2, w * y2, w * z2);

        let mut matrix = Matrix3D::IDENTITY;
        matrix.m11 = 1.0 - (yy + zz);
        matrix.m12 = xy + wz;
        matrix.m13 = xz - wy;
        matrix.m21 = xy - wz;
        matrix.m22 = 1.0 - (xx + zz);
        matrix.m23 = yz + wx;
        matrix.m31 = xz + wy;
        matrix.m32 = yz - wx;
        matrix.m33 = 1.0 - (xx + yy);
        if center != Point3D::default() {
            matrix.offset_x = -center.x * matrix.m11 - center.y * matrix.m21 - center.z * matrix.m31 + center.x;
            matrix.offset_y = -center.x * matrix.m12 - center.y * matrix.m22 - center.z * matrix.m32 + center.y;
            matrix.offset_z = -center.x * matrix.m13 - center.y * matrix.m23 - center.z * matrix.m33 + center.z;
        }
        matrix
    }

    /// The scale by the components of the vector about `center`.
    fn scaling(scale: Vector3D, center: Point3D) -> Matrix3D {
        let mut matrix = Matrix3D::IDENTITY;
        matrix.m11 = scale.x;
        matrix.m22 = scale.y;
        matrix.m33 = scale.z;
        matrix.offset_x = center.x - scale.x * center.x;
        matrix.offset_y = center.y - scale.y * center.y;
        matrix.offset_z = center.z - scale.z * center.z;
        matrix
    }

    /// The translation by the vector.
    fn translation(offset: Vector3D) -> Matrix3D {
        let mut matrix = Matrix3D::IDENTITY;
        matrix.offset_x = offset.x;
        matrix.offset_y = offset.y;
        matrix.offset_z = offset.z;
        matrix
    }

    #[inline]
    fn rows(&self) -> [[f64; 4]; 4] {
        [
            [self.m11, self.m12, self.m13, self.m14],
            [self.m21, self.m22, self.m23, self.m24],
            [self.m31, self.m32, self.m33, self.m34],
            [self.offset_x, self.offset_y, self.offset_z, self.m44],
        ]
    }

    #[inline]
    fn from_rows(rows: [[f64; 4]; 4]) -> Matrix3D {
        let [[m11, m12, m13, m14], [m21, m22, m23, m24], [m31, m32, m33, m34], [offset_x, offset_y, offset_z, m44]] =
            rows;
        Matrix3D::new(
            m11, m12, m13, m14, m21, m22, m23, m24, m31, m32, m33, m34, offset_x, offset_y, offset_z, m44,
        )
    }
}

/// The 2x2 minors of a 4x4 matrix that its determinant and its adjugate are made of: `s` of the
/// first two rows, `c` of the last two (Laplace expansion along the first two rows).
struct Minors {
    s: [f64; 6],
    c: [f64; 6],
}

impl Minors {
    fn of(a: &[[f64; 4]; 4]) -> Minors {
        Minors {
            s: [
                a[0][0] * a[1][1] - a[1][0] * a[0][1],
                a[0][0] * a[1][2] - a[1][0] * a[0][2],
                a[0][0] * a[1][3] - a[1][0] * a[0][3],
                a[0][1] * a[1][2] - a[1][1] * a[0][2],
                a[0][1] * a[1][3] - a[1][1] * a[0][3],
                a[0][2] * a[1][3] - a[1][2] * a[0][3],
            ],
            c: [
                a[2][0] * a[3][1] - a[3][0] * a[2][1],
                a[2][0] * a[3][2] - a[3][0] * a[2][2],
                a[2][0] * a[3][3] - a[3][0] * a[2][3],
                a[2][1] * a[3][2] - a[3][1] * a[2][2],
                a[2][1] * a[3][3] - a[3][1] * a[2][3],
                a[2][2] * a[3][3] - a[3][2] * a[2][3],
            ],
        }
    }

    fn determinant(&self) -> f64 {
        let (s, c) = (&self.s, &self.c);
        s[0] * c[5] - s[1] * c[4] + s[2] * c[3] + s[3] * c[2] - s[4] * c[1] + s[5] * c[0]
    }
}

impl Default for Matrix3D {
    /// [`Matrix3D::IDENTITY`], as WPF's default matrix.
    #[inline]
    fn default() -> Self {
        Matrix3D::IDENTITY
    }
}

/// The product `self * matrix`: this transformation, then `matrix` (row vectors).
impl Mul for Matrix3D {
    type Output = Matrix3D;
    fn mul(self, matrix: Matrix3D) -> Matrix3D {
        let a = self.rows();
        let b = matrix.rows();
        let mut rows = [[0.0; 4]; 4];
        for (row, a_row) in rows.iter_mut().zip(a.iter()) {
            for (column, element) in row.iter_mut().enumerate() {
                *element = a_row[0] * b[0][column]
                    + a_row[1] * b[1][column]
                    + a_row[2] * b[2][column]
                    + a_row[3] * b[3][column];
            }
        }
        Matrix3D::from_rows(rows)
    }
}

impl MulAssign for Matrix3D {
    #[inline]
    fn mul_assign(&mut self, matrix: Matrix3D) {
        *self = *self * matrix;
    }
}

impl FromStr for Matrix3D {
    type Err = FormatError;
    #[inline]
    fn from_str(text: &str) -> Result<Self, FormatError> {
        Matrix3D::parse(text)
    }
}

/// The text form in the invariant culture: `"Identity"`, or the 16 elements row by row.
impl fmt::Display for Matrix3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_identity() {
            return f.write_str("Identity");
        }
        let rows = self.rows();
        write_numbers(f, rows.as_flattened())
    }
}

#[cfg(test)]
#[path = "matrix_3d_tests.rs"]
mod tests;
