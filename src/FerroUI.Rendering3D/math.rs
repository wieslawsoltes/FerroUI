//! The few vector operations of the renderer, on plain arrays: what the WGSL of the executors
//! writes with `vec3<f32>` and `vec4<f32>`, in the same order of operations.

/// A 3-component vector.
pub(crate) type Float3 = [f32; 3];
/// A 4-component vector.
pub(crate) type Float4 = [f32; 4];

#[inline]
pub(crate) fn add(a: Float3, b: Float3) -> Float3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
pub(crate) fn sub(a: Float3, b: Float3) -> Float3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
pub(crate) fn scale(a: Float3, s: f32) -> Float3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

#[inline]
pub(crate) fn mul(a: Float3, b: Float3) -> Float3 {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

#[inline]
pub(crate) fn dot(a: Float3, b: Float3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
pub(crate) fn length(a: Float3) -> f32 {
    dot(a, a).sqrt()
}

/// The vector at unit length; the zero vector stays zero (WGSL's `normalize` of zero is
/// undefined: the CPU reference keeps it zero, which lights nothing).
#[inline]
pub(crate) fn normalize(a: Float3) -> Float3 {
    let length = length(a);
    if length > 0.0 {
        scale(a, 1.0 / length)
    } else {
        [0.0; 3]
    }
}

#[inline]
pub(crate) fn saturate(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

#[inline]
pub(crate) fn saturate3(a: Float3) -> Float3 {
    [saturate(a[0]), saturate(a[1]), saturate(a[2])]
}

#[inline]
pub(crate) fn xyz(a: Float4) -> Float3 {
    [a[0], a[1], a[2]]
}

/// `[x, y, z, w] * matrix`: the row-vector product of the numerics (and of WPF), which the tests
/// compare the uniform form with.
#[cfg(test)]
pub(crate) fn transform4(v: Float4, m: &ferroui_base::numerics::Matrix4x4) -> Float4 {
    [
        v[0] * m.m11 + v[1] * m.m21 + v[2] * m.m31 + v[3] * m.m41,
        v[0] * m.m12 + v[1] * m.m22 + v[2] * m.m32 + v[3] * m.m42,
        v[0] * m.m13 + v[1] * m.m23 + v[2] * m.m33 + v[3] * m.m43,
        v[0] * m.m14 + v[1] * m.m24 + v[2] * m.m34 + v[3] * m.m44,
    ]
}

/// `[x, y, z, w]` times a matrix stored as uniform columns (see `uniforms::matrix_uniform`):
/// the same product as [`transform4`] of the matrix the columns were made from.
#[inline]
pub(crate) fn transform_columns(v: Float4, columns: &[[f32; 4]; 4]) -> Float4 {
    // Column `i` of the uniform is row `i` of the row-vector matrix, so component `j` of the
    // result is the sum over `i` of `v[i] * columns[i][j]`.
    let mut result = [0.0f32; 4];
    for (i, column) in columns.iter().enumerate() {
        for j in 0..4 {
            result[j] += v[i] * column[j];
        }
    }
    result
}
