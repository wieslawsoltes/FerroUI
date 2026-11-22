use crate::{AnimationValue, RelativeUnit};
use ferroui_base::{RelativePoint, RelativeScalar};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientPoint {
    pub x: f64,
    pub y: f64,
    pub unit: RelativeUnit,
}

impl GradientPoint {
    pub fn value(self) -> AnimationValue {
        let Self { x, y, unit } = self;
        AnimationValue::RelativePoint { x, y, unit }
    }

    pub fn native(self) -> RelativePoint {
        RelativePoint::new(self.x, self.y, native_unit(self.unit))
    }
}

impl From<RelativePoint> for GradientPoint {
    fn from(point: RelativePoint) -> Self {
        Self {
            x: point.point.x,
            y: point.point.y,
            unit: unit(point.unit),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientDistance {
    pub scalar: f64,
    pub unit: RelativeUnit,
}

impl GradientDistance {
    pub fn value(self) -> AnimationValue {
        let Self { scalar, unit } = self;
        AnimationValue::RelativeScalar { scalar, unit }
    }

    pub fn native(self) -> RelativeScalar {
        RelativeScalar::new(self.scalar, native_unit(self.unit))
    }
}

impl From<RelativeScalar> for GradientDistance {
    fn from(value: RelativeScalar) -> Self {
        Self {
            scalar: value.scalar,
            unit: unit(value.unit),
        }
    }
}

fn native_unit(unit: RelativeUnit) -> ferroui_base::RelativeUnit {
    match unit {
        RelativeUnit::Absolute => ferroui_base::RelativeUnit::Absolute,
        RelativeUnit::Relative => ferroui_base::RelativeUnit::Relative,
    }
}

fn unit(unit: ferroui_base::RelativeUnit) -> RelativeUnit {
    match unit {
        ferroui_base::RelativeUnit::Absolute => RelativeUnit::Absolute,
        ferroui_base::RelativeUnit::Relative => RelativeUnit::Relative,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GradientShape {
    Linear {
        start: GradientPoint,
        end: GradientPoint,
    },
    Radial {
        center: GradientPoint,
        origin: GradientPoint,
        radius_x: GradientDistance,
        radius_y: GradientDistance,
    },
    Conic {
        center: GradientPoint,
        angle: f64,
    },
}

impl GradientShape {
    pub fn class_name(self) -> &'static str {
        match self {
            Self::Linear { .. } => "LinearGradientBrush",
            Self::Radial { .. } => "RadialGradientBrush",
            Self::Conic { .. } => "ConicGradientBrush",
        }
    }

    pub fn defaults(class: &str) -> Option<Self> {
        Some(match class {
            "LinearGradientBrush" => Self::Linear {
                start: RelativePoint::TOP_LEFT.into(),
                end: RelativePoint::BOTTOM_RIGHT.into(),
            },
            "RadialGradientBrush" => Self::Radial {
                center: RelativePoint::CENTER.into(),
                origin: RelativePoint::CENTER.into(),
                radius_x: RelativeScalar::MIDDLE.into(),
                radius_y: RelativeScalar::MIDDLE.into(),
            },
            "ConicGradientBrush" => Self::Conic {
                center: RelativePoint::CENTER.into(),
                angle: 0.0,
            },
            _ => return None,
        })
    }

    pub fn fields(self) -> Vec<(&'static str, AnimationValue)> {
        match self {
            Self::Linear { start, end } => {
                vec![("StartPoint", start.value()), ("EndPoint", end.value())]
            }
            Self::Radial {
                center,
                origin,
                radius_x,
                radius_y,
            } => vec![
                ("Center", center.value()),
                ("GradientOrigin", origin.value()),
                ("RadiusX", radius_x.value()),
                ("RadiusY", radius_y.value()),
            ],
            Self::Conic { center, angle } => vec![
                ("Center", center.value()),
                ("Angle", AnimationValue::Number(angle)),
            ],
        }
    }
}
