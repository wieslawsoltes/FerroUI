//! Numeric component models for source-defined properties across editor hosts.
use crate::{require, PropertyValueShape, Result, ToolError};
use ferroui_designer_core::MarkupValue;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PropertyComponentType {
    Shape(PropertyValueShape),
    PixelPoint,
    PixelSize,
    PixelRect,
    Thickness,
}

#[derive(Clone, Debug, Serialize)]
pub struct PropertyComponents {
    #[serde(skip)]
    pub shape: PropertyComponentType,
    pub labels: &'static [&'static str],
    pub values: Option<Vec<f64>>,
    pub relative: bool,
    pub percent: bool,
    pub linkable: bool,
}

impl PropertyComponentType {
    /// Identify a framework value type from source or native property metadata.
    pub fn for_type(name: &str) -> Option<Self> {
        use PropertyValueShape::*;
        let name = name
            .strip_prefix("Option<")
            .and_then(|inner| inner.strip_suffix('>'))
            .unwrap_or(name);
        let name = name.strip_prefix("::").unwrap_or(name);
        let name = if name.starts_with("ferroui_base::") {
            name.rsplit("::").next()?
        } else {
            name
        };
        Some(match name {
            "Point" => Self::Shape(Point),
            "Size" => Self::Shape(Size),
            "Vector" => Self::Shape(Vector),
            "Rect" => Self::Shape(Rect),
            "Matrix" => Self::Shape(Matrix),
            "RelativePoint" => Self::Shape(RelativePoint),
            "RelativeRect" => Self::Shape(RelativeRect),
            "CornerRadius" => Self::Shape(CornerRadius),
            "PixelPoint" => Self::PixelPoint,
            "PixelSize" => Self::PixelSize,
            "PixelRect" => Self::PixelRect,
            "Thickness" => Self::Thickness,
            _ => return None,
        })
    }

    pub fn value_shape(self) -> Option<PropertyValueShape> {
        match self {
            Self::Shape(shape) => Some(shape),
            Self::PixelPoint => Some(PropertyValueShape::Point),
            Self::PixelSize => Some(PropertyValueShape::Size),
            Self::PixelRect => Some(PropertyValueShape::Rect),
            Self::Thickness => None,
        }
    }

    pub fn describe_components(
        self,
        authored: Option<&str>,
        effective: Option<&str>,
    ) -> Option<PropertyComponents> {
        let parse = |value: &str| self.parse_components(value.strip_prefix("{}").unwrap_or(value));
        let parsed = match authored {
            Some(value) => Some(parse(value).ok()?),
            None => effective.and_then(|value| parse(value).ok()),
        };
        let count = parsed.as_ref().map_or(0, |(values, _)| values.len());
        let labels = self
            .value_shape()
            .map_or(&["L", "T", "R", "B"][..], |shape| {
                shape.component_labels(count)
            });
        if labels.is_empty() {
            return None;
        }
        let (values, percent) =
            parsed.map_or((None, false), |(values, percent)| (Some(values), percent));
        Some(PropertyComponents {
            shape: self,
            labels,
            values,
            percent,
            relative: self.value_shape().is_some_and(PropertyValueShape::relative),
            linkable: matches!(
                self,
                Self::Thickness | Self::Shape(PropertyValueShape::CornerRadius)
            ),
        })
    }

    pub fn validate_markup(self, property: &str, source: &str) -> Result<()> {
        match MarkupValue::parse(source).map_err(ToolError)? {
            MarkupValue::Literal(value) => {
                let (values, _) = self.parse_components(&value)?;
                require(
                    self != Self::Thickness
                        || !matches!(property, "Padding" | "BorderThickness")
                        || values.iter().all(|value| *value >= 0.0),
                    format!("{property} components must be nonnegative"),
                )
            }
            MarkupValue::Extension(_) => Ok(()),
        }
    }

    fn parse_components(self, source: &str) -> Result<(Vec<f64>, bool)> {
        let Some(shape) = self.value_shape() else {
            return Ok((crate::layout::thickness(source)?.into(), false));
        };
        if matches!(self, Self::PixelPoint | Self::PixelSize | Self::PixelRect) {
            require(
                super::values::component_tokens(source).all(|part| part.parse::<i32>().is_ok()),
                "Pixel components must be 32-bit integers",
            )?;
        }
        shape.parse_components(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_effective_thickness_and_guards_units_in_authored_pixel_values() {
        let insets = PropertyComponentType::for_type("Thickness").unwrap();
        let fields = insets.describe_components(None, Some("10, -6")).unwrap();
        assert_eq!(fields.labels, ["L", "T", "R", "B"]);
        assert_eq!(fields.values, Some(vec![10.0, -6.0, 10.0, -6.0]));
        let pixel =
            PropertyComponentType::for_type("Option<ferroui_base::pixel_point::PixelPoint>")
                .unwrap();
        assert!(pixel.validate_markup("Position", "-40, 100").is_ok());
        assert!(pixel.validate_markup("Position", "0.5, 100").is_err());
        assert!(pixel
            .validate_markup("Position", "2147483648, 100")
            .is_err());
        assert!(pixel
            .validate_markup("Position", "{Binding Location}")
            .is_ok());
        assert!(insets.validate_markup("Padding", "10, -6").is_err());
        assert!(insets.validate_markup("Margin", "10, -6").is_ok());
        assert!(insets
            .describe_components(Some("{Binding Padding}"), Some("12"))
            .is_none());
        assert!(PropertyComponentType::for_type("example::Point").is_none());
    }
}
