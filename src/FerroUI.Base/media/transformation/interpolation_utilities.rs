use crate::{Matrix, MatrixDecomposed, Vector};

/// Interpolation helpers shared by the transform operations.
pub struct InterpolationUtilities;

impl InterpolationUtilities {
    pub fn interpolate_scalars(from: f64, to: f64, progress: f64) -> f64 {
        from * (1.0 - progress) + to * progress
    }

    pub fn interpolate_vectors(from: Vector, to: Vector, progress: f64) -> Vector {
        let x = Self::interpolate_scalars(from.x, to.x, progress);
        let y = Self::interpolate_scalars(from.y, to.y, progress);
        Vector::new(x, y)
    }

    pub fn compose_transform(decomposed: MatrixDecomposed) -> Matrix {
        // According to https://www.w3.org/TR/css-transforms-1/#recomposing-to-a-2d-matrix
        Matrix::IDENTITY
            .prepend(Matrix::create_translation_vector(decomposed.translate))
            .prepend(Matrix::create_rotation(decomposed.angle))
            .prepend(Matrix::create_skew(decomposed.skew.x, decomposed.skew.y))
            .prepend(Matrix::create_scale_vector(decomposed.scale))
    }

    pub fn interpolate_decomposed_transforms(
        from: &MatrixDecomposed,
        to: &MatrixDecomposed,
        progress: f64,
    ) -> MatrixDecomposed {
        MatrixDecomposed {
            translate: Self::interpolate_vectors(from.translate, to.translate, progress),
            scale: Self::interpolate_vectors(from.scale, to.scale, progress),
            skew: Self::interpolate_vectors(from.skew, to.skew, progress),
            angle: Self::interpolate_scalars(from.angle, to.angle, progress),
        }
    }
}
