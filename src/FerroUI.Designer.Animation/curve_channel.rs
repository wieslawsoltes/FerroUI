use crate::{AnimationValue, ValueKind};
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
}

impl FromStr for CurveChannel {
    type Err = serde::de::value::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
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
            _ => &[],
        }
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
        }
    }

    pub fn is_color(self) -> bool {
        matches!(self, Self::Red | Self::Green | Self::Blue | Self::Alpha)
    }

    pub(crate) fn project(self, value: &AnimationValue) -> Option<f64> {
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
            _ => return None,
        };
        sample.is_finite().then_some(sample)
    }
}
