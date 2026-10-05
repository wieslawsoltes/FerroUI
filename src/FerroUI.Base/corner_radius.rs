//! Represents the radii of a rectangle's corners.

use std::fmt;
use std::str::FromStr;

use crate::utilities::math_utilities;
use crate::utilities::span_helpers::InvariantF64;
use crate::utilities::{FormatError, SpanStringTokenizer};

/// Represents the radii of a rectangle's corners.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct CornerRadius {
    /// Radius of the top left corner.
    pub top_left: f64,
    /// Radius of the top right corner.
    pub top_right: f64,
    /// Radius of the bottom right corner.
    pub bottom_right: f64,
    /// Radius of the bottom left corner.
    pub bottom_left: f64,
}

impl CornerRadius {
    /// Initializes a [`CornerRadius`] from the four corner radii, clockwise from the top left.
    #[inline]
    pub const fn new(top_left: f64, top_right: f64, bottom_right: f64, bottom_left: f64) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// Initializes a [`CornerRadius`] with the same radius on every corner.
    #[inline]
    pub const fn uniform(uniform_radius: f64) -> Self {
        Self::new(
            uniform_radius,
            uniform_radius,
            uniform_radius,
            uniform_radius,
        )
    }

    /// Initializes a [`CornerRadius`] from the radius of the top corners and the
    /// radius of the bottom corners.
    #[inline]
    pub const fn top_bottom(top: f64, bottom: f64) -> Self {
        Self::new(top, top, bottom, bottom)
    }

    /// Gets a value indicating whether the instance has equal values for all
    /// corners (NaN compares equal to NaN here).
    #[inline]
    pub fn is_uniform(&self) -> bool {
        math_utilities::equals(self.top_left, self.top_right)
            && math_utilities::equals(self.bottom_left, self.bottom_right)
            && math_utilities::equals(self.top_right, self.bottom_right)
    }

    /// Returns a boolean indicating whether the corner radius is equal to the other
    /// given corner radius (exact comparison).
    #[inline]
    pub fn equals(&self, other: CornerRadius) -> bool {
        self.top_left == other.top_left
            && self.top_right == other.top_right
            && self.bottom_right == other.bottom_right
            && self.bottom_left == other.bottom_left
    }

    /// Parses a [`CornerRadius`] string: one, two or four numbers.
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        const EXCEPTION_MESSAGE: &str = "Invalid CornerRadius.";

        SpanStringTokenizer::with_message(s, EXCEPTION_MESSAGE).scope(|t| {
            if let Some(a) = t.try_read_double()? {
                if let Some(b) = t.try_read_double()? {
                    if let Some(c) = t.try_read_double()? {
                        return Ok(CornerRadius::new(a, b, c, t.read_double()?));
                    }

                    return Ok(CornerRadius::top_bottom(a, b));
                }

                return Ok(CornerRadius::uniform(a));
            }

            Err(FormatError::new(EXCEPTION_MESSAGE))
        })
    }
}

impl FromStr for CornerRadius {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        CornerRadius::parse(s)
    }
}

impl fmt::Display for CornerRadius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{},{},{},{}",
            InvariantF64(self.top_left),
            InvariantF64(self.top_right),
            InvariantF64(self.bottom_right),
            InvariantF64(self.bottom_left)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_parses_single_uniform_radius() {
        let result = CornerRadius::parse("3.4").unwrap();

        assert_eq!(CornerRadius::uniform(3.4), result);
    }

    #[test]
    fn parse_parses_top_bottom() {
        let result = CornerRadius::parse("1.1,2.2").unwrap();

        assert_eq!(CornerRadius::top_bottom(1.1, 2.2), result);
    }

    #[test]
    fn parse_parses_top_left_top_right_bottom_right_bottom_left() {
        let result = CornerRadius::parse("1.1,2.2,3.3,4.4").unwrap();

        assert_eq!(CornerRadius::new(1.1, 2.2, 3.3, 4.4), result);
    }

    #[test]
    fn parse_accepts_spaces() {
        let result = CornerRadius::parse("1.1 2.2 3.3 4.4").unwrap();

        assert_eq!(CornerRadius::new(1.1, 2.2, 3.3, 4.4), result);
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn display_uniform_and_errors() {
        assert_eq!(
            CornerRadius::new(1.0, 2.0, 3.0, 4.5).to_string(),
            "1,2,3,4.5"
        );
        assert!(CornerRadius::uniform(3.0).is_uniform());
        assert!(!CornerRadius::top_bottom(1.0, 2.0).is_uniform());
        assert_eq!(
            CornerRadius::parse("a").unwrap_err().message(),
            "Invalid CornerRadius."
        );
        assert!(CornerRadius::parse("1,2,3").is_err());
    }
}
