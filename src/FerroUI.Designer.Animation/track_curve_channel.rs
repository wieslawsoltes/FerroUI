use crate::{AnimationError, CurveChannel, FillMode, Track};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{borrow::Cow, str::FromStr};

const EFFECT_PARAMETER: &str = "effect_parameter/";

/// A numeric component on a track, including parameters supplied by a native effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrackCurveChannel {
    Property(CurveChannel),
    EffectParameter {
        effect: String,
        parameter: String,
        component: CurveChannel,
    },
}

impl Default for TrackCurveChannel {
    fn default() -> Self {
        CurveChannel::Value.into()
    }
}

impl From<CurveChannel> for TrackCurveChannel {
    fn from(component: CurveChannel) -> Self {
        Self::Property(component)
    }
}

impl TrackCurveChannel {
    pub fn for_track(track: &Track, duration: f64, fill: FillMode) -> Vec<Self> {
        #[cfg(feature = "native-values")]
        if let Some(channels) = crate::value::registered_effect::channels(track) {
            return channels;
        }
        CurveChannel::for_track(track, duration, fill)
            .into_iter()
            .map(Self::from)
            .collect()
    }

    pub fn component(&self) -> CurveChannel {
        match self {
            Self::Property(component) | Self::EffectParameter { component, .. } => *component,
        }
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Property(CurveChannel::Value))
    }

    pub fn label(&self) -> Cow<'_, str> {
        match self {
            Self::Property(component) => component.label().into(),
            Self::EffectParameter {
                parameter,
                component,
                ..
            } => {
                let name = parameter
                    .split(['-', '_'])
                    .filter(|word| !word.is_empty())
                    .map(|word| {
                        let mut chars = word.chars();
                        chars
                            .next()
                            .into_iter()
                            .flat_map(char::to_uppercase)
                            .chain(chars)
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                if *component == CurveChannel::Value {
                    name.into()
                } else {
                    format!("{name} {}", component.label()).into()
                }
            }
        }
    }
}

pub(crate) fn parameter_identifier(name: &str) -> bool {
    name.chars().any(char::is_alphanumeric)
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

impl FromStr for TrackCurveChannel {
    type Err = AnimationError;

    fn from_str(value: &str) -> crate::Result<Self> {
        if let Some(path) = value.strip_prefix(EFFECT_PARAMETER) {
            let mut parts = path.split('/');
            if let (Some(effect), Some(parameter), Some(component), None) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            {
                if parameter_identifier(effect) && parameter_identifier(parameter) {
                    if let Ok(component) = component.parse() {
                        return Ok(Self::EffectParameter {
                            effect: effect.into(),
                            parameter: parameter.into(),
                            component,
                        });
                    }
                }
            }
        } else if let Ok(component) = value.parse() {
            return Ok(Self::Property(component));
        }
        Err(AnimationError(
            "Choose a valid curve channel from the track".into(),
        ))
    }
}

impl Serialize for TrackCurveChannel {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Property(component) => component.serialize(serializer),
            Self::EffectParameter {
                effect,
                parameter,
                component,
            } => {
                let component =
                    serde_json::to_value(component).map_err(serde::ser::Error::custom)?;
                let component = component
                    .as_str()
                    .ok_or_else(|| serde::ser::Error::custom("Invalid curve component"))?;
                serializer.serialize_str(&format!(
                    "{EFFECT_PARAMETER}{effect}/{parameter}/{component}"
                ))
            }
        }
    }
}

impl<'de> Deserialize<'de> for TrackCurveChannel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
