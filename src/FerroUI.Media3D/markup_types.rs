//! Markup metadata of the values: their text converters (`Parse`), their constructors, their
//! components and their distinguished values, so markup writes `Position="0,0,5"`,
//! `Transform="Identity"` or `Bounds="Empty"` for a property of one of these types.
//!
//! The types are in the namespace `FerroUI.Media.Media3D`. Mapping that namespace into the
//! framework's XML namespace (WPF's types are in its default XAML namespace) comes with the scene
//! objects of step D2 (`docs/porting/media3d.md`, section 3.12): the base library's composition
//! `Vector3D` already has that name there.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped};
use ferroui_base::{ferro_markup_type, TypeInfo};

use crate::{Matrix3D, Point3D, Point4D, Quaternion, Rect3D, Size3D, Vector3D};

/// The namespace of the types of this crate.
const NAMESPACES: &[(&str, &str)] = &[("ferroui_media3d", "FerroUI.Media.Media3D")];

/// What this crate states about itself for markup: its assembly name. No XML namespace maps to
/// its namespace yet (see the documentation of the module).
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Media3D",
    crate_name: "ferroui_media3d",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

ferro_markup_type!(struct Point3D {
    namespace: "FerroUI.Media.Media3D",
    handles: [Point3D],
    parse: Point3D::parse,
    constructors: [() => Point3D::default, (f64, f64, f64) => Point3D::new],
    properties: [
        X: f64 { get: |p: &Point3D| p.x },
        Y: f64 { get: |p: &Point3D| p.y },
        Z: f64 { get: |p: &Point3D| p.z },
    ],
});

ferro_markup_type!(struct Vector3D {
    namespace: "FerroUI.Media.Media3D",
    handles: [Vector3D],
    parse: Vector3D::parse,
    constructors: [() => Vector3D::default, (f64, f64, f64) => Vector3D::new],
    properties: [
        X: f64 { get: |v: &Vector3D| v.x },
        Y: f64 { get: |v: &Vector3D| v.y },
        Z: f64 { get: |v: &Vector3D| v.z },
        Length: f64 { get: |v: &Vector3D| v.length() },
        LengthSquared: f64 { get: |v: &Vector3D| v.length_squared() },
    ],
});

ferro_markup_type!(struct Size3D {
    namespace: "FerroUI.Media.Media3D",
    handles: [Size3D],
    parse: Size3D::parse,
    constructors: [() => Size3D::default, try (f64, f64, f64) => Size3D::new],
    properties: [
        X: f64 { get: |s: &Size3D| s.x() },
        Y: f64 { get: |s: &Size3D| s.y() },
        Z: f64 { get: |s: &Size3D| s.z() },
        IsEmpty: bool { get: |s: &Size3D| s.is_empty() },
    ],
    static_properties: [
        Empty: Size3D { get: || Size3D::EMPTY },
    ],
});

ferro_markup_type!(struct Rect3D {
    namespace: "FerroUI.Media.Media3D",
    handles: [Rect3D],
    parse: Rect3D::parse,
    constructors: [
        () => Rect3D::default,
        (Point3D, Size3D) => Rect3D::from_location_size,
        try (f64, f64, f64, f64, f64, f64) => Rect3D::new,
    ],
    properties: [
        X: f64 { get: |r: &Rect3D| r.x() },
        Y: f64 { get: |r: &Rect3D| r.y() },
        Z: f64 { get: |r: &Rect3D| r.z() },
        SizeX: f64 { get: |r: &Rect3D| r.size_x() },
        SizeY: f64 { get: |r: &Rect3D| r.size_y() },
        SizeZ: f64 { get: |r: &Rect3D| r.size_z() },
        Location: Point3D { get: |r: &Rect3D| r.location() },
        Size: Size3D { get: |r: &Rect3D| r.size() },
        IsEmpty: bool { get: |r: &Rect3D| r.is_empty() },
    ],
    static_properties: [
        Empty: Rect3D { get: || Rect3D::EMPTY },
    ],
});

ferro_markup_type!(struct Point4D {
    namespace: "FerroUI.Media.Media3D",
    handles: [Point4D],
    parse: Point4D::parse,
    constructors: [() => Point4D::default, (f64, f64, f64, f64) => Point4D::new],
    properties: [
        X: f64 { get: |p: &Point4D| p.x },
        Y: f64 { get: |p: &Point4D| p.y },
        Z: f64 { get: |p: &Point4D| p.z },
        W: f64 { get: |p: &Point4D| p.w },
    ],
});

ferro_markup_type!(struct Quaternion {
    namespace: "FerroUI.Media.Media3D",
    handles: [Quaternion],
    parse: Quaternion::parse,
    constructors: [
        () => Quaternion::default,
        (f64, f64, f64, f64) => Quaternion::new,
        try (Vector3D, f64) => Quaternion::from_axis_angle,
    ],
    properties: [
        X: f64 { get: |q: &Quaternion| q.x },
        Y: f64 { get: |q: &Quaternion| q.y },
        Z: f64 { get: |q: &Quaternion| q.z },
        W: f64 { get: |q: &Quaternion| q.w },
        IsIdentity: bool { get: |q: &Quaternion| q.is_identity() },
        IsNormalized: bool { get: |q: &Quaternion| q.is_normalized() },
        Angle: f64 { get: |q: &Quaternion| q.angle() },
        Axis: Vector3D { get: |q: &Quaternion| q.axis() },
    ],
    static_properties: [
        Identity: Quaternion { get: || Quaternion::IDENTITY },
    ],
});

// The constructor of the 16 elements is left out: markup writes a matrix as its text form.
ferro_markup_type!(struct Matrix3D {
    namespace: "FerroUI.Media.Media3D",
    handles: [Matrix3D],
    parse: Matrix3D::parse,
    constructors: [() => Matrix3D::default],
    properties: [
        M11: f64 { get: |m: &Matrix3D| m.m11 },
        M12: f64 { get: |m: &Matrix3D| m.m12 },
        M13: f64 { get: |m: &Matrix3D| m.m13 },
        M14: f64 { get: |m: &Matrix3D| m.m14 },
        M21: f64 { get: |m: &Matrix3D| m.m21 },
        M22: f64 { get: |m: &Matrix3D| m.m22 },
        M23: f64 { get: |m: &Matrix3D| m.m23 },
        M24: f64 { get: |m: &Matrix3D| m.m24 },
        M31: f64 { get: |m: &Matrix3D| m.m31 },
        M32: f64 { get: |m: &Matrix3D| m.m32 },
        M33: f64 { get: |m: &Matrix3D| m.m33 },
        M34: f64 { get: |m: &Matrix3D| m.m34 },
        OffsetX: f64 { get: |m: &Matrix3D| m.offset_x },
        OffsetY: f64 { get: |m: &Matrix3D| m.offset_y },
        OffsetZ: f64 { get: |m: &Matrix3D| m.offset_z },
        M44: f64 { get: |m: &Matrix3D| m.m44 },
        IsIdentity: bool { get: |m: &Matrix3D| m.is_identity() },
        IsAffine: bool { get: |m: &Matrix3D| m.is_affine() },
        HasInverse: bool { get: |m: &Matrix3D| m.has_inverse() },
        Determinant: f64 { get: |m: &Matrix3D| m.determinant() },
    ],
    static_properties: [
        Identity: Matrix3D { get: || Matrix3D::IDENTITY },
    ],
});

/// The types declared in this file.
pub(crate) const TYPES: &[&MarkupType] = &[
    <Point3D as MarkupTyped>::MARKUP,
    <Vector3D as MarkupTyped>::MARKUP,
    <Size3D as MarkupTyped>::MARKUP,
    <Rect3D as MarkupTyped>::MARKUP,
    <Point4D as MarkupTyped>::MARKUP,
    <Quaternion as MarkupTyped>::MARKUP,
    <Matrix3D as MarkupTyped>::MARKUP,
];

/// Registers the namespace, the markup metadata and the assembly of this crate (and those of the
/// base library) with the tables of known types, and the nullable forms of the values with the
/// untyped value conversions. Cheap and idempotent; nothing is initialised until a type is used.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_base::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        MarkupType::register_table(TYPES);
        MarkupAssembly::register_table(&ASSEMBLY);
        ValueTypes::register_global(register_value_types);
    });
}

/// The nullable forms of the values, for the untyped value conversions (per thread).
fn register_value_types() {
    ValueTypes::register_nullable::<Point3D>();
    ValueTypes::register_nullable::<Vector3D>();
    ValueTypes::register_nullable::<Size3D>();
    ValueTypes::register_nullable::<Rect3D>();
    ValueTypes::register_nullable::<Point4D>();
    ValueTypes::register_nullable::<Quaternion>();
    ValueTypes::register_nullable::<Matrix3D>();
}

#[cfg(test)]
#[path = "markup_types_tests.rs"]
mod tests;
