use crate::track_curve_channel::parameter_identifier;
use crate::{AnimationEffect, AnimationValue, CurveChannel, Result, Track, TrackCurveChannel};
use ferroui_base::media::{EffectParameterValue, ImmutableCustomEffect};

pub(super) fn validate_parameters(effect: &ImmutableCustomEffect) -> Result<()> {
    for (_, parameter) in effect.parameters() {
        if let Some(value) = numeric_value(parameter) {
            value.validate()?;
        }
    }
    Ok(())
}

/// Give a named channel only to tracks whose authored keys resolve to one
/// registered effect and have that parameter with the same value type.
pub(crate) fn channels(track: &Track) -> Option<Vec<TrackCurveChannel>> {
    let first = track.keyframes.first()?;
    let AnimationValue::Effect(AnimationEffect::Registered { .. }) = &first.value else {
        return None;
    };
    let effects = track
        .keyframes
        .iter()
        .map(|key| {
            let AnimationValue::Effect(effect @ AnimationEffect::Registered { .. }) = &key.value
            else {
                return None;
            };
            effect.to_native().ok()
        })
        .collect::<Option<Vec<_>>>();
    let Some(effects) = effects else {
        return Some(Vec::new());
    };
    let effects = effects
        .iter()
        .filter_map(|effect| effect.as_custom_effect())
        .collect::<Vec<_>>();
    let first = effects.first()?;
    let effect_name = first.name();
    if !parameter_identifier(effect_name)
        || effects.len() != track.keyframes.len()
        || effects.iter().any(|effect| effect.name() != effect_name)
    {
        return Some(Vec::new());
    }
    let mut channels = Vec::new();
    for (parameter, sample) in first.parameters() {
        let Some(value) = numeric_value(sample) else {
            continue;
        };
        if parameter_identifier(parameter)
            && effects.iter().all(|effect| {
                effect
                    .parameter(parameter)
                    .and_then(numeric_value)
                    .is_some_and(|candidate| candidate.kind() == value.kind())
            })
        {
            channels.extend(
                CurveChannel::for_kind(value.kind())
                    .iter()
                    .map(|&component| TrackCurveChannel::EffectParameter {
                        effect: effect_name.into(),
                        parameter: parameter.into(),
                        component,
                    }),
            );
        }
    }
    Some(channels)
}

pub(crate) fn numeric_value(parameter: EffectParameterValue) -> Option<AnimationValue> {
    Some(match parameter {
        EffectParameterValue::Bool(value) => AnimationValue::Number(f64::from(u8::from(value))),
        EffectParameterValue::Int(value) => AnimationValue::Number(f64::from(value)),
        EffectParameterValue::Float(value) => AnimationValue::Number(value),
        EffectParameterValue::Color(color) => {
            AnimationValue::from_argb8(color.to_uint32().to_be_bytes())
        }
        EffectParameterValue::Point(point) => AnimationValue::Point {
            x: point.x,
            y: point.y,
        },
        EffectParameterValue::Vector(vector) => AnimationValue::Vector {
            x: vector.x,
            y: vector.y,
        },
        EffectParameterValue::Size(size) => AnimationValue::Size {
            width: size.width,
            height: size.height,
        },
        EffectParameterValue::Rect(rect) => AnimationValue::Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        },
        _ => return None,
    })
}
