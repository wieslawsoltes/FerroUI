use super::{channel, require, AnimationError, AnimationValue, Result, ValueKind};
use serde::{Deserialize, Serialize};

#[cfg(feature = "native-values")]
#[path = "effect_native.rs"]
mod native;

/// The built-in effect literals that can round-trip through the XAML effect converter.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnimationEffect {
    Blur {
        radius: f64,
    },
    DropShadow {
        offset_x: f64,
        offset_y: f64,
        blur_radius: f64,
        argb: [u8; 4],
    },
}

impl AnimationEffect {
    pub fn parse(source: &str) -> Result<Self> {
        let invalid =
            || AnimationError("Use blur(radius) or drop-shadow(x y radius #AARRGGBB)".into());
        let (function, args) = source
            .trim()
            .strip_suffix(')')
            .and_then(|text| text.split_once('('))
            .ok_or_else(invalid)?;
        let number = |value: &str| value.parse::<f64>().map_err(|_| invalid());
        let fields = args.split_ascii_whitespace().collect::<Vec<_>>();
        let effect = match (function.trim(), fields.as_slice()) {
            ("blur", [radius]) => Self::Blur {
                radius: number(radius)?,
            },
            ("drop-shadow", [x, y, rest @ ..]) if rest.len() <= 2 => {
                let blur_radius = rest
                    .first()
                    .map(|value| number(value))
                    .transpose()?
                    .unwrap_or(0.0);
                let argb = match rest.get(1) {
                    Some(color) => AnimationValue::parse(color, ValueKind::Color)?
                        .argb8()
                        .ok_or_else(invalid)?,
                    None => [255, 0, 0, 0],
                };
                Self::DropShadow {
                    offset_x: number(x)?,
                    offset_y: number(y)?,
                    blur_radius,
                    argb,
                }
            }
            _ => return Err(invalid()),
        };
        effect.validate()?;
        Ok(effect)
    }

    pub(crate) fn validate(self) -> Result<()> {
        match self {
            Self::Blur { radius } => require(radius.is_finite(), "Blur radius must be finite"),
            Self::DropShadow {
                offset_x,
                offset_y,
                blur_radius,
                ..
            } => require(
                offset_x.is_finite()
                    && offset_y.is_finite()
                    && blur_radius.is_finite()
                    && blur_radius >= 0.0,
                "Shadow offsets and radius must be finite and the radius must be nonnegative",
            ),
        }
    }

    pub fn to_xaml(self) -> String {
        match self {
            Self::Blur { radius } => format!("blur({radius})"),
            Self::DropShadow {
                offset_x,
                offset_y,
                blur_radius,
                argb,
            } => format!(
                "drop-shadow({offset_x} {offset_y} {blur_radius} #{:08X})",
                u32::from_be_bytes(argb)
            ),
        }
    }

    pub(super) fn interpolate(
        self,
        to: Self,
        progress: f64,
        mix: impl Fn(f64, f64) -> f64,
        color: impl Fn(f64, f64) -> f64,
    ) -> Self {
        match (self, to) {
            (Self::Blur { radius }, Self::Blur { radius: end }) => Self::Blur {
                radius: mix(radius, end),
            },
            (
                Self::DropShadow {
                    offset_x,
                    offset_y,
                    blur_radius,
                    argb,
                },
                Self::DropShadow {
                    offset_x: x,
                    offset_y: y,
                    blur_radius: radius,
                    argb: end,
                },
            ) => {
                let normalized = |value: u8| f64::from(value) / 255.0;
                let [a, r, g, b] = argb.map(normalized);
                let [ea, er, eg, eb] = end.map(normalized);
                Self::DropShadow {
                    offset_x: mix(offset_x, x),
                    offset_y: mix(offset_y, y),
                    blur_radius: mix(blur_radius, radius),
                    argb: [mix(a, ea), color(r, er), color(g, eg), color(b, eb)].map(channel),
                }
            }
            _ if progress >= 0.5 => to,
            _ => self,
        }
    }
}
