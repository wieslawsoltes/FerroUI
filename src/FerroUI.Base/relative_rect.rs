//! Defines a rectangle that may be defined relative to a containing element.

use std::str::FromStr;

use crate::utilities::span_helpers::parse_double;
use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::{Point, Rect, RelativeUnit, Size};

/// Defines a rectangle that may be defined relative to a containing element.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct RelativeRect {
    /// The unit of the rectangle.
    pub unit: RelativeUnit,
    /// The rectangle.
    pub rect: Rect,
}

impl RelativeRect {
    /// A rectangle that represents 100% of an area.
    pub const FILL: RelativeRect = RelativeRect::new(0.0, 0.0, 1.0, 1.0, RelativeUnit::Relative);

    /// Initializes a new instance of the [`RelativeRect`] structure.
    #[inline]
    pub const fn new(x: f64, y: f64, width: f64, height: f64, unit: RelativeUnit) -> Self {
        Self {
            unit,
            rect: Rect::new(x, y, width, height),
        }
    }

    /// Initializes a [`RelativeRect`] from a [`Rect`].
    #[inline]
    pub const fn from_rect(rect: Rect, unit: RelativeUnit) -> Self {
        Self { unit, rect }
    }

    /// Initializes a [`RelativeRect`] at the origin with the given size.
    #[inline]
    pub const fn from_size(size: Size, unit: RelativeUnit) -> Self {
        Self::from_rect(Rect::from_size(size), unit)
    }

    /// Initializes a [`RelativeRect`] from its position and size.
    #[inline]
    pub const fn from_position_size(position: Point, size: Size, unit: RelativeUnit) -> Self {
        Self::from_rect(Rect::from_position_size(position, size), unit)
    }

    /// Initializes a [`RelativeRect`] from its top left and bottom right corners.
    #[inline]
    pub fn from_points(top_left: Point, bottom_right: Point, unit: RelativeUnit) -> Self {
        Self::from_rect(Rect::from_points(top_left, bottom_right), unit)
    }

    /// Checks if the [`RelativeRect`] equals another rectangle.
    #[inline]
    pub fn equals(&self, p: RelativeRect) -> bool {
        self.unit == p.unit && self.rect == p.rect
    }

    /// Converts a [`RelativeRect`] into pixels, using `size` as the size of the
    /// visual the rectangle is relative to.
    #[inline]
    pub fn to_pixels(&self, size: Size) -> Rect {
        if self.unit == RelativeUnit::Absolute {
            self.rect
        } else {
            Rect::new(
                self.rect.x * size.width,
                self.rect.y * size.height,
                self.rect.width * size.width,
                self.rect.height * size.height,
            )
        }
    }

    /// Converts a [`RelativeRect`] into pixels, using `bounding_box` as the box the
    /// rectangle is relative to.
    #[inline]
    pub fn to_pixels_rect(&self, bounding_box: Rect) -> Rect {
        if self.unit == RelativeUnit::Absolute {
            self.rect
        } else {
            Rect::new(
                bounding_box.x + self.rect.x * bounding_box.width,
                bounding_box.y + self.rect.y * bounding_box.height,
                self.rect.width * bounding_box.width,
                self.rect.height * bounding_box.height,
            )
        }
    }

    /// Parses a [`RelativeRect`] string: four numbers (absolute) or four
    /// percentages (relative).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        SpanStringTokenizer::with_separator(s, ',', Some("Invalid RelativeRect.")).scope(|t| {
            let mut x = t.read_span()?;
            let mut y = t.read_span()?;
            let mut width = t.read_span()?;
            let mut height = t.read_span()?;

            let mut unit = RelativeUnit::Absolute;
            let mut scale = 1.0;

            let x_relative = x.ends_with('%');
            let y_relative = y.ends_with('%');
            let width_relative = width.ends_with('%');
            let height_relative = height.ends_with('%');

            if x_relative && y_relative && width_relative && height_relative {
                x = x.trim_end_matches('%');
                y = y.trim_end_matches('%');
                width = width.trim_end_matches('%');
                height = height.trim_end_matches('%');

                unit = RelativeUnit::Relative;
                scale = 0.01;
            } else if x_relative || y_relative || width_relative || height_relative {
                return Err(FormatError::new(
                    "If one coordinate is relative, all must be.",
                ));
            }

            let parse = |v: &str| parse_double(v).ok_or_else(|| FormatError::invalid_input(v));

            Ok(RelativeRect::new(
                parse(x)? * scale,
                parse(y)? * scale,
                parse(width)? * scale,
                parse(height)? * scale,
                unit,
            ))
        })
    }
}

impl FromStr for RelativeRect {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        RelativeRect::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rounds to 3 decimals, ties to even.
    fn round3(value: f64) -> f64 {
        (value * 1000.0).round_ties_even() / 1000.0
    }

    /// Compares two rects after rounding each component to 3 decimals.
    fn compare(a: RelativeRect, b: RelativeRect) -> bool {
        a.unit == b.unit
            && round3(a.rect.x) == round3(b.rect.x)
            && round3(a.rect.y) == round3(b.rect.y)
            && round3(a.rect.width) == round3(b.rect.width)
            && round3(a.rect.height) == round3(b.rect.height)
    }

    #[test]
    fn parse_should_accept_absolute_value() {
        let result = RelativeRect::parse("4,5,50,60").unwrap();

        assert!(compare(
            RelativeRect::new(4.0, 5.0, 50.0, 60.0, RelativeUnit::Absolute),
            result
        ));
    }

    #[test]
    fn parse_should_accept_relative_value() {
        let result = RelativeRect::parse("10%, 20%, 40%, 70%").unwrap();

        assert!(compare(
            RelativeRect::new(0.1, 0.2, 0.4, 0.7, RelativeUnit::Relative),
            result
        ));
    }

    #[test]
    fn parse_should_throw_mixed_values() {
        assert!(RelativeRect::parse("10%, 20%, 40, 70%").is_err());
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn to_pixels_and_errors() {
        assert_eq!(
            RelativeRect::parse("10%, 20%, 40, 70%")
                .unwrap_err()
                .message(),
            "If one coordinate is relative, all must be."
        );
        assert_eq!(
            RelativeRect::parse("1,2,3").unwrap_err().message(),
            "Invalid RelativeRect."
        );
        assert_eq!(
            RelativeRect::FILL.to_pixels(Size::new(100.0, 50.0)),
            Rect::new(0.0, 0.0, 100.0, 50.0)
        );
        assert_eq!(
            RelativeRect::new(0.5, 0.5, 0.5, 0.5, RelativeUnit::Relative)
                .to_pixels_rect(Rect::new(10.0, 10.0, 100.0, 50.0)),
            Rect::new(60.0, 35.0, 50.0, 25.0)
        );
        let abs = RelativeRect::new(1.0, 2.0, 3.0, 4.0, RelativeUnit::Absolute);
        assert_eq!(abs.to_pixels(Size::new(100.0, 50.0)), abs.rect);
    }
}
