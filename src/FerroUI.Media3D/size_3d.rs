//! `Size3D`: the extent of a box in 3D space.

use std::fmt;
use std::str::FromStr;

use ferroui_base::utilities::FormatError;

use crate::text::{is_keyword, parse_numbers, write_numbers};
use crate::{Point3D, Size3DError, Vector3D};

/// The extent of a box in 3D space, with double-precision dimensions (WPF's
/// `System.Windows.Media.Media3D.Size3D`).
///
/// A dimension is never negative, except in [`Size3D::EMPTY`], the size of no box, whose dimensions
/// are negative infinity. The members that WPF guards with exceptions return a [`Size3DError`].
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Size3D {
    x: f64,
    y: f64,
    z: f64,
}

impl Size3D {
    /// The size of no box (WPF's `Size3D.Empty`): every dimension negative infinity.
    pub const EMPTY: Size3D = Size3D {
        x: f64::NEG_INFINITY,
        y: f64::NEG_INFINITY,
        z: f64::NEG_INFINITY,
    };

    /// A size with the given dimensions; [`Size3DError::NegativeDimension`] when one is negative.
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, Size3DError> {
        if x < 0.0 || y < 0.0 || z < 0.0 {
            return Err(Size3DError::NegativeDimension);
        }
        Ok(Self { x, y, z })
    }

    /// The size with the magnitudes of the given numbers, which is never negative.
    #[inline]
    pub(crate) fn from_magnitudes(x: f64, y: f64, z: f64) -> Self {
        Self {
            x: x.abs(),
            y: y.abs(),
            z: z.abs(),
        }
    }

    /// Whether this is [`Size3D::EMPTY`].
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.x < 0.0
    }

    /// The extent along the X axis.
    #[inline]
    pub fn x(&self) -> f64 {
        self.x
    }

    /// The extent along the Y axis.
    #[inline]
    pub fn y(&self) -> f64 {
        self.y
    }

    /// The extent along the Z axis.
    #[inline]
    pub fn z(&self) -> f64 {
        self.z
    }

    /// Sets the extent along the X axis.
    pub fn set_x(&mut self, value: f64) -> Result<(), Size3DError> {
        self.x = self.checked(value)?;
        Ok(())
    }

    /// Sets the extent along the Y axis.
    pub fn set_y(&mut self, value: f64) -> Result<(), Size3DError> {
        self.y = self.checked(value)?;
        Ok(())
    }

    /// Sets the extent along the Z axis.
    pub fn set_z(&mut self, value: f64) -> Result<(), Size3DError> {
        self.z = self.checked(value)?;
        Ok(())
    }

    /// A new dimension, checked as WPF's setters check it: the empty size is not changed, and a
    /// dimension is not negative.
    fn checked(&self, value: f64) -> Result<f64, Size3DError> {
        if self.is_empty() {
            return Err(Size3DError::ModifiesEmpty);
        }
        if value < 0.0 {
            return Err(Size3DError::NegativeDimension);
        }
        Ok(value)
    }

    /// Parses the text form: `"Empty"`, or `"x,y,z"` (commas, white space or both between the
    /// numbers) in the invariant culture, none of them negative.
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        if is_keyword(text, "Empty") {
            return Ok(Size3D::EMPTY);
        }
        let [x, y, z] = parse_numbers::<3>(text, "Invalid Size3D.")?;
        Size3D::new(x, y, z).map_err(|error| FormatError::new(error.message()))
    }
}

/// WPF's explicit conversion to a vector with the dimensions as components.
impl From<Size3D> for Vector3D {
    #[inline]
    fn from(size: Size3D) -> Vector3D {
        Vector3D::new(size.x, size.y, size.z)
    }
}

/// WPF's explicit conversion to a point with the dimensions as coordinates.
impl From<Size3D> for Point3D {
    #[inline]
    fn from(size: Size3D) -> Point3D {
        Point3D::new(size.x, size.y, size.z)
    }
}

impl FromStr for Size3D {
    type Err = FormatError;
    #[inline]
    fn from_str(text: &str) -> Result<Self, FormatError> {
        Size3D::parse(text)
    }
}

/// The text form in the invariant culture: `"Empty"`, or `"x,y,z"`.
impl fmt::Display for Size3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("Empty");
        }
        write_numbers(f, &[self.x, self.y, self.z])
    }
}

#[cfg(test)]
#[path = "size_3d_tests.rs"]
mod tests;
