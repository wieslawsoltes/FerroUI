use crate::{AnimationError, CurveChannel, FillMode, Track};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{borrow::Cow, num::NonZeroUsize, str::FromStr};

const EFFECT_PARAMETER: &str = "effect_parameter/";
const BOX_SHADOW: &str = "box_shadow/";

pub(crate) const BOX_SHADOW_CHANNELS: &[CurveChannel] = &[
    CurveChannel::OffsetX,
    CurveChannel::OffsetY,
    CurveChannel::BlurRadius,
    CurveChannel::Spread,
    CurveChannel::Red,
    CurveChannel::Green,
    CurveChannel::Blue,
    CurveChannel::Alpha,
];

/// A numeric component on a track, including parameters supplied by a native effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrackCurveChannel {
    Property(CurveChannel),
    EffectParameter {
        effect: String,
        parameter: String,
        component: CurveChannel,
    },
    BoxShadow {
        layer: NonZeroUsize,
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
        if matches!(track.keyframes.first().map(|key| &key.value), Some(crate::AnimationValue::BoxShadows(_))) {
            let count = track.keyframes.iter().map(|key| &key.value)
                .chain(&track.base_value)
                .filter_map(|value| match value {
                    crate::AnimationValue::BoxShadows(shadows) => Some(shadows.layers().len()),
                    _ => None,
                })
                .max()
                .unwrap_or_default();
            return (1..=count).filter_map(NonZeroUsize::new)
                .flat_map(|layer| BOX_SHADOW_CHANNELS.iter().map(move |&component| Self::BoxShadow { layer, component }))
                .collect();
        }
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
            Self::Property(component) | Self::EffectParameter { component, .. } | Self::BoxShadow { component, .. } => *component,
        }
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Property(CurveChannel::Value))
    }

    pub fn label(&self) -> Cow<'_, str> {
        match self {
            Self::Property(component) => component.label().into(),
            Self::BoxShadow { layer, component } => format!("Shadow {layer} · {}", component.label()).into(),
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
        } else if let Some(path) = value.strip_prefix(BOX_SHADOW) {
            if let Some((layer, component)) = path.split_once('/') {
                if let (Ok(layer), Ok(component)) = (layer.parse(), component.parse()) {
                    if BOX_SHADOW_CHANNELS.contains(&component) {
                        return Ok(Self::BoxShadow { layer, component });
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
            Self::BoxShadow { layer, component } => {
                let component = component_name::<S::Error>(*component)?;
                serializer.serialize_str(&format!("{BOX_SHADOW}{layer}/{component}"))
            }
            Self::EffectParameter {
                effect,
                parameter,
                component,
            } => {
                let component = component_name::<S::Error>(*component)?;
                serializer.serialize_str(&format!(
                    "{EFFECT_PARAMETER}{effect}/{parameter}/{component}"
                ))
            }
        }
    }
}

fn component_name<E: serde::ser::Error>(component: CurveChannel) -> std::result::Result<String, E> {
    match serde_json::to_value(component).map_err(E::custom)? {
        serde_json::Value::String(name) => Ok(name),
        _ => Err(E::custom("Invalid curve component")),
    }
}

impl<'de> Deserialize<'de> for TrackCurveChannel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
