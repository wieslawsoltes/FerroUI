//! Single-precision vector, quaternion and matrix value types used by the
//! composition engine. They follow the semantics of the reference runtime's
//! numerics library (row vectors, row-major matrices, right-handed rotations).
//!
//! The crate root has double-precision `Vector`, `Vector3D` and `Matrix`; the
//! conversions between the two families live next to the types of this module
//! (`From` implementations in `vector2.rs` and `vector3.rs`).

mod matrix3x2;
mod matrix4x4;
mod quaternion;
mod single;
mod vector2;
mod vector3;
mod vector4;

pub use matrix3x2::Matrix3x2;
pub use matrix4x4::Matrix4x4;
pub use quaternion::Quaternion;
pub use single::InvariantF32;
pub use vector2::Vector2;
pub use vector3::Vector3;
pub use vector4::Vector4;

#[cfg(test)]
mod tests;
