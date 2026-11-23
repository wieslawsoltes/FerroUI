use crate::{Result, require};
use ferroui_base::Matrix;
use ferroui_base::media::transformation::{
    OperationType, TransformOperation, TransformOperationsBuilder,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformOperationKind {
    Translate,
    Rotate,
    Scale,
    Skew,
    Matrix,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum AnimationTransformOperation {
    Translate([f64; 2]),
    Rotate(f64),
    Scale([f64; 2]),
    Skew([f64; 2]),
    Matrix([f64; 6]),
}

impl AnimationTransformOperation {
    pub fn kind(&self) -> TransformOperationKind {
        match self {
            Self::Translate(_) => TransformOperationKind::Translate,
            Self::Rotate(_) => TransformOperationKind::Rotate,
            Self::Scale(_) => TransformOperationKind::Scale,
            Self::Skew(_) => TransformOperationKind::Skew,
            Self::Matrix(_) => TransformOperationKind::Matrix,
        }
    }

    pub(super) fn from_operation(operation: &TransformOperation) -> Result<Self> {
        if !matches!(
            operation.type_,
            OperationType::Matrix | OperationType::Identity
        ) {
            let mut baked = *operation;
            baked.bake();
            if baked.matrix != operation.matrix {
                return Self::from_matrix(operation.matrix);
            }
        }
        Ok(match operation.type_ {
            OperationType::Translate => {
                let value = operation.data.translate();
                Self::Translate([value.x, value.y])
            }
            OperationType::Rotate => Self::Rotate(operation.data.rotate().angle),
            OperationType::Scale => {
                let value = operation.data.scale();
                Self::Scale([value.x, value.y])
            }
            OperationType::Skew => {
                let value = operation.data.skew();
                Self::Skew([value.x, value.y])
            }
            OperationType::Matrix | OperationType::Identity => Self::from_matrix(operation.matrix)?,
        })
    }

    pub(super) fn from_matrix(matrix: Matrix) -> Result<Self> {
        require(
            matrix.m13 == 0.0 && matrix.m23 == 0.0 && matrix.m33 == 1.0,
            "An authored transform list requires affine matrices",
        )?;
        Ok(Self::Matrix([
            matrix.m11, matrix.m12, matrix.m21, matrix.m22, matrix.m31, matrix.m32,
        ]))
    }

    pub(super) fn is_finite(&self) -> bool {
        let values: &[f64] = match self {
            Self::Translate(values) | Self::Scale(values) | Self::Skew(values) => values,
            Self::Rotate(angle) => std::slice::from_ref(angle),
            Self::Matrix(values) => values,
        };
        values.iter().all(|value| value.is_finite())
    }

    pub(super) fn append(self, builder: &mut TransformOperationsBuilder) {
        match self {
            Self::Translate([x, y]) => builder.append_translate(x, y),
            Self::Rotate(angle) => builder.append_rotate(angle),
            Self::Scale([x, y]) => builder.append_scale(x, y),
            Self::Skew([x, y]) => builder.append_skew(x, y),
            Self::Matrix([a, b, c, d, e, f]) => {
                builder.append_matrix(Matrix::new(a, b, c, d, e, f))
            }
        }
    }

    pub(super) fn xaml(&self) -> String {
        match self {
            Self::Translate([x, y]) => format!("translate({x}px, {y}px)"),
            Self::Rotate(angle) => format!("rotate({angle}rad)"),
            Self::Scale([x, y]) => format!("scale({x}, {y})"),
            Self::Skew([x, y]) => format!("skew({x}rad, {y}rad)"),
            Self::Matrix([a, b, c, d, e, f]) => format!("matrix({a}, {b}, {c}, {d}, {e}, {f})"),
        }
    }
}

pub(super) fn matrix_is_finite(matrix: Matrix) -> bool {
    [
        matrix.m11, matrix.m12, matrix.m13, matrix.m21, matrix.m22, matrix.m23, matrix.m31,
        matrix.m32, matrix.m33,
    ]
    .iter()
    .all(|value| value.is_finite())
}
