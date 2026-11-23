use crate::track_curve_channel::parameter_identifier;
use crate::{AnimationEffect, AnimationError, AnimationValue, CurveChannel, CurveStep, KeyframeComponentEdit, Result, Track, TrackCurveChannel};
use ferroui_base::media::{Color, CustomEffectRegistry, EffectParameterValue, ImmutableCustomEffect};
use ferroui_base::utilities::FormatError;
use ferroui_base::{Point, Rect, Size, Vector};
use std::{collections::BTreeMap, mem::discriminant};

#[cfg(test)]
#[path = "registered_effect_authoring_tests.rs"]
mod tests;

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
        .map(|key| parse_literal(&key.value))
        .collect::<Option<Vec<_>>>();
    let Some(effects) = effects else {
        return Some(Vec::new());
    };
    let first = effects.first()?;
    let effect_name = first.name();
    if !parameter_identifier(effect_name)
        || effects.iter().any(|effect| effect.name() != effect_name)
    {
        return Some(Vec::new());
    }
    let mut channels = Vec::new();
    for (parameter, sample) in first.parameters() {
        let Some(value) = numeric_value(sample.clone()) else {
            continue;
        };
        if parameter_identifier(parameter)
            && effects.iter().all(|effect| {
                effect
                    .parameter(parameter)
                    .is_some_and(|candidate| discriminant(&candidate) == discriminant(&sample))
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

pub(crate) fn parse_literal(value: &AnimationValue) -> Option<ImmutableCustomEffect> {
    let AnimationValue::Effect(AnimationEffect::Registered { source }) = value else {
        return None;
    };
    CustomEffectRegistry::parse(source)?.ok()
}

pub(crate) fn authored_component(
    effect: &ImmutableCustomEffect,
    channel: &TrackCurveChannel,
) -> Option<(f64, Option<CurveStep>)> {
    let TrackCurveChannel::EffectParameter { effect: name, parameter, component } = channel else {
        return None;
    };
    if effect.name() != name {
        return None;
    }
    let parameter = effect.parameter(parameter)?;
    let step = match parameter {
        EffectParameterValue::Bool(_) => Some(CurveStep::Boolean),
        EffectParameterValue::Int(_) => Some(CurveStep::Integer),
        _ => None,
    };
    Some((component.project(&numeric_value(parameter)?)?, step))
}

pub(crate) fn replace_components(value: &AnimationValue, edits: &[KeyframeComponentEdit]) -> Result<AnimationValue> {
    let invalid = || FormatError::new("This key has no writable literal for the registered effect parameter");
    let AnimationValue::Effect(AnimationEffect::Registered { source }) = value else {
        return Err(AnimationError(invalid().to_string()));
    };
    let source = CustomEffectRegistry::edit_literal_parameters(source, |effect| {
        let mut parameters: BTreeMap<&str, Vec<KeyframeComponentEdit>> = BTreeMap::new();
        for edit in edits {
            let TrackCurveChannel::EffectParameter { parameter, component, .. } = &edit.channel else {
                return Err(invalid());
            };
            if let (_, Some(step)) = authored_component(effect, &edit.channel).ok_or_else(invalid)? {
                if step.snap(edit.value) != edit.value {
                    return Err(FormatError::new(match step {
                        CurveStep::Boolean => "Enter 0 or 1 for a Boolean effect parameter",
                        CurveStep::Integer => "Enter a signed 32-bit integer for this effect parameter",
                    }));
                }
            }
            parameters.entry(parameter).or_default().push(KeyframeComponentEdit {
                channel: (*component).into(), value: edit.value,
            });
        }
        parameters.into_iter().map(|(name, edits)| {
            let original = effect.parameter(name).ok_or_else(invalid)?;
            let numeric = numeric_value(original.clone()).ok_or_else(invalid)?;
            let updated = TrackCurveChannel::replace_key_components(&numeric, &edits)
                .map_err(|error| FormatError::from_string(error.0))?;
            Ok((name.to_owned(), parameter_value(original, updated).ok_or_else(invalid)?))
        }).collect()
    }).map_err(|error| AnimationError(error.to_string()))?;
    let updated = AnimationValue::Effect(AnimationEffect::Registered { source });
    updated.validate()?;
    Ok(updated)
}

fn parameter_value(original: EffectParameterValue, value: AnimationValue) -> Option<EffectParameterValue> {
    use AnimationValue as V;
    use EffectParameterValue as P;
    Some(match (original, value) {
        (P::Bool(_), V::Number(value)) => P::Bool(value == 1.0),
        (P::Int(_), V::Number(value)) => P::Int(value as i32),
        (P::Float(_), V::Number(value)) => P::Float(value),
        (P::Color(_), value) => {
            let [a, r, g, b] = value.argb8()?;
            P::Color(Color::from_argb(a, r, g, b))
        }
        (P::Point(_), V::Point { x, y }) => P::Point(Point::new(x, y)),
        (P::Vector(_), V::Vector { x, y }) => P::Vector(Vector::new(x, y)),
        (P::Size(_), V::Size { width, height }) => P::Size(Size::new(width, height)),
        (P::Rect(_), V::Rect { x, y, width, height }) => P::Rect(Rect::new(x, y, width, height)),
        _ => return None,
    })
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
