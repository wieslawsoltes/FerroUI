//! The vector, quaternion and matrix values of the Windows Runtime
//! (`Windows.Foundation.Numerics`), by their layout: the reference passes
//! the types of its runtime's numerics library, which have this layout.

/// Two single-precision numbers.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector2 {
    #[allow(missing_docs)]
    pub x: f32,
    #[allow(missing_docs)]
    pub y: f32,
}

/// Three single-precision numbers.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector3 {
    #[allow(missing_docs)]
    pub x: f32,
    #[allow(missing_docs)]
    pub y: f32,
    #[allow(missing_docs)]
    pub z: f32,
}

/// A rotation as four single-precision numbers.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Quaternion {
    #[allow(missing_docs)]
    pub x: f32,
    #[allow(missing_docs)]
    pub y: f32,
    #[allow(missing_docs)]
    pub z: f32,
    #[allow(missing_docs)]
    pub w: f32,
}

/// A matrix of four rows of four single-precision numbers, row by row.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Matrix4x4 {
    #[allow(missing_docs)]
    pub m: [[f32; 4]; 4],
}

impl Matrix4x4 {
    /// The matrix that changes nothing.
    pub const IDENTITY: Self =
        Self { m: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]] };
}
