//! `Point3D`: a location in 3D space.

use std::fmt;
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};
use std::str::FromStr;

use ferroui_base::utilities::FormatError;

use crate::text::{parse_numbers, write_numbers};
use crate::{Matrix3D, Point4D, Vector3D};

/// A location in 3D space, with double-precision coordinates (WPF's
/// `System.Windows.Media.Media3D.Point3D`).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point3D {
    /// The X coordinate.
    pub x: f64,
    /// The Y coordinate.
    pub y: f64,
    /// The Z coordinate.
    pub z: f64,
}

impl Point3D {
    /// A point with the given coordinates.
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Moves the point by the given amounts.
    #[inline]
    pub fn offset(&mut self, offset_x: f64, offset_y: f64, offset_z: f64) {
        self.x += offset_x;
        self.y += offset_y;
        self.z += offset_z;
    }

    /// Parses the text form `"x,y,z"` (commas, white space or both between the numbers) in the
    /// invariant culture.
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        let [x, y, z] = parse_numbers::<3>(text, "Invalid Point3D.")?;
        Ok(Point3D::new(x, y, z))
    }
}

impl Add<Vector3D> for Point3D {
    type Output = Point3D;
    #[inline]
    fn add(self, vector: Vector3D) -> Point3D {
        Point3D::new(self.x + vector.x, self.y + vector.y, self.z + vector.z)
    }
}

impl Sub<Vector3D> for Point3D {
    type Output = Point3D;
    #[inline]
    fn sub(self, vector: Vector3D) -> Point3D {
        Point3D::new(self.x - vector.x, self.y - vector.y, self.z - vector.z)
    }
}

/// The displacement from the second point to the first.
impl Sub for Point3D {
    type Output = Vector3D;
    #[inline]
    fn sub(self, point: Point3D) -> Vector3D {
        Vector3D::new(self.x - point.x, self.y - point.y, self.z - point.z)
    }
}

/// The point transformed by the matrix: [`Matrix3D::transform`] (with the projective divide of a
/// matrix that is not affine).
impl Mul<Matrix3D> for Point3D {
    type Output = Point3D;
    #[inline]
    fn mul(self, matrix: Matrix3D) -> Point3D {
        matrix.transform(self)
    }
}

impl AddAssign<Vector3D> for Point3D {
    #[inline]
    fn add_assign(&mut self, vector: Vector3D) {
        *self = *self + vector;
    }
}

impl SubAssign<Vector3D> for Point3D {
    #[inline]
    fn sub_assign(&mut self, vector: Vector3D) {
        *self = *self - vector;
    }
}

/// WPF's explicit conversion to the vector from the origin to the point.
impl From<Point3D> for Vector3D {
    #[inline]
    fn from(point: Point3D) -> Vector3D {
        Vector3D::new(point.x, point.y, point.z)
    }
}

/// WPF's explicit conversion to homogeneous coordinates, with `w` 1.
impl From<Point3D> for Point4D {
    #[inline]
    fn from(point: Point3D) -> Point4D {
        Point4D::new(point.x, point.y, point.z, 1.0)
    }
}

impl FromStr for Point3D {
    type Err = FormatError;
    #[inline]
    fn from_str(text: &str) -> Result<Self, FormatError> {
        Point3D::parse(text)
    }
}

/// The text form `"x,y,z"` in the invariant culture.
impl fmt::Display for Point3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_numbers(f, &[self.x, self.y, self.z])
    }
}

#[cfg(test)]
#[path = "point_3d_tests.rs"]
mod tests;
