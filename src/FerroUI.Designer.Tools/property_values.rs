//! Portable value shapes for designer editors and headless property validation.
//! These describe serialization, not native widgets. Hosts can register richer value
//! editors without depending on a windowing or runtime reflection system.
use crate::{Result, ToolError, require};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyValueShape {
    Point,
    Size,
    Vector,
    Rect,
    Matrix,
    RelativePoint,
    RelativeRect,
    CornerRadius,
    TimeSpan,
    Duration,
    FontFamily,
    Uri,
    NumberList,
}

impl PropertyValueShape {
    pub fn components(self) -> &'static [&'static str] {
        match self {
            Self::Point | Self::Vector | Self::RelativePoint => &["X", "Y"],
            Self::Size => &["W", "H"],
            Self::Rect | Self::RelativeRect => &["X", "Y", "W", "H"],
            Self::Matrix => &["M11", "M12", "M21", "M22", "DX", "DY"],
            Self::CornerRadius => &["TL", "TR", "BR", "BL"],
            _ => &[],
        }
    }

    pub fn relative(self) -> bool {
        matches!(self, Self::RelativePoint | Self::RelativeRect)
    }

    pub fn parse_components(self, source: &str) -> Result<(Vec<f64>, bool)> {
        let expected = self.components().len();
        require(
            expected != 0,
            "This property does not have numeric components",
        )?;
        let values = source
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        let relative = values.first().is_some_and(|value| value.ends_with('%'));
        require(
            !relative || self.relative(),
            "Percent values are not supported here",
        )?;
        require(
            values.iter().all(|part| part.ends_with('%') == relative),
            "All components must use the same unit",
        )?;
        let mut parsed = values
            .iter()
            .map(|part| {
                part.trim_end_matches('%')
                    .parse::<f64>()
                    .ok()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| ToolError("Components must be finite numbers".into()))
            })
            .collect::<Result<Vec<_>>>()?;
        if self == Self::CornerRadius {
            parsed = match parsed.as_slice() {
                [all] => vec![*all; 4],
                [first, second] => vec![*first, *second, *first, *second],
                _ => parsed,
            };
        }
        require(
            parsed.len() == expected,
            format!("Expected {expected} components"),
        )?;
        for (index, value) in parsed.iter().enumerate() {
            let nonnegative = matches!(self, Self::Size | Self::CornerRadius)
                || matches!(self, Self::Rect | Self::RelativeRect) && index >= 2;
            require(
                !nonnegative || *value >= 0.0,
                "Sizes and corner radii must be nonnegative",
            )?;
        }
        Ok((parsed, relative))
    }

    pub fn validate(self, source: &str) -> Result<()> {
        if !self.components().is_empty() {
            self.parse_components(source)?;
            return Ok(());
        }
        let value = source.trim();
        match self {
            Self::TimeSpan => validate_time(value),
            Self::Duration if matches!(value, "Automatic" | "Forever") => Ok(()),
            Self::Duration => validate_time(value),
            Self::FontFamily => require(!value.is_empty(), "Enter at least one font family"),
            Self::Uri => require(
                !value.chars().any(|c| c.is_control()),
                "A URI cannot contain control characters",
            ),
            Self::NumberList => {
                for part in value
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter(|x| !x.is_empty())
                {
                    require(
                        part.parse::<f64>().is_ok_and(|n| n.is_finite()),
                        "Enter finite numbers separated by commas",
                    )?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn format_components(self, values: &[f64], percent: bool) -> Result<String> {
        let value = values
            .iter()
            .map(|value| {
                format!(
                    "{}{}",
                    if *value == 0.0 { 0.0 } else { *value },
                    if percent { "%" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.validate(&value)?;
        Ok(value)
    }
}

fn validate_time(value: &str) -> Result<()> {
    require(!value.is_empty(), "Enter a duration such as 00:00:00.250")?;
    let value = value.strip_prefix('-').unwrap_or(value);
    let parts = value.split(':').collect::<Vec<_>>();
    require(matches!(parts.len(), 1..=3), "Use days or hh:mm:ss.fffffff")?;
    if parts.len() == 1 {
        return require(
            value.parse::<u32>().is_ok(),
            "A duration without colons is a whole number of days",
        );
    }
    let first = parts[0].rsplit('.').next().unwrap_or(parts[0]);
    if let Some((days, _)) = parts[0].split_once('.') {
        require(days.parse::<u32>().is_ok(), "Invalid duration days")?;
    }
    require(
        first.parse::<u32>().is_ok_and(|hours| hours < 24),
        "Hours must be between 0 and 23",
    )?;
    require(
        parts[1].parse::<u32>().is_ok_and(|minutes| minutes < 60),
        "Minutes must be between 0 and 59",
    )?;
    if parts.len() == 3 {
        require(
            parts[2]
                .parse::<f64>()
                .is_ok_and(|seconds| seconds.is_finite() && (0.0..60.0).contains(&seconds)),
            "Seconds must be between 0 and 59.9999999",
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_values_preserve_relative_units_and_reject_mixed_or_invalid_dimensions() {
        assert_eq!(
            PropertyValueShape::RelativePoint
                .parse_components("25%, 50%")
                .unwrap(),
            (vec![25.0, 50.0], true)
        );
        assert!(
            PropertyValueShape::RelativePoint
                .validate("25%, 50")
                .is_err()
        );
        assert!(PropertyValueShape::Point.validate("25%, 50%").is_err());
        assert!(PropertyValueShape::Rect.validate("-4, 5, 30, 40").is_ok());
        assert!(PropertyValueShape::Rect.validate("4, 5, -30, 40").is_err());
        assert!(
            PropertyValueShape::Matrix
                .validate("1,0,0,1,10,-20")
                .is_ok()
        );
        assert!(
            PropertyValueShape::Matrix
                .validate("1,0,0,1,NaN,0")
                .is_err()
        );
        assert_eq!(
            PropertyValueShape::CornerRadius
                .parse_components("4,8")
                .unwrap()
                .0,
            vec![4.0, 8.0, 4.0, 8.0]
        );
        assert!(PropertyValueShape::Duration.validate("Forever").is_ok());
        assert!(
            PropertyValueShape::TimeSpan
                .validate("00:00:00.250")
                .is_ok()
        );
        assert!(PropertyValueShape::TimeSpan.validate("00:90:00").is_err());
    }
}
