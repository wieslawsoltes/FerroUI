use crate::{CurvePoint, CurveWindow};
use ferroui_base::animation::{Animatable, Animation};
use ferroui_base::FerroProperty;

/// Read the same scalar animator used by native playback. Resolve against the
/// loaded setters so the curve uses the exact registered property.
pub fn sample_scalar_track(
    animation: &Animation,
    target: &Animatable,
    name: &str,
    window: CurveWindow,
) -> Result<Vec<CurvePoint>, String> {
    let duration = animation.duration().total_seconds();
    let mut property: Option<&'static FerroProperty> = None;
    let mut keys = Vec::new();
    for frame in animation.children().snapshot().iter() {
        for setter in frame.setters().snapshot().iter() {
            let Some(candidate) = setter.property().filter(|p| matches_property(p, name)) else {
                continue;
            };
            if property.is_some_and(|registered| !std::ptr::eq(registered, candidate)) {
                return Err("Choose an unambiguous native animation property".into());
            }
            property = Some(candidate);
            keys.push(frame.cue().cue_value() * duration);
        }
    }
    let property = property.ok_or("The loaded animation has no matching native track")?;
    let times = window
        .sample_times(duration, keys)
        .map_err(|error| error.to_string())?;
    let positions = times.iter().map(|time| time / duration).collect::<Vec<_>>();
    let values = animation
        .sample_track_for_preview(target, property, &positions)
        .map_err(|error| error.to_string())?;
    times
        .into_iter()
        .zip(values)
        .map(|(time, value)| {
            let value = value
                .as_ref()
                .downcast_ref::<f64>()
                .copied()
                .or_else(|| value.as_ref().downcast_ref::<f32>().copied().map(f64::from))
                .filter(|value| value.is_finite())
                .ok_or("The native animation does not produce a finite scalar")?;
            Ok(CurvePoint { time, value })
        })
        .collect::<Result<_, &str>>()
        .map_err(str::to_owned)
}

pub(super) fn matches_property(property: &FerroProperty, name: &str) -> bool {
    if let Some((owner, name)) = name.rsplit_once('.') {
        owner.rsplit(':').next() == Some(property.owner_type().name()) && name == property.name()
    } else {
        !property.is_attached() && name == property.name()
    }
}
