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
//! The scene model of `Viewport3D` (transforms, cameras, lights, materials, geometry, models and
//! visuals) follows in step D2 of the port; the renderers are in another crate.
//!
//! [`register_types`] adds the markup metadata of the values (their text converters) to the
//! tables of the markup loader.
#![forbid(unsafe_code)]

mod error;
mod markup_types;
mod matrix_3d;
mod point_3d;
mod point_4d;
mod quaternion;
mod rect_3d;
mod size_3d;
mod text;
mod vector_3d;

pub use error::{Matrix3DError, QuaternionError, Size3DError};
pub use markup_types::{register_types, ASSEMBLY};
pub use matrix_3d::Matrix3D;
pub use point_3d::Point3D;
pub use point_4d::Point4D;
pub use quaternion::Quaternion;
pub use rect_3d::Rect3D;
pub use size_3d::Size3D;
pub use vector_3d::Vector3D;
