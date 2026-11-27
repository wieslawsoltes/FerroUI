//! 2D collision shapes, exact mass properties, AABB bounds, and GJK support functions.

use crate::math::{Aabb2, Transform2D, Vec2};
use smallvec::SmallVec;
use std::f32::consts::PI;

/// Maximum vertices stored inline for a convex polygon (avoids heap allocation).
pub const MAX_POLYGON_VERTICES: usize = 8;

/// Computed mass, rotational inertia about the local origin, and centroid of a shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MassProperties {
    pub mass: f32,
    pub inertia: f32,
    pub center_of_mass: Vec2,
    pub area: f32,
}

impl MassProperties {
    pub const ZERO: Self = Self { mass: 0.0, inertia: 0.0, center_of_mass: Vec2::ZERO, area: 0.0 };
}

/// Convex polygon in local coordinates with counter-clockwise winding and precomputed outward edge normals.
#[derive(Clone, Debug, PartialEq)]
pub struct ConvexPolygon {
    pub vertices: SmallVec<[Vec2; MAX_POLYGON_VERTICES]>,
    pub normals: SmallVec<[Vec2; MAX_POLYGON_VERTICES]>,
    pub radius: f32,
}

impl ConvexPolygon {
    /// Constructs a convex polygon from an arbitrary point cloud using Andrew's monotone chain
    /// convex hull algorithm, centering around the polygon centroid if requested.
    pub fn from_points(points: &[Vec2]) -> Option<Self> {
        if points.len() < 3 {
            return None;
        }
        let mut pts: Vec<Vec2> = points.to_vec();
        pts.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y)));
        pts.dedup_by(|a, b| a.distance_squared(*b) < 1e-8);
        if pts.len() < 3 {
            return None;
        }

        let mut hull: Vec<Vec2> = Vec::with_capacity(pts.len() * 2);
        // Lower hull
        for &p in &pts {
            while hull.len() >= 2 {
                let a = hull[hull.len() - 2];
                let b = hull[hull.len() - 1];
                if (b - a).cross(p - a) <= 1e-6 {
                    hull.pop();
                } else {
                    break;
                }
            }
            hull.push(p);
        }
        // Upper hull
        let lower_len = hull.len() + 1;
        for &p in pts.iter().rev().skip(1) {
            while hull.len() >= lower_len {
                let a = hull[hull.len() - 2];
                let b = hull[hull.len() - 1];
                if (b - a).cross(p - a) <= 1e-6 {
                    hull.pop();
                } else {
                    break;
                }
            }
            hull.push(p);
        }
        hull.pop();
        if hull.len() < 3 {
            return None;
        }
        // Clamp to MAX_POLYGON_VERTICES by uniform decimation if needed
        if hull.len() > MAX_POLYGON_VERTICES {
            let step = hull.len() as f32 / MAX_POLYGON_VERTICES as f32;
            let mut decimated = Vec::with_capacity(MAX_POLYGON_VERTICES);
            for i in 0..MAX_POLYGON_VERTICES {
                decimated.push(hull[((i as f32) * step) as usize % hull.len()]);
            }
            hull = decimated;
        }
        Some(Self::from_ccw_unchecked(hull))
    }

    /// Creates a regular convex polygon with `sides` in `3..=8` and circumradius `radius`.
    pub fn regular(sides: usize, radius: f32) -> Self {
        let n = sides.clamp(3, MAX_POLYGON_VERTICES);
        let r = radius.max(1e-3);
        let mut verts = Vec::with_capacity(n);
        for i in 0..n {
            let angle = (i as f32) * (2.0 * PI / n as f32);
            let (s, c) = angle.sin_cos();
            verts.push(Vec2::new(c * r, s * r));
        }
        Self::from_ccw_unchecked(verts)
    }

    /// Creates an axis-aligned box as a 4-vertex convex polygon.
    pub fn rectangle(half_width: f32, half_height: f32) -> Self {
        let hx = half_width.max(1e-3);
        let hy = half_height.max(1e-3);
        Self::from_ccw_unchecked(vec![
            Vec2::new(-hx, -hy),
            Vec2::new(hx, -hy),
            Vec2::new(hx, hy),
            Vec2::new(-hx, hy),
        ])
    }

    fn from_ccw_unchecked(verts: Vec<Vec2>) -> Self {
        let mut vertices: SmallVec<[Vec2; MAX_POLYGON_VERTICES]> = SmallVec::with_capacity(verts.len());
        vertices.extend(verts);
        let n = vertices.len();
        let mut normals = SmallVec::with_capacity(n);
        let mut max_r2 = 0.0_f32;
        for i in 0..n {
            let a = vertices[i];
            let b = vertices[(i + 1) % n];
            let edge = b - a;
            // Outward normal for CCW winding is (edge.y, -edge.x)
            normals.push(edge.perp_cw().normalize_or_zero());
            max_r2 = max_r2.max(a.length_squared());
        }
        Self { vertices, normals, radius: max_r2.sqrt() }
    }

    /// Returns the farthest vertex in local direction `dir`.
    #[inline]
    pub fn support(&self, dir: Vec2) -> Vec2 {
        let mut best_i = 0;
        let mut best_dot = self.vertices[0].dot(dir);
        for i in 1..self.vertices.len() {
            let d = self.vertices[i].dot(dir);
            if d > best_dot {
                best_dot = d;
                best_i = i;
            }
        }
        self.vertices[best_i]
    }
}

/// 2D geometric collision shape.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape2D {
    /// Solid disk centered at the local origin.
    Circle { radius: f32 },
    /// Oriented rectangle centered at the local origin with half-extents `(hx, hy)`.
    Box { half_extents: Vec2 },
    /// Pill shape: Minkowski sum of a vertical segment `[(0, -half_height), (0, +half_height)]` and `radius`.
    Capsule { half_height: f32, radius: f32 },
    /// General convex polygon in local coordinates (up to 8 vertices inline).
    Polygon(ConvexPolygon),
    /// Thin two-sided line segment (ideal for static terrain, walls, platforms).
    Segment { a: Vec2, b: Vec2 },
}

impl Shape2D {
    #[inline]
    pub fn circle(radius: f32) -> Self {
        Self::Circle { radius: radius.max(1e-3) }
    }

    #[inline]
    pub fn rect(width: f32, height: f32) -> Self {
        Self::Box { half_extents: Vec2::new((width * 0.5).max(1e-3), (height * 0.5).max(1e-3)) }
    }

    #[inline]
    pub fn box_half_extents(hx: f32, hy: f32) -> Self {
        Self::Box { half_extents: Vec2::new(hx.max(1e-3), hy.max(1e-3)) }
    }

    #[inline]
    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self::Capsule { half_height: half_height.max(0.0), radius: radius.max(1e-3) }
    }

    #[inline]
    pub fn regular_polygon(sides: usize, radius: f32) -> Self {
        Self::Polygon(ConvexPolygon::regular(sides, radius))
    }

    #[inline]
    pub fn segment(a: Vec2, b: Vec2) -> Self {
        Self::Segment { a, b }
    }

    /// Exact bounding radius from the local origin (used for CCD & quick cull).
    #[inline]
    pub fn bounding_radius(&self) -> f32 {
        match self {
            Self::Circle { radius } => *radius,
            Self::Box { half_extents } => half_extents.length(),
            Self::Capsule { half_height, radius } => *half_height + *radius,
            Self::Polygon(p) => p.radius,
            Self::Segment { a, b } => a.length().max(b.length()),
        }
    }

    /// Computes exact world-space AABB for this shape under `transform`.
    pub fn compute_aabb(&self, transform: Transform2D) -> Aabb2 {
        match self {
            Self::Circle { radius } => {
                Aabb2::from_center_half_extents(transform.position, Vec2::splat(*radius))
            }
            Self::Box { half_extents } => {
                let c = transform.rotation.cos.abs();
                let s = transform.rotation.sin.abs();
                let ex = c * half_extents.x + s * half_extents.y;
                let ey = s * half_extents.x + c * half_extents.y;
                Aabb2::from_center_half_extents(transform.position, Vec2::new(ex, ey))
            }
            Self::Capsule { half_height, radius } => {
                let p0 = transform.transform_point(Vec2::new(0.0, -*half_height));
                let p1 = transform.transform_point(Vec2::new(0.0, *half_height));
                let r = Vec2::splat(*radius);
                Aabb2::new(p0.min(p1) - r, p0.max(p1) + r)
            }
            Self::Polygon(poly) => {
                let first = transform.transform_point(poly.vertices[0]);
                let mut min = first;
                let mut max = first;
                for &v in &poly.vertices[1..] {
                    let w = transform.transform_point(v);
                    min = min.min(w);
                    max = max.max(w);
                }
                Aabb2::new(min, max)
            }
            Self::Segment { a, b } => {
                let wa = transform.transform_point(*a);
                let wb = transform.transform_point(*b);
                let pad = Vec2::splat(1e-3);
                Aabb2::new(wa.min(wb) - pad, wa.max(wb) + pad)
            }
        }
    }

    /// Computes exact mass, moment of inertia, and centroid for uniform `density`.
    pub fn compute_mass_properties(&self, density: f32) -> MassProperties {
        if density <= 0.0 {
            return MassProperties::ZERO;
        }
        match self {
            Self::Circle { radius } => {
                let r2 = radius * radius;
                let area = PI * r2;
                let mass = density * area;
                let inertia = 0.5 * mass * r2;
                MassProperties { mass, inertia, center_of_mass: Vec2::ZERO, area }
            }
            Self::Box { half_extents } => {
                let w = half_extents.x * 2.0;
                let h = half_extents.y * 2.0;
                let area = w * h;
                let mass = density * area;
                let inertia = mass * (w * w + h * h) / 12.0;
                MassProperties { mass, inertia, center_of_mass: Vec2::ZERO, area }
            }
            Self::Capsule { half_height, radius } => {
                let h = half_height * 2.0;
                let rect_area = 2.0 * radius * h;
                let circle_area = PI * radius * radius;
                let area = rect_area + circle_area;
                let rect_mass = density * rect_area;
                let circle_mass = density * circle_area;
                let mass = rect_mass + circle_mass;
                let rect_i = rect_mass * (4.0 * radius * radius + h * h) / 12.0;
                let circle_i = circle_mass * (0.5 * radius * radius + half_height * half_height);
                MassProperties { mass, inertia: rect_i + circle_i, center_of_mass: Vec2::ZERO, area }
            }
            Self::Polygon(poly) => {
                let mut area2 = 0.0_f32;
                let mut centroid = Vec2::ZERO;
                let mut inertia_num = 0.0_f32;
                let n = poly.vertices.len();
                for i in 0..n {
                    let p1 = poly.vertices[i];
                    let p2 = poly.vertices[(i + 1) % n];
                    let cross = p1.cross(p2);
                    area2 += cross;
                    centroid += (p1 + p2) * cross;
                    let intx2 = p1.x * p1.x + p2.x * p1.x + p2.x * p2.x;
                    let inty2 = p1.y * p1.y + p2.y * p1.y + p2.y * p2.y;
                    inertia_num += cross * (intx2 + inty2);
                }
                let area = (0.5 * area2).abs().max(1e-6);
                let center_of_mass = if area2.abs() > 1e-6 { centroid / (3.0 * area2) } else { Vec2::ZERO };
                let mass = density * area;
                let inertia = (density * inertia_num.abs() / 12.0).max(1e-4 * mass);
                MassProperties { mass, inertia, center_of_mass, area }
            }
            Self::Segment { a, b } => {
                let len = a.distance(*b).max(1e-3);
                let mass = density * len * 0.05;
                let inertia = mass * len * len / 12.0;
                MassProperties { mass, inertia, center_of_mass: (*a + *b) * 0.5, area: len * 0.05 }
            }
        }
    }

    /// World-space GJK support point in direction `world_dir`.
    #[inline]
    pub fn support_world(&self, transform: Transform2D, world_dir: Vec2) -> Vec2 {
        let local_dir = transform.inv_transform_vector(world_dir);
        let local_pt = match self {
            Self::Circle { radius } => local_dir.normalize_or_zero() * *radius,
            Self::Box { half_extents } => Vec2::new(
                if local_dir.x >= 0.0 { half_extents.x } else { -half_extents.x },
                if local_dir.y >= 0.0 { half_extents.y } else { -half_extents.y },
            ),
            Self::Capsule { half_height, radius } => {
                let end = Vec2::new(0.0, if local_dir.y >= 0.0 { *half_height } else { -*half_height });
                end + local_dir.normalize_or_zero() * *radius
            }
            Self::Polygon(poly) => poly.support(local_dir),
            Self::Segment { a, b } => {
                if a.dot(local_dir) >= b.dot(local_dir) {
                    *a
                } else {
                    *b
                }
            }
        };
        transform.transform_point(local_pt)
    }

    /// Tests whether a world-space point `p` lies inside (or on) this shape.
    pub fn contains_point(&self, transform: Transform2D, world_point: Vec2) -> bool {
        let local = transform.inv_transform_point(world_point);
        match self {
            Self::Circle { radius } => local.length_squared() <= radius * radius,
            Self::Box { half_extents } => {
                local.x.abs() <= half_extents.x && local.y.abs() <= half_extents.y
            }
            Self::Capsule { half_height, radius } => {
                let clamped_y = local.y.clamp(-*half_height, *half_height);
                let dx = local.x;
                let dy = local.y - clamped_y;
                dx * dx + dy * dy <= radius * radius
            }
            Self::Polygon(poly) => {
                for i in 0..poly.vertices.len() {
                    if poly.normals[i].dot(local - poly.vertices[i]) > 0.0 {
                        return false;
                    }
                }
                true
            }
            Self::Segment { a, b } => {
                let ab = *b - *a;
                let len2 = ab.length_squared().max(1e-12);
                let t = ((local - *a).dot(ab) / len2).clamp(0.0, 1.0);
                let closest = *a + ab * t;
                local.distance_squared(closest) <= 1e-4
            }
        }
    }

    /// Converts Box into a 4-vertex local polygon representation for unified clipping / rendering.
    pub fn as_polygon_vertices(&self) -> Option<SmallVec<[Vec2; MAX_POLYGON_VERTICES]>> {
        match self {
            Self::Box { half_extents } => {
                let mut v = SmallVec::new();
                v.push(Vec2::new(-half_extents.x, -half_extents.y));
                v.push(Vec2::new(half_extents.x, -half_extents.y));
                v.push(Vec2::new(half_extents.x, half_extents.y));
                v.push(Vec2::new(-half_extents.x, half_extents.y));
                Some(v)
            }
            Self::Polygon(p) => Some(p.vertices.clone()),
            _ => None,
        }
    }
}
