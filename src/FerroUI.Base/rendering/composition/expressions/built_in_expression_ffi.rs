//! Built-in functions available from composition animation expressions.

use std::sync::OnceLock;

use super::delegate_expression_ffi::DelegateExpressionFfi;
use super::expression_evaluation_context::IExpressionForeignFunctionInterface;
use super::expression_variant::ExpressionVariant;
use crate::media::Color;
use crate::numerics::{Matrix3x2, Matrix4x4, Quaternion, Vector2, Vector3, Vector4};
use crate::rendering::composition::animations::ColorInterpolator;
use crate::utilities::math_utilities::{max_f32, min_f32};
use crate::utilities::MathUtilities;
use crate::{RelativePoint, RelativeScalar, RelativeUnit, Vector, Vector3D};

/// Built-in functions for Foreign Function Interface available from composition animation expressions
pub struct BuiltInExpressionFfi {
    registry: DelegateExpressionFfi,
}

fn lerp(a: f32, b: f32, p: f32) -> f32 {
    (a * (1.0 - p)) + (b * p)
}

fn lerp_f64(a: f64, b: f64, p: f64) -> f64 {
    (a * (1.0 - p)) + (b * p)
}

fn smooth_step(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = MathUtilities::clamp_f32((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn smooth_step_f64(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = MathUtilities::clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn smooth_step_vector2(edge0: Vector2, edge1: Vector2, x: Vector2) -> Vector2 {
    Vector2::new(
        smooth_step(edge0.x, edge1.x, x.x),
        smooth_step(edge0.y, edge1.y, x.y),
    )
}

fn smooth_step_vector(edge0: Vector, edge1: Vector, x: Vector) -> Vector {
    Vector::new(
        smooth_step_f64(edge0.x, edge1.x, x.x),
        smooth_step_f64(edge0.y, edge1.y, x.y),
    )
}

fn smooth_step_vector3(edge0: Vector3, edge1: Vector3, x: Vector3) -> Vector3 {
    Vector3::new(
        smooth_step(edge0.x, edge1.x, x.x),
        smooth_step(edge0.y, edge1.y, x.y),
        smooth_step(edge0.z, edge1.z, x.z),
    )
}

fn smooth_step_vector3d(edge0: Vector3D, edge1: Vector3D, x: Vector3D) -> Vector3D {
    Vector3D::new(
        smooth_step_f64(edge0.x, edge1.x, x.x),
        smooth_step_f64(edge0.y, edge1.y, x.y),
        smooth_step_f64(edge0.z, edge1.z, x.z),
    )
}

fn smooth_step_vector4(edge0: Vector4, edge1: Vector4, x: Vector4) -> Vector4 {
    Vector4::new(
        smooth_step(edge0.x, edge1.x, x.x),
        smooth_step(edge0.y, edge1.y, x.y),
        smooth_step(edge0.z, edge1.z, x.z),
        smooth_step(edge0.w, edge1.w, x.w),
    )
}

/// Applies a double-precision function to a single-precision value.
#[inline]
fn via_f64(value: f32, function: impl FnOnce(f64) -> f64) -> f32 {
    function(value as f64) as f32
}

impl BuiltInExpressionFfi {
    fn new() -> Self {
        let mut r = DelegateExpressionFfi::new();

        r.add1("Abs", |f: f32| f.abs());
        r.add1("Abs", |v: Vector2| Vector2::abs(v));
        r.add1("Abs", |v: Vector| v.abs());
        r.add1("Abs", |v: Vector3| Vector3::abs(v));
        r.add1("Abs", |v: Vector3D| v.abs());
        r.add1("Abs", |v: Vector4| Vector4::abs(v));

        r.add1("ACos", |f: f32| via_f64(f, f64::acos));
        r.add1("ASin", |f: f32| via_f64(f, f64::asin));
        r.add1("ATan", |f: f32| via_f64(f, f64::atan));
        r.add1("Ceil", |f: f32| via_f64(f, f64::ceil));

        r.add3("Clamp", |a1: f32, a2: f32, a3: f32| MathUtilities::clamp_f32(a1, a2, a3));
        r.add3("Clamp", |a1: Vector2, a2: Vector2, a3: Vector2| Vector2::clamp(a1, a2, a3));
        r.add3("Clamp", |a1: Vector, a2: Vector, a3: Vector| Vector::clamp(a1, a2, a3));
        r.add3("Clamp", |a1: Vector3, a2: Vector3, a3: Vector3| Vector3::clamp(a1, a2, a3));
        r.add3("Clamp", |a1: Vector3D, a2: Vector3D, a3: Vector3D| {
            Vector3D::clamp(a1, a2, a3)
        });
        r.add3("Clamp", |a1: Vector4, a2: Vector4, a3: Vector4| Vector4::clamp(a1, a2, a3));

        r.add2("Concatenate", |a1: Quaternion, a2: Quaternion| {
            Quaternion::concatenate(a1, a2)
        });
        r.add1("Cos", |a: f32| via_f64(a, f64::cos));

        /*
        TODO:
            ColorHsl(Float h, Float s, Float l)
            ColorLerpHSL(Color colorTo, CompositionColorcolorFrom, Float progress)
        */

        r.add3("ColorLerp", |to: Color, from: Color, progress: f32| {
            ColorInterpolator::lerp_rgb(to, from, progress)
        });
        r.add3("ColorLerpRGB", |to: Color, from: Color, progress: f32| {
            ColorInterpolator::lerp_rgb(to, from, progress)
        });
        r.add4("ColorRGB", |a: f32, r: f32, g: f32, b: f32| {
            Color::from_argb(
                MathUtilities::clamp_f32(a, 0.0, 255.0) as u8,
                MathUtilities::clamp_f32(r, 0.0, 255.0) as u8,
                MathUtilities::clamp_f32(g, 0.0, 255.0) as u8,
                MathUtilities::clamp_f32(b, 0.0, 255.0) as u8,
            )
        });

        r.add2("Distance", |a1: Vector2, a2: Vector2| Vector2::distance(a1, a2));
        r.add2("Distance", |a1: Vector, a2: Vector| Vector::distance(a1, a2));
        r.add2("Distance", |a1: Vector3, a2: Vector3| Vector3::distance(a1, a2));
        r.add2("Distance", |a1: Vector3D, a2: Vector3D| Vector3D::distance(a1, a2));
        r.add2("Distance", |a1: Vector4, a2: Vector4| Vector4::distance(a1, a2));

        r.add2("DistanceSquared", |a1: Vector2, a2: Vector2| {
            Vector2::distance_squared(a1, a2)
        });
        r.add2("DistanceSquared", |a1: Vector, a2: Vector| {
            Vector::distance_squared(a1, a2)
        });
        r.add2("DistanceSquared", |a1: Vector3, a2: Vector3| {
            Vector3::distance_squared(a1, a2)
        });
        r.add2("DistanceSquared", |a1: Vector3D, a2: Vector3D| {
            Vector3D::distance_squared(a1, a2)
        });
        r.add2("DistanceSquared", |a1: Vector4, a2: Vector4| {
            Vector4::distance_squared(a1, a2)
        });

        r.add1("Floor", |v: f32| via_f64(v, f64::floor));

        r.add1("Inverse", |v: Matrix3x2| Matrix3x2::invert_or_nan(v));
        r.add1("Inverse", |v: Matrix4x4| Matrix4x4::invert_or_nan(v));

        r.add1("Length", |a1: Vector2| a1.length());
        r.add1("Length", |a1: Vector| a1.length());
        r.add1("Length", |a1: Vector3| a1.length());
        r.add1("Length", |a1: Vector3D| a1.length());
        r.add1("Length", |a1: Vector4| a1.length());
        r.add1("Length", |a1: Quaternion| a1.length());

        r.add1("LengthSquared", |a1: Vector2| a1.length_squared());
        r.add1("LengthSquared", |a1: Vector| a1 * a1);
        r.add1("LengthSquared", |a1: Vector3| a1.length_squared());
        r.add1("LengthSquared", |a1: Vector3D| Vector3D::dot(a1, a1));
        r.add1("LengthSquared", |a1: Vector4| a1.length_squared());
        r.add1("LengthSquared", |a1: Quaternion| a1.length_squared());

        r.add3("Lerp", |a1: f32, a2: f32, a3: f32| lerp(a1, a2, a3));
        r.add3("Lerp", |a1: Vector2, a2: Vector2, a3: f32| Vector2::lerp(a1, a2, a3));
        r.add3("Lerp", |a1: Vector, a2: Vector, a3: f32| {
            Vector::new(
                lerp_f64(a1.x, a2.x, a3 as f64),
                lerp_f64(a1.y, a2.y, a3 as f64),
            )
        });
        r.add3("Lerp", |a1: Vector3, a2: Vector3, a3: f32| Vector3::lerp(a1, a2, a3));
        r.add3("Lerp", |a1: Vector3D, a2: Vector3D, a3: f32| {
            Vector3D::new(
                lerp_f64(a1.x, a2.x, a3 as f64),
                lerp_f64(a1.y, a2.y, a3 as f64),
                lerp_f64(a1.z, a2.z, a3 as f64),
            )
        });
        r.add3("Lerp", |a1: Vector4, a2: Vector4, a3: f32| Vector4::lerp(a1, a2, a3));

        r.add1("Ln", |f: f32| via_f64(f, f64::ln));
        r.add1("Log10", |f: f32| via_f64(f, f64::log10));

        r.add1("Matrix3x2.CreateFromScale", |v: Vector2| Matrix3x2::create_scale(v));
        r.add1("Matrix3x2.CreateFromTranslation", |v: Vector2| {
            Matrix3x2::create_translation(v)
        });
        r.add1("Matrix3x2.CreateRotation", |v: f32| Matrix3x2::create_rotation(v));
        r.add1("Matrix3x2.CreateScale", |v: Vector2| Matrix3x2::create_scale(v));
        r.add3("Matrix3x2.CreateSkew", |a1: f32, a2: f32, a3: Vector2| {
            Matrix3x2::create_skew_at(a1, a2, a3)
        });
        // As in the reference implementation, this creates a scale matrix.
        r.add1("Matrix3x2.CreateTranslation", |v: Vector2| Matrix3x2::create_scale(v));
        r.add6(
            "Matrix3x2",
            |m11: f32, m12: f32, m21: f32, m22: f32, m31: f32, m32: f32| Matrix3x2::new(m11, m12, m21, m22, m31, m32),
        );
        r.add2("Matrix4x4.CreateFromAxisAngle", |v: Vector3, angle: f32| {
            Matrix4x4::create_from_axis_angle(v, angle)
        });
        r.add1("Matrix4x4.CreateFromScale", |v: Vector3| Matrix4x4::create_scale(v));
        r.add1("Matrix4x4.CreateFromTranslation", |v: Vector3| {
            Matrix4x4::create_translation(v)
        });
        r.add1("Matrix4x4.CreateScale", |v: Vector3| Matrix4x4::create_scale(v));
        // As in the reference implementation, this creates a scale matrix.
        r.add1("Matrix4x4.CreateTranslation", |v: Vector3| Matrix4x4::create_scale(v));
        r.add1("Matrix4x4", |m: Matrix3x2| Matrix4x4::from_matrix3x2(m));
        #[allow(clippy::too_many_arguments)]
        r.add16(
            "Matrix4x4",
            |m11: f32,
             m12: f32,
             m13: f32,
             m14: f32,
             m21: f32,
             m22: f32,
             m23: f32,
             m24: f32,
             m31: f32,
             m32: f32,
             m33: f32,
             m34: f32,
             m41: f32,
             m42: f32,
             m43: f32,
             m44: f32| {
                Matrix4x4::new(
                    m11, m12, m13, m14, m21, m22, m23, m24, m31, m32, m33, m34, m41, m42, m43, m44,
                )
            },
        );

        r.add2("Max", |a1: f32, a2: f32| max_f32(a1, a2));
        r.add2("Max", |a1: Vector2, a2: Vector2| Vector2::max(a1, a2));
        r.add2("Max", |a1: Vector, a2: Vector| Vector::max(a1, a2));
        r.add2("Max", |a1: Vector3, a2: Vector3| Vector3::max(a1, a2));
        r.add2("Max", |a1: Vector3D, a2: Vector3D| Vector3D::max(a1, a2));
        r.add2("Max", |a1: Vector4, a2: Vector4| Vector4::max(a1, a2));

        r.add2("Min", |a1: f32, a2: f32| min_f32(a1, a2));
        r.add2("Min", |a1: Vector2, a2: Vector2| Vector2::min(a1, a2));
        r.add2("Min", |a1: Vector, a2: Vector| Vector::min(a1, a2));
        r.add2("Min", |a1: Vector3, a2: Vector3| Vector3::min(a1, a2));
        r.add2("Min", |a1: Vector3D, a2: Vector3D| Vector3D::min(a1, a2));
        r.add2("Min", |a1: Vector4, a2: Vector4| Vector4::min(a1, a2));

        r.add2("Mod", |a: f32, b: f32| a % b);

        r.add1("Normalize", |a: Quaternion| Quaternion::normalize(a));
        r.add1("Normalize", |a: Vector2| Vector2::normalize(a));
        r.add1("Normalize", |a: Vector| Vector::normalize_vector(a));
        r.add1("Normalize", |a: Vector3| Vector3::normalize(a));
        r.add1("Normalize", |a: Vector3D| Vector3D::normalize(a));
        r.add1("Normalize", |a: Vector4| Vector4::normalize(a));

        r.add2("Pow", |a: f32, b: f32| (a as f64).powf(b as f64) as f32);
        r.add2("Quaternion.CreateFromAxisAngle", |a: Vector3, b: f32| {
            Quaternion::create_from_axis_angle(a, b)
        });
        r.add2("Quaternion.CreateFromAxisAngle", |a: Vector3D, b: f32| {
            Quaternion::create_from_axis_angle(a.into(), b)
        });
        r.add4("Quaternion", |a: f32, b: f32, c: f32, d: f32| Quaternion::new(a, b, c, d));

        r.add1("Round", |a: f32| via_f64(a, f64::round_ties_even));

        r.add2("Scale", |a: Matrix3x2, b: f32| a * b);
        r.add2("Scale", |a: Matrix4x4, b: f32| a * b);
        r.add2("Scale", |a: Vector2, b: f32| a * b);
        r.add2("Scale", |a: Vector, b: f32| a * (b as f64));
        r.add2("Scale", |a: Vector3, b: f32| a * b);
        r.add2("Scale", |a: Vector3D, b: f32| Vector3D::multiply_scalar(a, b as f64));
        r.add2("Scale", |a: Vector4, b: f32| a * b);

        r.add1("Sin", |a: f32| via_f64(a, f64::sin));

        r.add3("SmoothStep", |a1: f32, a2: f32, a3: f32| smooth_step(a1, a2, a3));
        r.add3("SmoothStep", |a1: Vector2, a2: Vector2, a3: Vector2| {
            smooth_step_vector2(a1, a2, a3)
        });
        r.add3("SmoothStep", |a1: Vector, a2: Vector, a3: Vector| {
            smooth_step_vector(a1, a2, a3)
        });
        r.add3("SmoothStep", |a1: Vector3, a2: Vector3, a3: Vector3| {
            smooth_step_vector3(a1, a2, a3)
        });
        r.add3("SmoothStep", |a1: Vector3D, a2: Vector3D, a3: Vector3D| {
            smooth_step_vector3d(a1, a2, a3)
        });
        r.add3("SmoothStep", |a1: Vector4, a2: Vector4, a3: Vector4| {
            smooth_step_vector4(a1, a2, a3)
        });

        // I have no idea how to do a spherical interpolation for a scalar value, so we are doing a linear one
        r.add3("Slerp", |a1: f32, a2: f32, a3: f32| lerp(a1, a2, a3));
        r.add3("Slerp", |a1: Quaternion, a2: Quaternion, a3: f32| {
            Quaternion::slerp(a1, a2, a3)
        });

        r.add1("Sqrt", |a: f32| via_f64(a, f64::sqrt));
        r.add1("Square", |a: f32| a * a);
        r.add1("Tan", |a: f32| via_f64(a, f64::tan));

        r.add1("ToRadians", |a: f32| (a as f64 * std::f64::consts::PI / 180.0) as f32);
        r.add1("ToDegrees", |a: f32| (a as f64 * 180.0 / std::f64::consts::PI) as f32);

        r.add2("Transform", |a: Vector2, b: Matrix3x2| Vector2::transform(a, b));
        r.add2("Transform", |a: Vector3, b: Matrix4x4| Vector3::transform(a, b));

        r.add0("RelativeUnit.Absolute", || RelativeUnit::Absolute);
        r.add0("RelativeUnit.Relative", || RelativeUnit::Relative);
        r.add3("RelativePoint", |x: f32, y: f32, unit: RelativeUnit| {
            RelativePoint::new(x as f64, y as f64, unit)
        });
        r.add3("RelativePoint", |x: f64, y: f64, unit: RelativeUnit| {
            RelativePoint::new(x, y, unit)
        });
        r.add2("RelativeScalar", |value: f32, unit: RelativeUnit| {
            RelativeScalar::new(value as f64, unit)
        });
        r.add2("RelativeScalar", |value: f64, unit: RelativeUnit| {
            RelativeScalar::new(value, unit)
        });

        r.add2("Vector2", |a: f32, b: f32| Vector::new(a as f64, b as f64));
        r.add2("Vector2", |a: f64, b: f64| Vector::new(a, b));
        r.add3("Vector3", |a: f32, b: f32, c: f32| {
            Vector3D::new(a as f64, b as f64, c as f64)
        });
        r.add3("Vector3", |a: f64, b: f64, c: f64| Vector3D::new(a, b, c));
        r.add2("Vector3", |v2: Vector2, z: f32| Vector3::from_vector2(v2, z));
        r.add2("Vector3", |v2: Vector, z: f32| Vector3D::new(v2.x, v2.y, z as f64));
        r.add2("Vector3", |v2: Vector, z: f64| Vector3D::new(v2.x, v2.y, z));
        r.add4("Vector4", |a: f32, b: f32, c: f32, d: f32| Vector4::new(a, b, c, d));
        r.add3("Vector4", |v2: Vector2, z: f32, w: f32| Vector4::from_vector2(v2, z, w));
        r.add2("Vector4", |v3: Vector3, w: f32| Vector4::from_vector3(v3, w));

        Self { registry: r }
    }

    /// The shared instance.
    pub fn instance() -> &'static BuiltInExpressionFfi {
        static INSTANCE: OnceLock<BuiltInExpressionFfi> = OnceLock::new();
        INSTANCE.get_or_init(BuiltInExpressionFfi::new)
    }
}

impl IExpressionForeignFunctionInterface for BuiltInExpressionFfi {
    fn call(&self, name: &str, arguments: &[ExpressionVariant]) -> Option<ExpressionVariant> {
        self.registry.call(name, arguments)
    }
}
