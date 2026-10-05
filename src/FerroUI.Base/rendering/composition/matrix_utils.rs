//! Helpers computing the transform of a composition visual.

use crate::numerics::{Matrix4x4, Quaternion, Vector3};
use crate::{Matrix, Vector, Vector3D};

/// Helpers computing the transform of a composition visual.
pub struct MatrixUtils;

impl MatrixUtils {
    /// Computes the transform of a visual from its transform properties.
    /// Returns `None` when the transform is the identity.
    #[allow(clippy::too_many_arguments)]
    pub fn compute_transform(
        size: Vector,
        anchor_point: Vector,
        center_point: Vector3D,
        transform_matrix: Matrix,
        scale: Vector3D,
        rotation_angle: f32,
        orientation: Quaternion,
        offset: Vector3D,
    ) -> Option<Matrix> {
        // The math here follows the *observed* UWP behavior since there are no docs on how it's supposed to work

        let anchor = Vector::multiply(size, anchor_point);
        let mut mat = Matrix::create_translation(-anchor.x, -anchor.y);

        let center = Vector3D::new(center_point.x, center_point.y, center_point.z);
        let center3: Vector3 = center.into();

        if !transform_matrix.is_identity() {
            mat = transform_matrix * mat;
        }

        if scale != Vector3D::new(1.0, 1.0, 1.0) {
            mat *= Self::to_matrix(Matrix4x4::create_scale_at(scale.into(), center3));
        }

        //TODO: RotationAxis support
        if rotation_angle != 0.0 {
            mat *= Self::to_matrix(Matrix4x4::create_rotation_z_at(rotation_angle, center3));
        }

        if orientation != Quaternion::IDENTITY {
            if center_point != Vector3D::default() {
                mat *= Self::to_matrix(
                    Matrix4x4::create_translation(-center3)
                        * Matrix4x4::create_from_quaternion(orientation)
                        * Matrix4x4::create_translation(center3),
                );
            } else {
                mat *= Self::to_matrix(Matrix4x4::create_from_quaternion(orientation));
            }
        }

        if offset != Vector3D::default() {
            if offset.z == 0.0 {
                mat *= Matrix::create_translation(offset.x, offset.y);
            } else {
                mat *= Self::to_matrix(Matrix4x4::create_translation(offset.into()));
            }
        }

        if mat.is_identity() {
            return None;
        }
        Some(mat)
    }

    /// Converts a 3x3 matrix to the 4x4 matrix that leaves Z unchanged.
    pub fn to_matrix4x4(matrix: Matrix) -> Matrix4x4 {
        Matrix4x4::new(
            matrix.m11 as f32,
            matrix.m12 as f32,
            0.0,
            matrix.m13 as f32,
            matrix.m21 as f32,
            matrix.m22 as f32,
            0.0,
            matrix.m23 as f32,
            0.0,
            0.0,
            1.0,
            0.0,
            matrix.m31 as f32,
            matrix.m32 as f32,
            0.0,
            matrix.m33 as f32,
        )
    }

    /// Converts a 4x4 matrix to a 3x3 matrix, dropping the Z row and column.
    pub fn to_matrix(matrix44: Matrix4x4) -> Matrix {
        Matrix::new_3x3(
            matrix44.m11 as f64,
            matrix44.m12 as f64,
            matrix44.m14 as f64,
            matrix44.m21 as f64,
            matrix44.m22 as f64,
            matrix44.m24 as f64,
            matrix44.m41 as f64,
            matrix44.m42 as f64,
            matrix44.m44 as f64,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;

    const ONE: Vector3D = Vector3D::new(1.0, 1.0, 1.0);
    const ZERO: Vector3D = Vector3D::new(0.0, 0.0, 0.0);

    #[track_caller]
    fn close(actual: Matrix, expected: Matrix) {
        let a = [
            actual.m11, actual.m12, actual.m13, actual.m21, actual.m22, actual.m23, actual.m31, actual.m32, actual.m33,
        ];
        let e = [
            expected.m11,
            expected.m12,
            expected.m13,
            expected.m21,
            expected.m22,
            expected.m23,
            expected.m31,
            expected.m32,
            expected.m33,
        ];
        for (a, e) in a.iter().zip(e) {
            assert!((a - e).abs() < 1e-5, "{actual:?} vs {expected:?}");
        }
    }

    #[test]
    fn identity_inputs_give_no_transform() {
        assert_eq!(
            MatrixUtils::compute_transform(
                Vector::new(100.0, 50.0),
                Vector::ZERO,
                ZERO,
                Matrix::IDENTITY,
                ONE,
                0.0,
                Quaternion::IDENTITY,
                ZERO
            ),
            None
        );
    }

    #[test]
    fn offset_and_anchor_translate() {
        assert_eq!(
            MatrixUtils::compute_transform(
                Vector::new(100.0, 50.0),
                Vector::new(0.5, 1.0),
                ZERO,
                Matrix::IDENTITY,
                ONE,
                0.0,
                Quaternion::IDENTITY,
                Vector3D::new(10.0, 20.0, 0.0)
            ),
            Some(Matrix::create_translation(-40.0, -30.0))
        );
        // An offset with a Z component goes through the 4x4 path.
        assert_eq!(
            MatrixUtils::compute_transform(
                Vector::ZERO,
                Vector::ZERO,
                ZERO,
                Matrix::IDENTITY,
                ONE,
                0.0,
                Quaternion::IDENTITY,
                Vector3D::new(10.0, 20.0, 5.0)
            ),
            Some(Matrix::create_translation(10.0, 20.0))
        );
    }

    #[test]
    fn scale_and_rotation_are_applied_around_the_center_point() {
        let center = Vector3D::new(10.0, 20.0, 0.0);
        let scaled = MatrixUtils::compute_transform(
            Vector::ZERO,
            Vector::ZERO,
            center,
            Matrix::IDENTITY,
            Vector3D::new(2.0, 3.0, 1.0),
            0.0,
            Quaternion::IDENTITY,
            ZERO,
        )
        .unwrap();
        assert_eq!(scaled, Matrix::new(2.0, 0.0, 0.0, 3.0, -10.0, -40.0));
        assert_eq!(scaled.transform(Point::new(10.0, 20.0)), Point::new(10.0, 20.0));

        let angle = 0.7f32;
        let rotated = MatrixUtils::compute_transform(
            Vector::ZERO,
            Vector::ZERO,
            center,
            Matrix::IDENTITY,
            ONE,
            angle,
            Quaternion::IDENTITY,
            ZERO,
        )
        .unwrap();
        close(
            rotated,
            Matrix::create_rotation_at(angle as f64, Point::new(10.0, 20.0)),
        );

        // The same rotation given as an orientation.
        let oriented = MatrixUtils::compute_transform(
            Vector::ZERO,
            Vector::ZERO,
            center,
            Matrix::IDENTITY,
            ONE,
            0.0,
            Quaternion::create_from_axis_angle(Vector3::UNIT_Z, angle),
            ZERO,
        )
        .unwrap();
        close(oriented, rotated);

        let oriented_at_origin = MatrixUtils::compute_transform(
            Vector::ZERO,
            Vector::ZERO,
            ZERO,
            Matrix::IDENTITY,
            ONE,
            0.0,
            Quaternion::create_from_axis_angle(Vector3::UNIT_Z, angle),
            ZERO,
        )
        .unwrap();
        close(oriented_at_origin, Matrix::create_rotation(angle as f64));
    }

    #[test]
    fn transform_matrix_is_applied_before_the_anchor_translation() {
        let transform = Matrix::create_scale(2.0, 2.0);
        let actual = MatrixUtils::compute_transform(
            Vector::new(10.0, 10.0),
            Vector::new(1.0, 1.0),
            ZERO,
            transform,
            ONE,
            0.0,
            Quaternion::IDENTITY,
            ZERO,
        )
        .unwrap();
        assert_eq!(actual, transform * Matrix::create_translation(-10.0, -10.0));
    }

    #[test]
    fn matrix_conversions_drop_and_restore_the_z_axis() {
        let m = Matrix::new_3x3(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
        let m44 = MatrixUtils::to_matrix4x4(m);
        assert_eq!(
            m44,
            Matrix4x4::new(1.0, 2.0, 0.0, 3.0, 4.0, 5.0, 0.0, 6.0, 0.0, 0.0, 1.0, 0.0, 7.0, 8.0, 0.0, 9.0)
        );
        assert_eq!(MatrixUtils::to_matrix(m44), m);
    }
}
