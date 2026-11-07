//! `Quaternion`: a rotation in 3D space.

use std::fmt;
use std::ops::{Add, Mul, Sub};
use std::str::FromStr;

use ferroui_base::utilities::FormatError;

use crate::text::{is_keyword, is_one, parse_numbers, write_numbers};
use crate::{QuaternionError, Vector3D};

/// A rotation in 3D space, with double-precision components (WPF's
/// `System.Windows.Media.Media3D.Quaternion`): `(x, y, z)` is the axis scaled by the sine of half
/// the angle, `w` the cosine of half the angle.
///
/// This is the quaternion of the 3D scene model, apart from the single-precision
/// `ferroui_base::numerics::Quaternion` of the numerics library. The default value is
/// [`Quaternion::IDENTITY`], as in WPF.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quaternion {
    /// The X component.
    pub x: f64,
    /// The Y component.
    pub y: f64,
    /// The Z component.
    pub z: f64,
    /// The W component.
    pub w: f64,
}

impl Quaternion {
    /// The rotation by no angle (WPF's `Quaternion.Identity`): `(0, 0, 0, 1)`.
    pub const IDENTITY: Quaternion = Quaternion::new(0.0, 0.0, 0.0, 1.0);

    /// A quaternion with the given components.
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    /// The rotation about the axis by the angle in degrees, counterclockwise when the axis points
    /// at the viewer. The angle is taken modulo 360 before it is converted to radians, as WPF does;
    /// [`QuaternionError::ZeroAxis`] when the axis is the zero vector.
    pub fn from_axis_angle(axis_of_rotation: Vector3D, angle_in_degrees: f64) -> Result<Self, QuaternionError> {
        let angle_in_radians = (angle_in_degrees % 360.0).to_radians();
        let length = axis_of_rotation.length();
        if length == 0.0 {
            return Err(QuaternionError::ZeroAxis);
        }
        let axis = (axis_of_rotation / length) * (0.5 * angle_in_radians).sin();
        Ok(Quaternion::new(axis.x, axis.y, axis.z, (0.5 * angle_in_radians).cos()))
    }

    /// Whether this is the identity: `(0, 0, 0, 1)`.
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.x == 0.0 && self.y == 0.0 && self.z == 0.0 && self.w == 1.0
    }

    /// Whether the quaternion has unit length, within ten machine epsilons of its squared norm.
    #[inline]
    pub fn is_normalized(&self) -> bool {
        is_one(self.norm_squared())
    }

    /// The axis of the rotation, a unit vector; `(0, 1, 0)` when the axis is undetermined (the
    /// identity, and every quaternion whose `x`, `y` and `z` are zero).
    pub fn axis(&self) -> Vector3D {
        if self.x == 0.0 && self.y == 0.0 && self.z == 0.0 {
            return Vector3D::new(0.0, 1.0, 0.0);
        }
        let mut axis = Vector3D::new(self.x, self.y, self.z);
        axis.normalize();
        axis
    }

    /// The angle of the rotation in degrees, from 0 to 360, for a quaternion of any length.
    pub fn angle(&self) -> f64 {
        let mut sine = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        let mut cosine = self.w;
        if !sine.is_finite() {
            // The squares overflowed: scale by the largest component of the axis first.
            let largest = self.x.abs().max(self.y.abs()).max(self.z.abs());
            let (x, y, z) = (self.x / largest, self.y / largest, self.z / largest);
            sine = (x * x + y * y + z * z).sqrt();
            cosine = self.w / largest;
        }
        sine.atan2(cosine) * (360.0 / std::f64::consts::PI)
    }

    /// Replaces the quaternion by its conjugate: the axis reversed.
    #[inline]
    pub fn conjugate(&mut self) {
        self.x = -self.x;
        self.y = -self.y;
        self.z = -self.z;
    }

    /// Replaces the quaternion by its inverse: the conjugate divided by the squared norm. The zero
    /// quaternion becomes `NaN` components, as in WPF.
    pub fn invert(&mut self) {
        self.conjugate();
        let norm_squared = self.norm_squared();
        self.x /= norm_squared;
        self.y /= norm_squared;
        self.z /= norm_squared;
        self.w /= norm_squared;
    }

    /// Scales the quaternion to unit length. When the squared norm overflows, the components are
    /// first divided by the largest of their magnitudes.
    pub fn normalize(&mut self) {
        let mut norm_squared = self.norm_squared();
        if norm_squared > f64::MAX {
            let reciprocal = 1.0 / self.x.abs().max(self.y.abs()).max(self.z.abs()).max(self.w.abs());
            self.scale(reciprocal);
            norm_squared = self.norm_squared();
        }
        self.scale(1.0 / norm_squared.sqrt());
    }

    /// The spherical linear interpolation from `from` to `to` at `t` (0 is `from`, 1 is `to`),
    /// along the shorter arc.
    #[inline]
    pub fn slerp(from: Quaternion, to: Quaternion, t: f64) -> Quaternion {
        Quaternion::slerp_with_path(from, to, t, true)
    }

    /// The spherical linear interpolation from `from` to `to` at `t` (WPF's `Slerp` with
    /// `useShortestPath`): along the shorter arc when `use_shortest_path` is true (`to` is negated
    /// when the two are more than 90 degrees apart in four dimensions), else along the arc between
    /// the quaternions as given.
    ///
    /// The inputs need not have unit length: the direction is interpolated between the unit
    /// quaternions, and the length of the result is interpolated geometrically between the lengths
    /// of the inputs. Quaternions closer than a cosine of `1 - 1e-6` are interpolated linearly;
    /// opposite quaternions turn about an arbitrary perpendicular.
    pub fn slerp_with_path(from: Quaternion, to: Quaternion, t: f64, use_shortest_path: bool) -> Quaternion {
        // The limits of the main formula: a better fallback exists for large cosines, hence the
        // asymmetric bounds.
        const MAX_COSINE: f64 = 1.0 - 1e-6;
        const MIN_COSINE: f64 = 1e-10 - 1.0;

        let mut from = from;
        let mut to = to;
        let length_from = from.norm_squared().sqrt();
        let length_to = to.norm_squared().sqrt();
        from.scale(1.0 / length_from);
        to.scale(1.0 / length_to);

        let mut cos_omega = from.x * to.x + from.y * to.y + from.z * to.z + from.w * to.w;
        if use_shortest_path {
            if cos_omega < 0.0 {
                cos_omega = -cos_omega;
                to.scale(-1.0);
            }
        } else if cos_omega < -1.0 {
            cos_omega = -1.0;
        }
        if cos_omega > 1.0 {
            cos_omega = 1.0;
        }

        let (scale_from, scale_to);
        if cos_omega > MAX_COSINE {
            // Too close for the sine of the angle: linear interpolation.
            scale_from = 1.0 - t;
            scale_to = t;
        } else if cos_omega < MIN_COSINE {
            // Nearly opposite: `to` is taken as exactly opposite to `from`, through an arbitrary
            // quaternion perpendicular to `from`.
            to = Quaternion::new(-from.y, from.x, -from.w, from.z);
            let theta = t * std::f64::consts::PI;
            scale_from = theta.cos();
            scale_to = theta.sin();
        } else {
            let omega = cos_omega.acos();
            let sin_omega = (1.0 - cos_omega * cos_omega).sqrt();
            scale_from = ((1.0 - t) * omega).sin() / sin_omega;
            scale_to = (t * omega).sin() / sin_omega;
        }

        // The length of the result: length_from ^ (1 - t) * length_to ^ t.
        let length_out = length_from * (length_to / length_from).powf(t);
        let scale_from = scale_from * length_out;
        let scale_to = scale_to * length_out;
        Quaternion::new(
            scale_from * from.x + scale_to * to.x,
            scale_from * from.y + scale_to * to.y,
            scale_from * from.z + scale_to * to.z,
            scale_from * from.w + scale_to * to.w,
        )
    }

    /// Parses the text form: `"Identity"`, or `"x,y,z,w"` (commas, white space or both between the
    /// numbers) in the invariant culture.
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        if is_keyword(text, "Identity") {
            return Ok(Quaternion::IDENTITY);
        }
        let [x, y, z, w] = parse_numbers::<4>(text, "Invalid Quaternion.")?;
        Ok(Quaternion::new(x, y, z, w))
    }

    #[inline]
    fn norm_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    #[inline]
    fn scale(&mut self, scale: f64) {
        self.x *= scale;
        self.y *= scale;
        self.z *= scale;
        self.w *= scale;
    }
}

impl Default for Quaternion {
    /// [`Quaternion::IDENTITY`], as WPF's default quaternion.
    #[inline]
    fn default() -> Self {
        Quaternion::IDENTITY
    }
}

impl Add for Quaternion {
    type Output = Quaternion;
    #[inline]
    fn add(self, right: Quaternion) -> Quaternion {
        Quaternion::new(self.x + right.x, self.y + right.y, self.z + right.z, self.w + right.w)
    }
}

impl Sub for Quaternion {
    type Output = Quaternion;
    #[inline]
    fn sub(self, right: Quaternion) -> Quaternion {
        Quaternion::new(self.x - right.x, self.y - right.y, self.z - right.z, self.w - right.w)
    }
}

/// The Hamilton product `self * right`: as a rotation, `right` and then `self`.
impl Mul for Quaternion {
    type Output = Quaternion;
    fn mul(self, right: Quaternion) -> Quaternion {
        let left = self;
        Quaternion::new(
            left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y,
            left.w * right.y + left.y * right.w + left.z * right.x - left.x * right.z,
            left.w * right.z + left.z * right.w + left.x * right.y - left.y * right.x,
            left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z,
        )
    }
}

impl FromStr for Quaternion {
    type Err = FormatError;
    #[inline]
    fn from_str(text: &str) -> Result<Self, FormatError> {
        Quaternion::parse(text)
    }
}

/// The text form in the invariant culture: `"Identity"`, or `"x,y,z,w"`.
impl fmt::Display for Quaternion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_identity() {
            return f.write_str("Identity");
        }
        write_numbers(f, &[self.x, self.y, self.z, self.w])
    }
}

#[cfg(test)]
#[path = "quaternion_tests.rs"]
mod tests;
