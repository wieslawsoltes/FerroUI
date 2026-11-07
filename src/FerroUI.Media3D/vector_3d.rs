//! `Vector3D`: a displacement in 3D space.

use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

use ferroui_base::utilities::FormatError;

use crate::text::{parse_numbers, write_numbers};
use crate::{Matrix3D, Point3D, Size3D};

/// A displacement in 3D space, with double-precision components (WPF's
/// `System.Windows.Media.Media3D.Vector3D`).
///
/// This is the vector of the 3D scene model. The base library's `ferroui_base::Vector3D` is the
/// vector of the reference's composition API; the two convert into each other with `From`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vector3D {
    /// The X component.
    pub x: f64,
    /// The Y component.
    pub y: f64,
    /// The Z component.
    pub z: f64,
}

impl Vector3D {
    /// A vector with the given components.
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// The length of the vector.
    #[inline]
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    /// The square of the length of the vector.
    #[inline]
    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Makes the vector a unit vector of the same direction. The components are first divided by
    /// the largest of their magnitudes, so the length does not overflow. The zero vector becomes
    /// `NaN` components, as in WPF.
    pub fn normalize(&mut self) {
        let largest = self.x.abs().max(self.y.abs()).max(self.z.abs());
        self.x /= largest;
        self.y /= largest;
        self.z /= largest;
        let length = self.length();
        *self /= length;
    }

    /// Reverses the direction of the vector.
    #[inline]
    pub fn negate(&mut self) {
        *self = -*self;
    }

    /// The dot product of two vectors.
    #[inline]
    pub fn dot_product(vector1: Vector3D, vector2: Vector3D) -> f64 {
        vector1.x * vector2.x + vector1.y * vector2.y + vector1.z * vector2.z
    }

    /// The cross product of two vectors: perpendicular to both, in the right-handed sense.
    #[inline]
    pub fn cross_product(vector1: Vector3D, vector2: Vector3D) -> Vector3D {
        Vector3D::new(
            vector1.y * vector2.z - vector1.z * vector2.y,
            vector1.z * vector2.x - vector1.x * vector2.z,
            vector1.x * vector2.y - vector1.y * vector2.x,
        )
    }

    /// The angle between two vectors, in degrees, from 0 to 180. The angle is computed from the
    /// length of the difference of the unit vectors, which is precise also near 0 and 180 degrees
    /// where the arc cosine of the dot product is not.
    pub fn angle_between(vector1: Vector3D, vector2: Vector3D) -> f64 {
        let mut vector1 = vector1;
        let mut vector2 = vector2;
        vector1.normalize();
        vector2.normalize();
        let ratio = Vector3D::dot_product(vector1, vector2);
        let theta = if ratio < 0.0 {
            std::f64::consts::PI - 2.0 * ((-vector1 - vector2).length() / 2.0).asin()
        } else {
            2.0 * ((vector1 - vector2).length() / 2.0).asin()
        };
        theta.to_degrees()
    }

    /// Parses the text form `"x,y,z"` (commas, white space or both between the numbers) in the
    /// invariant culture.
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        let [x, y, z] = parse_numbers::<3>(text, "Invalid Vector3D.")?;
        Ok(Vector3D::new(x, y, z))
    }
}

impl Add for Vector3D {
    type Output = Vector3D;
    #[inline]
    fn add(self, vector: Vector3D) -> Vector3D {
        Vector3D::new(self.x + vector.x, self.y + vector.y, self.z + vector.z)
    }
}

impl Add<Point3D> for Vector3D {
    type Output = Point3D;
    #[inline]
    fn add(self, point: Point3D) -> Point3D {
        Point3D::new(self.x + point.x, self.y + point.y, self.z + point.z)
    }
}

impl Sub for Vector3D {
    type Output = Vector3D;
    #[inline]
    fn sub(self, vector: Vector3D) -> Vector3D {
        Vector3D::new(self.x - vector.x, self.y - vector.y, self.z - vector.z)
    }
}

impl Sub<Point3D> for Vector3D {
    type Output = Point3D;
    #[inline]
    fn sub(self, point: Point3D) -> Point3D {
        Point3D::new(self.x - point.x, self.y - point.y, self.z - point.z)
    }
}

impl Neg for Vector3D {
    type Output = Vector3D;
    #[inline]
    fn neg(self) -> Vector3D {
        Vector3D::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<f64> for Vector3D {
    type Output = Vector3D;
    #[inline]
    fn mul(self, scalar: f64) -> Vector3D {
        Vector3D::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

impl Mul<Vector3D> for f64 {
    type Output = Vector3D;
    #[inline]
    fn mul(self, vector: Vector3D) -> Vector3D {
        vector * self
    }
}

/// The vector transformed by the matrix: [`Matrix3D::transform_vector`] (the translation of the
/// matrix does not apply to a vector).
impl Mul<Matrix3D> for Vector3D {
    type Output = Vector3D;
    #[inline]
    fn mul(self, matrix: Matrix3D) -> Vector3D {
        matrix.transform_vector(self)
    }
}

impl Div<f64> for Vector3D {
    type Output = Vector3D;
    #[inline]
    fn div(self, scalar: f64) -> Vector3D {
        self * (1.0 / scalar)
    }
}

impl AddAssign for Vector3D {
    #[inline]
    fn add_assign(&mut self, vector: Vector3D) {
        *self = *self + vector;
    }
}

impl SubAssign for Vector3D {
    #[inline]
    fn sub_assign(&mut self, vector: Vector3D) {
        *self = *self - vector;
    }
}

impl MulAssign<f64> for Vector3D {
    #[inline]
    fn mul_assign(&mut self, scalar: f64) {
        *self = *self * scalar;
    }
}

impl DivAssign<f64> for Vector3D {
    #[inline]
    fn div_assign(&mut self, scalar: f64) {
        *self = *self / scalar;
    }
}

/// WPF's explicit conversion to a point at the end of the vector.
impl From<Vector3D> for Point3D {
    #[inline]
    fn from(vector: Vector3D) -> Point3D {
        Point3D::new(vector.x, vector.y, vector.z)
    }
}

/// WPF's explicit conversion to a size: the magnitudes of the components.
impl From<Vector3D> for Size3D {
    #[inline]
    fn from(vector: Vector3D) -> Size3D {
        Size3D::from_magnitudes(vector.x, vector.y, vector.z)
    }
}

impl From<ferroui_base::Vector3D> for Vector3D {
    #[inline]
    fn from(vector: ferroui_base::Vector3D) -> Vector3D {
        Vector3D::new(vector.x, vector.y, vector.z)
    }
}

impl From<Vector3D> for ferroui_base::Vector3D {
    #[inline]
    fn from(vector: Vector3D) -> ferroui_base::Vector3D {
        ferroui_base::Vector3D::new(vector.x, vector.y, vector.z)
    }
}

impl FromStr for Vector3D {
    type Err = FormatError;
    #[inline]
    fn from_str(text: &str) -> Result<Self, FormatError> {
        Vector3D::parse(text)
    }
}

/// The text form `"x,y,z"` in the invariant culture.
impl fmt::Display for Vector3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_numbers(f, &[self.x, self.y, self.z])
    }
}

#[cfg(test)]
#[path = "vector_3d_tests.rs"]
mod tests;
