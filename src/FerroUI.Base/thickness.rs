//! Describes the thickness of a frame around a rectangle.

use std::fmt;
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};
use std::str::FromStr;

use crate::utilities::math_utilities;
use crate::utilities::span_helpers::InvariantF64;
use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::Size;

/// Describes the thickness of a frame around a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Thickness {
    /// The thickness on the left.
    pub left: f64,
    /// The thickness on the top.
    pub top: f64,
    /// The thickness on the right.
    pub right: f64,
    /// The thickness on the bottom.
    pub bottom: f64,
}

impl Thickness {
    /// Initializes a new instance of the [`Thickness`] structure from the four sides.
    #[inline]
    pub const fn new(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Initializes a [`Thickness`] with the same length on every side.
    #[inline]
    pub const fn uniform(uniform_length: f64) -> Self {
        Self::new(
            uniform_length,
            uniform_length,
            uniform_length,
            uniform_length,
        )
    }

    /// Initializes a [`Thickness`] from the thickness on the left and right, and
    /// the thickness on the top and bottom.
    #[inline]
    pub const fn symmetric(horizontal: f64, vertical: f64) -> Self {
        Self::new(horizontal, vertical, horizontal, vertical)
    }

    /// Gets a value indicating whether all sides are equal (NaN compares equal to NaN here).
    #[inline]
    pub fn is_uniform(&self) -> bool {
        math_utilities::equals(self.left, self.right)
            && math_utilities::equals(self.top, self.bottom)
            && math_utilities::equals(self.right, self.bottom)
    }

    /// Parses a [`Thickness`] string: one, two or four numbers.
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        const EXCEPTION_MESSAGE: &str = "Invalid Thickness.";

        SpanStringTokenizer::with_message(s, EXCEPTION_MESSAGE).scope(|t| {
            if let Some(a) = t.try_read_double()? {
                if let Some(b) = t.try_read_double()? {
                    if let Some(c) = t.try_read_double()? {
                        return Ok(Thickness::new(a, b, c, t.read_double()?));
                    }

                    return Ok(Thickness::symmetric(a, b));
                }

                return Ok(Thickness::uniform(a));
            }

            Err(FormatError::new(EXCEPTION_MESSAGE))
        })
    }

    /// Returns a boolean indicating whether the thickness is equal to the other
    /// given thickness (exact comparison of the sides).
    #[inline]
    pub fn equals(&self, other: Thickness) -> bool {
        self.left == other.left
            && self.top == other.top
            && self.right == other.right
            && self.bottom == other.bottom
    }

    /// Deconstructs the thickness into its left, top, right and bottom values.
    #[inline]
    pub const fn deconstruct(&self) -> (f64, f64, f64, f64) {
        (self.left, self.top, self.right, self.bottom)
    }
}

impl From<Thickness> for (f64, f64, f64, f64) {
    #[inline]
    fn from(t: Thickness) -> Self {
        (t.left, t.top, t.right, t.bottom)
    }
}

impl Add for Thickness {
    type Output = Thickness;
    #[inline]
    fn add(self, b: Thickness) -> Thickness {
        Thickness::new(
            self.left + b.left,
            self.top + b.top,
            self.right + b.right,
            self.bottom + b.bottom,
        )
    }
}

impl Sub for Thickness {
    type Output = Thickness;
    #[inline]
    fn sub(self, b: Thickness) -> Thickness {
        Thickness::new(
            self.left - b.left,
            self.top - b.top,
            self.right - b.right,
            self.bottom - b.bottom,
        )
    }
}

impl Mul<f64> for Thickness {
    type Output = Thickness;
    #[inline]
    fn mul(self, b: f64) -> Thickness {
        Thickness::new(self.left * b, self.top * b, self.right * b, self.bottom * b)
    }
}

/// Adds a thickness to a size.
impl Add<Thickness> for Size {
    type Output = Size;
    #[inline]
    fn add(self, thickness: Thickness) -> Size {
        Size::new(
            self.width + thickness.left + thickness.right,
            self.height + thickness.top + thickness.bottom,
        )
    }
}

/// Subtracts a thickness from a size.
impl Sub<Thickness> for Size {
    type Output = Size;
    #[inline]
    fn sub(self, thickness: Thickness) -> Size {
        Size::new(
            self.width - (thickness.left + thickness.right),
            self.height - (thickness.top + thickness.bottom),
        )
    }
}

impl AddAssign for Thickness {
    #[inline]
    fn add_assign(&mut self, b: Thickness) {
        *self = *self + b;
    }
}

impl SubAssign for Thickness {
    #[inline]
    fn sub_assign(&mut self, b: Thickness) {
        *self = *self - b;
    }
}

impl MulAssign<f64> for Thickness {
    #[inline]
    fn mul_assign(&mut self, b: f64) {
        *self = *self * b;
    }
}

impl AddAssign<Thickness> for Size {
    #[inline]
    fn add_assign(&mut self, thickness: Thickness) {
        *self = *self + thickness;
    }
}

impl SubAssign<Thickness> for Size {
    #[inline]
    fn sub_assign(&mut self, thickness: Thickness) {
        *self = *self - thickness;
    }
}

impl FromStr for Thickness {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        Thickness::parse(s)
    }
}

impl fmt::Display for Thickness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{},{},{},{}",
            InvariantF64(self.left),
            InvariantF64(self.top),
            InvariantF64(self.right),
            InvariantF64(self.bottom)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_parses_single_uniform_size() {
        let result = Thickness::parse("1.2").unwrap();

        assert_eq!(Thickness::uniform(1.2), result);
    }

    #[test]
    fn parse_parses_horizontal_vertical() {
        let result = Thickness::parse("1.2,3.4").unwrap();

        assert_eq!(Thickness::symmetric(1.2, 3.4), result);
    }

    #[test]
    fn parse_parses_left_top_right_bottom() {
        let result = Thickness::parse("1.2, 3.4, 5, 6").unwrap();

        assert_eq!(Thickness::new(1.2, 3.4, 5.0, 6.0), result);
    }

    #[test]
    fn parse_accepts_spaces() {
        let result = Thickness::parse("1.2 3.4 5 6").unwrap();

        assert_eq!(Thickness::new(1.2, 3.4, 5.0, 6.0), result);
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn parse_errors_display_and_operators() {
        assert_eq!(
            Thickness::parse("").unwrap_err().message(),
            "Invalid Thickness."
        );
        assert!(Thickness::parse("1,2,3").is_err());
        assert!(Thickness::parse("1,2,3,4,5").is_err());
        assert_eq!(Thickness::new(1.0, 2.5, 3.0, 4.0).to_string(), "1,2.5,3,4");
        assert!(Thickness::uniform(2.0).is_uniform());
        assert!(Thickness::uniform(f64::NAN).is_uniform());
        assert!(!Thickness::symmetric(1.0, 2.0).is_uniform());
        let t = Thickness::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(t + t, t * 2.0);
        assert_eq!(t - t, Thickness::default());
        assert_eq!(Size::new(10.0, 10.0) + t, Size::new(14.0, 16.0));
        assert_eq!(Size::new(10.0, 10.0) - t, Size::new(6.0, 4.0));
    }
}
