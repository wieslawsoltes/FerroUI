//! Edit one authored component without replacing the other fields of a key.
#[cfg(feature = "native-values")]
mod native;
#[cfg(test)]
#[path = "keyframe_component_tests.rs"]
mod tests;

use crate::{
    require, AnimationEffect, AnimationError, AnimationProgram, AnimationValue, CurveChannel,
    EffectForm, RelativeUnit, Result, TrackCurveChannel,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyframeComponentEdit {
    pub channel: TrackCurveChannel,
    /// Uses the same units as the value curve for this channel.
    pub value: f64,
}

impl AnimationProgram {
    pub fn keyframe_component(
        &self,
        target: u64,
        property: &str,
        previous: f64,
        edit: &KeyframeComponentEdit,
    ) -> Result<AnimationValue> {
        let key = self
            .timeline
            .as_ref()
            .and_then(|timeline| timeline.track(target, property))
            .and_then(|track| track.keyframes.iter().find(|key| key.time == previous))
            .ok_or_else(|| {
                AnimationError("Choose an existing authored keyframe for this component".into())
            })?;
        edit.channel.replace_key_component(&key.value, edit.value)
    }
}

impl TrackCurveChannel {
    pub fn authored_key_value(&self, value: &AnimationValue) -> Option<f64> {
        let component = slot(self, &mut value.clone())?.get();
        component.is_finite().then_some(component)
    }

    pub fn uses_normalized_values(&self) -> bool {
        let component = self.component();
        if component.is_color() || component == CurveChannel::ShadowOpacity {
            return true;
        }
        #[cfg(feature = "native-values")]
        if matches!(
            self,
            Self::Brush(crate::BrushCurveChannel::Field {
                field: crate::BrushField::Opacity,
                ..
            })
        ) {
            return true;
        }
        false
    }

    /// Values of authored handles only. A missing component or a derived
    /// channel remains available for sampling and horizontal retiming.
    pub fn key_components(channels: &[Self], value: &AnimationValue) -> Vec<KeyframeComponentEdit> {
        if channels.is_empty() {
            return Vec::new();
        }
        let mut value = value.clone();
        channels
            .iter()
            .filter_map(|channel| {
                let component = slot(channel, &mut value)?.get();
                component.is_finite().then(|| KeyframeComponentEdit {
                    channel: channel.clone(),
                    value: component,
                })
            })
            .collect()
    }

    pub fn replace_key_component(
        &self,
        value: &AnimationValue,
        next: f64,
    ) -> Result<AnimationValue> {
        require(next.is_finite(), "Enter a finite curve value")?;
        let mut edited = value.clone();
        let component = slot(self, &mut edited).ok_or_else(|| {
            AnimationError("This channel has no editable component on the authored keyframe".into())
        })?;
        if component.get() == next {
            return Ok(value.clone());
        }
        component.set(next)?;
        #[cfg(feature = "native-values")]
        native::keep_writable_form(&mut edited);
        edited.validate()?;
        Ok(edited)
    }
}

#[derive(Clone, Copy)]
enum Units {
    Direct,
    Percent,
    #[cfg(feature = "native-values")]
    Degrees,
    Normalized,
}

enum Component<'a> {
    Number(&'a mut f64, Units),
    Color(&'a mut u8),
}

impl<'a> From<&'a mut f64> for Component<'a> {
    fn from(value: &'a mut f64) -> Self {
        Component::Number(value, Units::Direct)
    }
}

impl Component<'_> {
    fn get(&self) -> f64 {
        match self {
            Self::Number(value, Units::Direct | Units::Normalized) => **value,
            Self::Number(value, Units::Percent) => **value * 100.0,
            #[cfg(feature = "native-values")]
            Self::Number(value, Units::Degrees) => value.to_degrees(),
            Self::Color(value) => f64::from(**value) / 255.0,
        }
    }

    fn set(self, next: f64) -> Result<()> {
        match self {
            Self::Number(value, Units::Direct) => *value = next,
            Self::Number(value, Units::Percent) => *value = next / 100.0,
            #[cfg(feature = "native-values")]
            Self::Number(value, Units::Degrees) => *value = next.to_radians(),
            Self::Number(value, Units::Normalized) => {
                normalized(next)?;
                *value = next;
            }
            Self::Color(value) => {
                normalized(next)?;
                *value = (255.0 * next).round() as u8;
            }
        }
        Ok(())
    }
}

fn normalized(value: f64) -> Result<()> {
    require(
        (0.0..=1.0).contains(&value),
        "Enter a value between 0 and 1",
    )
}

fn slot<'a>(channel: &TrackCurveChannel, value: &'a mut AnimationValue) -> Option<Component<'a>> {
    match channel {
        TrackCurveChannel::Property(channel) => property(*channel, value),
        #[cfg(feature = "native-values")]
        _ => native::slot(channel, value),
        #[cfg(not(feature = "native-values"))]
        _ => None,
    }
}

fn property(channel: CurveChannel, value: &mut AnimationValue) -> Option<Component<'_>> {
    use AnimationValue as V;
    use CurveChannel as C;
    let unit = channel.relative_unit();
    if unit.is_some() && unit != value.relative_unit() {
        return None;
    }
    let value = match (channel, value) {
        (C::Value, V::Number(value)) => value,
        (C::X, V::Point { x, .. } | V::Vector { x, .. } | V::Rect { x, .. }) => x,
        (C::Y, V::Point { y, .. } | V::Vector { y, .. } | V::Rect { y, .. }) => y,
        (C::Width, V::Size { width, .. } | V::Rect { width, .. }) => width,
        (C::Height, V::Size { height, .. } | V::Rect { height, .. }) => height,
        (C::TopLeft, V::CornerRadius { top_left, .. }) => top_left,
        (C::TopRight, V::CornerRadius { top_right, .. }) => top_right,
        (C::BottomRight, V::CornerRadius { bottom_right, .. }) => bottom_right,
        (C::BottomLeft, V::CornerRadius { bottom_left, .. }) => bottom_left,
        (C::Left, V::Thickness { left, .. }) => left,
        (C::Top, V::Thickness { top, .. }) => top,
        (C::Right, V::Thickness { right, .. }) => right,
        (C::Bottom, V::Thickness { bottom, .. }) => bottom,
        (C::PercentX | C::AbsoluteX, V::RelativePoint { x, .. }) => x,
        (C::PercentY | C::AbsoluteY, V::RelativePoint { y, .. }) => y,
        (C::Percent | C::Absolute, V::RelativeScalar { scalar, .. }) => scalar,
        (C::Alpha, V::Color { a, .. }) => return Some(Component::Number(a, Units::Normalized)),
        (C::Red, V::Color { r, .. }) => return Some(Component::Number(r, Units::Normalized)),
        (C::Green, V::Color { g, .. }) => return Some(Component::Number(g, Units::Normalized)),
        (C::Blue, V::Color { b, .. }) => return Some(Component::Number(b, Units::Normalized)),
        (channel, V::Effect(effect)) => return effect_component(channel, effect),
        _ => return None,
    };
    let units = if unit == Some(RelativeUnit::Relative) {
        Units::Percent
    } else {
        Units::Direct
    };
    Some(Component::Number(value, units))
}

fn effect_component(channel: CurveChannel, effect: &mut AnimationEffect) -> Option<Component<'_>> {
    use AnimationEffect::{Blur, DirectionalDropShadow, DropShadow};
    use CurveChannel as C;
    Some(match (channel, effect) {
        (C::BlurRadius, Blur { radius, .. }) => radius.into(),
        (
            C::BlurRadius,
            DropShadow { blur_radius, .. } | DirectionalDropShadow { blur_radius, .. },
        ) => blur_radius.into(),
        (C::OffsetX, DropShadow { offset_x, .. }) => offset_x.into(),
        (C::OffsetY, DropShadow { offset_y, .. }) => offset_y.into(),
        (C::Direction, DirectionalDropShadow { direction, .. }) => direction.into(),
        (C::ShadowDepth, DirectionalDropShadow { shadow_depth, .. }) => shadow_depth.into(),
        (
            C::ShadowOpacity,
            DropShadow { opacity, form, .. } | DirectionalDropShadow { opacity, form, .. },
        ) => {
            if *form == EffectForm::Literal {
                return None;
            }
            Component::Number(opacity, Units::Normalized)
        }
        (channel, DropShadow { argb, .. } | DirectionalDropShadow { argb, .. }) => {
            return color(channel, argb)
        }
        _ => return None,
    })
}

fn color(channel: CurveChannel, argb: &mut [u8; 4]) -> Option<Component<'_>> {
    let index = match channel {
        CurveChannel::Alpha => 0,
        CurveChannel::Red => 1,
        CurveChannel::Green => 2,
        CurveChannel::Blue => 3,
        _ => return None,
    };
    Some(Component::Color(&mut argb[index]))
}
