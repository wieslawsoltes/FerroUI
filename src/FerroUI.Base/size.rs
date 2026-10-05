//! Defines a size.

use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use std::str::FromStr;

use crate::utilities::math_utilities::{self, MathUtilities};
use crate::utilities::span_helpers::InvariantF64;
use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::{Thickness, Vector};

/// Defines a size.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Size {
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

impl Size {
    /// A size representing infinity.
    pub const INFINITY: Size = Size::new(f64::INFINITY, f64::INFINITY);

    /// Initializes a new instance of the [`Size`] structure.
    #[inline]
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Gets the aspect ratio of the size.
    #[inline]
    pub fn aspect_ratio(&self) -> f64 {
        self.width / self.height
    }

    /// Parses a [`Size`] string (`"width, height"` or `"width height"`).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        SpanStringTokenizer::with_message(s, "Invalid Size.")
            .scope(|t| Ok(Size::new(t.read_double()?, t.read_double()?)))
    }

    /// Constrains the size: each dimension becomes the smaller of this size's and
    /// the constraint's.
    #[inline]
    pub fn constrain(&self, constraint: Size) -> Size {
        Size::new(
            math_utilities::min(self.width, constraint.width),
            math_utilities::min(self.height, constraint.height),
        )
    }

    /// Deflates the size by a [`Thickness`]. The result never has negative dimensions.
    #[inline]
    pub fn deflate(&self, thickness: Thickness) -> Size {
        let mut width = self.width - thickness.left - thickness.right;
        if width < 0.0 {
            width = 0.0;
        }

        let mut height = self.height - thickness.top - thickness.bottom;
        if height < 0.0 {
            height = 0.0;
        }

        Size::new(width, height)
    }

    /// Returns a boolean indicating whether the size is equal to the other given
    /// size (exact comparison of the dimensions).
    #[inline]
    pub fn equals(&self, other: Size) -> bool {
        self.width == other.width && self.height == other.height
    }

    /// Returns a boolean indicating whether the size is equal to the other given
    /// size (numerically, within a relative epsilon).
    #[inline]
    pub fn nearly_equals(&self, other: Size) -> bool {
        MathUtilities::are_close(self.width, other.width)
            && MathUtilities::are_close(self.height, other.height)
    }

    /// Inflates the size by a [`Thickness`].
    #[inline]
    pub fn inflate(&self, thickness: Thickness) -> Size {
        Size::new(
            self.width + thickness.left + thickness.right,
            self.height + thickness.top + thickness.bottom,
        )
    }

    /// Returns a new [`Size`] with the same height and the specified width.
    #[inline]
    pub const fn with_width(&self, width: f64) -> Size {
        Size::new(width, self.height)
    }

    /// Returns a new [`Size`] with the same width and the specified height.
    #[inline]
    pub const fn with_height(&self, height: f64) -> Size {
        Size::new(self.width, height)
    }

    /// Deconstructs the size into its width and height values.
    #[inline]
    pub const fn deconstruct(&self) -> (f64, f64) {
        (self.width, self.height)
    }
}

impl From<Size> for (f64, f64) {
    #[inline]
    fn from(s: Size) -> Self {
        (s.width, s.height)
    }
}

/// Scales a size.
impl Mul<Vector> for Size {
    type Output = Size;
    #[inline]
    fn mul(self, scale: Vector) -> Size {
        Size::new(self.width * scale.x, self.height * scale.y)
    }
}

/// Scales a size.
impl Div<Vector> for Size {
    type Output = Size;
    #[inline]
    fn div(self, scale: Vector) -> Size {
        Size::new(self.width / scale.x, self.height / scale.y)
    }
}

/// Divides a size by another size to produce a scaling factor.
impl Div for Size {
    type Output = Vector;
    #[inline]
    fn div(self, right: Size) -> Vector {
        Vector::new(self.width / right.width, self.height / right.height)
    }
}

/// Scales a size.
impl Mul<f64> for Size {
    type Output = Size;
    #[inline]
    fn mul(self, scale: f64) -> Size {
        Size::new(self.width * scale, self.height * scale)
    }
}

/// Scales a size.
impl Div<f64> for Size {
    type Output = Size;
    #[inline]
    fn div(self, scale: f64) -> Size {
        Size::new(self.width / scale, self.height / scale)
    }
}

impl Add for Size {
    type Output = Size;
    #[inline]
    fn add(self, to_add: Size) -> Size {
        Size::new(self.width + to_add.width, self.height + to_add.height)
    }
}

impl Sub for Size {
    type Output = Size;
    #[inline]
    fn sub(self, to_subtract: Size) -> Size {
        Size::new(
            self.width - to_subtract.width,
            self.height - to_subtract.height,
        )
    }
}

impl AddAssign for Size {
    #[inline]
    fn add_assign(&mut self, rhs: Size) {
        *self = *self + rhs;
    }
}

impl SubAssign for Size {
    #[inline]
    fn sub_assign(&mut self, rhs: Size) {
        *self = *self - rhs;
    }
}

impl MulAssign<f64> for Size {
    #[inline]
    fn mul_assign(&mut self, scale: f64) {
        *self = *self * scale;
    }
}

impl DivAssign<f64> for Size {
    #[inline]
    fn div_assign(&mut self, scale: f64) {
        *self = *self / scale;
    }
}

impl MulAssign<Vector> for Size {
    #[inline]
    fn mul_assign(&mut self, scale: Vector) {
        *self = *self * scale;
    }
}

impl DivAssign<Vector> for Size {
    #[inline]
    fn div_assign(&mut self, scale: Vector) {
        *self = *self / scale;
    }
}

impl FromStr for Size {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        Size::parse(s)
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}, {}",
            InvariantF64(self.width),
            InvariantF64(self.height)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_produce_correct_aspect_ratio() {
        let result = Size::new(3.0, 2.0).aspect_ratio();

        assert_eq!(1.5, result);
    }

    #[test]
    fn dividing_should_produce_scaling_factor() {
        let result = Size::new(15.0, 10.0) / Size::new(5.0, 5.0);

        assert_eq!(Vector::new(3.0, 2.0), result);
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn parse_display_inflate_deflate_constrain() {
        assert_eq!(Size::parse("10, 20").unwrap(), Size::new(10.0, 20.0));
        assert_eq!(Size::parse("10").unwrap_err().message(), "Invalid Size.");
        assert_eq!(Size::INFINITY.to_string(), "Infinity, Infinity");
        let t = Thickness::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(Size::new(10.0, 10.0).inflate(t), Size::new(14.0, 16.0));
        assert_eq!(Size::new(10.0, 10.0).deflate(t), Size::new(6.0, 4.0));
        assert_eq!(Size::new(1.0, 1.0).deflate(t), Size::new(0.0, 0.0));
        assert_eq!(
            Size::new(10.0, 30.0).constrain(Size::new(20.0, 20.0)),
            Size::new(10.0, 20.0)
        );
        assert!(Size::new(f64::NAN, 1.0)
            .constrain(Size::new(1.0, 1.0))
            .width
            .is_nan());
    }
}
