//! Spatial queries (raycasting, point picking, AABB queries), collision/trigger events,
//! and programmable force fields (attractors, repulsors, wind, vortex, buoyancy, drag).

use crate::body::{BodyHandle, RigidBody};
use crate::material::CollisionFilter;
use crate::math::{Aabb2, Ray2, Transform2D, Vec2};
use crate::shapes::Shape2D;

/// Result of a raycast hitting a rigid body.
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub body: BodyHandle,
    /// Distance along the normalized ray (`0.0..=max_toi`).
    pub toi: f32,
    /// World-space hit point.
    pub point: Vec2,
    /// Outward surface normal at the hit point.
    pub normal: Vec2,
}

/// Result of a swept [`Shape2D`] cast hitting a rigid body.
#[derive(Clone, Copy, Debug)]
pub struct ShapeCastHit {
    pub body: BodyHandle,
    /// Distance traveled along the cast direction before first impact (`0.0..=max_toi`).
    pub toi: f32,
    /// Approximate world-space impact point.
    pub point: Vec2,
    /// Outward surface normal of the hit body pointing toward the cast shape.
    pub normal: Vec2,
}

/// Result of a nearest-point query against rigid bodies.
#[derive(Clone, Copy, Debug)]
pub struct ClosestPointHit {
    pub body: BodyHandle,
    pub point: Vec2,
    pub distance: f32,
}

/// Lifecycle collision event emitted between two solid bodies.
#[derive(Clone, Copy, Debug)]
pub enum CollisionEvent {
    Started { body_a: BodyHandle, body_b: BodyHandle, normal: Vec2, point: Vec2, impact_speed: f32 },
    Persisted { body_a: BodyHandle, body_b: BodyHandle, normal: Vec2, point: Vec2 },
    Ended { body_a: BodyHandle, body_b: BodyHandle },
}

/// Sensor / trigger overlap event emitted when at least one colliding body has `is_sensor = true`.
#[derive(Clone, Copy, Debug)]
pub enum TriggerEvent {
    Entered { sensor: BodyHandle, other: BodyHandle },
    Exited { sensor: BodyHandle, other: BodyHandle },
}

/// Programmable spatial force field applied to dynamic bodies each substep.
#[derive(Clone, Debug)]
pub enum ForceField {
    /// Radial point gravity/magnet (`strength > 0` attracts toward `center`, `< 0` repels).
    Radial { center: Vec2, strength: f32, radius: f32, falloff_power: f32 },
    /// True inverse-square Newtonian orbital gravity (`a = mu / max(r, min_radius)^2`) for stable Keplerian orbits.
    NewtonianGravity { center: Vec2, mu: f32, min_radius: f32, max_radius: f32 },
    /// Swirl/vortex around `center` combining tangential spin and inward pull.
    Vortex { center: Vec2, tangent_strength: f32, inward_strength: f32, radius: f32 },
    /// Uniform directional wind/thrust inside an optional AABB (`None` = global).
    Wind { acceleration: Vec2, region: Option<Aabb2> },
    /// Fluid buoyancy zone applying Archimedes lift + linear/angular drag to bodies inside `region`.
    Buoyancy { region: Aabb2, fluid_density: f32, linear_drag: f32, angular_drag: f32 },
}

impl ForceField {
    pub fn apply_to_body(&self, body: &mut RigidBody, world_gravity: Vec2) {
        if !body.is_dynamic() || body.is_sleeping {
            return;
        }
        match *self {
            Self::Radial { center, strength, radius, falloff_power } => {
                let delta = center - body.position;
                let dist = delta.length();
                if dist > 1e-4 && dist < radius {
                    let norm = delta * (1.0 / dist);
                    let falloff = (1.0 - dist / radius).powf(falloff_power.max(0.0));
                    body.force += norm * (strength * falloff * body.mass);
                }
            }
            Self::NewtonianGravity { center, mu, min_radius, max_radius } => {
                let delta = center - body.position;
                let dist = delta.length();
                if dist > 1e-4 && dist < max_radius {
                    let norm = delta * (1.0 / dist);
                    let r_eff = dist.max(min_radius.max(0.1));
                    let accel = mu / (r_eff * r_eff);
                    body.force += norm * (accel * body.mass);
                }
            }
            Self::Vortex { center, tangent_strength, inward_strength, radius } => {
                let delta = center - body.position;
                let dist = delta.length();
                if dist > 1e-4 && dist < radius {
                    let inward = delta * (1.0 / dist);
                    let tangent = inward.perp();
                    let falloff = 1.0 - dist / radius;
                    let accel = (tangent * tangent_strength + inward * inward_strength) * falloff;
                    body.force += accel * body.mass;
                }
            }
            Self::Wind { acceleration, region } => {
                if region.map_or(true, |r| r.contains_point(body.position)) {
                    body.force += acceleration * body.mass;
                }
            }
            Self::Buoyancy { region, fluid_density, linear_drag, angular_drag } => {
                if !body.aabb.intersects(region) {
                    return;
                }
                // Approximate submerged fraction from vertical AABB overlap
                let overlap_min = body.aabb.min.y.max(region.min.y);
                let overlap_max = body.aabb.max.y.min(region.max.y);
                let h = (body.aabb.max.y - body.aabb.min.y).max(1e-4);
                let submerged = ((overlap_max - overlap_min) / h).clamp(0.0, 1.0);
                if submerged <= 0.0 {
                    return;
                }
                let area = body.mass / body.material.density.max(1e-4);
                let displaced_mass = fluid_density * area * submerged;
                // Buoyancy opposes gravity
                body.force -= world_gravity * displaced_mass;
                body.force -= body.linear_velocity * (linear_drag * submerged * body.mass);
                body.torque -= body.angular_velocity * (angular_drag * submerged * body.inertia);
            }
        }
    }
}

/// Exact analytical raycast against any [`Shape2D`] at `transform`.
pub fn raycast_shape(
    shape: &Shape2D,
    transform: Transform2D,
    ray: Ray2,
    max_toi: f32,
) -> Option<(f32, Vec2)> {
    let local_origin = transform.inv_transform_point(ray.origin);
    let local_dir = transform.inv_transform_vector(ray.direction);

    let (toi, local_normal) = match shape {
        Shape2D::Circle { radius } => raycast_circle(local_origin, local_dir, *radius, max_toi)?,
        Shape2D::Box { half_extents } => raycast_aabb(local_origin, local_dir, -*half_extents, *half_extents, max_toi)?,
        Shape2D::Polygon(poly) => {
            let mut t_enter = 0.0_f32;
            let mut t_exit = max_toi;
            let mut hit_normal = Vec2::ZERO;
            let mut hit = false;

            for i in 0..poly.vertices.len() {
                let n = poly.normals[i];
                let num = n.dot(poly.vertices[i] - local_origin);
                let den = n.dot(local_dir);
                if den.abs() < 1e-8 {
                    if num < 0.0 {
                        return None;
                    }
                } else {
                    let t = num / den;
                    if den < 0.0 {
                        if t > t_enter {
                            t_enter = t;
                            hit_normal = n;
                            hit = true;
                        }
                    } else if t < t_exit {
                        t_exit = t;
                    }
                    if t_enter > t_exit {
                        return None;
                    }
                }
            }
            if hit && t_enter >= 0.0 && t_enter <= max_toi {
                (t_enter, hit_normal)
            } else {
                return None;
            }
        }
        Shape2D::Capsule { half_height, radius } => {
            // Test top disk, bottom disk, and center slab
            let mut best: Option<(f32, Vec2)> = None;
            for cy in [-*half_height, *half_height] {
                if let Some(hit) = raycast_circle(local_origin - Vec2::new(0.0, cy), local_dir, *radius, max_toi) {
                    if best.map_or(true, |b| hit.0 < b.0) {
                        best = Some(hit);
                    }
                }
            }
            if let Some(hit) = raycast_aabb(
                local_origin,
                local_dir,
                Vec2::new(-*radius, -*half_height),
                Vec2::new(*radius, *half_height),
                max_toi,
            ) {
                if best.map_or(true, |b| hit.0 < b.0) {
                    best = Some(hit);
                }
            }
            best?
        }
        Shape2D::Segment { a, b } => {
            let v1 = local_origin - *a;
            let v2 = *b - *a;
            let v3 = Vec2::new(-local_dir.y, local_dir.x);
            let dot = v2.dot(v3);
            if dot.abs() < 1e-8 {
                return None;
            }
            let t1 = v2.cross(v1) / dot;
            let t2 = v1.dot(v3) / dot;
            if t1 >= 0.0 && t1 <= max_toi && (0.0..=1.0).contains(&t2) {
                let mut n = v2.perp().normalize_or_zero();
                if n.dot(local_dir) > 0.0 {
                    n = -n;
                }
                (t1, n)
            } else {
                return None;
            }
        }
    };

    Some((toi, transform.transform_vector(local_normal).normalize_or_zero()))
}

fn raycast_circle(origin: Vec2, dir: Vec2, radius: f32, max_toi: f32) -> Option<(f32, Vec2)> {
    let b = origin.dot(dir);
    let c = origin.length_squared() - radius * radius;
    if c <= 0.0 {
        return None;
    }
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t >= 0.0 && t <= max_toi {
        let p = origin + dir * t;
        Some((t, p.normalize_or_zero()))
    } else {
        None
    }
}

fn raycast_aabb(origin: Vec2, dir: Vec2, min: Vec2, max: Vec2, max_toi: f32) -> Option<(f32, Vec2)> {
    let mut tmin = 0.0_f32;
    let mut tmax = max_toi;
    let mut normal = Vec2::ZERO;

    // X slab
    if dir.x.abs() < 1e-8 {
        if origin.x < min.x || origin.x > max.x {
            return None;
        }
    } else {
        let inv = 1.0 / dir.x;
        let mut t1 = (min.x - origin.x) * inv;
        let mut t2 = (max.x - origin.x) * inv;
        let mut nx = -1.0_f32;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            nx = 1.0;
        }
        if t1 > tmin {
            tmin = t1;
            normal = Vec2::new(nx, 0.0);
        }
        tmax = tmax.min(t2);
        if tmin > tmax {
            return None;
        }
    }

    // Y slab
    if dir.y.abs() < 1e-8 {
        if origin.y < min.y || origin.y > max.y {
            return None;
        }
    } else {
        let inv = 1.0 / dir.y;
        let mut t1 = (min.y - origin.y) * inv;
        let mut t2 = (max.y - origin.y) * inv;
        let mut ny = -1.0_f32;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            ny = 1.0;
        }
        if t1 > tmin {
            tmin = t1;
            normal = Vec2::new(0.0, ny);
        }
        tmax = tmax.min(t2);
        if tmin > tmax {
            return None;
        }
    }

    if tmin >= 0.0 && tmin <= max_toi && normal.length_squared() > 0.5 {
        Some((tmin, normal))
    } else {
        None
    }
}

/// Filter predicate helper for queries.
#[inline]
pub fn filter_matches(query: CollisionFilter, body: CollisionFilter) -> bool {
    query.should_collide(body)
}
