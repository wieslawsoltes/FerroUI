//! Defines a scalar that may be defined relative to a containing element.

use std::fmt;
use std::str::FromStr;

use crate::utilities::math_utilities;
use crate::utilities::span_helpers::{parse_double, InvariantF64};
use crate::utilities::FormatError;
use crate::RelativeUnit;

/// Defines a scalar that may be defined relative to a containing element.
///
/// Equality treats NaN scalars as equal to each other.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelativeScalar {
    /// The scalar.
    pub scalar: f64,
    /// The unit of the scalar.
    pub unit: RelativeUnit,
}

impl RelativeScalar {
    /// The beginning of the containing element (0%).
    pub const BEGINNING: RelativeScalar = RelativeScalar::new(0.0, RelativeUnit::Relative);

    /// The middle of the containing element (50%).
    pub const MIDDLE: RelativeScalar = RelativeScalar::new(0.5, RelativeUnit::Relative);

    /// The end of the containing element (100%).
    pub const END: RelativeScalar = RelativeScalar::new(1.0, RelativeUnit::Relative);

    #[inline]
    pub const fn new(scalar: f64, unit: RelativeUnit) -> Self {
        Self { scalar, unit }
    }

    #[inline]
    pub fn equals(&self, other: RelativeScalar) -> bool {
        math_utilities::equals(self.scalar, other.scalar) && self.unit == other.unit
    }

    /// Converts the scalar into a final value, using `size` as the size the scalar
    /// is relative to.
    #[inline]
    pub fn to_value(&self, size: f64) -> f64 {
        if self.unit == RelativeUnit::Absolute {
            self.scalar
        } else {
            size * self.scalar
        }
    }

    /// Parses a [`RelativeScalar`] string: `"n"` (absolute) or `"n%"` (relative).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        let trimmed = s.trim();
        if trimmed.ends_with('%') {
            let number = trimmed.trim_end_matches('%');
            let value = parse_double(number).ok_or_else(|| FormatError::invalid_input(number))?;
            return Ok(RelativeScalar::new(value * 0.01, RelativeUnit::Relative));
        }

        let value = parse_double(trimmed).ok_or_else(|| FormatError::invalid_input(trimmed))?;
        Ok(RelativeScalar::new(value, RelativeUnit::Absolute))
    }
}

impl PartialEq for RelativeScalar {
    #[inline]
    fn eq(&self, other: &RelativeScalar) -> bool {
        self.equals(*other)
    }
}

impl FromStr for RelativeScalar {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        RelativeScalar::parse(s)
    }
}

impl fmt::Display for RelativeScalar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.unit == RelativeUnit::Absolute {
            write!(f, "{}", InvariantF64(self.scalar))
        } else {
            write!(f, "{}%", InvariantF64(self.scalar * 100.0))
        }
    }
}

#[cfg(test)]
mod tests {
    // The reference test-suite has no tests for this type.
    use super::*;

    #[test]
    fn parse_display_and_to_value() {
        assert_eq!(
            RelativeScalar::parse(" 50% ").unwrap(),
            RelativeScalar::MIDDLE
        );
        assert_eq!(
            RelativeScalar::parse("12.5").unwrap(),
            RelativeScalar::new(12.5, RelativeUnit::Absolute)
        );
        assert_eq!(
            RelativeScalar::parse("1,000").unwrap(),
            RelativeScalar::new(1000.0, RelativeUnit::Absolute)
        );
        assert!(RelativeScalar::parse("").is_err());
        assert!(RelativeScalar::parse("abc%").is_err());
        assert_eq!(RelativeScalar::MIDDLE.to_string(), "50%");
        assert_eq!(
            RelativeScalar::new(12.5, RelativeUnit::Absolute).to_string(),
            "12.5"
        );
        assert_eq!(RelativeScalar::MIDDLE.to_value(200.0), 100.0);
        assert_eq!(
            RelativeScalar::new(12.5, RelativeUnit::Absolute).to_value(200.0),
            12.5
        );
        assert_eq!(
            RelativeScalar::new(f64::NAN, RelativeUnit::Absolute),
            RelativeScalar::new(f64::NAN, RelativeUnit::Absolute)
        );
    }
}
