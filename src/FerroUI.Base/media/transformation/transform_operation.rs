use crate::media::transformation::InterpolationUtilities;
use crate::Matrix;

/// The kind of a [`TransformOperation`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum OperationType {
    #[default]
    Translate = 0,
    Rotate = 1,
    Scale = 2,
    Skew = 3,
    Matrix = 4,
    Identity = 5,
}

/// The skew view of [`DataLayout`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkewLayout {
    pub x: f64,
    pub y: f64,
}

/// The scale view of [`DataLayout`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScaleLayout {
    pub x: f64,
    pub y: f64,
}

/// The translation view of [`DataLayout`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TranslateLayout {
    pub x: f64,
    pub y: f64,
}

/// The rotation view of [`DataLayout`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RotateLayout {
    pub angle: f64,
}

/// The parameters of a [`TransformOperation`].
///
/// The skew, scale, translation and rotation views share the same storage:
/// the first component holds `x` (or the rotation angle) and the second `y`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DataLayout {
    first: f64,
    second: f64,
}

impl DataLayout {
    #[inline]
    pub fn skew(&self) -> SkewLayout {
        SkewLayout { x: self.first, y: self.second }
    }

    #[inline]
    pub fn set_skew(&mut self, x: f64, y: f64) {
        self.first = x;
        self.second = y;
    }

    #[inline]
    pub fn scale(&self) -> ScaleLayout {
        ScaleLayout { x: self.first, y: self.second }
    }

    #[inline]
    pub fn set_scale(&mut self, x: f64, y: f64) {
        self.first = x;
        self.second = y;
    }

    #[inline]
    pub fn translate(&self) -> TranslateLayout {
        TranslateLayout { x: self.first, y: self.second }
    }

    #[inline]
    pub fn set_translate(&mut self, x: f64, y: f64) {
        self.first = x;
        self.second = y;
    }

    #[inline]
    pub fn rotate(&self) -> RotateLayout {
        RotateLayout { angle: self.first }
    }

    #[inline]
    pub fn set_rotate(&mut self, angle: f64) {
        self.first = angle;
    }
}

/// Represents a single primitive transform (like translation, rotation,
/// scale, etc.).
///
/// The default value is a translation by zero whose matrix is all zeros; call
/// [`bake`](Self::bake) after setting the type and data.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TransformOperation {
    pub type_: OperationType,
    pub matrix: Matrix,
    pub data: DataLayout,
}

impl TransformOperation {
    /// Returns whether the operation produces the identity matrix.
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.matrix.is_identity()
    }

    /// Bakes this operation to a transform matrix.
    pub fn bake(&mut self) {
        self.matrix = Matrix::IDENTITY;

        match self.type_ {
            OperationType::Translate => {
                self.matrix = Matrix::create_translation(self.data.translate().x, self.data.translate().y);
            }
            OperationType::Rotate => {
                self.matrix = Matrix::create_rotation(self.data.rotate().angle);
            }
            OperationType::Scale => {
                self.matrix = Matrix::create_scale(self.data.scale().x, self.data.scale().y);
            }
            OperationType::Skew => {
                self.matrix = Matrix::create_skew(self.data.skew().x, self.data.skew().y);
            }
            OperationType::Matrix | OperationType::Identity => {}
        }
    }

    /// Returns a new identity operation.
    pub fn identity() -> TransformOperation {
        TransformOperation { matrix: Matrix::IDENTITY, type_: OperationType::Identity, data: DataLayout::default() }
    }

    /// Attempts to interpolate between two transform operations. Returns
    /// false if the operations cannot be interpolated.
    pub fn try_interpolate(
        from: Option<TransformOperation>,
        to: Option<TransformOperation>,
        progress: f64,
        result: &mut TransformOperation,
    ) -> bool {
        let from_identity = Self::is_operation_identity(&from);
        let to_identity = Self::is_operation_identity(&to);

        if from_identity && to_identity {
            result.matrix = Matrix::IDENTITY;
            return true;
        }

        let from_value = match from {
            Some(from) if !from_identity => from,
            _ => Self::identity(),
        };
        let to_value = match to {
            Some(to) if !to_identity => to,
            _ => Self::identity(),
        };

        let interpolation_type = if to_identity { from_value.type_ } else { to_value.type_ };
        result.type_ = interpolation_type;

        match interpolation_type {
            OperationType::Translate => {
                let from_x = if from_identity { 0.0 } else { from_value.data.translate().x };
                let from_y = if from_identity { 0.0 } else { from_value.data.translate().y };
                let to_x = if to_identity { 0.0 } else { to_value.data.translate().x };
                let to_y = if to_identity { 0.0 } else { to_value.data.translate().y };

                result.data.set_translate(
                    InterpolationUtilities::interpolate_scalars(from_x, to_x, progress),
                    InterpolationUtilities::interpolate_scalars(from_y, to_y, progress),
                );
                result.bake();
            }
            OperationType::Rotate => {
                let from_angle = if from_identity { 0.0 } else { from_value.data.rotate().angle };
                let to_angle = if to_identity { 0.0 } else { to_value.data.rotate().angle };

                result.data.set_rotate(InterpolationUtilities::interpolate_scalars(from_angle, to_angle, progress));
                result.bake();
            }
            OperationType::Scale => {
                let from_x = if from_identity { 1.0 } else { from_value.data.scale().x };
                let from_y = if from_identity { 1.0 } else { from_value.data.scale().y };
                let to_x = if to_identity { 1.0 } else { to_value.data.scale().x };
                let to_y = if to_identity { 1.0 } else { to_value.data.scale().y };

                result.data.set_scale(
                    InterpolationUtilities::interpolate_scalars(from_x, to_x, progress),
                    InterpolationUtilities::interpolate_scalars(from_y, to_y, progress),
                );
                result.bake();
            }
            OperationType::Skew => {
                let from_x = if from_identity { 0.0 } else { from_value.data.skew().x };
                let from_y = if from_identity { 0.0 } else { from_value.data.skew().y };
                let to_x = if to_identity { 0.0 } else { to_value.data.skew().x };
                let to_y = if to_identity { 0.0 } else { to_value.data.skew().y };

                result.data.set_skew(
                    InterpolationUtilities::interpolate_scalars(from_x, to_x, progress),
                    InterpolationUtilities::interpolate_scalars(from_y, to_y, progress),
                );
                result.bake();
            }
            OperationType::Matrix => {
                let from_matrix = if from_identity { Matrix::IDENTITY } else { from_value.matrix };
                let to_matrix = if to_identity { Matrix::IDENTITY } else { to_value.matrix };

                let (Some(from_decomposed), Some(to_decomposed)) =
                    (Matrix::try_decompose_transform(from_matrix), Matrix::try_decompose_transform(to_matrix))
                else {
                    return false;
                };

                let interpolated = InterpolationUtilities::interpolate_decomposed_transforms(
                    &from_decomposed,
                    &to_decomposed,
                    progress,
                );
                result.matrix = InterpolationUtilities::compose_transform(interpolated);
            }
            OperationType::Identity => {
                result.matrix = Matrix::IDENTITY;
            }
        }

        true
    }

    fn is_operation_identity(operation: &Option<TransformOperation>) -> bool {
        match operation {
            Some(operation) => operation.is_identity(),
            None => true,
        }
    }
}
