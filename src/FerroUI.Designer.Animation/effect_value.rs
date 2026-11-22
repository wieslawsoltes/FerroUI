use super::{channel, require, AnimationError, AnimationValue, Result, ValueKind};
use serde::{Deserialize, Serialize};

#[cfg(feature = "native-values")]
#[path = "effect_native.rs"]
mod native;
#[path = "effect_xaml.rs"]
mod xaml;

/// The authored representation of a built-in effect key, retained through source edits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectForm {
    #[default]
    Literal,
    Object,
    PropertyElement,
}

impl EffectForm {
    fn is_literal(&self) -> bool {
        *self == Self::Literal
    }
}

/// Built-in effect values whose components the Timeline can edit and sample.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnimationEffect {
    Blur {
        radius: f64,
        #[serde(default, skip_serializing_if = "EffectForm::is_literal")]
        form: EffectForm,
    },
    DropShadow {
        offset_x: f64,
        offset_y: f64,
        blur_radius: f64,
        argb: [u8; 4],
        #[serde(default = "opaque", skip_serializing_if = "is_opaque")]
        opacity: f64,
        #[serde(default, skip_serializing_if = "EffectForm::is_literal")]
        form: EffectForm,
    },
    DirectionalDropShadow {
        direction: f64,
        shadow_depth: f64,
        blur_radius: f64,
        argb: [u8; 4],
        opacity: f64,
        form: EffectForm,
    },
}

fn opaque() -> f64 {
    1.0
}
fn is_opaque(value: &f64) -> bool {
    *value == opaque()
}

impl AnimationEffect {
    pub fn parse(source: &str) -> Result<Self> {
        if source.trim_start().starts_with('<') {
            return Self::parse_object(source, None);
        }
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
                form: EffectForm::Literal,
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
                    opacity: 1.0,
                    form: EffectForm::Literal,
                }
            }
            _ => return Err(invalid()),
        };
        effect.validate()?;
        Ok(effect)
    }

    pub(crate) fn validate(self) -> Result<()> {
        match self {
            Self::Blur { radius, .. } => require(radius.is_finite(), "Blur radius must be finite"),
            Self::DropShadow {
                offset_x,
                offset_y,
                blur_radius,
                opacity,
                form,
                ..
            } => require(
                [offset_x, offset_y, blur_radius, opacity]
                    .into_iter()
                    .all(f64::is_finite)
                    && (form != EffectForm::Literal || (blur_radius >= 0.0 && opacity == 1.0)),
                "Use finite shadow values and an effect object for independent opacity",
            ),
            Self::DirectionalDropShadow {
                direction,
                shadow_depth,
                blur_radius,
                opacity,
                form,
                ..
            } => require(
                [direction, shadow_depth, blur_radius, opacity]
                    .into_iter()
                    .all(f64::is_finite)
                    && form != EffectForm::Literal,
                "A directional shadow requires an effect object with finite values",
            ),
        }
    }

    pub fn form(self) -> EffectForm {
        match self {
            Self::Blur { form, .. }
            | Self::DropShadow { form, .. }
            | Self::DirectionalDropShadow { form, .. } => form,
        }
    }

    /// Prefer the authored form when it can represent the sampled effect.
    pub fn with_form(mut self, preferred: EffectForm) -> Self {
        match &mut self {
            Self::Blur { form, .. } => *form = preferred,
            Self::DropShadow {
                blur_radius,
                opacity,
                form,
                ..
            } if preferred != EffectForm::Literal || (*opacity == 1.0 && *blur_radius >= 0.0) => {
                *form = preferred
            }
            Self::DirectionalDropShadow { form, .. } if preferred != EffectForm::Literal => {
                *form = preferred
            }
            _ => {}
        }
        self
    }

    pub(crate) fn argb(self) -> Option<[u8; 4]> {
        match self {
            Self::Blur { .. } => None,
            Self::DropShadow { argb, .. } | Self::DirectionalDropShadow { argb, .. } => Some(argb),
        }
    }

    pub fn to_xaml(self) -> String {
        if self.form() != EffectForm::Literal {
            return self.object_xaml();
        }
        match self {
            Self::Blur { radius, .. } => format!("blur({radius})"),
            Self::DropShadow {
                offset_x,
                offset_y,
                blur_radius,
                argb,
                ..
            } => format!(
                "drop-shadow({offset_x} {offset_y} {blur_radius} #{:08X})",
                u32::from_be_bytes(argb)
            ),
            Self::DirectionalDropShadow { .. } => self.object_xaml(),
        }
    }

    pub(super) fn interpolate(
        self,
        to: Self,
        progress: f64,
        mix: impl Fn(f64, f64) -> f64,
        color: impl Fn(f64, f64) -> f64,
    ) -> Self {
        let color_channels = |argb: [u8; 4], end: [u8; 4]| {
            let normalized = |value: u8| f64::from(value) / 255.0;
            let [a, r, g, b] = argb.map(normalized);
            let [ea, er, eg, eb] = end.map(normalized);
            [mix(a, ea), color(r, er), color(g, eg), color(b, eb)].map(channel)
        };
        match (self, to) {
            (Self::Blur { radius, form }, Self::Blur { radius: end, .. }) => Self::Blur {
                radius: mix(radius, end),
                form,
            },
            (
                Self::DirectionalDropShadow {
                    direction,
                    shadow_depth,
                    blur_radius,
                    argb,
                    opacity,
                    form,
                },
                Self::DirectionalDropShadow {
                    direction: angle,
                    shadow_depth: depth,
                    blur_radius: radius,
                    argb: end,
                    opacity: alpha,
                    ..
                },
            ) => Self::DirectionalDropShadow {
                direction: mix(direction, angle),
                shadow_depth: mix(shadow_depth, depth),
                blur_radius: mix(blur_radius, radius),
                argb: color_channels(argb, end),
                opacity: mix(opacity, alpha),
                form,
            },
            (from, to) if from.argb().is_some() && to.argb().is_some() => {
                let (offset_x, offset_y, blur_radius, argb, opacity) =
                    from.offset_shadow().expect("shadow pair");
                let (x, y, radius, end, alpha) = to.offset_shadow().expect("shadow pair");
                let value = Self::DropShadow {
                    offset_x: mix(offset_x, x),
                    offset_y: mix(offset_y, y),
                    blur_radius: mix(blur_radius, radius),
                    argb: color_channels(argb, end),
                    opacity: mix(opacity, alpha),
                    form: EffectForm::Object,
                };
                value.with_form(from.form())
            }
            _ if progress >= 0.5 => to,
            _ => self,
        }
    }

    fn offset_shadow(self) -> Option<(f64, f64, f64, [u8; 4], f64)> {
        match self {
            Self::Blur { .. } => None,
            Self::DropShadow {
                offset_x,
                offset_y,
                blur_radius,
                argb,
                opacity,
                ..
            } => Some((offset_x, offset_y, blur_radius, argb, opacity)),
            Self::DirectionalDropShadow {
                direction,
                shadow_depth,
                blur_radius,
                argb,
                opacity,
                ..
            } => {
                let (sin, cos) = direction.to_radians().sin_cos();
                Some((
                    cos * shadow_depth,
                    sin * shadow_depth,
                    blur_radius,
                    argb,
                    opacity,
                ))
            }
        }
    }
}
