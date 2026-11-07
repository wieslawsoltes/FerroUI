//! Markup metadata of the values and the collections: their text converters (`Parse`), their
//! constructors, their components and their distinguished values, so markup writes
//! `Position="0,0,5"`, `Transform="Identity"`, `Bounds="Empty"`, `Positions="0,0,0 1,0,0 0,1,0"` or
//! `TriangleIndices="0 1 2"` for a property of one of these types; and the registration of the
//! classes of the scene model and of the control `Viewport3D`.
//!
//! The types are in the namespace `FerroUI.Media.Media3D` (WPF's `System.Windows.Media.Media3D`),
//! `Viewport3D` in `FerroUI.Controls` (WPF's `System.Windows.Controls`). Both are mapped into the
//! XML namespace of the framework, as WPF maps its namespaces into its default XAML namespace, so
//! markup names `<Viewport3D>`, `<PerspectiveCamera>` or `<MeshGeometry3D>` without a prefix.
//!
//! One name is in two of the mapped namespaces: `Vector3D`, the 3D vector of this crate and the
//! composition vector of the base library (namespace `FerroUI`). A property declares its type,
//! so `LookDirection="0,0,-1"` or `Axis="0,1,0"` is read as the vector of this crate whatever
//! the mapping; only a type named by its name in markup (an element `<Vector3D>`, `x:Type`,
//! `x:DataType`) is looked up in the XML namespace, where the first mapped namespace that has
//! the name wins: the base library's, which is registered first, so markup that worked keeps
//! working. Such markup names the 3D vector with the prefix of its namespace
//! (`xmlns:media3d="using:FerroUI.Media.Media3D"`, `<media3d:Vector3D>`); the base library's
//! `Vector3D` stays unchanged (`docs/porting/media3d.md`, section 3.12).

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{ferro_markup_type, Ref, StaticType, TypeInfo};

use crate::{
    Int32Collection, Material, MaterialCollection, Matrix3D, Model3D, Model3DCollection, Point3D, Point3DCollection,
    Point4D, Quaternion, Rect3D, Size3D, Transform3D, Transform3DCollection, Vector3D, Vector3DCollection, Visual3D,
    Visual3DCollection,
};

/// The namespaces of the modules of this crate: the scene model in WPF's 3D namespace, the
/// control with the controls.
const NAMESPACES: &[(&str, &str)] =
    &[("ferroui_media3d", "FerroUI.Media.Media3D"), ("ferroui_media3d::controls", "FerroUI.Controls")];

/// What this crate states about itself for markup: its assembly name and the namespaces the XML
/// namespace of the framework maps to (see the documentation of the module for the one name they
/// share with the base library).
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Media3D",
    crate_name: "ferroui_media3d",
    xmlns_definitions: &[
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Media.Media3D" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls" },
    ],
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

// The collections. A collection of values is read from its text form; every collection takes
// its items from markup through `Add`.

ferro_markup_type!(class Point3DCollection {
    namespace: "FerroUI.Media.Media3D",
    handles: [Point3DCollection, Option<Point3DCollection>],
    parse: Point3DCollection::parse,
    constructors: [() => Point3DCollection::new],
    properties: [Count: i32 { get: |c: &Point3DCollection| c.len() as i32 }],
    methods: [fn Add(Point3D) => |c: &Point3DCollection, item: Point3D| c.add(item)],
});

ferro_markup_type!(class Vector3DCollection {
    namespace: "FerroUI.Media.Media3D",
    handles: [Vector3DCollection, Option<Vector3DCollection>],
    parse: Vector3DCollection::parse,
    constructors: [() => Vector3DCollection::new],
    properties: [Count: i32 { get: |c: &Vector3DCollection| c.len() as i32 }],
    methods: [fn Add(Vector3D) => |c: &Vector3DCollection, item: Vector3D| c.add(item)],
});

ferro_markup_type!(class Int32Collection {
    namespace: "FerroUI.Media.Media3D",
    handles: [Int32Collection, Option<Int32Collection>],
    parse: Int32Collection::parse,
    constructors: [() => Int32Collection::new],
    properties: [Count: i32 { get: |c: &Int32Collection| c.len() as i32 }],
    methods: [fn Add(i32) => |c: &Int32Collection, item: i32| c.add(item)],
});

ferro_markup_type!(class Model3DCollection {
    namespace: "FerroUI.Media.Media3D",
    handles: [Model3DCollection, Option<Model3DCollection>],
    constructors: [() => Model3DCollection::new],
    properties: [Count: i32 { get: |c: &Model3DCollection| c.len() as i32 }],
    methods: [fn Add(Ref<Model3D>) => |c: &Model3DCollection, item: Ref<Model3D>| c.add(item)],
});

ferro_markup_type!(class Transform3DCollection {
    namespace: "FerroUI.Media.Media3D",
    handles: [Transform3DCollection, Option<Transform3DCollection>],
    constructors: [() => Transform3DCollection::new],
    properties: [Count: i32 { get: |c: &Transform3DCollection| c.len() as i32 }],
    methods: [fn Add(Ref<Transform3D>) => |c: &Transform3DCollection, item: Ref<Transform3D>| c.add(item)],
});

ferro_markup_type!(class MaterialCollection {
    namespace: "FerroUI.Media.Media3D",
    handles: [MaterialCollection, Option<MaterialCollection>],
    constructors: [() => MaterialCollection::new],
    properties: [Count: i32 { get: |c: &MaterialCollection| c.len() as i32 }],
    methods: [fn Add(Ref<Material>) => |c: &MaterialCollection, item: Ref<Material>| c.add(item)],
});

ferro_markup_type!(class Visual3DCollection {
    namespace: "FerroUI.Media.Media3D",
    handles: [Visual3DCollection, Option<Visual3DCollection>],
    constructors: [() => Visual3DCollection::new],
    properties: [Count: i32 { get: |c: &Visual3DCollection| c.len() as i32 }],
    methods: [fn Add(Ref<Visual3D>) => |c: &Visual3DCollection, item: Ref<Visual3D>| c.add(item)],
});

/// The values declared in this file.
pub(crate) const TYPES: &[&MarkupType] = &[
    <Point3D as MarkupTyped>::MARKUP,
    <Vector3D as MarkupTyped>::MARKUP,
    <Size3D as MarkupTyped>::MARKUP,
    <Rect3D as MarkupTyped>::MARKUP,
    <Point4D as MarkupTyped>::MARKUP,
    <Quaternion as MarkupTyped>::MARKUP,
    <Matrix3D as MarkupTyped>::MARKUP,
];

/// The collections declared in this file.
pub(crate) const COLLECTIONS: &[&MarkupType] = &[
    <Point3DCollection as MarkupTyped>::MARKUP,
    <Vector3DCollection as MarkupTyped>::MARKUP,
    <Int32Collection as MarkupTyped>::MARKUP,
    <Model3DCollection as MarkupTyped>::MARKUP,
    <Transform3DCollection as MarkupTyped>::MARKUP,
    <MaterialCollection as MarkupTyped>::MARKUP,
    <Visual3DCollection as MarkupTyped>::MARKUP,
];

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

/// The classes of the crate.
const CLASSES: &[&TypeInfo] = types![
    // FerroUI.Media.Media3D
    crate::Transform3D,
    crate::TranslateTransform3D,
    crate::ScaleTransform3D,
    crate::RotateTransform3D,
    crate::MatrixTransform3D,
    crate::Transform3DGroup,
    crate::Rotation3D,
    crate::AxisAngleRotation3D,
    crate::QuaternionRotation3D,
    crate::Camera,
    crate::ProjectionCamera,
    crate::PerspectiveCamera,
    crate::OrthographicCamera,
    crate::MatrixCamera,
    crate::Model3D,
    crate::GeometryModel3D,
    crate::Model3DGroup,
    crate::Light,
    crate::AmbientLight,
    crate::DirectionalLight,
    crate::PointLightBase,
    crate::PointLight,
    crate::SpotLight,
    crate::Geometry3D,
    crate::MeshGeometry3D,
    crate::Material,
    crate::DiffuseMaterial,
    crate::SpecularMaterial,
    crate::EmissiveMaterial,
    crate::MaterialGroup,
    crate::Visual3D,
    crate::ModelVisual3D,
    crate::Viewport3DVisual,
    // FerroUI.Controls
    crate::Viewport3D,
];

/// Registers the namespaces, the classes, the markup metadata and the assembly of this crate
/// (and those of the control library and the base library) with the tables of known types, and
/// the nullable forms of the values and the collections with the untyped value conversions.
/// Cheap and idempotent; nothing is initialised until a type is used.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_table(CLASSES);
        MarkupType::register_table(TYPES);
        MarkupType::register_table(COLLECTIONS);
        MarkupAssembly::register_table(&ASSEMBLY);
        ValueTypes::register_global(register_value_types);
    });
}

/// The nullable forms of the values and the collections, for the untyped value conversions (per
/// thread).
fn register_value_types() {
    ValueTypes::register_nullable::<Point3D>();
    ValueTypes::register_nullable::<Vector3D>();
    ValueTypes::register_nullable::<Size3D>();
    ValueTypes::register_nullable::<Rect3D>();
    ValueTypes::register_nullable::<Point4D>();
    ValueTypes::register_nullable::<Quaternion>();
    ValueTypes::register_nullable::<Matrix3D>();
    ValueTypes::register_nullable::<Point3DCollection>();
    ValueTypes::register_nullable::<Vector3DCollection>();
    ValueTypes::register_nullable::<Int32Collection>();
    ValueTypes::register_nullable::<Model3DCollection>();
    ValueTypes::register_nullable::<Transform3DCollection>();
    ValueTypes::register_nullable::<MaterialCollection>();
    ValueTypes::register_nullable::<Visual3DCollection>();
    // The texture coordinates are the base library's `Points`, which it registers itself.
}

#[cfg(test)]
#[path = "markup_types_tests.rs"]
mod tests;
