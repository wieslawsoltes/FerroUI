//! WPF-style 3D for FerroUI (#608, `docs/porting/media3d.md`).
//!
//! The values of `System.Windows.Media.Media3D`, with WPF's names and behaviour and Rust types:
//! [`Point3D`], [`Vector3D`], [`Size3D`], [`Rect3D`], [`Point4D`], [`Matrix3D`] and
//! [`Quaternion`], with double-precision components (narrowed to single precision only when a
//! scene is handed to the GPU). Their operators are the traits of `std::ops`, the members WPF guards
//! with exceptions return a `Result`, and each value parses and writes WPF's text form in the
//! invariant culture (`"0,0,5"`, `"Identity"`, `"Empty"`), which is also the form markup writes
//! them in.
//!
//! The scene model of `Viewport3D`, with WPF's classes and properties:
//!
//! - the transforms [`Transform3D`], [`TranslateTransform3D`], [`ScaleTransform3D`],
//!   [`RotateTransform3D`], [`MatrixTransform3D`] and [`Transform3DGroup`], and the rotations
//!   [`Rotation3D`], [`AxisAngleRotation3D`] and [`QuaternionRotation3D`];
//! - the cameras [`Camera`], [`ProjectionCamera`], [`PerspectiveCamera`],
//!   [`OrthographicCamera`] and [`MatrixCamera`];
//! - the models [`Model3D`], [`GeometryModel3D`] and [`Model3DGroup`], and the lights
//!   [`Light`], [`AmbientLight`], [`DirectionalLight`], [`PointLightBase`], [`PointLight`] and
//!   [`SpotLight`];
//! - the geometry [`Geometry3D`] and [`MeshGeometry3D`], with the collections
//!   [`Point3DCollection`], [`Vector3DCollection`] and [`Int32Collection`] (and the base library's
//!   `Points` for the texture coordinates);
//! - the materials [`Material`], [`DiffuseMaterial`], [`SpecularMaterial`],
//!   [`EmissiveMaterial`] and [`MaterialGroup`], painting with the brushes of FerroUI;
//! - the visuals [`Visual3D`], [`ModelVisual3D`] and [`Viewport3DVisual`], and the control
//!   [`Viewport3D`].
//!
//! A `Viewport3DVisual` compiles its scene into an immutable [`Scene3DSnapshot`] (world
//! matrices, shared meshes, material layers, lights in world space, the camera with its planes
//! fitted to the scene, and three generations), which `Viewport3D` draws through the
//! render-data operation `DrawScene3D`; the renderers are in another crate.
//!
//! [`register_types`] adds the classes, the markup metadata of the values and collections (their
//! text converters) and the XML namespace of the framework to the tables of the markup loader.
#![forbid(unsafe_code)]

mod camera;
mod change;
mod collections;
mod controls;
mod error;
mod geometry_3d;
mod light;
mod markup_types;
mod material;
mod matrix_3d;
mod model_3d;
mod point_3d;
mod point_4d;
mod quaternion;
mod rect_3d;
mod rotation_3d;
mod scene_compiler;
mod size_3d;
mod text;
mod transform_3d;
mod vector_3d;
mod viewport_3d_visual;
mod visual_3d;

pub use camera::{
    Camera, CameraImpl, CameraImplExt, CameraVTable, MatrixCamera, OrthographicCamera, PerspectiveCamera,
    ProjectionCamera,
};
pub use change::Scene3DChange;
pub use collections::{
    Int32Collection, MaterialCollection, Model3DCollection, Point3DCollection, Transform3DCollection,
    Vector3DCollection, Visual3DCollection,
};
pub use controls::Viewport3D;
pub use error::{Matrix3DError, QuaternionError, Size3DError};
pub use geometry_3d::{Geometry3D, Geometry3DImpl, Geometry3DImplExt, Geometry3DVTable, MeshGeometry3D};
pub use light::{AmbientLight, DirectionalLight, Light, PointLight, PointLightBase, SpotLight};
pub use markup_types::{register_types, ASSEMBLY};
pub use material::{DiffuseMaterial, EmissiveMaterial, Material, MaterialGroup, SpecularMaterial};
pub use matrix_3d::Matrix3D;
pub use model_3d::{GeometryModel3D, Model3D, Model3DGroup, Model3DImpl, Model3DImplExt, Model3DVTable};
pub use point_3d::Point3D;
pub use point_4d::Point4D;
pub use quaternion::Quaternion;
pub use rect_3d::Rect3D;
pub use rotation_3d::{
    AxisAngleRotation3D, QuaternionRotation3D, Rotation3D, Rotation3DImpl, Rotation3DImplExt, Rotation3DVTable,
};
pub use size_3d::Size3D;
pub use transform_3d::{
    MatrixTransform3D, RotateTransform3D, ScaleTransform3D, Transform3D, Transform3DGroup, Transform3DImpl,
    Transform3DImplExt, Transform3DVTable, TranslateTransform3D,
};
pub use vector_3d::Vector3D;
pub use viewport_3d_visual::Viewport3DVisual;
pub use visual_3d::{ModelVisual3D, Visual3D};

/// The compiled form of a scene, shared with the renderers (defined by the base library, which
/// the backends depend on).
pub use ferroui_base::rendering::scene3d::{
    MeshData, Scene3DBounds, Scene3DCamera, Scene3DCameraKind, Scene3DFrame, Scene3DLight, Scene3DMaterialKind,
    Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot,
};

#[cfg(test)]
mod scene_tests;

#[cfg(test)]
mod xaml_tests;
