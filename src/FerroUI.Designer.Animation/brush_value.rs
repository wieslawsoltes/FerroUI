//! Literal brush keyframes use the same values as native playback.

mod geometry;
mod native;
mod xaml;
mod xaml_write;

use crate::{AnimationTransformList, AnimationValue, Result};
use ferroui_base::{RelativePoint, media::GradientSpreadMethod};
use serde::{Deserialize, Serialize};

pub use geometry::{GradientDistance, GradientPoint, GradientShape};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientSpread {
    #[default]
    Pad,
    Reflect,
    Repeat,
}

impl GradientSpread {
    pub fn name(self) -> &'static str {
        match self {
            Self::Pad => "Pad",
            Self::Reflect => "Reflect",
            Self::Repeat => "Repeat",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "Pad" => Some(Self::Pad),
            "Reflect" => Some(Self::Reflect),
            "Repeat" => Some(Self::Repeat),
            _ => None,
        }
    }
}

impl From<GradientSpreadMethod> for GradientSpread {
    fn from(value: GradientSpreadMethod) -> Self {
        match value {
            GradientSpreadMethod::Pad => Self::Pad,
            GradientSpreadMethod::Reflect => Self::Reflect,
            GradientSpreadMethod::Repeat => Self::Repeat,
        }
    }
}

impl From<GradientSpread> for GradientSpreadMethod {
    fn from(value: GradientSpread) -> Self {
        match value {
            GradientSpread::Pad => Self::Pad,
            GradientSpread::Reflect => Self::Reflect,
            GradientSpread::Repeat => Self::Repeat,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimationGradientStop {
    pub offset: f64,
    pub argb: [u8; 4],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimationGradient {
    pub shape: GradientShape,
    pub stops: Vec<AnimationGradientStop>,
    pub opacity: f64,
    pub spread: GradientSpread,
    pub transform_origin: GradientPoint,
    pub transform: Option<AnimationTransformList>,
    pub relative_transform: Option<AnimationTransformList>,
}

impl AnimationGradient {
    pub fn from_stops(shape: GradientShape, stops: Vec<AnimationGradientStop>) -> Self {
        Self {
            shape,
            stops,
            opacity: 1.0,
            spread: GradientSpread::Pad,
            transform_origin: RelativePoint::default().into(),
            transform: None,
            relative_transform: None,
        }
    }

    fn validate(&self) -> Result<()> {
        self.transform_origin.value().validate()?;
        for (_, value) in self.shape.fields() {
            value.validate()?;
        }
        for list in [&self.transform, &self.relative_transform]
            .into_iter()
            .flatten()
        {
            list.validate()?;
        }
        AnimationValue::Number(self.opacity).validate()?;
        for stop in &self.stops {
            AnimationValue::Number(stop.offset).validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrushForm {
    #[default]
    Literal,
    Object,
    PropertyElement,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnimationBrush {
    Gradient {
        value: Box<AnimationGradient>,
        #[serde(default)]
        wrapped: bool,
    },
    Solid {
        argb: [u8; 4],
        opacity: f64,
        #[serde(default)]
        form: BrushForm,
    },
}

impl PartialEq for AnimationBrush {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Gradient { value: left, .. }, Self::Gradient { value: right, .. }) => {
                left == right
            }
            (
                Self::Solid {
                    argb: left,
                    opacity,
                    ..
                },
                Self::Solid {
                    argb: right,
                    opacity: other,
                    ..
                },
            ) => left == right && opacity == other,
            _ => false,
        }
    }
}

impl AnimationBrush {
    pub fn gradient(gradient: AnimationGradient) -> Self {
        Self::Gradient {
            value: Box::new(gradient),
            wrapped: false,
        }
    }

    pub fn solid(argb: [u8; 4]) -> Self {
        Self::Solid {
            argb,
            opacity: 1.0,
            form: BrushForm::Literal,
        }
    }

    pub fn as_gradient(&self) -> Option<&AnimationGradient> {
        match self {
            Self::Gradient { value, .. } => Some(value),
            Self::Solid { .. } => None,
        }
    }

    pub(crate) fn keep_source_form(&mut self, authored: &Self) {
        match (self, authored) {
            (Self::Gradient { wrapped, .. }, Self::Gradient { wrapped: prior, .. }) => {
                *wrapped = *prior;
            }
            (Self::Solid { form, opacity, .. }, Self::Solid { form: prior, .. })
                if *prior != BrushForm::Literal || *opacity == 1.0 =>
            {
                *form = *prior;
            }
            _ => {}
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        match self {
            Self::Gradient { value, .. } => value.validate(),
            Self::Solid { opacity, form, .. } => {
                crate::require(
                    *form != BrushForm::Literal || *opacity == 1.0,
                    "Use a SolidColorBrush object for brush opacity",
                )?;
                AnimationValue::Number(*opacity).validate()
            }
        }
    }
}
