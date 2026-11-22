use crate::{CurveChannel, CurvePoint};
use ferroui_base::{BoxedValue, media::BoxShadows};
use std::num::NonZeroUsize;

pub(super) fn project(
    layer: NonZeroUsize,
    component: CurveChannel,
    samples: impl IntoIterator<Item = (f64, BoxedValue)>,
) -> Result<Vec<CurvePoint>, String> {
    let index = layer.get() - 1;
    let mut points = Vec::new();
    let mut previous_mode = None;
    for (time, sample) in samples {
        let list = sample
            .downcast_ref::<BoxShadows>()
            .ok_or("The native animator did not produce a box shadow collection")?;
        if index >= list.count() {
            previous_mode = None;
            continue;
        }
        let shadow = list.get(index);
        let value = match component {
            CurveChannel::OffsetX => shadow.offset_x,
            CurveChannel::OffsetY => shadow.offset_y,
            CurveChannel::BlurRadius => shadow.blur,
            CurveChannel::Spread => shadow.spread,
            CurveChannel::Red => f64::from(shadow.color.r) / 255.0,
            CurveChannel::Green => f64::from(shadow.color.g) / 255.0,
            CurveChannel::Blue => f64::from(shadow.color.b) / 255.0,
            CurveChannel::Alpha => f64::from(shadow.color.a) / 255.0,
            _ => return Err("Choose a box shadow component".into()),
        };
        if !value.is_finite() {
            return Err("The box shadow channel has a nonfinite sample".into());
        }
        let current_mode = Some(shadow.is_inset);
        points.push(CurvePoint {
            break_before: !points.is_empty() && previous_mode != current_mode,
            ..CurvePoint::new(time, value)
        });
        previous_mode = current_mode;
    }
    if points.is_empty() {
        return Err(format!("Shadow {layer} is absent in the selected range"));
    }
    Ok(points)
}
