use super::curve_value;
use crate::{value::registered_effect, AnimationEffect, AnimationError, AnimationValue, CurveChannel, CurvePoint, CurveWindow, TrackCurveChannel};
use ferroui_base::animation::{Animatable, Animation};
use ferroui_base::media::IEffect;
use ferroui_base::{BoxedValue, FerroProperty};
use std::rc::Rc;

#[cfg(test)]
#[path = "effect_curve_tests.rs"]
mod effect_tests;
#[cfg(test)]
#[path = "registered_effect_tests.rs"]
mod registered_effect_tests;

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
    channel: impl Into<TrackCurveChannel>,
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
    let channel = channel.into();
    let samples = times.into_iter().zip(values);
    if let TrackCurveChannel::BoxShadow { layer, component } = channel {
        return super::box_shadow_curve::project(layer, component, samples);
    }
    let effect = AnimationEffect::is_effect_property(property);
    let component = channel.component();
    component.project_samples(samples.map(|(time, sample)| {
        let value = if let TrackCurveChannel::EffectParameter { effect, parameter, .. } = &channel {
            effect_parameter(&sample, effect, parameter)
                .filter(|value| CurveChannel::for_kind(value.kind()).contains(&component))
        } else {
            curve_value::value(&sample)
        };
        if value.is_none() && !effect {
            return Err(AnimationError("The native animation does not produce a sample for the selected channel".into()));
        }
        Ok((time, value))
    })).map_err(|error| error.to_string())
}

fn effect_parameter(sample: &BoxedValue, effect: &str, parameter: &str) -> Option<AnimationValue> {
    let sample = sample.downcast_ref::<Option<Rc<dyn IEffect>>>()?.as_deref()?.as_custom_effect()?;
    if sample.name() != effect {
        return None;
    }
    sample.parameter(parameter).and_then(registered_effect::numeric_value)
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

/// Capture a native sample in the portable authoring format.
pub fn sample_typed_keyframe(
    animation: &Animation,
    target: &Animatable,
    name: &str,
    time: f64,
) -> Result<AnimationValue, String> {
    let sample = sample_keyframe_value(animation, target, name, time)?;
    curve_value::value(&sample).ok_or_else(|| "This native value has no typed keyframe representation".into())
}

pub(super) fn track(animation: &Animation, name: &str) -> Result<(&'static FerroProperty, Vec<f64>), String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::NativePlayback;
    use crate::CurveMetric;
    use ferroui_base::animation::{FillMode, KeyFrame, SignedDuration};
    use ferroui_base::media::RadialGradientBrush;
    use ferroui_base::styling::Setter;
    use ferroui_base::{RelativeScalar, RelativeUnit};

    #[test]
    fn relative_scalar_curves_match_the_live_native_units_and_pose() {
        let brush = RadialGradientBrush::new();
        let animation = Animation::new();
        animation.set_duration(SignedDuration::from_seconds(4.0));
        animation.set_fill_mode(FillMode::Both);
        for (time, value) in [(0.0, "10%"), (1.0, "70%"), (2.0, "24"), (3.0, "20%"), (4.0, "60%")] {
            animation.children().add(KeyFrame::with_key_time(SignedDuration::from_seconds(time), [
                Setter::new(RadialGradientBrush::radius_x_property(), RelativeScalar::parse(value).unwrap()) as _,
            ]));
        }
        let window = CurveWindow { start: 0.0, end: 4.0 };
        let percent = sample_curve_track(&animation, &brush, "RadiusX", CurveChannel::Percent, window).unwrap();
        let pixels = sample_curve_track(&animation, &brush, "RadiusX", CurveChannel::Absolute, window).unwrap();
        assert_eq!(brush.radius_x(), RelativeScalar::MIDDLE);
        assert!(percent.iter().any(|point| point.time == 2.5 && point.break_before));
        let velocity = CurveMetric::Velocity.measure(percent.clone()).unwrap();
        assert!(velocity.iter().any(|point| point.time > 2.5 && point.break_before));
        assert!(velocity.iter().all(|point| !(1.5..2.5).contains(&point.time)));
        let mut playback = NativePlayback::default();
        for (time, unit, points) in [(0.5, RelativeUnit::Relative, &percent), (1.5, RelativeUnit::Absolute, &pixels),
            (2.5, RelativeUnit::Relative, &percent), (3.5, RelativeUnit::Relative, &percent)]
        {
            playback.seek(vec![(animation.clone(), brush.clone().upcast())], time).unwrap();
            let actual = brush.radius_x();
            let scale = if unit == RelativeUnit::Relative { 100.0 } else { 1.0 };
            assert_eq!(actual.unit, unit);
            assert_eq!(points.iter().find(|point| point.time == time).unwrap().value, actual.scalar * scale);
        }
        playback.stop();
        assert_eq!(brush.radius_x(), RelativeScalar::MIDDLE);
    }
}
