use super::curve_value;
use crate::{CurveChannel, CurvePoint, CurveWindow};
use ferroui_base::animation::{Animatable, Animation};
use ferroui_base::{BoxedValue, FerroProperty};

/// Read the same scalar animator used by native playback. Resolve against the
/// loaded setters so the curve uses the exact registered property.
pub fn sample_scalar_track(
    animation: &Animation,
    target: &Animatable,
    name: &str,
    window: CurveWindow,
) -> Result<Vec<CurvePoint>, String> {
    sample_curve_track(animation, target, name, CurveChannel::Value, window)
}

pub fn sample_curve_track(
    animation: &Animation,
    target: &Animatable,
    name: &str,
    channel: CurveChannel,
    window: CurveWindow,
) -> Result<Vec<CurvePoint>, String> {
    let duration = animation.duration().total_seconds();
    let (property, keys) = track(animation, name)?;
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
            let value = curve_value::value(&value)
                .and_then(|value| channel.project(&value))
                .ok_or("The native animation does not produce a finite sample for the selected channel")?;
            Ok(CurvePoint { time, value })
        })
        .collect::<Result<_, &str>>()
        .map_err(str::to_owned)
}

/// Read one effective value from the same native animator used to draw the curve.
pub fn sample_keyframe_value(
    animation: &Animation,
    target: &Animatable,
    name: &str,
    time: f64,
) -> Result<BoxedValue, String> {
    let (property, _) = track(animation, name)?;
    let position = time / animation.duration().total_seconds();
    animation
        .sample_track_for_preview(target, property, &[position])
        .map_err(|error| error.to_string())?
        .pop()
        .ok_or_else(|| "The native track did not return a value".into())
}

fn track(animation: &Animation, name: &str) -> Result<(&'static FerroProperty, Vec<f64>), String> {
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
    Ok((
        property.ok_or("The loaded animation has no matching native track")?,
        keys,
    ))
}

pub(super) fn matches_property(property: &FerroProperty, name: &str) -> bool {
    if let Some((owner, name)) = name.rsplit_once('.') {
        owner.rsplit(':').next() == Some(property.owner_type().name()) && name == property.name()
    } else {
        !property.is_attached() && name == property.name()
    }
}
