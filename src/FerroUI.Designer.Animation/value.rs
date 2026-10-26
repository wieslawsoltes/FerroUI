use crate::{require, AnimationError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueKind {
    Number,
    Color,
    Point,
    Vector,
    Size,
    Rect,
    CornerRadius,
    RelativePoint,
    RelativeScalar,
    Thickness,
    Discrete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelativeUnit {
    Absolute,
    Relative,
}

/// Typed values preserve the difference between a point and a two-component thickness. Colors
/// use normalized [0,1] sRGB channels. Interpolation converts RGB to linear light, matching
/// FerroUI's ColorAnimator; alpha stays linear.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum AnimationValue {
    Number(f64),
    Color {
        a: f64,
        r: f64,
        g: f64,
        b: f64,
    },
    Point {
        x: f64,
        y: f64,
    },
    Vector {
        x: f64,
        y: f64,
    },
    Size {
        width: f64,
        height: f64,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    CornerRadius {
        top_left: f64,
        top_right: f64,
        bottom_right: f64,
        bottom_left: f64,
    },
    RelativePoint {
        x: f64,
        y: f64,
        unit: RelativeUnit,
    },
    RelativeScalar {
        scalar: f64,
        unit: RelativeUnit,
    },
    Thickness {
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
    },
    Discrete(String),
}
impl AnimationValue {
    pub fn kind(&self) -> ValueKind {
        match self {
            Self::Number(_) => ValueKind::Number,
            Self::Color { .. } => ValueKind::Color,
            Self::Point { .. } => ValueKind::Point,
            Self::Vector { .. } => ValueKind::Vector,
            Self::Size { .. } => ValueKind::Size,
            Self::Rect { .. } => ValueKind::Rect,
            Self::CornerRadius { .. } => ValueKind::CornerRadius,
            Self::RelativePoint { .. } => ValueKind::RelativePoint,
            Self::RelativeScalar { .. } => ValueKind::RelativeScalar,
            Self::Thickness { .. } => ValueKind::Thickness,
            Self::Discrete(_) => ValueKind::Discrete,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if let Self::RelativePoint {
            x,
            y,
            unit: RelativeUnit::Relative,
        } = self
        {
            require(
                (x * 100.0).is_finite() && (y * 100.0).is_finite(),
                "Relative coordinates exceed finite percentage range",
            )?;
        }
        if let Self::RelativeScalar {
            scalar,
            unit: RelativeUnit::Relative,
        } = self
        {
            require(
                (scalar * 100.0).is_finite(),
                "Relative scalar exceeds finite percentage range",
            )?;
        }
        match self {
            Self::Number(value) => require(value.is_finite(), "Animation value must be finite"),
            Self::Color { a, r, g, b } => require(
                [*a, *r, *g, *b]
                    .iter()
                    .all(|n| n.is_finite() && (0.0..=1.0).contains(n)),
                "Color channels must be finite and in [0,1]",
            ),
            Self::Point { x, y } | Self::Vector { x, y } | Self::RelativePoint { x, y, .. } => {
                require(x.is_finite() && y.is_finite(), "Coordinates must be finite")
            }
            Self::Size { width, height } => require(
                width.is_finite() && height.is_finite(),
                "Size must be finite",
            ),
            Self::Rect {
                x,
                y,
                width,
                height,
            } => require(
                [*x, *y, *width, *height]
                    .iter()
                    .all(|value| value.is_finite()),
                "Rectangle must be finite",
            ),
            Self::CornerRadius {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            } => require(
                [*top_left, *top_right, *bottom_right, *bottom_left]
                    .iter()
                    .all(|value| value.is_finite()),
                "Corner radius must be finite",
            ),
            Self::RelativeScalar { scalar, .. } => {
                require(scalar.is_finite(), "Scalar must be finite")
            }
            Self::Thickness {
                left,
                top,
                right,
                bottom,
            } => require(
                [*left, *top, *right, *bottom].iter().all(|n| n.is_finite()),
                "Thickness must be finite",
            ),
            Self::Discrete(value) => require(
                value.len() <= 1_048_576
                    && value.chars().all(|c| {
                        matches!(c, '\t' | '\n' | '\r')
                            || (c >= ' ' && !matches!(c, '\u{FFFE}' | '\u{FFFF}'))
                    }),
                "Discrete value is too large or contains invalid XML control characters",
            ),
        }
    }
    pub fn interpolate(&self, to: &Self, progress: f64) -> Result<Self> {
        self.validate()?;
        to.validate()?;
        require(
            progress.is_finite(),
            "Interpolation progress must be finite",
        )?;
        let mix = |a: f64, b: f64| {
            if (0.0..=1.0).contains(&progress) {
                a * (1.0 - progress) + b * progress
            } else {
                a + (b - a) * progress
            }
        };
        let color = |a: f64, b: f64| {
            linear_to_srgb(mix(srgb_to_linear(a), srgb_to_linear(b))).clamp(0.0, 1.0)
        };
        let result = match (self, to) {
            (Self::Number(a), Self::Number(b)) => Self::Number(mix(*a, *b)),
            (
                Self::Color { a, r, g, b },
                Self::Color {
                    a: a1,
                    r: r1,
                    g: g1,
                    b: b1,
                },
            ) => Self::Color {
                a: mix(*a, *a1).clamp(0.0, 1.0),
                r: color(*r, *r1),
                g: color(*g, *g1),
                b: color(*b, *b1),
            },
            (Self::Point { x, y }, Self::Point { x: x1, y: y1 }) => Self::Point {
                x: mix(*x, *x1),
                y: mix(*y, *y1),
            },
            (Self::Vector { x, y }, Self::Vector { x: x1, y: y1 }) => Self::Vector {
                x: mix(*x, *x1),
                y: mix(*y, *y1),
            },
            (
                Self::Size { width, height },
                Self::Size {
                    width: w,
                    height: h,
                },
            ) => Self::Size {
                width: mix(*width, *w),
                height: mix(*height, *h),
            },
            (
                Self::Rect {
                    x,
                    y,
                    width,
                    height,
                },
                Self::Rect {
                    x: x1,
                    y: y1,
                    width: w,
                    height: h,
                },
            ) => Self::Rect {
                x: mix(*x, *x1),
                y: mix(*y, *y1),
                width: mix(*width, *w),
                height: mix(*height, *h),
            },
            (
                Self::CornerRadius {
                    top_left,
                    top_right,
                    bottom_right,
                    bottom_left,
                },
                Self::CornerRadius {
                    top_left: tl,
                    top_right: tr,
                    bottom_right: br,
                    bottom_left: bl,
                },
            ) => Self::CornerRadius {
                top_left: mix(*top_left, *tl),
                top_right: mix(*top_right, *tr),
                bottom_right: mix(*bottom_right, *br),
                bottom_left: mix(*bottom_left, *bl),
            },
            (
                Self::RelativePoint { x, y, unit },
                Self::RelativePoint {
                    x: x1,
                    y: y1,
                    unit: other,
                },
            ) => {
                if unit != other {
                    if progress >= 0.5 {
                        to.clone()
                    } else {
                        self.clone()
                    }
                } else {
                    Self::RelativePoint {
                        x: mix(*x, *x1),
                        y: mix(*y, *y1),
                        unit: *unit,
                    }
                }
            }
            (
                Self::RelativeScalar { scalar, unit },
                Self::RelativeScalar {
                    scalar: value,
                    unit: other,
                },
            ) => {
                if unit != other {
                    if progress >= 0.5 {
                        to.clone()
                    } else {
                        self.clone()
                    }
                } else {
                    Self::RelativeScalar {
                        scalar: mix(*scalar, *value),
                        unit: *unit,
                    }
                }
            }
            (
                Self::Thickness {
                    left,
                    top,
                    right,
                    bottom,
                },
                Self::Thickness {
                    left: l,
                    top: t,
                    right: r,
                    bottom: b,
                },
            ) => Self::Thickness {
                left: mix(*left, *l),
                top: mix(*top, *t),
                right: mix(*right, *r),
                bottom: mix(*bottom, *b),
            },
            (Self::Discrete(_), Self::Discrete(_)) => {
                if progress < 1.0 {
                    self.clone()
                } else {
                    to.clone()
                }
            }
            _ => {
                return Err(AnimationError(
                    "A track cannot interpolate different value types".into(),
                ))
            }
        };
        result.validate()?;
        Ok(result)
    }
    pub fn to_xaml(&self) -> String {
        match self {
            Self::Number(v) => v.to_string(),
            Self::Color { a, r, g, b } => format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                channel(*a),
                channel(*r),
                channel(*g),
                channel(*b)
            ),
            Self::Point { x, y } | Self::Vector { x, y } => format!("{x},{y}"),
            Self::Size { width, height } => format!("{width},{height}"),
            Self::Rect {
                x,
                y,
                width,
                height,
            } => format!("{x},{y},{width},{height}"),
            Self::CornerRadius {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            } => format!("{top_left},{top_right},{bottom_right},{bottom_left}"),
            Self::RelativePoint { x, y, unit } => match unit {
                RelativeUnit::Absolute => format!("{x},{y}"),
                RelativeUnit::Relative => format!("{}%,{}%", x * 100.0, y * 100.0),
            },
            Self::RelativeScalar { scalar, unit } => match unit {
                RelativeUnit::Absolute => scalar.to_string(),
                RelativeUnit::Relative => format!("{}%", scalar * 100.0),
            },
            Self::Thickness {
                left,
                top,
                right,
                bottom,
            } => format!("{left},{top},{right},{bottom}"),
            Self::Discrete(v) => v.clone(),
        }
    }
    pub fn parse(source: &str, kind: ValueKind) -> Result<Self> {
        let number = |s: &str| {
            s.trim()
                .parse::<f64>()
                .map_err(|_| AnimationError(format!("Invalid numeric animation value: {s}")))
        };
        let values = || {
            source
                .split(|c: char| c == ',' || c.is_ascii_whitespace())
                .filter(|s| !s.is_empty())
                .map(number)
                .collect::<Result<Vec<_>>>()
        };
        let result = match kind {
            ValueKind::Number => Self::Number(number(source)?),
            ValueKind::Color => {
                let expanded = match source.trim().strip_prefix('#') {
                    Some(hex) if hex.len() == 3 => format!("FF{}", hex.chars().flat_map(|c| [c,c]).collect::<String>()),
                    Some(hex) if hex.len() == 4 => hex.chars().flat_map(|c| [c,c]).collect(),
                    Some(hex) if hex.len() == 6 => format!("FF{hex}"),
                    Some(hex) if hex.len() == 8 => hex.to_owned(),
                    _ => return Err(AnimationError("Color interchange requires #RGB, #ARGB, #RRGGBB or #AARRGGBB; resolve named brushes through the host".into())),
                };
                let bits = u32::from_str_radix(&expanded, 16)
                    .map_err(|_| AnimationError("Invalid hexadecimal color".into()))?;
                Self::Color {
                    a: ((bits >> 24) & 255) as f64 / 255.0,
                    r: ((bits >> 16) & 255) as f64 / 255.0,
                    g: ((bits >> 8) & 255) as f64 / 255.0,
                    b: (bits & 255) as f64 / 255.0,
                }
            }
            ValueKind::Point => {
                let v = values()?;
                require(v.len() == 2, "Point requires exactly two components")?;
                Self::Point { x: v[0], y: v[1] }
            }
            ValueKind::Vector => {
                let v = values()?;
                require(v.len() == 2, "Vector requires exactly two components")?;
                Self::Vector { x: v[0], y: v[1] }
            }
            ValueKind::Size => {
                let v = values()?;
                require(v.len() == 2, "Size requires exactly two components")?;
                Self::Size {
                    width: v[0],
                    height: v[1],
                }
            }
            ValueKind::Rect => {
                let v = values()?;
                require(v.len() == 4, "Rectangle requires exactly four components")?;
                Self::Rect {
                    x: v[0],
                    y: v[1],
                    width: v[2],
                    height: v[3],
                }
            }
            ValueKind::CornerRadius => {
                let v = values()?;
                let (top_left, top_right, bottom_right, bottom_left) = match v.as_slice() {
                    [all] => (*all, *all, *all, *all),
                    [top, bottom] => (*top, *top, *bottom, *bottom),
                    [tl, tr, br, bl] => (*tl, *tr, *br, *bl),
                    _ => {
                        return Err(AnimationError(
                            "Corner radius requires one, two or four components".into(),
                        ))
                    }
                };
                Self::CornerRadius {
                    top_left,
                    top_right,
                    bottom_right,
                    bottom_left,
                }
            }
            ValueKind::RelativePoint => {
                let parts = source
                    .split(|c: char| c == ',' || c.is_ascii_whitespace())
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>();
                require(
                    parts.len() == 2,
                    "Relative point requires exactly two components",
                )?;
                let relative = parts[0].ends_with('%');
                require(
                    relative == parts[1].ends_with('%'),
                    "Relative point coordinates must use the same unit",
                )?;
                let scale = if relative { 0.01 } else { 1.0 };
                Self::RelativePoint {
                    x: number(parts[0].strip_suffix('%').unwrap_or(parts[0]))? * scale,
                    y: number(parts[1].strip_suffix('%').unwrap_or(parts[1]))? * scale,
                    unit: if relative {
                        RelativeUnit::Relative
                    } else {
                        RelativeUnit::Absolute
                    },
                }
            }
            ValueKind::RelativeScalar => {
                let source = source.trim();
                match source.strip_suffix('%') {
                    Some(value) => Self::RelativeScalar {
                        scalar: number(value)? * 0.01,
                        unit: RelativeUnit::Relative,
                    },
                    None => Self::RelativeScalar {
                        scalar: number(source)?,
                        unit: RelativeUnit::Absolute,
                    },
                }
            }
            ValueKind::Thickness => {
                let v = values()?;
                match v.as_slice() {
                    [all] => Self::Thickness {
                        left: *all,
                        top: *all,
                        right: *all,
                        bottom: *all,
                    },
                    [x, y] => Self::Thickness {
                        left: *x,
                        top: *y,
                        right: *x,
                        bottom: *y,
                    },
                    [left, top, right, bottom] => Self::Thickness {
                        left: *left,
                        top: *top,
                        right: *right,
                        bottom: *bottom,
                    },
                    _ => {
                        return Err(AnimationError(
                            "Thickness requires one, two or four components".into(),
                        ))
                    }
                }
            }
            ValueKind::Discrete => Self::Discrete(source.into()),
        };
        result.validate()?;
        Ok(result)
    }
}
fn channel(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8
}
fn srgb_to_linear(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
fn linear_to_srgb(value: f64) -> f64 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        value.powf(1.0 / 2.4) * 1.055 - 0.055
    }
}
