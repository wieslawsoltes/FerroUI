use crate::{AnimationValue, BrushCurveChannel, BrushField, CurveChannel, CurvePoint};
use ferroui_base::{BoxedValue, media::IBrush};
use std::rc::Rc;

pub(super) fn project(
    channel: BrushCurveChannel,
    samples: impl IntoIterator<Item = (f64, BoxedValue)>,
) -> Result<Vec<CurvePoint>, String> {
    let component = channel.component();
    let mut points = Vec::new();
    let mut previous_kind = None;
    for (time, sample) in samples {
        let brush = sample
            .downcast_ref::<Option<Rc<dyn IBrush>>>()
            .ok_or("The native animator did not produce a brush")?;
        let Some((brush, sample)) = brush
            .as_deref()
            .and_then(|brush| read(channel, brush).map(|value| (brush, value)))
            .filter(|(_, sample)| {
                component.relative_unit().is_none()
                    || component.relative_unit() == sample.relative_unit()
            })
        else {
            previous_kind = None;
            continue;
        };
        let value = component
            .project(&sample)
            .ok_or("The brush channel has a nonfinite sample")?;
        let kind = Some(BrushKind::of(brush));
        points.push(CurvePoint {
            break_before: !points.is_empty() && previous_kind != kind,
            ..CurvePoint::new(time, value)
        });
        previous_kind = kind;
    }
    if points.is_empty() {
        return Err("The selected brush channel has no samples in this time range".into());
    }
    Ok(points)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrushKind {
    Linear,
    Radial,
    Conic,
    Other,
}

impl BrushKind {
    fn of(brush: &dyn IBrush) -> Self {
        if brush.as_linear_gradient_brush().is_some() {
            return Self::Linear;
        }
        if brush.as_radial_gradient_brush().is_some() {
            return Self::Radial;
        }
        if brush.as_conic_gradient_brush().is_some() {
            return Self::Conic;
        }
        Self::Other
    }
}

fn read(channel: BrushCurveChannel, brush: &dyn IBrush) -> Option<AnimationValue> {
    let field = match channel {
        BrushCurveChannel::Stop { index, component } => {
            let stops = brush.as_gradient_brush()?.gradient_stops();
            let stop = stops.get(index.get() - 1)?;
            return Some(if component == CurveChannel::StopOffset {
                AnimationValue::Number(stop.offset())
            } else {
                stop.color().into()
            });
        }
        BrushCurveChannel::Field { field, .. } => field,
    };
    use crate::{GradientDistance, GradientPoint};
    Some(match field {
        BrushField::Start => {
            GradientPoint::from(brush.as_linear_gradient_brush()?.start_point()).value()
        }
        BrushField::End => {
            GradientPoint::from(brush.as_linear_gradient_brush()?.end_point()).value()
        }
        BrushField::Center => GradientPoint::from(
            brush
                .as_radial_gradient_brush()
                .map(|brush| brush.center())
                .or_else(|| brush.as_conic_gradient_brush().map(|brush| brush.center()))?,
        )
        .value(),
        BrushField::GradientOrigin => {
            GradientPoint::from(brush.as_radial_gradient_brush()?.gradient_origin()).value()
        }
        BrushField::TransformOrigin => GradientPoint::from(brush.transform_origin()).value(),
        BrushField::RadiusX => {
            GradientDistance::from(brush.as_radial_gradient_brush()?.radius_x()).value()
        }
        BrushField::RadiusY => {
            GradientDistance::from(brush.as_radial_gradient_brush()?.radius_y()).value()
        }
        BrushField::Angle => AnimationValue::Number(brush.as_conic_gradient_brush()?.angle()),
        BrushField::Opacity => AnimationValue::Number(brush.opacity()),
    })
}
