use crate::{require, AnimationError, Result};
use serde::{Deserialize, Serialize};

/// Cubic Bezier easings invert the X polynomial; evaluating Y at progress directly is incorrect.
/// Named curves are FerroUI's animation-wide easing names. Per-keyframe easing must be Linear or
/// CubicBezier to serialize as a native KeySpline without changing semantics.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Easing {
    #[default]
    Linear,
    CubicBezier {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Named(String),
}
impl Easing {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Linear => Ok(()),
            Self::CubicBezier { x1, y1, x2, y2 } => require(
                [*x1, *y1, *x2, *y2].iter().all(|n| n.is_finite())
                    && (0.0..=1.0).contains(x1)
                    && (0.0..=1.0).contains(x2),
                "Bezier controls must be finite; X controls must be in [0,1]",
            ),
            Self::Named(name) => require(
                split_name(name).is_some(),
                format!("Unsupported easing: {name}"),
            ),
        }
    }
    pub fn sample(&self, progress: f64) -> Result<f64> {
        self.validate()?;
        require(progress.is_finite(), "Easing progress must be finite")?;
        let t = progress.clamp(0.0, 1.0);
        if (t == 0.0 || t == 1.0)
            && !matches!(self, Self::Named(name) if name == "ExponentialEaseInOut")
        {
            return Ok(t);
        }
        Ok(match self {
            Self::Linear => t,
            Self::CubicBezier { x1, y1, x2, y2 } => {
                let cubic = |u: f64, a: f64, b: f64| {
                    3.0 * (1.0 - u).powi(2) * u * a + 3.0 * (1.0 - u) * u * u * b + u * u * u
                };
                let (mut low, mut high) = (0.0, 1.0);
                // Bisection is stable even for flat end tangents or coincident controls.
                for _ in 0..52 {
                    let middle = (low + high) * 0.5;
                    if cubic(middle, *x1, *x2) < t {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }
                cubic((low + high) * 0.5, *y1, *y2)
            }
            Self::Named(name) => {
                let (family, mode) = split_name(name).expect("validated easing");
                match mode {
                    "In" => ease_in(family, t),
                    "Out" => 1.0 - ease_in(family, 1.0 - t),
                    _ => {
                        if t < 0.5 {
                            ease_in(family, t * 2.0) * 0.5
                        } else {
                            1.0 - ease_in(family, (1.0 - t) * 2.0) * 0.5
                        }
                    }
                }
            }
        })
    }
    pub fn parse(source: &str) -> Result<Self> {
        let source = source.trim();
        let result = if source == "LinearEasing" || source.is_empty() {
            Self::Linear
        } else if source.contains(',') || source.chars().any(|ch| ch.is_ascii_whitespace()) {
            let values = source
                .split(|c: char| c == ',' || c.is_ascii_whitespace())
                .filter(|s| !s.is_empty())
                .map(|v| {
                    v.parse::<f64>()
                        .map_err(|_| AnimationError("Invalid cubic Bezier".into()))
                })
                .collect::<Result<Vec<_>>>()?;
            require(values.len() == 4, "Cubic Bezier needs four coordinates")?;
            Self::CubicBezier {
                x1: values[0],
                y1: values[1],
                x2: values[2],
                y2: values[3],
            }
        } else {
            Self::Named(source.into())
        };
        result.validate()?;
        Ok(result)
    }
    pub fn to_xaml(&self) -> String {
        match self {
            Self::Linear => "LinearEasing".into(),
            Self::Named(name) => name.clone(),
            Self::CubicBezier { x1, y1, x2, y2 } => format!("{x1},{y1},{x2},{y2}"),
        }
    }
}
fn split_name(name: &str) -> Option<(&str, &str)> {
    let (family, mode) = name.split_once("Ease")?;
    if !matches!(mode, "In" | "Out" | "InOut")
        || !matches!(
            family,
            "Quadratic"
                | "Cubic"
                | "Quartic"
                | "Quintic"
                | "Sine"
                | "Circular"
                | "Exponential"
                | "Back"
                | "Bounce"
                | "Elastic"
        )
    {
        None
    } else {
        Some((family, mode))
    }
}
fn ease_in(family: &str, t: f64) -> f64 {
    match family {
        "Quadratic" => t * t,
        "Cubic" => t.powi(3),
        "Quartic" => t.powi(4),
        "Quintic" => t.powi(5),
        "Sine" => 1.0 - (t * std::f64::consts::FRAC_PI_2).cos(),
        "Circular" => 1.0 - (1.0 - t * t).sqrt(),
        "Exponential" => 2.0_f64.powf(10.0 * (t - 1.0)),
        "Back" => t * (t * t - (t * std::f64::consts::PI).sin()),
        "Bounce" => 1.0 - bounce_out(1.0 - t),
        "Elastic" => {
            (13.0 * std::f64::consts::FRAC_PI_2 * t).sin() * 2.0_f64.powf(10.0 * (t - 1.0))
        }
        _ => t,
    }
}
fn bounce_out(t: f64) -> f64 {
    if t < 4.0 / 11.0 {
        121.0 * t * t / 16.0
    } else if t < 8.0 / 11.0 {
        363.0 / 40.0 * t * t - 99.0 / 10.0 * t + 17.0 / 5.0
    } else if t < 9.0 / 10.0 {
        4356.0 / 361.0 * t * t - 35442.0 / 1805.0 * t + 16061.0 / 1805.0
    } else {
        54.0 / 5.0 * t * t - 513.0 / 25.0 * t + 268.0 / 25.0
    }
}
