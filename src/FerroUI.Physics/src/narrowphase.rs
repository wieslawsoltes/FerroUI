//! Narrowphase collision detection: analytic primitives, SAT + Sutherland-Hodgman 2-point
//! manifold clipping, GJK/EPA general convex fallback, and swept conservative-advancement CCD.

use crate::body::BodyHandle;
use crate::math::{Transform2D, Vec2};
use crate::shapes::{ConvexPolygon, Shape2D};
use smallvec::SmallVec;

/// Maximum contact points per 2D manifold (a face-face contact has up to 2 points).
pub const MAX_MANIFOLD_POINTS: usize = 2;

/// Persistent geometric feature identifier used to match contact points across frames for warm-starting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FeatureId {
    pub reference_edge: u8,
    pub incident_edge: u8,
    pub vertex: u8,
    pub flipped: u8,
}

/// A single contact point in a [`ContactManifold`].
#[derive(Clone, Copy, Debug)]
pub struct ContactPoint {
    /// World-space contact location.
    pub position: Vec2,
    /// Penetration depth (`> 0` when overlapping).
    pub depth: f32,
    /// Feature ID for warm-start impulse caching.
    pub feature_id: FeatureId,
    /// Warm-started / accumulated normal impulse.
    pub normal_impulse: f32,
    /// Warm-started / accumulated tangent (friction) impulse.
    pub tangent_impulse: f32,
}

/// Complete contact manifold between two bodies `body_a` and `body_b`.
///
/// The contact `normal` always points **from `body_a` toward `body_b`**.
#[derive(Clone, Debug)]
pub struct ContactManifold {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub normal: Vec2,
    pub points: SmallVec<[ContactPoint; MAX_MANIFOLD_POINTS]>,
    pub friction: f32,
    pub restitution: f32,
    pub is_sensor: bool,
}

impl ContactManifold {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

/// Computes a contact manifold between `shape_a` (at `tf_a`) and `shape_b` (at `tf_b`).
/// Returns `None` if separated.
pub fn collide_shapes(
    body_a: BodyHandle,
    shape_a: &Shape2D,
    tf_a: Transform2D,
    body_b: BodyHandle,
    shape_b: &Shape2D,
    tf_b: Transform2D,
) -> Option<ContactManifold> {
    let mut manifold = match (shape_a, shape_b) {
        (Shape2D::Circle { radius: ra }, Shape2D::Circle { radius: rb }) => {
            collide_circle_circle(tf_a.position, *ra, tf_b.position, *rb)
        }
        (Shape2D::Circle { radius }, Shape2D::Box { half_extents }) => {
            collide_circle_box(tf_a.position, *radius, *half_extents, tf_b)
        }
        (Shape2D::Box { half_extents }, Shape2D::Circle { radius }) => {
            collide_circle_box(tf_b.position, *radius, *half_extents, tf_a).map(flip_manifold)
        }
        (Shape2D::Circle { radius }, Shape2D::Polygon(poly)) => {
            collide_circle_polygon(tf_a.position, *radius, poly, tf_b)
        }
        (Shape2D::Polygon(poly), Shape2D::Circle { radius }) => {
            collide_circle_polygon(tf_b.position, *radius, poly, tf_a).map(flip_manifold)
        }
        (Shape2D::Circle { radius }, Shape2D::Segment { a, b }) => {
            collide_circle_segment(tf_a.position, *radius, tf_b.transform_point(*a), tf_b.transform_point(*b))
        }
        (Shape2D::Segment { a, b }, Shape2D::Circle { radius }) => {
            collide_circle_segment(tf_b.position, *radius, tf_a.transform_point(*a), tf_a.transform_point(*b))
                .map(flip_manifold)
        }
        (Shape2D::Circle { radius: ra }, Shape2D::Capsule { half_height, radius: rb }) => {
            let p0 = tf_b.transform_point(Vec2::new(0.0, -*half_height));
            let p1 = tf_b.transform_point(Vec2::new(0.0, *half_height));
            let closest = closest_point_on_segment(tf_a.position, p0, p1);
            collide_circle_circle(tf_a.position, *ra, closest, *rb)
        }
        (Shape2D::Capsule { half_height, radius: ra }, Shape2D::Circle { radius: rb }) => {
            let p0 = tf_a.transform_point(Vec2::new(0.0, -*half_height));
            let p1 = tf_a.transform_point(Vec2::new(0.0, *half_height));
            let closest = closest_point_on_segment(tf_b.position, p0, p1);
            collide_circle_circle(closest, *ra, tf_b.position, *rb)
        }
        (Shape2D::Capsule { half_height: ha, radius: ra }, Shape2D::Capsule { half_height: hb, radius: rb }) => {
            let a0 = tf_a.transform_point(Vec2::new(0.0, -*ha));
            let a1 = tf_a.transform_point(Vec2::new(0.0, *ha));
            let b0 = tf_b.transform_point(Vec2::new(0.0, -*hb));
            let b1 = tf_b.transform_point(Vec2::new(0.0, *hb));
            let (ca, cb) = closest_points_between_segments(a0, a1, b0, b1);
            collide_circle_circle(ca, *ra, cb, *rb)
        }
        (Shape2D::Box { half_extents: ha }, Shape2D::Box { half_extents: hb }) => {
            let pa = ConvexPolygon::rectangle(ha.x, ha.y);
            let pb = ConvexPolygon::rectangle(hb.x, hb.y);
            collide_polygon_polygon(&pa, tf_a, &pb, tf_b)
        }
        (Shape2D::Box { half_extents }, Shape2D::Polygon(poly)) => {
            let pa = ConvexPolygon::rectangle(half_extents.x, half_extents.y);
            collide_polygon_polygon(&pa, tf_a, poly, tf_b)
        }
        (Shape2D::Polygon(poly), Shape2D::Box { half_extents }) => {
            let pb = ConvexPolygon::rectangle(half_extents.x, half_extents.y);
            collide_polygon_polygon(poly, tf_a, &pb, tf_b)
        }
        (Shape2D::Polygon(pa), Shape2D::Polygon(pb)) => collide_polygon_polygon(pa, tf_a, pb, tf_b),
        (Shape2D::Capsule { half_height, radius }, Shape2D::Box { half_extents }) => {
            collide_capsule_box(*half_height, *radius, tf_a, *half_extents, tf_b)
        }
        (Shape2D::Box { half_extents }, Shape2D::Capsule { half_height, radius }) => {
            collide_capsule_box(*half_height, *radius, tf_b, *half_extents, tf_a).map(flip_manifold)
        }
        (Shape2D::Capsule { half_height, radius }, Shape2D::Polygon(poly)) => {
            collide_capsule_polygon(*half_height, *radius, tf_a, poly, tf_b)
        }
        (Shape2D::Polygon(poly), Shape2D::Capsule { half_height, radius }) => {
            collide_capsule_polygon(*half_height, *radius, tf_b, poly, tf_a).map(flip_manifold)
        }
        (Shape2D::Segment { a, b }, other) => {
            // Treat segment as thin capsule with minimal thickness for robust contact generation
            let wa = tf_a.transform_point(*a);
            let wb = tf_a.transform_point(*b);
            collide_segment_general(wa, wb, other, tf_b)
        }
        (other, Shape2D::Segment { a, b }) => {
            let wa = tf_b.transform_point(*a);
            let wb = tf_b.transform_point(*b);
            collide_segment_general(wa, wb, other, tf_a).map(flip_manifold)
        }
    }?;

    manifold.body_a = body_a;
    manifold.body_b = body_b;
    Some(manifold)
}

fn flip_manifold(mut m: ContactManifold) -> ContactManifold {
    m.normal = -m.normal;
    for pt in &mut m.points {
        pt.feature_id.flipped ^= 1;
    }
    m
}

fn make_single_manifold(normal: Vec2, position: Vec2, depth: f32, feature_id: FeatureId) -> ContactManifold {
    let mut points = SmallVec::new();
    points.push(ContactPoint {
        position,
        depth,
        feature_id,
        normal_impulse: 0.0,
        tangent_impulse: 0.0,
    });
    ContactManifold {
        body_a: BodyHandle::INVALID,
        body_b: BodyHandle::INVALID,
        normal,
        points,
        friction: 0.0,
        restitution: 0.0,
        is_sensor: false,
    }
}

/// Circle vs Circle exact contact.
fn collide_circle_circle(pos_a: Vec2, ra: f32, pos_b: Vec2, rb: f32) -> Option<ContactManifold> {
    let delta = pos_b - pos_a;
    let dist_sq = delta.length_squared();
    let r_sum = ra + rb;
    if dist_sq >= r_sum * r_sum {
        return None;
    }
    let dist = dist_sq.sqrt();
    let (normal, depth, contact_pos) = if dist > 1e-6 {
        let n = delta * (1.0 / dist);
        let depth = r_sum - dist;
        let p = pos_a + n * (ra - 0.5 * depth);
        (n, depth, p)
    } else {
        (Vec2::Y, r_sum, pos_a)
    };
    Some(make_single_manifold(normal, contact_pos, depth, FeatureId::default()))
}

/// Circle (A) vs Oriented Box (B) exact closest-point contact.
fn collide_circle_box(
    circle_pos: Vec2,
    radius: f32,
    half_extents: Vec2,
    box_tf: Transform2D,
) -> Option<ContactManifold> {
    let local_center = box_tf.inv_transform_point(circle_pos);
    let clamped = local_center.clamp(-half_extents, half_extents);
    let diff = local_center - clamped;
    let dist_sq = diff.length_squared();

    if dist_sq > radius * radius {
        return None;
    }

    if dist_sq > 1e-8 {
        let dist = dist_sq.sqrt();
        // Normal from circle (A) toward box (B)
        let local_normal = -diff * (1.0 / dist);
        let world_normal = box_tf.transform_vector(local_normal);
        let world_contact = box_tf.transform_point(clamped);
        return Some(make_single_manifold(
            world_normal,
            world_contact,
            radius - dist,
            FeatureId { reference_edge: 1, ..Default::default() },
        ));
    }

    // Circle center is inside the box: push out along shallowest axis
    let dx = half_extents.x - local_center.x.abs();
    let dy = half_extents.y - local_center.y.abs();
    let (local_normal, depth, local_contact) = if dx < dy {
        let sign = if local_center.x >= 0.0 { 1.0 } else { -1.0 };
        // Outward from box is +sign X, so normal from circle -> box is -sign X
        (Vec2::new(-sign, 0.0), dx + radius, Vec2::new(sign * half_extents.x, local_center.y))
    } else {
        let sign = if local_center.y >= 0.0 { 1.0 } else { -1.0 };
        (Vec2::new(0.0, -sign), dy + radius, Vec2::new(local_center.x, sign * half_extents.y))
    };

    Some(make_single_manifold(
        box_tf.transform_vector(local_normal),
        box_tf.transform_point(local_contact),
        depth,
        FeatureId { reference_edge: 2, ..Default::default() },
    ))
}

/// Circle (A) vs ConvexPolygon (B) using Voronoi edge projection.
fn collide_circle_polygon(
    circle_pos: Vec2,
    radius: f32,
    poly: &ConvexPolygon,
    poly_tf: Transform2D,
) -> Option<ContactManifold> {
    let c = poly_tf.inv_transform_point(circle_pos);
    let n = poly.vertices.len();

    let mut max_sep = f32::NEG_INFINITY;
    let mut best_edge = 0usize;

    for i in 0..n {
        let sep = poly.normals[i].dot(c - poly.vertices[i]);
        if sep > radius {
            return None;
        }
        if sep > max_sep {
            max_sep = sep;
            best_edge = i;
        }
    }

    let v1 = poly.vertices[best_edge];
    let v2 = poly.vertices[(best_edge + 1) % n];

    // Inside polygon
    if max_sep < 1e-6 {
        let world_normal = -poly_tf.transform_vector(poly.normals[best_edge]);
        let world_pos = circle_pos + world_normal * radius;
        return Some(make_single_manifold(
            world_normal,
            world_pos,
            radius - max_sep,
            FeatureId { reference_edge: best_edge as u8, ..Default::default() },
        ));
    }

    // Voronoi region test on best edge
    let edge = v2 - v1;
    let u1 = (c - v1).dot(edge);
    let u2 = (c - v2).dot(-edge);

    let closest = if u1 <= 0.0 {
        v1
    } else if u2 <= 0.0 {
        v2
    } else {
        let t = (u1 / edge.length_squared().max(1e-12)).clamp(0.0, 1.0);
        v1 + edge * t
    };

    let diff = c - closest;
    let dist_sq = diff.length_squared();
    if dist_sq > radius * radius {
        return None;
    }

    let dist = dist_sq.sqrt();
    let local_normal = if dist > 1e-6 { -diff * (1.0 / dist) } else { -poly.normals[best_edge] };
    Some(make_single_manifold(
        poly_tf.transform_vector(local_normal),
        poly_tf.transform_point(closest),
        radius - dist,
        FeatureId { reference_edge: best_edge as u8, ..Default::default() },
    ))
}

fn collide_circle_segment(circle_pos: Vec2, radius: f32, a: Vec2, b: Vec2) -> Option<ContactManifold> {
    let closest = closest_point_on_segment(circle_pos, a, b);
    let diff = closest - circle_pos;
    let dist_sq = diff.length_squared();
    if dist_sq > radius * radius {
        return None;
    }
    let dist = dist_sq.sqrt();
    let normal = if dist > 1e-6 {
        diff * (1.0 / dist)
    } else {
        (b - a).perp().normalize_or_zero()
    };
    Some(make_single_manifold(normal, closest, radius - dist, FeatureId::default()))
}

/// SAT + Sutherland-Hodgman 2-point contact manifold clipping for Polygon vs Polygon (and Box vs Box).
fn collide_polygon_polygon(
    poly_a: &ConvexPolygon,
    tf_a: Transform2D,
    poly_b: &ConvexPolygon,
    tf_b: Transform2D,
) -> Option<ContactManifold> {
    let (sep_a, edge_a) = find_max_separation(poly_a, tf_a, poly_b, tf_b);
    if sep_a > 0.0 {
        return None;
    }
    let (sep_b, edge_b) = find_max_separation(poly_b, tf_b, poly_a, tf_a);
    if sep_b > 0.0 {
        return None;
    }

    // Small bias toward A to reduce axis flipping jitter on near-tied edges
    const RELATIVE_TOL: f32 = 0.98;
    const ABSOLUTE_TOL: f32 = 0.001;
    let flip = sep_b > RELATIVE_TOL * sep_a + ABSOLUTE_TOL;

    let (ref_poly, ref_tf, inc_poly, inc_tf, ref_edge) = if !flip {
        (poly_a, tf_a, poly_b, tf_b, edge_a)
    } else {
        (poly_b, tf_b, poly_a, tf_a, edge_b)
    };

    let ref_normal_world = ref_tf.transform_vector(ref_poly.normals[ref_edge]);
    let ref_v1 = ref_tf.transform_point(ref_poly.vertices[ref_edge]);
    let ref_v2 = ref_tf.transform_point(ref_poly.vertices[(ref_edge + 1) % ref_poly.vertices.len()]);

    // Find most anti-parallel incident face on inc_poly
    let ref_normal_in_inc = inc_tf.inv_transform_vector(ref_normal_world);
    let mut inc_edge = 0usize;
    let mut min_dot = f32::INFINITY;
    for i in 0..inc_poly.normals.len() {
        let d = ref_normal_in_inc.dot(inc_poly.normals[i]);
        if d < min_dot {
            min_dot = d;
            inc_edge = i;
        }
    }

    let inc_i1 = inc_edge;
    let inc_i2 = (inc_edge + 1) % inc_poly.vertices.len();
    let clip_in = [
        ClipVertex {
            pos: inc_tf.transform_point(inc_poly.vertices[inc_i1]),
            vertex: inc_i1 as u8,
        },
        ClipVertex {
            pos: inc_tf.transform_point(inc_poly.vertices[inc_i2]),
            vertex: inc_i2 as u8,
        },
    ];

    let tangent = (ref_v2 - ref_v1).normalize_or_zero();
    let neg_offset = -tangent.dot(ref_v1);
    let pos_offset = tangent.dot(ref_v2);

    let clip1 = clip_segment_to_line(&clip_in, -tangent, neg_offset)?;
    let clip2 = clip_segment_to_line(&clip1, tangent, pos_offset)?;

    let ref_plane_offset = ref_normal_world.dot(ref_v1);
    let mut points = SmallVec::new();
    for cv in clip2 {
        let separation = ref_normal_world.dot(cv.pos) - ref_plane_offset;
        if separation <= 1e-4 {
            let depth = (-separation).max(0.0);
            points.push(ContactPoint {
                position: cv.pos,
                depth,
                feature_id: FeatureId {
                    reference_edge: ref_edge as u8,
                    incident_edge: inc_edge as u8,
                    vertex: cv.vertex,
                    flipped: u8::from(flip),
                },
                normal_impulse: 0.0,
                tangent_impulse: 0.0,
            });
        }
    }

    if points.is_empty() {
        return None;
    }

    let normal = if flip { -ref_normal_world } else { ref_normal_world };
    Some(ContactManifold {
        body_a: BodyHandle::INVALID,
        body_b: BodyHandle::INVALID,
        normal,
        points,
        friction: 0.0,
        restitution: 0.0,
        is_sensor: false,
    })
}

fn find_max_separation(
    poly_a: &ConvexPolygon,
    tf_a: Transform2D,
    poly_b: &ConvexPolygon,
    tf_b: Transform2D,
) -> (f32, usize) {
    let mut max_sep = f32::NEG_INFINITY;
    let mut best_edge = 0usize;
    for i in 0..poly_a.vertices.len() {
        let n_world = tf_a.transform_vector(poly_a.normals[i]);
        let v_world = tf_a.transform_point(poly_a.vertices[i]);
        let support_b = tf_b.transform_point(poly_b.support(tf_b.inv_transform_vector(-n_world)));
        let sep = (support_b - v_world).dot(n_world);
        if sep > max_sep {
            max_sep = sep;
            best_edge = i;
        }
    }
    (max_sep, best_edge)
}

#[derive(Clone, Copy)]
struct ClipVertex {
    pos: Vec2,
    vertex: u8,
}

fn clip_segment_to_line(v_in: &[ClipVertex; 2], normal: Vec2, offset: f32) -> Option<[ClipVertex; 2]> {
    let d0 = normal.dot(v_in[0].pos) - offset;
    let d1 = normal.dot(v_in[1].pos) - offset;
    let mut out = [v_in[0]; 2];
    let mut count = 0usize;

    if d0 <= 0.0 {
        out[count] = v_in[0];
        count += 1;
    }
    if d1 <= 0.0 {
        out[count] = v_in[1];
        count += 1;
    }
    if d0 * d1 < 0.0 && count < 2 {
        let alpha = d0 / (d0 - d1);
        out[count] = ClipVertex {
            pos: v_in[0].pos + (v_in[1].pos - v_in[0].pos) * alpha,
            vertex: if d0 > 0.0 { v_in[0].vertex } else { v_in[1].vertex },
        };
        count += 1;
    }
    if count < 2 { None } else { Some(out) }
}

/// Capsule (A) vs Box (B) by sampling endpoints and closest segment-to-box point.
fn collide_capsule_box(
    half_height: f32,
    radius: f32,
    cap_tf: Transform2D,
    half_extents: Vec2,
    box_tf: Transform2D,
) -> Option<ContactManifold> {
    let p0 = cap_tf.transform_point(Vec2::new(0.0, -half_height));
    let p1 = cap_tf.transform_point(Vec2::new(0.0, half_height));
    let m0 = collide_circle_box(p0, radius, half_extents, box_tf);
    let m1 = collide_circle_box(p1, radius, half_extents, box_tf);

    match (m0, m1) {
        (Some(mut a), Some(b)) => {
            if a.normal.dot(b.normal) > 0.85 {
                let mut pt_b = b.points[0];
                pt_b.feature_id.vertex = 1;
                a.points.push(pt_b);
                Some(a)
            } else if a.points[0].depth >= b.points[0].depth {
                Some(a)
            } else {
                Some(b)
            }
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => {
            // Check midpoint & closest point to box center in case box edge cuts capsule middle
            let mid = closest_point_on_segment(box_tf.position, p0, p1);
            collide_circle_box(mid, radius, half_extents, box_tf)
                .or_else(|| gjk_epa_fallback(&Shape2D::Capsule { half_height, radius }, cap_tf, &Shape2D::Box { half_extents }, box_tf))
        }
    }
}

fn collide_capsule_polygon(
    half_height: f32,
    radius: f32,
    cap_tf: Transform2D,
    poly: &ConvexPolygon,
    poly_tf: Transform2D,
) -> Option<ContactManifold> {
    let p0 = cap_tf.transform_point(Vec2::new(0.0, -half_height));
    let p1 = cap_tf.transform_point(Vec2::new(0.0, half_height));
    let m0 = collide_circle_polygon(p0, radius, poly, poly_tf);
    let m1 = collide_circle_polygon(p1, radius, poly, poly_tf);
    match (m0, m1) {
        (Some(mut a), Some(b)) => {
            if a.normal.dot(b.normal) > 0.85 {
                let mut pt_b = b.points[0];
                pt_b.feature_id.vertex = 1;
                a.points.push(pt_b);
                Some(a)
            } else if a.points[0].depth >= b.points[0].depth {
                Some(a)
            } else {
                Some(b)
            }
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => gjk_epa_fallback(
            &Shape2D::Capsule { half_height, radius },
            cap_tf,
            &Shape2D::Polygon(poly.clone()),
            poly_tf,
        ),
    }
}

fn collide_segment_general(
    wa: Vec2,
    wb: Vec2,
    other: &Shape2D,
    other_tf: Transform2D,
) -> Option<ContactManifold> {
    let mid = (wa + wb) * 0.5;
    let dir = wb - wa;
    let len = dir.length();
    if len < 1e-6 {
        return None;
    }
    let angle = dir.y.atan2(dir.x) - std::f32::consts::FRAC_PI_2;
    let seg_tf = Transform2D::new(mid, angle);
    let thin_capsule = Shape2D::Capsule { half_height: len * 0.5, radius: 0.015 };
    match other {
        Shape2D::Box { half_extents } => {
            collide_capsule_box(len * 0.5, 0.015, seg_tf, *half_extents, other_tf)
        }
        Shape2D::Polygon(poly) => collide_capsule_polygon(len * 0.5, 0.015, seg_tf, poly, other_tf),
        Shape2D::Capsule { half_height, radius } => {
            let b0 = other_tf.transform_point(Vec2::new(0.0, -*half_height));
            let b1 = other_tf.transform_point(Vec2::new(0.0, *half_height));
            let (ca, cb) = closest_points_between_segments(wa, wb, b0, b1);
            collide_circle_circle(ca, 0.015, cb, *radius)
        }
        _ => gjk_epa_fallback(&thin_capsule, seg_tf, other, other_tf),
    }
}

/// General convex GJK intersection + EPA penetration depth & normal fallback.
pub fn gjk_epa_fallback(
    shape_a: &Shape2D,
    tf_a: Transform2D,
    shape_b: &Shape2D,
    tf_b: Transform2D,
) -> Option<ContactManifold> {
    let support = |dir: Vec2| -> (Vec2, Vec2, Vec2) {
        let sa = shape_a.support_world(tf_a, dir);
        let sb = shape_b.support_world(tf_b, -dir);
        (sa - sb, sa, sb)
    };

    let mut dir = tf_b.position - tf_a.position;
    if dir.length_squared() < 1e-8 {
        dir = Vec2::X;
    }

    let mut simplex: SmallVec<[(Vec2, Vec2, Vec2); 4]> = SmallVec::new();
    let p0 = support(dir);
    simplex.push(p0);
    dir = -p0.0;

    let mut hit = false;
    for _ in 0..24 {
        if dir.length_squared() < 1e-10 {
            hit = true;
            break;
        }
        let a = support(dir);
        if a.0.dot(dir) < 0.0 {
            return None;
        }
        simplex.push(a);
        if update_gjk_simplex(&mut simplex, &mut dir) {
            hit = true;
            break;
        }
    }

    if !hit || simplex.len() < 3 {
        return None;
    }

    // Ensure CCW winding for EPA polytope
    let mut poly: Vec<(Vec2, Vec2, Vec2)> = simplex.into_iter().collect();
    let cross = (poly[1].0 - poly[0].0).cross(poly[2].0 - poly[0].0);
    if cross < 0.0 {
        poly.swap(1, 2);
    }

    for _ in 0..24 {
        let mut best_dist = f32::INFINITY;
        let mut best_normal = Vec2::Y;
        let mut best_idx = 0usize;

        for i in 0..poly.len() {
            let a = poly[i].0;
            let b = poly[(i + 1) % poly.len()].0;
            let e = b - a;
            let n = e.perp_cw().normalize_or_zero();
            let d = n.dot(a);
            if d < best_dist {
                best_dist = d;
                best_normal = n;
                best_idx = i;
            }
        }

        let sup = support(best_normal);
        let s_dist = sup.0.dot(best_normal);
        if (s_dist - best_dist) < 1e-4 || poly.len() >= 32 {
            let a = poly[best_idx];
            let b = poly[(best_idx + 1) % poly.len()];
            let contact_pos = (a.1 + b.1 + a.2 + b.2) * 0.25;
            return Some(make_single_manifold(
                best_normal,
                contact_pos,
                best_dist.max(0.0),
                FeatureId::default(),
            ));
        }
        poly.insert(best_idx + 1, sup);
    }

    None
}

fn update_gjk_simplex(simplex: &mut SmallVec<[(Vec2, Vec2, Vec2); 4]>, dir: &mut Vec2) -> bool {
    if simplex.len() == 2 {
        let b = simplex[0].0;
        let a = simplex[1].0;
        let ab = b - a;
        let ao = -a;
        let perp = triple_product(ab, ao, ab);
        *dir = if perp.length_squared() > 1e-10 { perp } else { ab.perp() };
        return false;
    }

    let c = simplex[0].0;
    let b = simplex[1].0;
    let a = simplex[2].0;
    let ab = b - a;
    let ac = c - a;
    let ao = -a;

    let ab_perp = triple_product(ac, ab, ab);
    let ac_perp = triple_product(ab, ac, ac);

    if ab_perp.dot(ao) > 0.0 {
        simplex.remove(0);
        *dir = ab_perp;
        false
    } else if ac_perp.dot(ao) > 0.0 {
        simplex.remove(1);
        *dir = ac_perp;
        false
    } else {
        true
    }
}

#[inline]
fn triple_product(a: Vec2, b: Vec2, c: Vec2) -> Vec2 {
    // (a x b) x c = b*(a.c) - a*(b.c)
    b * a.dot(c) - a * b.dot(c)
}

#[inline]
pub fn closest_point_on_segment(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-12 {
        return a;
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    a + ab * t
}

pub fn closest_points_between_segments(a0: Vec2, a1: Vec2, b0: Vec2, b1: Vec2) -> (Vec2, Vec2) {
    let d1 = a1 - a0;
    let d2 = b1 - b0;
    let r = a0 - b0;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);

    let (s, t) = if a <= 1e-10 && e <= 1e-10 {
        (0.0, 0.0)
    } else if a <= 1e-10 {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = d1.dot(r);
        if e <= 1e-10 {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s = if denom.abs() > 1e-10 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };

    (a0 + d1 * s, b0 + d2 * t)
}

/// Swept Continuous Collision Detection (CCD) via binary search along linear motion.
///
/// Returns `Some(toi)` in `(0.0, 1.0]` when a fast bullet moving from `tf_a0` to `tf_a1`
/// first impacts static/dynamic shape `shape_b` at `tf_b`.
pub fn swept_toi(
    shape_a: &Shape2D,
    tf_a0: Transform2D,
    tf_a1: Transform2D,
    shape_b: &Shape2D,
    tf_b: Transform2D,
) -> Option<f32> {
    let disp = tf_a1.position - tf_a0.position;
    let travel = disp.length();
    let r_a = shape_a.bounding_radius().max(1e-3);
    if travel < r_a * 0.5 {
        return None;
    }

    // Coarse sweep along travel path to catch thin walls
    let steps = ((travel / (r_a * 0.5)).ceil() as usize).clamp(2, 32);
    let mut hit_t = None;
    let mut prev_t = 0.0_f32;
    for step in 1..=steps {
        let t = step as f32 / steps as f32;
        let tf_t = tf_a0.interpolate(tf_a1, t);
        if collide_shapes(BodyHandle::INVALID, shape_a, tf_t, BodyHandle::INVALID, shape_b, tf_b).is_some() {
            hit_t = Some((prev_t, t));
            break;
        }
        prev_t = t;
    }

    let (mut lo, mut hi) = hit_t?;
    // Binary-search refine TOI to sub-millimeter precision
    for _ in 0..8 {
        let mid = 0.5 * (lo + hi);
        let tf_m = tf_a0.interpolate(tf_a1, mid);
        if collide_shapes(BodyHandle::INVALID, shape_a, tf_m, BodyHandle::INVALID, shape_b, tf_b).is_some() {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Some(lo)
}
