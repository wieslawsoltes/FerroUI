//! Interpolation of the value types composition animations can animate.

use crate::media::Color;
use crate::numerics::{Quaternion, Vector2, Vector3, Vector4};
use crate::{RelativePoint, RelativeScalar, Vector, Vector3D};

/// An interface to define interpolation logic for a particular type
pub trait IInterpolator<T> {
    fn interpolate(&self, from: T, to: T, progress: f32) -> T;
}

macro_rules! instance {
    ($ty:ident) => {
        impl $ty {
            /// The shared instance.
            pub fn instance() -> &'static $ty {
                static INSTANCE: $ty = $ty;
                &INSTANCE
            }
        }
    };
}

/// Interpolates single-precision scalars.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScalarInterpolator;

impl IInterpolator<f32> for ScalarInterpolator {
    fn interpolate(&self, from: f32, to: f32, progress: f32) -> f32 {
        from + (to - from) * progress
    }
}

instance!(ScalarInterpolator);

/// Interpolates double-precision scalars.
#[derive(Clone, Copy, Debug, Default)]
pub struct DoubleInterpolator;

impl IInterpolator<f64> for DoubleInterpolator {
    fn interpolate(&self, from: f64, to: f64, progress: f32) -> f64 {
        from + (to - from) * progress as f64
    }
}

instance!(DoubleInterpolator);

/// Interpolates [`Vector2`] values.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vector2Interpolator;

impl IInterpolator<Vector2> for Vector2Interpolator {
    fn interpolate(&self, from: Vector2, to: Vector2, progress: f32) -> Vector2 {
        Vector2::lerp(from, to, progress)
    }
}

instance!(Vector2Interpolator);

/// Interpolates [`Vector`] values.
#[derive(Clone, Copy, Debug, Default)]
pub struct VectorInterpolator;

impl IInterpolator<Vector> for VectorInterpolator {
    fn interpolate(&self, from: Vector, to: Vector, progress: f32) -> Vector {
        Vector::new(
            DoubleInterpolator::instance().interpolate(from.x, to.x, progress),
            DoubleInterpolator::instance().interpolate(from.y, to.y, progress),
        )
    }
}

instance!(VectorInterpolator);

/// Interpolates [`Vector3`] values.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vector3Interpolator;

impl IInterpolator<Vector3> for Vector3Interpolator {
    fn interpolate(&self, from: Vector3, to: Vector3, progress: f32) -> Vector3 {
        Vector3::lerp(from, to, progress)
    }
}

instance!(Vector3Interpolator);

/// Interpolates [`Vector3D`] values.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vector3DInterpolator;

impl IInterpolator<Vector3D> for Vector3DInterpolator {
    fn interpolate(&self, from: Vector3D, to: Vector3D, progress: f32) -> Vector3D {
        Vector3D::new(
            DoubleInterpolator::instance().interpolate(from.x, to.x, progress),
            DoubleInterpolator::instance().interpolate(from.y, to.y, progress),
            DoubleInterpolator::instance().interpolate(from.z, to.z, progress),
        )
    }
}

instance!(Vector3DInterpolator);

/// Interpolates [`Vector4`] values.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vector4Interpolator;

impl IInterpolator<Vector4> for Vector4Interpolator {
    fn interpolate(&self, from: Vector4, to: Vector4, progress: f32) -> Vector4 {
        Vector4::lerp(from, to, progress)
    }
}

instance!(Vector4Interpolator);

/// Interpolates [`Quaternion`] values (a normalized linear interpolation).
#[derive(Clone, Copy, Debug, Default)]
pub struct QuaternionInterpolator;

impl IInterpolator<Quaternion> for QuaternionInterpolator {
    fn interpolate(&self, from: Quaternion, to: Quaternion, progress: f32) -> Quaternion {
        Quaternion::lerp(from, to, progress)
    }
}

instance!(QuaternionInterpolator);

/// Interpolates [`Color`] values per ARGB channel.
#[derive(Clone, Copy, Debug, Default)]
pub struct ColorInterpolator;

impl ColorInterpolator {
    fn lerp(a: f32, b: f32, p: f32) -> u8 {
        // The conversion truncates; the value is already within the byte range
        // (a NaN becomes zero).
        ((a * (1.0 - p)) + (b * p)).clamp(0.0, 255.0) as u8
    }

    /// Interpolates each of the alpha, red, green and blue channels linearly.
    pub fn lerp_rgb(to: Color, from: Color, progress: f32) -> Color {
        Color::new(
            Self::lerp(to.a as f32, from.a as f32, progress),
            Self::lerp(to.r as f32, from.r as f32, progress),
            Self::lerp(to.g as f32, from.g as f32, progress),
            Self::lerp(to.b as f32, from.b as f32, progress),
        )
    }
}

impl IInterpolator<Color> for ColorInterpolator {
    fn interpolate(&self, from: Color, to: Color, progress: f32) -> Color {
        Self::lerp_rgb(from, to, progress)
    }
}

instance!(ColorInterpolator);

/// "Interpolates" booleans: the value switches when the animation completes.
#[derive(Clone, Copy, Debug, Default)]
pub struct BooleanInterpolator;

impl IInterpolator<bool> for BooleanInterpolator {
    fn interpolate(&self, from: bool, to: bool, progress: f32) -> bool {
        if progress >= 1.0 {
            to
        } else {
            from
        }
    }
}

instance!(BooleanInterpolator);

/// Interpolates [`RelativePoint`] values of the same unit; points of
/// different units switch halfway.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelativePointInterpolator;

impl IInterpolator<RelativePoint> for RelativePointInterpolator {
    fn interpolate(&self, from: RelativePoint, to: RelativePoint, progress: f32) -> RelativePoint {
        if from.unit != to.unit {
            return if progress >= 0.5 { to } else { from };
        }

        let x = DoubleInterpolator::instance().interpolate(from.point.x, to.point.x, progress);
        let y = DoubleInterpolator::instance().interpolate(from.point.y, to.point.y, progress);
        RelativePoint::new(x, y, from.unit)
    }
}

instance!(RelativePointInterpolator);

/// Interpolates [`RelativeScalar`] values of the same unit; scalars of
/// different units switch halfway.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelativeScalarInterpolator;

impl IInterpolator<RelativeScalar> for RelativeScalarInterpolator {
    fn interpolate(&self, from: RelativeScalar, to: RelativeScalar, progress: f32) -> RelativeScalar {
        if from.unit != to.unit {
            return if progress >= 0.5 { to } else { from };
        }
        RelativeScalar::new(from.scalar + (to.scalar - from.scalar) * progress as f64, from.unit)
    }
}

instance!(RelativeScalarInterpolator);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RelativeUnit;

    #[test]
    fn scalars() {
        assert_eq!(ScalarInterpolator::instance().interpolate(2.0, 4.0, 0.25), 2.5);
        assert_eq!(ScalarInterpolator::instance().interpolate(2.0, 4.0, 1.5), 5.0);
        assert_eq!(DoubleInterpolator::instance().interpolate(2.0, 4.0, 0.25), 2.5);
        assert_eq!(DoubleInterpolator::instance().interpolate(4.0, 2.0, 0.5), 3.0);
    }

    #[test]
    fn vectors() {
        assert_eq!(
            Vector2Interpolator::instance().interpolate(Vector2::new(0.0, 2.0), Vector2::new(4.0, 4.0), 0.5),
            Vector2::new(2.0, 3.0)
        );
        assert_eq!(
            VectorInterpolator::instance().interpolate(Vector::new(0.0, 2.0), Vector::new(4.0, 4.0), 0.5),
            Vector::new(2.0, 3.0)
        );
        assert_eq!(
            Vector3Interpolator::instance().interpolate(
                Vector3::new(0.0, 2.0, -2.0),
                Vector3::new(4.0, 4.0, 2.0),
                0.5
            ),
            Vector3::new(2.0, 3.0, 0.0)
        );
        assert_eq!(
            Vector3DInterpolator::instance().interpolate(
                Vector3D::new(0.0, 2.0, -2.0),
                Vector3D::new(4.0, 4.0, 2.0),
                0.5
            ),
            Vector3D::new(2.0, 3.0, 0.0)
        );
        assert_eq!(
            Vector4Interpolator::instance().interpolate(
                Vector4::new(0.0, 2.0, -2.0, 1.0),
                Vector4::new(4.0, 4.0, 2.0, 1.0),
                0.5
            ),
            Vector4::new(2.0, 3.0, 0.0, 1.0)
        );
    }

    #[test]
    fn quaternion_is_a_normalized_lerp() {
        let from = Quaternion::create_from_axis_angle(Vector3::UNIT_Z, 0.0);
        let to = Quaternion::create_from_axis_angle(Vector3::UNIT_Z, 1.0);
        let actual = QuaternionInterpolator::instance().interpolate(from, to, 0.5);
        assert_eq!(actual, Quaternion::lerp(from, to, 0.5));
        assert!((actual.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn color_channels_are_interpolated_and_truncated() {
        let from = Color::from_argb(255, 10, 0, 255);
        let to = Color::from_argb(0, 200, 1, 255);
        let i = ColorInterpolator::instance();
        // (10 + 200) / 2 = 105; 127.5 and 0.5 truncate.
        assert_eq!(i.interpolate(from, to, 0.5), Color::from_argb(127, 105, 0, 255));
        assert_eq!(i.interpolate(from, to, 0.0), from);
        assert_eq!(i.interpolate(from, to, 1.0), to);
        // Out of range progress is clamped per channel.
        assert_eq!(i.interpolate(from, to, 2.0), Color::from_argb(0, 255, 2, 255));
        assert_eq!(i.interpolate(from, to, -1.0), Color::from_argb(255, 0, 0, 255));
        assert_eq!(ColorInterpolator::lerp_rgb(from, to, 0.5), i.interpolate(from, to, 0.5));
    }

    #[test]
    fn boolean_switches_at_the_end() {
        let i = BooleanInterpolator::instance();
        assert!(!i.interpolate(false, true, 0.0));
        assert!(!i.interpolate(false, true, 0.999));
        assert!(i.interpolate(false, true, 1.0));
        assert!(i.interpolate(true, false, 0.5));
    }

    #[test]
    fn relative_values() {
        let p = RelativePointInterpolator::instance();
        let from = RelativePoint::new(0.0, 1.0, RelativeUnit::Relative);
        let to = RelativePoint::new(1.0, 0.0, RelativeUnit::Relative);
        assert_eq!(
            p.interpolate(from, to, 0.25),
            RelativePoint::new(0.25, 0.75, RelativeUnit::Relative)
        );
        let absolute = RelativePoint::new(10.0, 20.0, RelativeUnit::Absolute);
        assert_eq!(p.interpolate(from, absolute, 0.25), from);
        assert_eq!(p.interpolate(from, absolute, 0.5), absolute);

        let s = RelativeScalarInterpolator::instance();
        let from = RelativeScalar::new(0.0, RelativeUnit::Relative);
        let to = RelativeScalar::new(1.0, RelativeUnit::Relative);
        assert_eq!(
            s.interpolate(from, to, 0.25),
            RelativeScalar::new(0.25, RelativeUnit::Relative)
        );
        let absolute = RelativeScalar::new(10.0, RelativeUnit::Absolute);
        assert_eq!(s.interpolate(from, absolute, 0.25), from);
        assert_eq!(s.interpolate(from, absolute, 0.75), absolute);
    }
}
