//! The errors of the values: what WPF reports with an exception.

use std::fmt;

/// The error of [`Matrix3D::invert`](crate::Matrix3D::invert): the matrix has no inverse (its
/// determinant is zero, within the tolerance of [`Matrix3D::has_inverse`](crate::Matrix3D::has_inverse)).
/// WPF throws `InvalidOperationException`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Matrix3DError {
    /// The matrix is not invertible.
    NotInvertible,
}

impl fmt::Display for Matrix3DError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Matrix3DError::NotInvertible => f.write_str("Transform is not invertible."),
        }
    }
}

impl std::error::Error for Matrix3DError {}

/// The error of [`Quaternion::from_axis_angle`](crate::Quaternion::from_axis_angle): the axis is the
/// zero vector, which names no rotation. WPF throws `InvalidOperationException`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum QuaternionError {
    /// The axis of the rotation is the zero vector.
    ZeroAxis,
}

impl fmt::Display for QuaternionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuaternionError::ZeroAxis => f.write_str("The axis of a rotation cannot be the zero vector."),
        }
    }
}

impl std::error::Error for QuaternionError {}

/// The error of the members of [`Size3D`](crate::Size3D) and [`Rect3D`](crate::Rect3D) that WPF
/// guards: a negative dimension (WPF's `ArgumentException`), or a change of the `Empty` value
/// (WPF's `InvalidOperationException`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Size3DError {
    /// A dimension is negative.
    NegativeDimension,
    /// The value is `Empty`, which cannot be changed or moved.
    ModifiesEmpty,
}

impl Size3DError {
    /// The message of the error, as a static text (the parsers report it in their `FormatError`).
    pub const fn message(&self) -> &'static str {
        match self {
            Size3DError::NegativeDimension => "Dimension cannot be a negative value.",
            Size3DError::ModifiesEmpty => "Cannot modify or move the Empty value.",
        }
    }
}

impl fmt::Display for Size3DError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for Size3DError {}
