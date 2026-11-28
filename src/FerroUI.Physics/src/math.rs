//! High-performance 2D and 3D linear algebra primitives for physics simulation.
//!
//! All types are `#[repr(C)]`, `Copy`, and layout-compatible with GPU storage/uniform buffers
//! and SIMD vector lanes.

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// 2D single-precision floating-point vector.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0 };

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    #[inline]
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    /// 2D scalar cross product (`a.x * b.y - a.y * b.x`).
    #[inline]
    pub fn cross(self, rhs: Self) -> f32 {
        self.x * rhs.y - self.y * rhs.x
    }

    /// Cross product of scalar `s` (along Z) and 2D vector `self`: `(-s * y, s * x)`.
    #[inline]
    pub fn cross_scalar_left(s: f32, v: Self) -> Self {
        Self {
            x: -s * v.y,
            y: s * v.x,
        }
    }

    /// Cross product of 2D vector `self` and scalar `s` (along Z): `(s * y, -s * x)`.
    #[inline]
    pub fn cross_scalar_right(self, s: f32) -> Self {
        Self {
            x: s * self.y,
            y: -s * self.x,
        }
    }

    /// Perpendicular vector rotated +90 degrees CCW (`(-y, x)`).
    #[inline]
    pub fn perp(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    /// Perpendicular vector rotated -90 degrees CW (`(y, -x)`).
    #[inline]
    pub fn perp_cw(self) -> Self {
        Self {
            x: self.y,
            y: -self.x,
        }
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn distance_squared(self, other: Self) -> f32 {
        (self - other).length_squared()
    }

    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        self.distance_squared(other).sqrt()
    }

    #[inline]
    pub fn normalize_or_zero(self) -> Self {
        let len_sq = self.length_squared();
        if len_sq > 1e-12 {
            self * (1.0 / len_sq.sqrt())
        } else {
            Self::ZERO
        }
    }

    #[inline]
    pub fn normalize_with_length(self) -> (Self, f32) {
        let len = self.length();
        if len > 1e-6 {
            (self * (1.0 / len), len)
        } else {
            (Self::ZERO, 0.0)
        }
    }

    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }

    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            x: self.x.clamp(min.x, max.x),
            y: self.y.clamp(min.y, max.y),
        }
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    #[inline]
    pub fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}

impl MulAssign<f32> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        let inv = 1.0 / rhs;
        Self {
            x: self.x * inv,
            y: self.y * inv,
        }
    }
}

impl DivAssign<f32> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        let inv = 1.0 / rhs;
        self.x *= inv;
        self.y *= inv;
    }
}

impl Neg for Vec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl From<[f32; 2]> for Vec2 {
    #[inline]
    fn from(a: [f32; 2]) -> Self {
        Self { x: a[0], y: a[1] }
    }
}

impl From<(f32, f32)> for Vec2 {
    #[inline]
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

/// 3D single-precision floating-point vector (for 3D UI springs, camera/spatial effects, and GPU alignment).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    #[inline]
    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn normalize_or_zero(self) -> Self {
        let l2 = self.length_squared();
        if l2 > 1e-12 {
            self * (1.0 / l2.sqrt())
        } else {
            Self::ZERO
        }
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

/// 2D rotation stored as `(cos, sin)` for branchless, trig-free vector rotations.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rot2 {
    pub cos: f32,
    pub sin: f32,
}

impl Default for Rot2 {
    #[inline]
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Rot2 {
    pub const IDENTITY: Self = Self { cos: 1.0, sin: 0.0 };

    #[inline]
    pub fn from_radians(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self { cos, sin }
    }

    #[inline]
    pub fn radians(self) -> f32 {
        self.sin.atan2(self.cos)
    }

    /// Rotates a vector by this rotation.
    #[inline]
    pub fn rotate(self, v: Vec2) -> Vec2 {
        Vec2::new(
            self.cos * v.x - self.sin * v.y,
            self.sin * v.x + self.cos * v.y,
        )
    }

    /// Rotates a vector by the inverse of this rotation.
    #[inline]
    pub fn inv_rotate(self, v: Vec2) -> Vec2 {
        Vec2::new(
            self.cos * v.x + self.sin * v.y,
            -self.sin * v.x + self.cos * v.y,
        )
    }

    #[inline]
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            cos: self.cos * rhs.cos - self.sin * rhs.sin,
            sin: self.sin * rhs.cos + self.cos * rhs.sin,
        }
    }

    #[inline]
    pub fn inverse(self) -> Self {
        Self {
            cos: self.cos,
            sin: -self.sin,
        }
    }

    #[inline]
    pub fn x_axis(self) -> Vec2 {
        Vec2::new(self.cos, self.sin)
    }

    #[inline]
    pub fn y_axis(self) -> Vec2 {
        Vec2::new(-self.sin, self.cos)
    }

    #[inline]
    pub fn normalized(self) -> Self {
        let inv = (self.cos * self.cos + self.sin * self.sin).sqrt();
        if inv > 1e-6 {
            Self {
                cos: self.cos / inv,
                sin: self.sin / inv,
            }
        } else {
            Self::IDENTITY
        }
    }

    /// Shortest-arc spherical linear interpolation between two 2D rotations.
    #[inline]
    pub fn nlerp(self, other: Self, t: f32) -> Self {
        let dot = self.cos * other.cos + self.sin * other.sin;
        let sign = if dot < 0.0 { -1.0 } else { 1.0 };
        let c = self.cos + (other.cos * sign - self.cos) * t;
        let s = self.sin + (other.sin * sign - self.sin) * t;
        Self { cos: c, sin: s }.normalized()
    }
}

/// 2D rigid-body transform (translation + rotation).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Transform2D {
    pub position: Vec2,
    pub rotation: Rot2,
}

impl Transform2D {
    pub const IDENTITY: Self = Self {
        position: Vec2::ZERO,
        rotation: Rot2::IDENTITY,
    };

    #[inline]
    pub fn new(position: Vec2, angle_radians: f32) -> Self {
        Self {
            position,
            rotation: Rot2::from_radians(angle_radians),
        }
    }

    #[inline]
    pub fn from_position(position: Vec2) -> Self {
        Self {
            position,
            rotation: Rot2::IDENTITY,
        }
    }

    #[inline]
    pub fn transform_point(self, p: Vec2) -> Vec2 {
        self.position + self.rotation.rotate(p)
    }

    #[inline]
    pub fn inv_transform_point(self, p: Vec2) -> Vec2 {
        self.rotation.inv_rotate(p - self.position)
    }

    #[inline]
    pub fn transform_vector(self, v: Vec2) -> Vec2 {
        self.rotation.rotate(v)
    }

    #[inline]
    pub fn inv_transform_vector(self, v: Vec2) -> Vec2 {
        self.rotation.inv_rotate(v)
    }

    #[inline]
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            position: self.transform_point(rhs.position),
            rotation: self.rotation.mul(rhs.rotation),
        }
    }

    #[inline]
    pub fn interpolate(self, other: Self, alpha: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, alpha),
            rotation: self.rotation.nlerp(other.rotation, alpha),
        }
    }
}

/// 2x2 matrix for 2-DOF joint effective mass inversion.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat2 {
    pub col0: Vec2,
    pub col1: Vec2,
}

impl Mat2 {
    #[inline]
    pub const fn from_cols(col0: Vec2, col1: Vec2) -> Self {
        Self { col0, col1 }
    }

    #[inline]
    pub fn mul_vec(self, v: Vec2) -> Vec2 {
        Vec2::new(
            self.col0.x * v.x + self.col1.x * v.y,
            self.col0.y * v.x + self.col1.y * v.y,
        )
    }

    /// Solves `A * x = b` for 2x2 matrix `A`; returns zero vector if singular.
    #[inline]
    pub fn solve(self, b: Vec2) -> Vec2 {
        let a11 = self.col0.x;
        let a12 = self.col1.x;
        let a21 = self.col0.y;
        let a22 = self.col1.y;
        let det = a11 * a22 - a12 * a21;
        if det.abs() < 1e-10 {
            return Vec2::ZERO;
        }
        let inv = 1.0 / det;
        Vec2::new(inv * (a22 * b.x - a12 * b.y), inv * (a11 * b.y - a21 * b.x))
    }
}

/// Axis-aligned bounding box in 2D.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb2 {
    pub min: Vec2,
    pub max: Vec2,
}

impl Default for Aabb2 {
    fn default() -> Self {
        Self {
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        }
    }
}

impl Aabb2 {
    #[inline]
    pub const fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn from_center_half_extents(center: Vec2, half: Vec2) -> Self {
        Self {
            min: center - half,
            max: center + half,
        }
    }

    #[inline]
    pub fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    #[inline]
    pub fn extents(self) -> Vec2 {
        self.max - self.min
    }

    #[inline]
    pub fn half_extents(self) -> Vec2 {
        (self.max - self.min) * 0.5
    }

    /// Half-perimeter (surface area metric for 2D BVH SAH).
    #[inline]
    pub fn perimeter(self) -> f32 {
        let e = self.max - self.min;
        2.0 * (e.x + e.y)
    }

    #[inline]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[inline]
    pub fn expanded(self, margin: f32) -> Self {
        let m = Vec2::splat(margin);
        Self {
            min: self.min - m,
            max: self.max + m,
        }
    }

    #[inline]
    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    #[inline]
    pub fn contains_aabb(self, inner: Self) -> bool {
        self.min.x <= inner.min.x
            && self.min.y <= inner.min.y
            && self.max.x >= inner.max.x
            && self.max.y >= inner.max.y
    }

    #[inline]
    pub fn contains_point(self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// Fast slab ray-AABB intersection; returns `Some(t_enter)` in `[0, max_toi]`.
    #[inline]
    pub fn ray_intersect(self, origin: Vec2, inv_dir: Vec2, max_toi: f32) -> Option<f32> {
        if max_toi.is_nan() || max_toi < 0.0 {
            return None;
        }
        let mut tmin = 0.0_f32;
        let mut tmax = max_toi;
        for (o, inv, min, max) in [
            (origin.x, inv_dir.x, self.min.x, self.max.x),
            (origin.y, inv_dir.y, self.min.y, self.max.y),
        ] {
            if inv.is_infinite() {
                // Parallel rays on a slab boundary must not evaluate 0 * inf.
                if o < min || o > max {
                    return None;
                }
            } else {
                let near = (min - o) * inv;
                let far = (max - o) * inv;
                tmin = tmin.max(near.min(far));
                tmax = tmax.min(near.max(far));
                if tmin > tmax {
                    return None;
                }
            }
        }
        Some(tmin)
    }
}

/// 2D ray with normalized direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray2 {
    pub origin: Vec2,
    pub direction: Vec2,
}

impl Ray2 {
    #[inline]
    pub fn new(origin: Vec2, direction: Vec2) -> Self {
        Self {
            origin,
            direction: direction.normalize_or_zero(),
        }
    }

    #[inline]
    pub fn point_at(self, t: f32) -> Vec2 {
        self.origin + self.direction * t
    }

    #[inline]
    pub fn inv_direction(self) -> Vec2 {
        Vec2::new(
            if self.direction.x.abs() > 1e-12 {
                1.0 / self.direction.x
            } else {
                f32::INFINITY
            },
            if self.direction.y.abs() > 1e-12 {
                1.0 / self.direction.y
            } else {
                f32::INFINITY
            },
        )
    }
}
