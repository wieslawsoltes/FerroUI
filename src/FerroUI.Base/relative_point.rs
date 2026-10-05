//! Defines a point that may be defined relative to a containing element.

use std::fmt;
use std::str::FromStr;

use crate::utilities::span_helpers::{parse_double, InvariantF64};
use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::{Point, Rect, Size};

/// Defines the reference point units of a [`RelativePoint`] or `RelativeRect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum RelativeUnit {
    /// The point is expressed as a fraction of the containing element's size.
    #[default]
    Relative,

    /// The point is absolute (i.e. in pixels).
    Absolute,
}

/// Defines a point that may be defined relative to a containing element.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct RelativePoint {
    /// The point.
    pub point: Point,
    /// The unit of the point.
    pub unit: RelativeUnit,
}

impl RelativePoint {
    /// A point at the top left of the containing element.
    pub const TOP_LEFT: RelativePoint = RelativePoint::new(0.0, 0.0, RelativeUnit::Relative);

    /// A point at the center of the containing element.
    pub const CENTER: RelativePoint = RelativePoint::new(0.5, 0.5, RelativeUnit::Relative);

    /// A point at the bottom right of the containing element.
    pub const BOTTOM_RIGHT: RelativePoint = RelativePoint::new(1.0, 1.0, RelativeUnit::Relative);

    /// Initializes a new instance of the [`RelativePoint`] structure.
    #[inline]
    pub const fn new(x: f64, y: f64, unit: RelativeUnit) -> Self {
        Self::from_point(Point::new(x, y), unit)
    }

    /// Initializes a new instance of the [`RelativePoint`] structure from a [`Point`].
    #[inline]
    pub const fn from_point(point: Point, unit: RelativeUnit) -> Self {
        Self { point, unit }
    }

    /// Checks if the [`RelativePoint`] equals another point.
    #[inline]
    pub fn equals(&self, p: RelativePoint) -> bool {
        self.unit == p.unit && self.point == p.point
    }

    /// Converts a [`RelativePoint`] into pixels, using `size` as the size of the
    /// visual the point is relative to.
    #[inline]
    pub fn to_pixels(&self, size: Size) -> Point {
        if self.unit == RelativeUnit::Absolute {
            self.point
        } else {
            Point::new(self.point.x * size.width, self.point.y * size.height)
        }
    }

    /// Converts a [`RelativePoint`] into pixels, using `rect` as the bounding
    /// rectangle the point is relative to.
    #[inline]
    pub fn to_pixels_rect(&self, rect: Rect) -> Point {
        if self.unit == RelativeUnit::Absolute {
            self.point
        } else {
            Point::new(
                rect.x + self.point.x * rect.width,
                rect.y + self.point.y * rect.height,
            )
        }
    }

    /// Parses a [`RelativePoint`] string: `"x, y"` (absolute) or `"x%, y%"` (relative).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        SpanStringTokenizer::with_message(s, "Invalid RelativePoint.").scope(|t| {
            let mut x = t.read_string()?;
            let mut y = t.read_string()?;

            let mut unit = RelativeUnit::Absolute;
            let mut scale = 1.0;

            if x.ends_with('%') {
                if !y.ends_with('%') {
                    return Err(FormatError::new(
                        "If one coordinate is relative, both must be.",
                    ));
                }

                x = x.trim_end_matches('%');
                y = y.trim_end_matches('%');
                unit = RelativeUnit::Relative;
                scale = 0.01;
            }

            let x = parse_double(x).ok_or_else(|| FormatError::invalid_input(x))?;
            let y = parse_double(y).ok_or_else(|| FormatError::invalid_input(y))?;

            Ok(RelativePoint::new(x * scale, y * scale, unit))
        })
    }
}

impl FromStr for RelativePoint {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        RelativePoint::parse(s)
    }
}

impl fmt::Display for RelativePoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.unit == RelativeUnit::Absolute {
            fmt::Display::fmt(&self.point, f)
        } else {
            write!(
                f,
                "{}%, {}%",
                InvariantF64(self.point.x * 100.0),
                InvariantF64(self.point.y * 100.0)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_should_accept_absolute_value() {
        let result = RelativePoint::parse("4,5").unwrap();

        assert_eq!(RelativePoint::new(4.0, 5.0, RelativeUnit::Absolute), result);
    }

    #[test]
    fn parse_should_accept_relative_value() {
        let result = RelativePoint::parse("25%, 50%").unwrap();

        assert_eq!(
            RelativePoint::new(0.25, 0.5, RelativeUnit::Relative),
            result
        );
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn parse_errors_display_and_to_pixels() {
        assert_eq!(
            RelativePoint::parse("25%, 50").unwrap_err().message(),
            "If one coordinate is relative, both must be."
        );
        assert!(RelativePoint::parse("25, 50%").is_err());
        assert_eq!(
            RelativePoint::parse("1").unwrap_err().message(),
            "Invalid RelativePoint."
        );
        // Unconsumed input wins over the inner error.
        assert_eq!(
            RelativePoint::parse("25%, 50, 3").unwrap_err().message(),
            "Invalid RelativePoint."
        );
        assert_eq!(RelativePoint::CENTER.to_string(), "50%, 50%");
        assert_eq!(
            RelativePoint::new(4.0, 5.5, RelativeUnit::Absolute).to_string(),
            "4, 5.5"
        );
        assert_eq!(
            RelativePoint::CENTER.to_pixels(Size::new(100.0, 50.0)),
            Point::new(50.0, 25.0)
        );
        assert_eq!(
            RelativePoint::CENTER.to_pixels_rect(Rect::new(10.0, 10.0, 100.0, 50.0)),
            Point::new(60.0, 35.0)
        );
        assert_eq!(
            RelativePoint::new(4.0, 5.0, RelativeUnit::Absolute).to_pixels(Size::new(100.0, 50.0)),
            Point::new(4.0, 5.0)
        );
        assert_eq!(RelativePoint::default(), RelativePoint::TOP_LEFT);
    }
}
