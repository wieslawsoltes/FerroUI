//! `Point4D`: a point in homogeneous coordinates.

use std::fmt;
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};
use std::str::FromStr;

use ferroui_base::utilities::FormatError;

use crate::text::{parse_numbers, write_numbers};
use crate::Matrix3D;

/// A point in homogeneous coordinates `(x, y, z, w)`, with double-precision coordinates (WPF's
/// `System.Windows.Media.Media3D.Point4D`). A [`Point3D`](crate::Point3D) is the point with `w`
/// 1; a projective [`Matrix3D`] transforms a `Point4D` without the divide by `w`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point4D {
    /// The X coordinate.
    pub x: f64,
    /// The Y coordinate.
    pub y: f64,
    /// The Z coordinate.
    pub z: f64,
    /// The W coordinate.
    pub w: f64,
}

impl Point4D {
    /// A point with the given coordinates.
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    /// Moves the point by the given amounts.
    #[inline]
    pub fn offset(&mut self, delta_x: f64, delta_y: f64, delta_z: f64, delta_w: f64) {
        self.x += delta_x;
        self.y += delta_y;
        self.z += delta_z;
        self.w += delta_w;
    }

    /// Parses the text form `"x,y,z,w"` (commas, white space or both between the numbers) in the
    /// invariant culture.
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        let [x, y, z, w] = parse_numbers::<4>(text, "Invalid Point4D.")?;
        Ok(Point4D::new(x, y, z, w))
    }
}

impl Add for Point4D {
    type Output = Point4D;
    #[inline]
    fn add(self, point: Point4D) -> Point4D {
        Point4D::new(self.x + point.x, self.y + point.y, self.z + point.z, self.w + point.w)
    }
}

impl Sub for Point4D {
    type Output = Point4D;
    #[inline]
    fn sub(self, point: Point4D) -> Point4D {
        Point4D::new(self.x - point.x, self.y - point.y, self.z - point.z, self.w - point.w)
    }
}

/// The point transformed by the matrix: [`Matrix3D::transform_point4d`].
impl Mul<Matrix3D> for Point4D {
    type Output = Point4D;
    #[inline]
    fn mul(self, matrix: Matrix3D) -> Point4D {
        matrix.transform_point4d(self)
    }
}

impl AddAssign for Point4D {
    #[inline]
    fn add_assign(&mut self, point: Point4D) {
        *self = *self + point;
    }
}

impl SubAssign for Point4D {
    #[inline]
    fn sub_assign(&mut self, point: Point4D) {
        *self = *self - point;
    }
}

impl FromStr for Point4D {
    type Err = FormatError;
    #[inline]
    fn from_str(text: &str) -> Result<Self, FormatError> {
        Point4D::parse(text)
    }
}

/// The text form `"x,y,z,w"` in the invariant culture.
impl fmt::Display for Point4D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_numbers(f, &[self.x, self.y, self.z, self.w])
    }
}

#[cfg(test)]
#[path = "point_4d_tests.rs"]
mod tests;
