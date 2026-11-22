use crate::{
    require, AnimationEffect, AnimationError, AnimationValue, CurvePoint, EffectForm, FillMode, RelativeUnit, Result, Track,
    ValueKind,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// A numeric component of an authored animation value, in the property's units.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveChannel {
    #[default]
    Value,
    X,
    Y,
    Width,
    Height,
    Left,
    Top,
    Right,
    Bottom,
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Red,
    Green,
    Blue,
    Alpha,
    PercentX,
    PercentY,
    AbsoluteX,
    AbsoluteY,
    Percent,
    Absolute,
    BlurRadius,
    OffsetX,
    OffsetY,
    Direction,
    ShadowDepth,
    ShadowOpacity,
}

impl FromStr for CurveChannel {
    type Err = serde::de::value::Error;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Self::deserialize(serde::de::value::StrDeserializer::new(value))
    }
}

impl CurveChannel {
    pub fn for_authored_kind(kind: &str) -> &'static [Self] {
        use serde::de::value::{Error, StrDeserializer};
        ValueKind::deserialize(StrDeserializer::<Error>::new(kind))
            .map(Self::for_kind)
            .unwrap_or_default()
    }

    pub fn for_kind(kind: ValueKind) -> &'static [Self] {
        use CurveChannel::*;
        match kind {
            ValueKind::Number => &[Value],
            ValueKind::Color => &[Red, Green, Blue, Alpha],
            ValueKind::Point | ValueKind::Vector => &[X, Y],
            ValueKind::Size => &[Width, Height],
            ValueKind::Rect => &[X, Y, Width, Height],
            ValueKind::Thickness => &[Left, Top, Right, Bottom],
            ValueKind::CornerRadius => &[TopLeft, TopRight, BottomRight, BottomLeft],
            ValueKind::RelativePoint => &[PercentX, PercentY, AbsoluteX, AbsoluteY],
            ValueKind::RelativeScalar => &[Percent, Absolute],
            ValueKind::Effect => &[BlurRadius, OffsetX, OffsetY, Direction, ShadowDepth, ShadowOpacity, Red, Green, Blue, Alpha],
            _ => &[],
        }
    }

    pub fn for_track(track: &Track, duration: f64, fill: FillMode) -> Vec<Self> {
        let Some(first) = track.keyframes.first() else {
            return Vec::new();
        };
        if let AnimationValue::Effect(effect) = &first.value {
            let mut channels = Self::effect_channels(effect).to_vec();
            let mut has_object = false;
            for key in &track.keyframes {
                let AnimationValue::Effect(value) = &key.value else { return Vec::new() };
                if value.argb().is_some() != effect.argb().is_some() {
                    return Vec::new();
                }
                has_object |= value.form() != EffectForm::Literal;
                channels.retain(|channel| Self::effect_channels(value).contains(channel));
            }
            if !has_object {
                channels.retain(|channel| *channel != Self::ShadowOpacity);
            }
            return channels;
        }
        let mut channels = Self::for_kind(first.value.kind()).to_vec();
        if let Some(unit) = first.value.relative_unit() {
            // The live target may supply the units at an unauthored endpoint.
            let uses_base = (first.time > 0.0 && !matches!(fill, FillMode::Backward | FillMode::Both))
                || (!matches!(fill, FillMode::Forward | FillMode::Both)
                    && track.keyframes.last().is_some_and(|last| last.time < duration));
            let base_may_use_other_unit = uses_base && track.base_value.as_ref()
                .and_then(AnimationValue::relative_unit).is_none_or(|base_unit| base_unit != unit);
            let has_other_unit = track
                .keyframes
                .iter()
                .any(|key| key.value.relative_unit() != Some(unit));
            if base_may_use_other_unit || has_other_unit {
                channels.sort_by_key(|channel| channel.relative_unit() != Some(unit));
            } else {
                channels.retain(|channel| channel.relative_unit() == Some(unit));
            }
        }
        channels
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Value => "Value",
            Self::X => "X",
            Self::Y => "Y",
            Self::Width => "Width",
            Self::Height => "Height",
            Self::Left => "Left",
            Self::Top => "Top",
            Self::Right => "Right",
            Self::Bottom => "Bottom",
            Self::TopLeft => "Top left",
            Self::TopRight => "Top right",
            Self::BottomRight => "Bottom right",
            Self::BottomLeft => "Bottom left",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Alpha => "Alpha",
            Self::PercentX => "X (%)",
            Self::PercentY => "Y (%)",
            Self::AbsoluteX => "X (px)",
            Self::AbsoluteY => "Y (px)",
            Self::Percent => "Percent",
            Self::Absolute => "Pixels",
            Self::BlurRadius => "Blur radius (px)",
            Self::OffsetX => "Offset X (px)",
            Self::OffsetY => "Offset Y (px)",
            Self::Direction => "Direction (°)",
            Self::ShadowDepth => "Shadow depth (px)",
            Self::ShadowOpacity => "Shadow opacity",
        }
    }

    pub fn is_color(self) -> bool {
        matches!(self, Self::Red | Self::Green | Self::Blue | Self::Alpha)
    }

    pub fn relative_unit(self) -> Option<RelativeUnit> {
        match self {
            Self::PercentX | Self::PercentY | Self::Percent => Some(RelativeUnit::Relative),
            Self::AbsoluteX | Self::AbsoluteY | Self::Absolute => Some(RelativeUnit::Absolute),
            _ => None,
        }
    }

    pub(crate) fn project_samples(
        self,
        samples: impl IntoIterator<Item = Result<(f64, Option<AnimationValue>)>>,
    ) -> Result<Vec<CurvePoint>> {
        let mut points = Vec::new();
        let mut gap = false;
        let mut effect_family = None;
        for sample in samples {
            let (time, sample) = sample?;
            let Some(sample) = sample else {
                gap = true;
                continue;
            };
            if let AnimationValue::Effect(effect) = &sample {
                let family = effect.argb().is_some();
                gap |= effect_family.is_some_and(|previous| previous != family);
                effect_family = Some(family);
            }
            require(
                Self::for_kind(sample.kind()).contains(&self),
                "The track does not produce the selected curve channel",
            )?;
            if self
                .relative_unit()
                .is_some_and(|unit| sample.relative_unit() != Some(unit))
                || matches!(&sample, AnimationValue::Effect(effect) if !Self::effect_channels(effect).contains(&self))
            {
                gap = true;
                continue;
            }
            let value = self.project(&sample).ok_or_else(|| {
                AnimationError("The track has no finite sample for the selected channel".into())
            })?;
            points.push(CurvePoint {
                break_before: std::mem::take(&mut gap) && !points.is_empty(),
                ..CurvePoint::new(time, value)
            });
        }
        require(
            !points.is_empty(),
            "The selected channel has no samples in this time range",
        )?;
        Ok(points)
    }

    fn project(self, value: &AnimationValue) -> Option<f64> {
        use AnimationValue as V;
        let sample = match (self, value) {
            (Self::Value, V::Number(n)) => *n,
            (Self::Red, V::Color { r, .. }) => *r,
            (Self::Green, V::Color { g, .. }) => *g,
            (Self::Blue, V::Color { b, .. }) => *b,
            (Self::Alpha, V::Color { a, .. }) => *a,
            (Self::X, V::Point { x, .. } | V::Vector { x, .. } | V::Rect { x, .. }) => *x,
            (Self::Y, V::Point { y, .. } | V::Vector { y, .. } | V::Rect { y, .. }) => *y,
            (Self::Width, V::Size { width, .. } | V::Rect { width, .. }) => *width,
            (Self::Height, V::Size { height, .. } | V::Rect { height, .. }) => *height,
            (Self::Left, V::Thickness { left, .. }) => *left,
            (Self::Top, V::Thickness { top, .. }) => *top,
            (Self::Right, V::Thickness { right, .. }) => *right,
            (Self::Bottom, V::Thickness { bottom, .. }) => *bottom,
            (Self::TopLeft, V::CornerRadius { top_left, .. }) => *top_left,
            (Self::TopRight, V::CornerRadius { top_right, .. }) => *top_right,
            (Self::BottomRight, V::CornerRadius { bottom_right, .. }) => *bottom_right,
            (Self::BottomLeft, V::CornerRadius { bottom_left, .. }) => *bottom_left,
            (Self::AbsoluteX | Self::PercentX, V::RelativePoint { x, .. }) => *x,
            (Self::AbsoluteY | Self::PercentY, V::RelativePoint { y, .. }) => *y,
            (Self::Absolute | Self::Percent, V::RelativeScalar { scalar, .. }) => *scalar,
            (channel, V::Effect(effect)) => channel.project_effect(effect)?,
            _ => return None,
        };
        let sample = if self.relative_unit() == Some(RelativeUnit::Relative) {
            sample * 100.0
        } else {
            sample
        };
        sample.is_finite().then_some(sample)
    }

    fn effect_channels(effect: &AnimationEffect) -> &'static [Self] {
        use CurveChannel::*;
        match effect {
            AnimationEffect::Blur { .. } => &[BlurRadius],
            AnimationEffect::DropShadow { .. } => &[BlurRadius, OffsetX, OffsetY, ShadowOpacity, Red, Green, Blue, Alpha],
            AnimationEffect::DirectionalDropShadow { .. } => &[BlurRadius, Direction, ShadowDepth, ShadowOpacity, Red, Green, Blue, Alpha],
        }
    }

    fn project_effect(self, effect: &AnimationEffect) -> Option<f64> {
        let value = match (self, *effect) {
            (Self::BlurRadius, AnimationEffect::Blur { radius, .. }) => radius,
            (Self::BlurRadius, AnimationEffect::DropShadow { blur_radius, .. } | AnimationEffect::DirectionalDropShadow { blur_radius, .. }) => blur_radius,
            (Self::OffsetX, AnimationEffect::DropShadow { offset_x, .. }) => offset_x,
            (Self::OffsetY, AnimationEffect::DropShadow { offset_y, .. }) => offset_y,
            (Self::Direction, AnimationEffect::DirectionalDropShadow { direction, .. }) => direction,
            (Self::ShadowDepth, AnimationEffect::DirectionalDropShadow { shadow_depth, .. }) => shadow_depth,
            (Self::ShadowOpacity, AnimationEffect::DropShadow { opacity, .. } | AnimationEffect::DirectionalDropShadow { opacity, .. }) => opacity,
            (Self::Alpha, shadow) => f64::from(shadow.argb()?[0]) / 255.0,
            (Self::Red, shadow) => f64::from(shadow.argb()?[1]) / 255.0,
            (Self::Green, shadow) => f64::from(shadow.argb()?[2]) / 255.0,
            (Self::Blue, shadow) => f64::from(shadow.argb()?[3]) / 255.0,
            _ => return None,
        };
        Some(value)
    }
}
