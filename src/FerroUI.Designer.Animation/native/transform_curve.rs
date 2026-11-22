use crate::{
    AnimationError, AnimationValue, CurveChannel, CurvePoint, TransformCurveChannel,
    TransformOperationKind,
};
use ferroui_base::media::{
    ITransform,
    transformation::{OperationType, TransformOperation, TransformOperations},
};
use ferroui_base::{BoxedValue, Matrix};
use std::rc::Rc;

pub(super) fn project(
    channel: TransformCurveChannel,
    samples: impl IntoIterator<Item = (f64, BoxedValue)>,
) -> Result<Vec<CurvePoint>, String> {
    CurveChannel::Value
        .project_samples(samples.into_iter().map(|(time, value)| {
            let transform = value
                .downcast_ref::<Option<Rc<dyn ITransform>>>()
                .ok_or_else(|| {
                    AnimationError("The native animator did not produce a transform".into())
                })?;
            let sample = match channel {
                TransformCurveChannel::Matrix(component) => {
                    let matrix = transform
                        .as_deref()
                        .map_or(Matrix::IDENTITY, ITransform::value);
                    coefficient(component, matrix)
                }
                TransformCurveChannel::Operation {
                    index,
                    kind,
                    component,
                } => transform
                    .as_deref()
                    .and_then(|value| value.as_any().downcast_ref::<TransformOperations>())
                    .and_then(|list| list.operations().get(index.get() - 1))
                    .and_then(|operation| scalar(kind, component, operation)),
            };
            Ok((time, sample.map(AnimationValue::Number)))
        }))
        .map_err(|error| error.to_string())
}

fn scalar(
    kind: TransformOperationKind,
    component: CurveChannel,
    operation: &TransformOperation,
) -> Option<f64> {
    use CurveChannel as C;
    use TransformOperationKind as K;
    Some(match (kind, component, operation.type_) {
        (K::Translate, C::AbsoluteX, OperationType::Translate) => operation.data.translate().x,
        (K::Translate, C::AbsoluteY, OperationType::Translate) => operation.data.translate().y,
        (K::Rotate, C::Angle, OperationType::Rotate) => operation.data.rotate().angle.to_degrees(),
        (K::Scale, C::X, OperationType::Scale) => operation.data.scale().x,
        (K::Scale, C::Y, OperationType::Scale) => operation.data.scale().y,
        (K::Skew, C::AngleX, OperationType::Skew) => operation.data.skew().x.to_degrees(),
        (K::Skew, C::AngleY, OperationType::Skew) => operation.data.skew().y.to_degrees(),
        (K::Matrix, component, OperationType::Matrix) => {
            return coefficient(component, operation.matrix);
        }
        _ => return None,
    })
}

fn coefficient(channel: CurveChannel, matrix: Matrix) -> Option<f64> {
    use CurveChannel::*;
    Some(match channel {
        M11 => matrix.m11,
        M12 => matrix.m12,
        M21 => matrix.m21,
        M22 => matrix.m22,
        M31 => matrix.m31,
        M32 => matrix.m32,
        _ => return None,
    })
}
