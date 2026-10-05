//! The variant value type of expression animations.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Not, Rem, Sub};

use crate::media::Color;
use crate::numerics::{Matrix3x2, Matrix4x4, Quaternion, Vector2, Vector3, Vector4};
use crate::utilities::span_helpers::InvariantF64;
use crate::{Matrix, RelativePoint, RelativeScalar, RelativeUnit, Vector, Vector3D};

/// The type of the value held by an [`ExpressionVariant`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum VariantType {
    #[default]
    Invalid,
    Boolean,
    Double,
    Vector2,
    Vector3,
    Vector4,
    Vector,
    Vector3D,
    FerroMatrix,
    Matrix3x2,
    Matrix4x4,
    Quaternion,
    Color,
    RelativePoint,
    RelativeScalar,
    RelativeUnit,
}

/// A VARIANT type used in expression animations. Can represent multiple value types.
///
/// Every operation that is not defined for its operand types gives
/// [`ExpressionVariant::Invalid`] rather than failing.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ExpressionVariant {
    #[default]
    Invalid,
    Boolean(bool),
    Double(f64),
    Vector2(Vector2),
    Vector3(Vector3),
    Vector4(Vector4),
    Vector(Vector),
    Vector3D(Vector3D),
    FerroMatrix(Matrix),
    Matrix3x2(Matrix3x2),
    Matrix4x4(Matrix4x4),
    Quaternion(Quaternion),
    Color(Color),
    RelativePoint(RelativePoint),
    RelativeScalar(RelativeScalar),
    RelativeUnit(RelativeUnit),
}

use ExpressionVariant as V;

impl ExpressionVariant {
    /// The type of the held value.
    pub fn variant_type(&self) -> VariantType {
        match self {
            V::Invalid => VariantType::Invalid,
            V::Boolean(_) => VariantType::Boolean,
            V::Double(_) => VariantType::Double,
            V::Vector2(_) => VariantType::Vector2,
            V::Vector3(_) => VariantType::Vector3,
            V::Vector4(_) => VariantType::Vector4,
            V::Vector(_) => VariantType::Vector,
            V::Vector3D(_) => VariantType::Vector3D,
            V::FerroMatrix(_) => VariantType::FerroMatrix,
            V::Matrix3x2(_) => VariantType::Matrix3x2,
            V::Matrix4x4(_) => VariantType::Matrix4x4,
            V::Quaternion(_) => VariantType::Quaternion,
            V::Color(_) => VariantType::Color,
            V::RelativePoint(_) => VariantType::RelativePoint,
            V::RelativeScalar(_) => VariantType::RelativeScalar,
            V::RelativeUnit(_) => VariantType::RelativeUnit,
        }
    }

    /// Gets a member (a component or a swizzle) of the held value. Member
    /// names are case-sensitive; an unknown member gives `Invalid`.
    pub fn get_property(&self, property: &str) -> ExpressionVariant {
        match *self {
            V::Vector2(v) => match property {
                "X" => f(v.x),
                "Y" => f(v.y),
                _ => V::Invalid,
            },

            V::Vector(v) => match property {
                "X" => v.x.into(),
                "Y" => v.y.into(),
                _ => V::Invalid,
            },

            V::Vector3(v) => match property {
                "X" => f(v.x),
                "Y" => f(v.y),
                "Z" => f(v.z),
                "XY" => Vector2::new(v.x, v.y).into(),
                "YX" => Vector2::new(v.y, v.x).into(),
                "XZ" => Vector2::new(v.x, v.z).into(),
                "ZX" => Vector2::new(v.z, v.x).into(),
                "YZ" => Vector2::new(v.y, v.z).into(),
                "ZY" => Vector2::new(v.z, v.y).into(),
                _ => V::Invalid,
            },

            V::Vector3D(v) => match property {
                "X" => v.x.into(),
                "Y" => v.y.into(),
                "Z" => v.z.into(),
                "XY" => Vector::new(v.x, v.y).into(),
                "YX" => Vector::new(v.y, v.x).into(),
                "XZ" => Vector::new(v.x, v.z).into(),
                "ZX" => Vector::new(v.z, v.x).into(),
                "YZ" => Vector::new(v.y, v.z).into(),
                "ZY" => Vector::new(v.z, v.y).into(),
                _ => V::Invalid,
            },

            V::Vector4(v) => match property {
                "X" => f(v.x),
                "Y" => f(v.y),
                "Z" => f(v.z),
                "W" => f(v.w),
                _ => V::Invalid,
            },

            V::Matrix3x2(m) => match property {
                "M11" => f(m.m11),
                "M12" => f(m.m12),
                "M21" => f(m.m21),
                "M22" => f(m.m22),
                "M31" => f(m.m31),
                "M32" => f(m.m32),
                _ => V::Invalid,
            },

            V::FerroMatrix(m) => match property {
                "M11" => m.m11.into(),
                "M12" => m.m12.into(),
                "M13" => m.m13.into(),
                "M21" => m.m21.into(),
                "M22" => m.m22.into(),
                "M23" => m.m23.into(),
                "M31" => m.m31.into(),
                "M32" => m.m32.into(),
                "M33" => m.m33.into(),
                _ => V::Invalid,
            },

            V::Matrix4x4(m) => match property {
                "M11" => f(m.m11),
                "M12" => f(m.m12),
                "M13" => f(m.m13),
                "M14" => f(m.m14),
                "M21" => f(m.m21),
                "M22" => f(m.m22),
                "M23" => f(m.m23),
                "M24" => f(m.m24),
                "M31" => f(m.m31),
                "M32" => f(m.m32),
                "M33" => f(m.m33),
                "M34" => f(m.m34),
                "M41" => f(m.m41),
                "M42" => f(m.m42),
                "M43" => f(m.m43),
                "M44" => f(m.m44),
                _ => V::Invalid,
            },

            V::Quaternion(q) => match property {
                "X" => f(q.x),
                "Y" => f(q.y),
                "Z" => f(q.z),
                "W" => f(q.w),
                _ => V::Invalid,
            },

            V::Color(c) => match property {
                "A" => (c.a as f64).into(),
                "R" => (c.r as f64).into(),
                "G" => (c.g as f64).into(),
                "B" => (c.b as f64).into(),
                _ => V::Invalid,
            },

            // The point coordinates are narrowed to single precision here, as
            // in the reference implementation.
            V::RelativePoint(p) => match property {
                "X" => f(p.point.x as f32),
                "Y" => f(p.point.y as f32),
                "Unit" => p.unit.into(),
                _ => V::Invalid,
            },

            V::RelativeScalar(s) => match property {
                "Scalar" => s.scalar.into(),
                "Unit" => s.unit.into(),
                _ => V::Invalid,
            },

            _ => V::Invalid,
        }
    }

    /// The `==` operator of expressions: a `Boolean` for two values of the
    /// same comparable type, `Invalid` otherwise.
    pub fn equals_to(&self, right: ExpressionVariant) -> ExpressionVariant {
        match (*self, right) {
            (V::Double(l), V::Double(r)) => (l == r).into(),
            (V::Vector2(l), V::Vector2(r)) => (l == r).into(),
            (V::Vector(l), V::Vector(r)) => (l == r).into(),
            (V::Vector3(l), V::Vector3(r)) => (l == r).into(),
            (V::Vector3D(l), V::Vector3D(r)) => (l == r).into(),
            (V::Vector4(l), V::Vector4(r)) => (l == r).into(),
            (V::Boolean(l), V::Boolean(r)) => (l == r).into(),
            (V::Matrix3x2(l), V::Matrix3x2(r)) => (l == r).into(),
            (V::FerroMatrix(l), V::FerroMatrix(r)) => (l == r).into(),
            (V::Matrix4x4(l), V::Matrix4x4(r)) => (l == r).into(),
            (V::Quaternion(l), V::Quaternion(r)) => (l == r).into(),
            (V::RelativePoint(l), V::RelativePoint(r)) => (l == r).into(),
            (V::RelativeScalar(l), V::RelativeScalar(r)) => (l == r).into(),
            (V::RelativeUnit(l), V::RelativeUnit(r)) => (l == r).into(),
            _ => V::Invalid,
        }
    }

    /// The `!=` operator of expressions.
    pub fn not_equals_to(&self, right: ExpressionVariant) -> ExpressionVariant {
        match self.equals_to(right) {
            V::Boolean(r) => (!r).into(),
            _ => V::Invalid,
        }
    }

    /// The `<` operator of expressions.
    pub fn less_than(&self, right: ExpressionVariant) -> ExpressionVariant {
        match (*self, right) {
            (V::Double(l), V::Double(r)) => (l < r).into(),
            (V::RelativeScalar(l), V::RelativeScalar(r)) if l.unit == r.unit => (l.scalar < r.scalar).into(),
            _ => V::Invalid,
        }
    }

    /// The `>` operator of expressions.
    pub fn more_than(&self, right: ExpressionVariant) -> ExpressionVariant {
        match (*self, right) {
            (V::Double(l), V::Double(r)) => (l > r).into(),
            (V::RelativeScalar(l), V::RelativeScalar(r)) if l.unit == r.unit => (l.scalar > r.scalar).into(),
            _ => V::Invalid,
        }
    }

    /// The `&&` operator of expressions.
    pub fn and(&self, right: ExpressionVariant) -> ExpressionVariant {
        match (*self, right) {
            (V::Boolean(l), V::Boolean(r)) => (l && r).into(),
            _ => V::Invalid,
        }
    }

    /// The `||` operator of expressions.
    pub fn or(&self, right: ExpressionVariant) -> ExpressionVariant {
        match (*self, right) {
            (V::Boolean(l), V::Boolean(r)) => (l || r).into(),
            _ => V::Invalid,
        }
    }

    /// Converts the held value to `T`, or `None` when the variant does not
    /// hold a value convertible to `T`.
    #[inline]
    pub fn try_cast<T: ExpressionVariantValue>(&self) -> Option<T> {
        T::try_from_variant(self)
    }

    /// Creates a variant from a value.
    #[inline]
    pub fn create<T: ExpressionVariantValue>(v: T) -> ExpressionVariant {
        v.into_variant()
    }

    /// Converts the held value to `T`, or gives the default value of `T`.
    #[inline]
    pub fn cast_or_default<T: ExpressionVariantValue>(&self) -> T {
        self.try_cast::<T>().unwrap_or_default()
    }
}

/// A single-precision member widened to the `Double` variant.
#[inline]
fn f(value: f32) -> ExpressionVariant {
    V::Double(value as f64)
}

/// A value type an [`ExpressionVariant`] can be created from and cast to.
pub trait ExpressionVariantValue: Copy + Default + 'static {
    /// The variant type a foreign function parameter of this type is matched
    /// against, or `None` when the type cannot be a foreign function parameter.
    const FFI_VARIANT_TYPE: Option<VariantType>;

    /// See [`ExpressionVariant::try_cast`].
    fn try_from_variant(variant: &ExpressionVariant) -> Option<Self>;

    /// See [`ExpressionVariant::create`].
    fn into_variant(self) -> ExpressionVariant;
}

macro_rules! variant_value {
    ($ty:ty, $variant:ident, $ffi:expr, |$v:ident| $cast:expr) => {
        impl From<$ty> for ExpressionVariant {
            #[inline]
            fn from(value: $ty) -> ExpressionVariant {
                ExpressionVariant::$variant(value)
            }
        }

        impl ExpressionVariantValue for $ty {
            const FFI_VARIANT_TYPE: Option<VariantType> = $ffi;

            #[inline]
            fn try_from_variant($v: &ExpressionVariant) -> Option<Self> {
                $cast
            }

            #[inline]
            fn into_variant(self) -> ExpressionVariant {
                ExpressionVariant::$variant(self)
            }
        }
    };
    ($ty:ty, $variant:ident, $ffi:expr) => {
        variant_value!($ty, $variant, $ffi, |v| match *v {
            ExpressionVariant::$variant(value) => Some(value),
            _ => None,
        });
    };
}

variant_value!(bool, Boolean, Some(VariantType::Boolean));
variant_value!(f64, Double, Some(VariantType::Double));
variant_value!(Vector2, Vector2, Some(VariantType::Vector2), |v| match *v {
    V::Vector2(value) => Some(value),
    V::Vector(value) => Some(value.into()),
    _ => None,
});
variant_value!(Vector, Vector, Some(VariantType::Vector), |v| match *v {
    V::Vector(value) => Some(value),
    V::Vector2(value) => Some(value.into()),
    _ => None,
});
variant_value!(Vector3, Vector3, Some(VariantType::Vector3), |v| match *v {
    V::Vector3(value) => Some(value),
    V::Vector3D(value) => Some(value.into()),
    _ => None,
});
variant_value!(Vector3D, Vector3D, Some(VariantType::Vector3D), |v| match *v {
    V::Vector3D(value) => Some(value),
    V::Vector3(value) => Some(value.into()),
    _ => None,
});
variant_value!(Vector4, Vector4, Some(VariantType::Vector4));
variant_value!(Matrix3x2, Matrix3x2, Some(VariantType::Matrix3x2));
// The 3x3 matrix is not in the foreign function type map of the reference
// implementation.
variant_value!(Matrix, FerroMatrix, None);
variant_value!(Matrix4x4, Matrix4x4, Some(VariantType::Matrix4x4));
variant_value!(Quaternion, Quaternion, Some(VariantType::Quaternion));
variant_value!(Color, Color, Some(VariantType::Color));
variant_value!(RelativePoint, RelativePoint, Some(VariantType::RelativePoint));
variant_value!(RelativeScalar, RelativeScalar, Some(VariantType::RelativeScalar));
variant_value!(RelativeUnit, RelativeUnit, Some(VariantType::RelativeUnit));

/// A single-precision scalar is stored as a `Double`.
impl From<f32> for ExpressionVariant {
    #[inline]
    fn from(value: f32) -> ExpressionVariant {
        V::Double(value as f64)
    }
}

impl ExpressionVariantValue for f32 {
    const FFI_VARIANT_TYPE: Option<VariantType> = Some(VariantType::Double);

    #[inline]
    fn try_from_variant(variant: &ExpressionVariant) -> Option<Self> {
        match *variant {
            V::Double(value) => Some(value as f32),
            _ => None,
        }
    }

    #[inline]
    fn into_variant(self) -> ExpressionVariant {
        self.into()
    }
}

impl Add for ExpressionVariant {
    type Output = ExpressionVariant;

    fn add(self, right: ExpressionVariant) -> ExpressionVariant {
        match (self, right) {
            (V::Double(l), V::Double(r)) => (l + r).into(),
            (V::Vector2(l), V::Vector2(r)) => (l + r).into(),
            (V::Vector(l), V::Vector(r)) => (l + r).into(),
            (V::Vector3(l), V::Vector3(r)) => (l + r).into(),
            (V::Vector3D(l), V::Vector3D(r)) => Vector3D::add(l, r).into(),
            (V::Vector4(l), V::Vector4(r)) => (l + r).into(),
            (V::Matrix3x2(l), V::Matrix3x2(r)) => (l + r).into(),
            (V::Matrix4x4(l), V::Matrix4x4(r)) => (l + r).into(),
            (V::Quaternion(l), V::Quaternion(r)) => (l + r).into(),
            (V::RelativePoint(l), V::RelativePoint(r)) if l.unit == r.unit => {
                RelativePoint::new(l.point.x + r.point.x, l.point.y + r.point.y, l.unit).into()
            }
            (V::RelativeScalar(l), V::RelativeScalar(r)) if l.unit == r.unit => {
                RelativeScalar::new(l.scalar + r.scalar, l.unit).into()
            }
            _ => V::Invalid,
        }
    }
}

impl Sub for ExpressionVariant {
    type Output = ExpressionVariant;

    fn sub(self, right: ExpressionVariant) -> ExpressionVariant {
        match (self, right) {
            (V::Double(l), V::Double(r)) => (l - r).into(),
            (V::Vector2(l), V::Vector2(r)) => (l - r).into(),
            (V::Vector(l), V::Vector(r)) => (l - r).into(),
            (V::Vector3(l), V::Vector3(r)) => (l - r).into(),
            (V::Vector3D(l), V::Vector3D(r)) => Vector3D::add(l, -r).into(),
            (V::Vector4(l), V::Vector4(r)) => (l - r).into(),
            (V::Matrix3x2(l), V::Matrix3x2(r)) => (l - r).into(),
            (V::Matrix4x4(l), V::Matrix4x4(r)) => (l - r).into(),
            (V::Quaternion(l), V::Quaternion(r)) => (l - r).into(),
            (V::RelativePoint(l), V::RelativePoint(r)) if l.unit == r.unit => {
                RelativePoint::new(l.point.x - r.point.x, l.point.y - r.point.y, l.unit).into()
            }
            (V::RelativeScalar(l), V::RelativeScalar(r)) if l.unit == r.unit => {
                RelativeScalar::new(l.scalar - r.scalar, l.unit).into()
            }
            _ => V::Invalid,
        }
    }
}

impl Neg for ExpressionVariant {
    type Output = ExpressionVariant;

    fn neg(self) -> ExpressionVariant {
        match self {
            V::Double(l) => (-l).into(),
            V::Vector2(l) => (-l).into(),
            V::Vector(l) => (-l).into(),
            V::Vector3(l) => (-l).into(),
            V::Vector3D(l) => (-l).into(),
            V::Vector4(l) => (-l).into(),
            V::Matrix3x2(l) => (-l).into(),
            V::FerroMatrix(l) => (-l).into(),
            V::Matrix4x4(l) => (-l).into(),
            V::Quaternion(l) => (-l).into(),
            V::RelativePoint(l) => RelativePoint::new(-l.point.x, -l.point.y, l.unit).into(),
            V::RelativeScalar(l) => RelativeScalar::new(-l.scalar, l.unit).into(),
            _ => V::Invalid,
        }
    }
}

impl Mul for ExpressionVariant {
    type Output = ExpressionVariant;

    fn mul(self, right: ExpressionVariant) -> ExpressionVariant {
        match (self, right) {
            (V::Double(l), V::Double(r)) => (l * r).into(),
            (V::Vector2(l), V::Vector2(r)) => (l * r).into(),
            (V::Vector(l), V::Vector(r)) => Vector::multiply(l, r).into(),
            (V::Vector2(l), V::Double(r)) => (l * (r as f32)).into(),
            (V::Vector(l), V::Double(r)) => (l * r).into(),
            (V::Vector3(l), V::Vector3(r)) => (l * r).into(),
            (V::Vector3D(l), V::Vector3D(r)) => Vector3D::multiply(l, r).into(),
            (V::Vector3(l), V::Double(r)) => (l * (r as f32)).into(),
            (V::Vector3D(l), V::Double(r)) => Vector3D::multiply_scalar(l, r).into(),
            (V::Vector4(l), V::Vector4(r)) => (l * r).into(),
            (V::Vector4(l), V::Double(r)) => (l * (r as f32)).into(),
            (V::Matrix3x2(l), V::Matrix3x2(r)) => (l * r).into(),
            (V::Matrix3x2(l), V::Double(r)) => (l * (r as f32)).into(),
            (V::FerroMatrix(l), V::FerroMatrix(r)) => (l * r).into(),
            (V::Matrix4x4(l), V::Matrix4x4(r)) => (l * r).into(),
            (V::Matrix4x4(l), V::Double(r)) => (l * (r as f32)).into(),
            (V::Quaternion(l), V::Quaternion(r)) => (l * r).into(),
            (V::Quaternion(l), V::Double(r)) => (l * (r as f32)).into(),
            (V::RelativePoint(l), V::Double(r)) => RelativePoint::new(l.point.x * r, l.point.y * r, l.unit).into(),
            (V::Double(l), V::RelativePoint(r)) => RelativePoint::new(l * r.point.x, l * r.point.y, r.unit).into(),
            (V::RelativeScalar(l), V::Double(r)) => RelativeScalar::new(l.scalar * r, l.unit).into(),
            (V::Double(l), V::RelativeScalar(r)) => RelativeScalar::new(l * r.scalar, r.unit).into(),
            (V::RelativePoint(l), V::RelativePoint(r)) if l.unit == r.unit => {
                RelativePoint::new(l.point.x * r.point.x, l.point.y * r.point.y, l.unit).into()
            }
            (V::RelativeScalar(l), V::RelativeScalar(r)) if l.unit == r.unit => {
                RelativeScalar::new(l.scalar * r.scalar, l.unit).into()
            }
            _ => V::Invalid,
        }
    }
}

impl Div for ExpressionVariant {
    type Output = ExpressionVariant;

    fn div(self, right: ExpressionVariant) -> ExpressionVariant {
        match (self, right) {
            (V::Double(l), V::Double(r)) => (l / r).into(),
            (V::Vector2(l), V::Vector2(r)) => (l / r).into(),
            (V::Vector(l), V::Vector(r)) => Vector::divide(l, r).into(),
            (V::Vector2(l), V::Double(r)) => (l / (r as f32)).into(),
            (V::Vector(l), V::Double(r)) => (l / r).into(),
            (V::Vector3(l), V::Vector3(r)) => (l / r).into(),
            (V::Vector3D(l), V::Vector3D(r)) => Vector3D::divide(l, r).into(),
            (V::Vector3(l), V::Double(r)) => (l / (r as f32)).into(),
            (V::Vector3D(l), V::Double(r)) => Vector3D::divide_scalar(l, r).into(),
            (V::Vector4(l), V::Vector4(r)) => (l / r).into(),
            (V::Vector4(l), V::Double(r)) => (l / (r as f32)).into(),
            (V::Quaternion(l), V::Quaternion(r)) => (l / r).into(),
            (V::RelativePoint(l), V::Double(r)) => RelativePoint::new(l.point.x / r, l.point.y / r, l.unit).into(),
            (V::RelativeScalar(l), V::Double(r)) => RelativeScalar::new(l.scalar / r, l.unit).into(),
            (V::RelativePoint(l), V::RelativePoint(r)) if l.unit == r.unit => {
                RelativePoint::new(l.point.x / r.point.x, l.point.y / r.point.y, l.unit).into()
            }
            (V::RelativeScalar(l), V::RelativeScalar(r)) if l.unit == r.unit => {
                RelativeScalar::new(l.scalar / r.scalar, l.unit).into()
            }
            _ => V::Invalid,
        }
    }
}

impl Not for ExpressionVariant {
    type Output = ExpressionVariant;

    fn not(self) -> ExpressionVariant {
        match self {
            V::Boolean(v) => (!v).into(),
            _ => V::Invalid,
        }
    }
}

impl Rem for ExpressionVariant {
    type Output = ExpressionVariant;

    fn rem(self, right: ExpressionVariant) -> ExpressionVariant {
        match (self, right) {
            (V::Double(l), V::Double(r)) => (l % r).into(),
            (V::RelativeScalar(l), V::RelativeScalar(r)) if l.unit == r.unit => {
                RelativeScalar::new(l.scalar % r.scalar, l.unit).into()
            }
            _ => V::Invalid,
        }
    }
}

impl fmt::Display for ExpressionVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            V::Boolean(v) => f.write_str(if *v { "True" } else { "False" }),
            V::Double(v) => write!(f, "{}", InvariantF64(*v)),
            V::Vector2(v) => v.fmt(f),
            V::Vector(v) => v.fmt(f),
            V::Vector3(v) => v.fmt(f),
            V::Vector3D(v) => v.fmt(f),
            V::Vector4(v) => v.fmt(f),
            V::Quaternion(v) => v.fmt(f),
            V::Matrix3x2(v) => v.fmt(f),
            V::FerroMatrix(v) => v.fmt(f),
            V::Matrix4x4(v) => v.fmt(f),
            V::Color(v) => v.fmt(f),
            V::RelativePoint(v) => v.fmt(f),
            V::RelativeScalar(v) => v.fmt(f),
            V::RelativeUnit(v) => fmt::Debug::fmt(v, f),
            V::Invalid => f.write_str("Invalid"),
        }
    }
}
