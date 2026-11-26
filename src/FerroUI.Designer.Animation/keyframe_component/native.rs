use super::{color, Component, Units};
use crate::{
    AnimationBrush, AnimationTransformOperation, AnimationValue, BrushCurveChannel, BrushField,
    BrushForm, CurveChannel, GradientDistance, GradientPoint, GradientShape, RelativeUnit,
    TrackCurveChannel, TransformCurveChannel,
};

pub(super) fn slot<'a>(
    channel: &TrackCurveChannel,
    value: &'a mut AnimationValue,
) -> Option<Component<'a>> {
    use AnimationValue as V;
    use CurveChannel as C;
    use TrackCurveChannel as T;
    match (channel, value) {
        (T::Brush(channel), V::Brush(value)) => brush(*channel, value),
        (T::BoxShadow { layer, component }, V::BoxShadows(value)) => {
            let shadow = value.layers_mut().get_mut(layer.get() - 1)?;
            Some(match component {
                C::OffsetX => (&mut shadow.offset_x).into(),
                C::OffsetY => (&mut shadow.offset_y).into(),
                C::BlurRadius => (&mut shadow.blur).into(),
                C::Spread => (&mut shadow.spread).into(),
                channel => return color(*channel, &mut shadow.argb),
            })
        }
        (
            T::Transform(TransformCurveChannel::Operation {
                index,
                kind,
                component,
            }),
            V::TransformList(value),
        ) => {
            let operation = value.operations_mut().get_mut(index.get() - 1)?;
            if operation.kind() != *kind {
                return None;
            }
            transform(*component, operation)
        }
        _ => None,
    }
}

pub(super) fn keep_writable_form(value: &mut AnimationValue) {
    if let AnimationValue::Brush(AnimationBrush::Solid { opacity, form, .. }) = value {
        if *form == BrushForm::Literal && *opacity != 1.0 {
            *form = BrushForm::Object;
        }
    }
}

fn brush(channel: BrushCurveChannel, brush: &mut AnimationBrush) -> Option<Component<'_>> {
    use BrushCurveChannel::{Field, Stop};
    use BrushField as F;
    use CurveChannel as C;
    if matches!(
        channel,
        Field {
            field: F::Opacity,
            component: C::Value
        }
    ) {
        let value = match brush {
            AnimationBrush::Solid { opacity, .. } => opacity,
            AnimationBrush::Gradient { value, .. } => &mut value.opacity,
        };
        return Some(Component::Number(value, Units::Normalized));
    }
    let AnimationBrush::Gradient { value, .. } = brush else {
        return None;
    };
    if let Stop { index, component } = channel {
        let stop = value.stops.get_mut(index.get() - 1)?;
        return if component == C::StopOffset {
            Some((&mut stop.offset).into())
        } else {
            color(component, &mut stop.argb)
        };
    }
    let Field { field, component } = channel else {
        return None;
    };
    use GradientShape::{Conic, Linear, Radial};
    match (field, &mut value.shape) {
        (F::TransformOrigin, _) => point(component, &mut value.transform_origin),
        (F::Start, Linear { start, .. }) => point(component, start),
        (F::End, Linear { end, .. }) => point(component, end),
        (F::Center, Radial { center, .. } | Conic { center, .. }) => point(component, center),
        (F::GradientOrigin, Radial { origin, .. }) => point(component, origin),
        (F::RadiusX, Radial { radius_x, .. }) => distance(component, radius_x),
        (F::RadiusY, Radial { radius_y, .. }) => distance(component, radius_y),
        (F::Angle, Conic { angle, .. }) if component == C::Angle => Some(angle.into()),
        _ => None,
    }
}

fn point(channel: CurveChannel, point: &mut GradientPoint) -> Option<Component<'_>> {
    let units = relative_units(channel, point.unit)?;
    let value = match channel {
        CurveChannel::PercentX | CurveChannel::AbsoluteX => &mut point.x,
        CurveChannel::PercentY | CurveChannel::AbsoluteY => &mut point.y,
        _ => return None,
    };
    Some(Component::Number(value, units))
}

fn distance(channel: CurveChannel, value: &mut GradientDistance) -> Option<Component<'_>> {
    match channel {
        CurveChannel::Percent | CurveChannel::Absolute => Some(Component::Number(
            &mut value.scalar,
            relative_units(channel, value.unit)?,
        )),
        _ => None,
    }
}

fn relative_units(channel: CurveChannel, unit: RelativeUnit) -> Option<Units> {
    (channel.relative_unit() == Some(unit)).then_some(match unit {
        RelativeUnit::Relative => Units::Percent,
        RelativeUnit::Absolute => Units::Direct,
    })
}

fn transform(
    channel: CurveChannel,
    operation: &mut AnimationTransformOperation,
) -> Option<Component<'_>> {
    use AnimationTransformOperation::{Matrix, Rotate, Scale, Skew, Translate};
    use CurveChannel as C;
    Some(match (channel, operation) {
        (C::AbsoluteX, Translate(values)) | (C::X, Scale(values)) => (&mut values[0]).into(),
        (C::AbsoluteY, Translate(values)) | (C::Y, Scale(values)) => (&mut values[1]).into(),
        (C::Angle, Rotate(angle)) => Component::Number(angle, Units::Degrees),
        (C::AngleX, Skew(values)) => Component::Number(&mut values[0], Units::Degrees),
        (C::AngleY, Skew(values)) => Component::Number(&mut values[1], Units::Degrees),
        (channel, Matrix(values)) => values.get_mut(matrix_index(channel)?)?.into(),
        _ => return None,
    })
}

fn matrix_index(channel: CurveChannel) -> Option<usize> {
    use CurveChannel::*;
    Some(match channel {
        M11 => 0,
        M12 => 1,
        M21 => 2,
        M22 => 3,
        M31 => 4,
        M32 => 5,
        _ => return None,
    })
}
