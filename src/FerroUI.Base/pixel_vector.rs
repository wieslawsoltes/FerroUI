//! Defines a vector in device pixels.

use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::PixelPoint;

/// Defines a vector in device pixels. Integer arithmetic wraps on overflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PixelVector {
    /// The X component.
    pub x: i32,
    /// The Y component.
    pub y: i32,
}

impl PixelVector {
    /// Initializes a new instance of the [`PixelVector`] structure.
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Length of the vector. The squared length is computed in (wrapping) 32-bit
    /// integer arithmetic, so very large vectors yield NaN or a wrapped length.
    #[inline]
    pub fn length(&self) -> f64 {
        (self
            .x
            .wrapping_mul(self.x)
            .wrapping_add(self.y.wrapping_mul(self.y)) as f64)
            .sqrt()
    }

    /// Check if two vectors are equal (bitwise).
    #[inline]
    pub const fn equals(&self, other: PixelVector) -> bool {
        self.x == other.x && self.y == other.y
    }

    /// Check if two vectors are nearly equal (numerically). For integer components
    /// this is the same as equality.
    ///
    /// # Panics
    /// Panics if a component difference is `i32::MIN`, whose absolute value is not
    /// representable.
    pub fn nearly_equals(&self, other: PixelVector) -> bool {
        // Smallest positive (subnormal) single-precision value.
        const TOLERANCE: f32 = f32::from_bits(1);
        const OVERFLOW: &str = "Negating the minimum value of a twos complement number is invalid.";

        let dx = self.x.wrapping_sub(other.x).checked_abs().expect(OVERFLOW);
        let dy = self.y.wrapping_sub(other.y).checked_abs().expect(OVERFLOW);
        (dx as f32) < TOLERANCE && (dy as f32) < TOLERANCE
    }

    /// Returns a new vector with the specified X component.
    #[inline]
    pub const fn with_x(&self, x: i32) -> PixelVector {
        PixelVector::new(x, self.y)
    }

    /// Returns a new vector with the specified Y component.
    #[inline]
    pub const fn with_y(&self, y: i32) -> PixelVector {
        PixelVector::new(self.x, y)
    }
}

/// Converts the [`PixelVector`] to a [`PixelPoint`] (an explicit conversion in the
/// reference API).
impl From<PixelVector> for PixelPoint {
    #[inline]
    fn from(a: PixelVector) -> PixelPoint {
        PixelPoint::new(a.x, a.y)
    }
}

/// Calculates the dot product of two vectors.
impl Mul for PixelVector {
    type Output = i32;
    #[inline]
    fn mul(self, b: PixelVector) -> i32 {
        self.x
            .wrapping_mul(b.x)
            .wrapping_add(self.y.wrapping_mul(b.y))
    }
}

/// Scales a vector.
impl Mul<i32> for PixelVector {
    type Output = PixelVector;
    #[inline]
    fn mul(self, scale: i32) -> PixelVector {
        PixelVector::new(self.x.wrapping_mul(scale), self.y.wrapping_mul(scale))
    }
}

/// Scales a vector.
///
/// # Panics
/// Panics on division by zero and on `i32::MIN / -1`.
impl Div<i32> for PixelVector {
    type Output = PixelVector;
    #[inline]
    fn div(self, scale: i32) -> PixelVector {
        PixelVector::new(self.x / scale, self.y / scale)
    }
}

impl Neg for PixelVector {
    type Output = PixelVector;
    #[inline]
    fn neg(self) -> PixelVector {
        PixelVector::new(self.x.wrapping_neg(), self.y.wrapping_neg())
    }
}

impl Add for PixelVector {
    type Output = PixelVector;
    #[inline]
    fn add(self, b: PixelVector) -> PixelVector {
        PixelVector::new(self.x.wrapping_add(b.x), self.y.wrapping_add(b.y))
    }
}

impl Sub for PixelVector {
    type Output = PixelVector;
    #[inline]
    fn sub(self, b: PixelVector) -> PixelVector {
        PixelVector::new(self.x.wrapping_sub(b.x), self.y.wrapping_sub(b.y))
    }
}

impl AddAssign for PixelVector {
    #[inline]
    fn add_assign(&mut self, b: PixelVector) {
        *self = *self + b;
    }
}

impl SubAssign for PixelVector {
    #[inline]
    fn sub_assign(&mut self, b: PixelVector) {
        *self = *self - b;
    }
}

impl MulAssign<i32> for PixelVector {
    #[inline]
    fn mul_assign(&mut self, scale: i32) {
        *self = *self * scale;
    }
}

impl DivAssign<i32> for PixelVector {
    #[inline]
    fn div_assign(&mut self, scale: i32) {
        *self = *self / scale;
    }
}

impl fmt::Display for PixelVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    // The reference test-suite has no tests for this type.
    use super::*;

    #[test]
    fn arithmetic_and_display() {
        let a = PixelVector::new(3, 4);
        let b = PixelVector::new(1, 2);
        assert_eq!(a * b, 11);
        assert_eq!(a * 2, PixelVector::new(6, 8));
        assert_eq!(a / 2, PixelVector::new(1, 2));
        assert_eq!(-a, PixelVector::new(-3, -4));
        assert_eq!(a + b, PixelVector::new(4, 6));
        assert_eq!(a - b, PixelVector::new(2, 2));
        assert_eq!(a.length(), 5.0);
        assert_eq!(a.to_string(), "3, 4");
        assert_eq!(PixelPoint::from(a), PixelPoint::new(3, 4));
        assert!(a.nearly_equals(a));
        assert!(!a.nearly_equals(b));
        assert_eq!(a.with_x(9).with_y(8), PixelVector::new(9, 8));
    }

    #[test]
    #[should_panic]
    fn division_by_zero_panics() {
        let _ = PixelVector::new(1, 1) / 0;
    }
}
