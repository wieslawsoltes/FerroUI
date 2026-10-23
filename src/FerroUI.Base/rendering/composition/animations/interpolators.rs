//! Interpolation of the value types composition animations can animate.

use crate::media::{Color, HslColor};
use crate::numerics::{Quaternion, Vector2, Vector3, Vector4};
use crate::{RelativePoint, RelativeScalar, Vector, Vector3D};
use std::any::Any;

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

/// The colour space a colour animation interpolates in (WinUI
/// `CompositionColorSpace`, the type of
/// `ColorKeyFrameAnimation.InterpolationColorSpace`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CompositionColorSpace {
    /// The compositor chooses: here the gamma-encoded RGB space, as `Rgb`.
    // Deviation (winui-composition.md): WinUI does not document what `Auto`
    // picks; the port interpolates in RGB, the behaviour before the property
    // existed.
    #[default]
    Auto,
    /// Hue, saturation and lightness.
    Hsl,
    /// The red, green and blue channels as stored (gamma-encoded sRGB).
    Rgb,
    /// Hue, saturation and lightness of the linear-light colour.
    HslLinear,
    /// The red, green and blue channels in linear light.
    RgbLinear,
}

/// Linear interpolation of `f64` values.
fn lerp_f64(a: f64, b: f64, p: f64) -> f64 {
    (a * (1.0 - p)) + (b * p)
}

/// The sRGB transfer function: a gamma-encoded channel (0..1) in linear light.
fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The inverse of [`srgb_to_linear`].
fn linear_to_srgb(c: f64) -> f64 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// A channel in 0..1 as a byte, rounded.
fn unit_to_byte(c: f64) -> u8 {
    (c.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Hue (degrees, 0..360), saturation and lightness of an RGB colour whose
/// channels are in 0..1.
fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let chroma = max - min;
    if chroma <= 0.0 {
        return (0.0, 0.0, l);
    }
    let s = chroma / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * (((g - b) / chroma).rem_euclid(6.0))
    } else if max == g {
        60.0 * (((b - r) / chroma) + 2.0)
    } else {
        60.0 * (((r - g) / chroma) + 4.0)
    };
    (h, s.clamp(0.0, 1.0), l)
}

/// The RGB channels (0..1) of a hue (degrees, wrapped), saturation and
/// lightness (clamped to 0..1).
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let h = h.rem_euclid(360.0);
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);
    let chroma = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h1 = h / 60.0;
    let x = chroma * (1.0 - ((h1 % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if h1 < 1.0 {
        (chroma, x, 0.0)
    } else if h1 < 2.0 {
        (x, chroma, 0.0)
    } else if h1 < 3.0 {
        (0.0, chroma, x)
    } else if h1 < 4.0 {
        (0.0, x, chroma)
    } else if h1 < 5.0 {
        (x, 0.0, chroma)
    } else {
        (chroma, 0.0, x)
    };
    let m = l - chroma / 2.0;
    (r1 + m, g1 + m, b1 + m)
}

/// The hue to start from and the change of hue to interpolate: the change
/// takes the shorter way round, and the hue of an achromatic colour (no
/// saturation) is that of the other colour.
fn hue_path(from_hue: f64, from_saturation: f64, to_hue: f64, to_saturation: f64) -> (f64, f64) {
    let (from_hue, to_hue) = match (from_saturation == 0.0, to_saturation == 0.0) {
        (true, false) => (to_hue, to_hue),
        (false, true) => (from_hue, from_hue),
        _ => (from_hue, to_hue),
    };
    let mut delta = to_hue - from_hue;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    (from_hue, delta)
}

impl ColorInterpolator {
    /// Interpolates in the HSL colour space, giving `from` at progress 0 and
    /// `to` at progress 1 (the expression function `ColorLerpHSL` and the
    /// `Hsl` colour space of a colour animation). Saturation, lightness and
    /// alpha are interpolated linearly; the hue takes the shorter way around
    /// the colour wheel, and the hue of an achromatic colour (black, white,
    /// greys) is that of the other colour, so a grey turns into a colour
    /// without passing through other hues.
    //
    // Deviation (winui-composition.md): WinUI does not say which way the hue
    // turns or what an achromatic colour's hue is; this is the choice of the
    // port.
    pub fn lerp_hsl(from: Color, to: Color, progress: f32) -> Color {
        let a = from.to_hsl();
        let b = to.to_hsl();
        let p = progress as f64;
        let (from_hue, delta) = hue_path(a.h, a.s, b.h, b.s);
        HslColor::hsl_to_rgb(
            from_hue + delta * p,
            lerp_f64(a.s, b.s, p),
            lerp_f64(a.l, b.l, p),
            lerp_f64(a.a, b.a, p),
        )
    }

    /// Interpolates the red, green and blue channels in linear light (the
    /// `RgbLinear` colour space): the channels are decoded with the sRGB
    /// transfer function, interpolated, and encoded again. Alpha is
    /// interpolated linearly.
    pub fn lerp_rgb_linear(from: Color, to: Color, progress: f32) -> Color {
        let p = progress as f64;
        let channel = |a: u8, b: u8| {
            let a = srgb_to_linear(a as f64 / 255.0);
            let b = srgb_to_linear(b as f64 / 255.0);
            unit_to_byte(linear_to_srgb(lerp_f64(a, b, p)))
        };
        let alpha = unit_to_byte(lerp_f64(from.a as f64 / 255.0, to.a as f64 / 255.0, p));
        Color::new(alpha, channel(from.r, to.r), channel(from.g, to.g), channel(from.b, to.b))
    }

    /// Interpolates in the HSL colour space of the linear-light colour (the
    /// `HslLinear` colour space): the channels are decoded with the sRGB
    /// transfer function, the colour is interpolated in HSL as
    /// [`lerp_hsl`](Self::lerp_hsl) does, and the result is encoded again.
    pub fn lerp_hsl_linear(from: Color, to: Color, progress: f32) -> Color {
        let p = progress as f64;
        let linear = |c: Color| {
            rgb_to_hsl(
                srgb_to_linear(c.r as f64 / 255.0),
                srgb_to_linear(c.g as f64 / 255.0),
                srgb_to_linear(c.b as f64 / 255.0),
            )
        };
        let (ah, as_, al) = linear(from);
        let (bh, bs, bl) = linear(to);
        let (from_hue, delta) = hue_path(ah, as_, bh, bs);
        let (r, g, b) = hsl_to_rgb(from_hue + delta * p, lerp_f64(as_, bs, p), lerp_f64(al, bl, p));
        let alpha = unit_to_byte(lerp_f64(from.a as f64 / 255.0, to.a as f64 / 255.0, p));
        Color::new(
            alpha,
            unit_to_byte(linear_to_srgb(r)),
            unit_to_byte(linear_to_srgb(g)),
            unit_to_byte(linear_to_srgb(b)),
        )
    }
}

/// Interpolates [`Color`] values in the HSL colour space
/// ([`ColorInterpolator::lerp_hsl`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct ColorHslInterpolator;

impl IInterpolator<Color> for ColorHslInterpolator {
    fn interpolate(&self, from: Color, to: Color, progress: f32) -> Color {
        ColorInterpolator::lerp_hsl(from, to, progress)
    }
}

instance!(ColorHslInterpolator);

/// Interpolates [`Color`] values in linear-light RGB
/// ([`ColorInterpolator::lerp_rgb_linear`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct ColorRgbLinearInterpolator;

impl IInterpolator<Color> for ColorRgbLinearInterpolator {
    fn interpolate(&self, from: Color, to: Color, progress: f32) -> Color {
        ColorInterpolator::lerp_rgb_linear(from, to, progress)
    }
}

instance!(ColorRgbLinearInterpolator);

/// Interpolates [`Color`] values in the HSL colour space of linear light
/// ([`ColorInterpolator::lerp_hsl_linear`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct ColorHslLinearInterpolator;

impl IInterpolator<Color> for ColorHslLinearInterpolator {
    fn interpolate(&self, from: Color, to: Color, progress: f32) -> Color {
        ColorInterpolator::lerp_hsl_linear(from, to, progress)
    }
}

instance!(ColorHslLinearInterpolator);

/// The interpolator of a key frame animation of `T` values for a colour
/// space: for colours, the interpolator of the space; for any other type, and
/// for the RGB spaces of colours, `default`.
pub(crate) fn interpolator_for_color_space<T: 'static>(
    default: &'static (dyn IInterpolator<T> + Sync),
    color_space: CompositionColorSpace,
) -> &'static (dyn IInterpolator<T> + Sync) {
    static HSL: &(dyn IInterpolator<Color> + Sync) = &ColorHslInterpolator;
    static RGB_LINEAR: &(dyn IInterpolator<Color> + Sync) = &ColorRgbLinearInterpolator;
    static HSL_LINEAR: &(dyn IInterpolator<Color> + Sync) = &ColorHslLinearInterpolator;
    let chosen: &'static dyn Any = match color_space {
        CompositionColorSpace::Auto | CompositionColorSpace::Rgb => return default,
        CompositionColorSpace::Hsl => &HSL,
        CompositionColorSpace::RgbLinear => &RGB_LINEAR,
        CompositionColorSpace::HslLinear => &HSL_LINEAR,
    };
    // The colour interpolators are of `Color` values: they are only chosen
    // when `T` is `Color`.
    chosen.downcast_ref::<&'static (dyn IInterpolator<T> + Sync)>().copied().unwrap_or(default)
}

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

    /// Not from upstream: the colour spaces of `InterpolationColorSpace`.
    #[test]
    fn hsl_interpolation_turns_the_hue_the_short_way() {
        let red = Color::from_argb(255, 255, 0, 0);
        let blue = Color::from_argb(255, 0, 0, 255);
        // Red (0 degrees) to blue (240 degrees) goes through magenta (300).
        assert_eq!(ColorInterpolator::lerp_hsl(red, blue, 0.5), Color::from_argb(255, 255, 0, 255));
        assert_eq!(ColorHslInterpolator::instance().interpolate(red, blue, 0.0), red);
        assert_eq!(ColorHslInterpolator::instance().interpolate(red, blue, 1.0), blue);
        // The RGB interpolation goes through a dark purple.
        assert_eq!(ColorInterpolator::instance().interpolate(red, blue, 0.5), Color::from_argb(255, 127, 0, 127));
        // A grey takes the hue of the other colour.
        let grey = Color::from_argb(255, 128, 128, 128);
        let mid = ColorInterpolator::lerp_hsl(grey, red, 0.5);
        assert!(mid.r > mid.g && mid.g == mid.b, "{mid:?}");
    }

    #[test]
    fn linear_rgb_interpolation_is_brighter_in_the_middle() {
        let black = Color::from_argb(255, 0, 0, 0);
        let white = Color::from_argb(255, 255, 255, 255);
        // Half the light of white is 188 in sRGB.
        let mid = ColorRgbLinearInterpolator::instance().interpolate(black, white, 0.5);
        assert_eq!(mid, Color::from_argb(255, 188, 188, 188));
        assert_eq!(ColorInterpolator::lerp_rgb_linear(black, white, 0.0), black);
        assert_eq!(ColorInterpolator::lerp_rgb_linear(black, white, 1.0), white);
        let transparent = Color::from_argb(0, 255, 0, 0);
        assert_eq!(ColorInterpolator::lerp_rgb_linear(transparent, white, 0.5).a, 128);
    }

    #[test]
    fn linear_hsl_interpolation_keeps_the_end_points() {
        let red = Color::from_argb(255, 255, 0, 0);
        let blue = Color::from_argb(255, 0, 0, 255);
        let i = ColorHslLinearInterpolator::instance();
        assert_eq!(i.interpolate(red, blue, 0.0), red);
        assert_eq!(i.interpolate(red, blue, 1.0), blue);
        assert_eq!(i.interpolate(red, blue, 0.5), Color::from_argb(255, 255, 0, 255));
        let dark = Color::from_argb(255, 40, 80, 120);
        assert_eq!(i.interpolate(dark, dark, 0.3), dark);
    }

    #[test]
    fn color_space_selects_the_interpolator_of_colours_only() {
        let red = Color::from_argb(255, 255, 0, 0);
        let blue = Color::from_argb(255, 0, 0, 255);
        let hsl = interpolator_for_color_space::<Color>(ColorInterpolator::instance(), CompositionColorSpace::Hsl);
        assert_eq!(hsl.interpolate(red, blue, 0.5), Color::from_argb(255, 255, 0, 255));
        let auto = interpolator_for_color_space::<Color>(ColorInterpolator::instance(), CompositionColorSpace::Auto);
        assert_eq!(auto.interpolate(red, blue, 0.5), Color::from_argb(255, 127, 0, 127));
        let scalar = interpolator_for_color_space::<f32>(ScalarInterpolator::instance(), CompositionColorSpace::Hsl);
        assert_eq!(scalar.interpolate(2.0, 4.0, 0.5), 3.0);
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
