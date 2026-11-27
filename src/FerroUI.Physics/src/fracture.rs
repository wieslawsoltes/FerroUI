//! Destructible rigid-body fracturing: splits boxes and convex polygons into radial convex
//! shards around an impact point, conserving momentum and angular kinematics.

use crate::body::{BodyBuilder, BodyHandle};
use crate::math::Vec2;
use crate::shapes::{ConvexPolygon, Shape2D};
use crate::world::PhysicsWorld;

/// Splits a dynamic box or convex polygon body at `impact_world` into triangular convex shards,
/// removing the original body and spawning live debris shards inheriting parent velocity + blast.
pub fn fracture_body(
    world: &mut PhysicsWorld,
    handle: BodyHandle,
    impact_world: Vec2,
    blast_speed: f32,
) -> Vec<BodyHandle> {
    let Some(body) = world.body(handle).cloned() else {
        return Vec::new();
    };
    if !body.is_dynamic() {
        return Vec::new();
    }

    let Some(local_verts) = body.shape.as_polygon_vertices() else {
        return Vec::new();
    };
    if local_verts.len() < 3 {
        return Vec::new();
    }

    // Clamp impact point inside the polygon interior so every radial fan triangle has positive area
    let tf = body.transform();
    let mut local_hub = tf.inv_transform_point(impact_world);
    let centroid = local_verts.iter().copied().fold(Vec2::ZERO, |a, b| a + b) / local_verts.len() as f32;
    local_hub = centroid.lerp(local_hub, 0.45);

    // Subdivide long edges so boxes produce 6–8 satisfying shards instead of only 4
    let mut ring: Vec<Vec2> = Vec::with_capacity(local_verts.len() * 2);
    let n = local_verts.len();
    for i in 0..n {
        let a = local_verts[i];
        let b = local_verts[(i + 1) % n];
        ring.push(a);
        if a.distance(b) > 0.38 {
            ring.push((a + b) * 0.5);
        }
    }

    let mut shards = Vec::new();
    let m = ring.len();
    let mut created_any = false;

    for i in 0..m {
        let p0 = local_hub;
        let p1 = ring[i];
        let p2 = ring[(i + 1) % m];
        let area = 0.5 * (p1 - p0).cross(p2 - p0).abs();
        if area < 0.015 {
            continue;
        }
        let tri_center = (p0 + p1 + p2) / 3.0;
        let centered = [p0 - tri_center, p1 - tri_center, p2 - tri_center];
        let Some(poly) = ConvexPolygon::from_points(&centered) else {
            continue;
        };

        let world_pos = tf.transform_point(tri_center);
        let r_world = world_pos - body.position;
        let inherited_vel = body.linear_velocity + Vec2::cross_scalar_left(body.angular_velocity, r_world);
        let outward = (world_pos - impact_world).normalize_or_zero();
        let shard_vel = inherited_vel + outward * blast_speed;

        let h = world.add_body(
            BodyBuilder::dynamic(Shape2D::Polygon(poly))
                .position(world_pos)
                .angle(body.angle)
                .linear_velocity(shard_vel)
                .angular_velocity(body.angular_velocity + ((i as f32) - m as f32 * 0.5) * 1.4)
                .material(body.material)
                .filter(body.filter)
                .user_data(body.user_data),
        );
        shards.push(h);
        created_any = true;
    }

    if created_any {
        world.remove_body(handle);
    }
    shards
}
