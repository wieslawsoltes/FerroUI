//! Extended Position-Based Dynamics (XPBD) for real-time tearable cloth meshes and
//! pressure-inflated deformable 2D soft bodies (jelly, cushions, membranes).

use crate::body::RigidBody;
use crate::math::{Aabb2, Vec2};
use crate::shapes::Shape2D;
use std::f32::consts::TAU;

/// A single simulated vertex/node in an XPBD cloth or soft body.
#[derive(Clone, Copy, Debug)]
pub struct SoftNode {
    pub pos: Vec2,
    pub prev_pos: Vec2,
    pub vel: Vec2,
    /// `0.0` = pinned/infinite mass.
    pub inv_mass: f32,
}

/// An XPBD distance constraint connecting two nodes with compliance and optional tear limit.
#[derive(Clone, Copy, Debug)]
pub struct DistanceConstraint {
    pub a: usize,
    pub b: usize,
    pub rest_length: f32,
    /// XPBD inverse stiffness (`0.0` = infinitely stiff).
    pub compliance: f32,
    /// Optional stretch ratio above which the link tears (e.g. `2.1` = tears at 210% rest length).
    pub tear_ratio: Option<f32>,
    pub broken: bool,
    /// Current stretch ratio (`length / rest_length`), useful for visual stress heatmaps.
    pub stretch_ratio: f32,
}

/// Tearable 2D XPBD cloth grid with structural, shear, and bending constraints, wind forces,
/// pointer dragging/slicing, and rigid-body collision response.
#[derive(Clone, Debug)]
pub struct ClothMesh {
    pub cols: usize,
    pub rows: usize,
    pub nodes: Vec<SoftNode>,
    pub constraints: Vec<DistanceConstraint>,
    pub damping: f32,
    pub substeps: u32,
    pub bounds: Aabb2,
    pub grab_node: Option<(usize, Vec2)>,
}

impl ClothMesh {
    /// Creates a rectangular hanging cloth grid with top-edge pins and structural + shear + bending links.
    pub fn grid(
        origin: Vec2,
        cols: usize,
        rows: usize,
        spacing: f32,
        compliance: f32,
        tear_ratio: Option<f32>,
        bounds: Aabb2,
    ) -> Self {
        let cols = cols.max(2);
        let rows = rows.max(2);
        let mut nodes = Vec::with_capacity(cols * rows);

        for r in 0..rows {
            for c in 0..cols {
                let p = origin + Vec2::new(c as f32 * spacing, r as f32 * spacing);
                // Pin top corners and evenly spaced top grommets
                let pinned = r == 0 && (c == 0 || c == cols - 1 || c % 3 == 0);
                nodes.push(SoftNode {
                    pos: p,
                    prev_pos: p,
                    vel: Vec2::ZERO,
                    inv_mass: if pinned { 0.0 } else { 1.0 },
                });
            }
        }

        let mut constraints = Vec::new();
        let idx = |c: usize, r: usize| r * cols + c;

        for r in 0..rows {
            for c in 0..cols {
                let i = idx(c, r);
                // Structural horizontal & vertical
                if c + 1 < cols {
                    constraints.push(DistanceConstraint {
                        a: i,
                        b: idx(c + 1, r),
                        rest_length: spacing,
                        compliance,
                        tear_ratio,
                        broken: false,
                        stretch_ratio: 1.0,
                    });
                }
                if r + 1 < rows {
                    constraints.push(DistanceConstraint {
                        a: i,
                        b: idx(c, r + 1),
                        rest_length: spacing,
                        compliance,
                        tear_ratio,
                        broken: false,
                        stretch_ratio: 1.0,
                    });
                }
                // Diagonal shear constraints (softer)
                if c + 1 < cols && r + 1 < rows {
                    let diag = spacing * std::f32::consts::SQRT_2;
                    constraints.push(DistanceConstraint {
                        a: i,
                        b: idx(c + 1, r + 1),
                        rest_length: diag,
                        compliance: compliance * 4.0 + 0.0005,
                        tear_ratio: tear_ratio.map(|t| t * 1.15),
                        broken: false,
                        stretch_ratio: 1.0,
                    });
                    constraints.push(DistanceConstraint {
                        a: idx(c + 1, r),
                        b: idx(c, r + 1),
                        rest_length: diag,
                        compliance: compliance * 4.0 + 0.0005,
                        tear_ratio: tear_ratio.map(|t| t * 1.15),
                        broken: false,
                        stretch_ratio: 1.0,
                    });
                }
                // 2-hop bending constraints (soft resistance to creasing)
                if c + 2 < cols {
                    constraints.push(DistanceConstraint {
                        a: i,
                        b: idx(c + 2, r),
                        rest_length: spacing * 2.0,
                        compliance: compliance * 12.0 + 0.004,
                        tear_ratio: None,
                        broken: false,
                        stretch_ratio: 1.0,
                    });
                }
                if r + 2 < rows {
                    constraints.push(DistanceConstraint {
                        a: i,
                        b: idx(c, r + 2),
                        rest_length: spacing * 2.0,
                        compliance: compliance * 12.0 + 0.004,
                        tear_ratio: None,
                        broken: false,
                        stretch_ratio: 1.0,
                    });
                }
            }
        }

        Self {
            cols,
            rows,
            nodes,
            constraints,
            damping: 0.994,
            substeps: 6,
            bounds,
            grab_node: None,
        }
    }

    /// Finds the nearest dynamic node within `max_radius` to grab interactively.
    pub fn pick_node(&self, world_pos: Vec2, max_radius: f32) -> Option<usize> {
        let r2 = max_radius * max_radius;
        let mut best = None;
        let mut best_d2 = r2;
        for (i, n) in self.nodes.iter().enumerate() {
            if n.inv_mass <= 0.0 {
                continue;
            }
            let d2 = n.pos.distance_squared(world_pos);
            if d2 < best_d2 {
                best_d2 = d2;
                best = Some(i);
            }
        }
        best
    }

    /// Tears all cloth links intersecting a circular brush around `world_pos`.
    pub fn tear_at(&mut self, world_pos: Vec2, radius: f32) -> usize {
        let r2 = radius * radius;
        let mut torn = 0;
        for c in &mut self.constraints {
            if c.broken {
                continue;
            }
            let mid = (self.nodes[c.a].pos + self.nodes[c.b].pos) * 0.5;
            if mid.distance_squared(world_pos) <= r2 {
                c.broken = true;
                torn += 1;
            }
        }
        torn
    }

    /// Advances the XPBD cloth simulation with gravity, dynamic wind, and rigid-body collisions.
    pub fn step(&mut self, dt: f32, gravity: Vec2, wind: Vec2, time: f32, bodies: &[&RigidBody]) {
        if dt <= 0.0 || self.nodes.is_empty() {
            return;
        }
        let substeps = self.substeps.max(1);
        let h = dt / substeps as f32;
        let damp = self.damping.powf(1.0 / substeps as f32);

        for _ in 0..substeps {
            // 1. Predict positions
            for (i, node) in self.nodes.iter_mut().enumerate() {
                node.prev_pos = node.pos;
                if node.inv_mass <= 0.0 {
                    continue;
                }
                let gust = 1.0 + 0.45 * (time * 3.1 + node.pos.y * 1.4 + i as f32 * 0.13).sin();
                let accel = gravity + wind * gust;
                node.vel = (node.vel + accel * h) * damp;
                node.pos += node.vel * h;
            }

            if let Some((idx, target)) = self.grab_node {
                if let Some(n) = self.nodes.get_mut(idx) {
                    n.pos = n.pos.lerp(target, 0.55);
                }
            }

            // 2. Solve XPBD distance constraints
            for c in &mut self.constraints {
                if c.broken {
                    continue;
                }
                let w1 = self.nodes[c.a].inv_mass;
                let w2 = self.nodes[c.b].inv_mass;
                let w_sum = w1 + w2;
                if w_sum <= 0.0 {
                    continue;
                }
                let delta = self.nodes[c.b].pos - self.nodes[c.a].pos;
                let len = delta.length();
                if len < 1e-6 {
                    continue;
                }
                c.stretch_ratio = len / c.rest_length.max(1e-4);
                if let Some(limit) = c.tear_ratio {
                    if c.stretch_ratio > limit {
                        c.broken = true;
                        continue;
                    }
                }
                let alpha_tilde = c.compliance / (h * h);
                let err = len - c.rest_length;
                let lambda = -err / (w_sum + alpha_tilde);
                let corr = delta * (lambda / len);
                self.nodes[c.a].pos -= corr * w1;
                self.nodes[c.b].pos += corr * w2;
            }

            // 3. Collide with rigid bodies & arena bounds
            for node in &mut self.nodes {
                if node.inv_mass <= 0.0 {
                    continue;
                }
                collide_soft_node_with_bodies(&mut node.pos, bodies, 0.04);
                node.pos = node.pos.clamp(self.bounds.min, self.bounds.max);
                node.vel = (node.pos - node.prev_pos) / h;
            }
        }
    }

    pub fn active_constraint_count(&self) -> usize {
        self.constraints.iter().filter(|c| !c.broken).count()
    }
}

/// Pressure-inflated 2D deformable XPBD soft body (jelly blob / cushion).
///
/// Combines perimeter/cross distance constraints with a global 2D signed-area preservation
/// constraint `C(p) = Area(p) - pressure * rest_area`, giving realistic squishing and bounce.
#[derive(Clone, Debug)]
pub struct SoftBody2D {
    pub nodes: Vec<SoftNode>,
    pub constraints: Vec<DistanceConstraint>,
    pub rest_area: f32,
    /// Target internal pressure multiplier (`1.0` = neutral, `1.2` = firm inflated ball).
    pub pressure: f32,
    pub area_compliance: f32,
    pub damping: f32,
    pub substeps: u32,
    pub bounds: Aabb2,
    pub color_argb: u32,
}

impl SoftBody2D {
    /// Creates a circular ring jelly soft body with `segments` perimeter nodes and cross-bracing.
    pub fn circle_blob(
        center: Vec2,
        radius: f32,
        segments: usize,
        pressure: f32,
        edge_compliance: f32,
        bounds: Aabb2,
        color_argb: u32,
    ) -> Self {
        let n = segments.clamp(8, 32);
        let mut nodes = Vec::with_capacity(n);
        for i in 0..n {
            let ang = (i as f32 / n as f32) * TAU;
            let p = center + Vec2::new(ang.cos() * radius, ang.sin() * radius);
            nodes.push(SoftNode { pos: p, prev_pos: p, vel: Vec2::ZERO, inv_mass: 1.0 });
        }

        let mut constraints = Vec::new();
        for i in 0..n {
            let next = (i + 1) % n;
            let len = nodes[i].pos.distance(nodes[next].pos);
            constraints.push(DistanceConstraint {
                a: i,
                b: next,
                rest_length: len,
                compliance: edge_compliance,
                tear_ratio: None,
                broken: false,
                stretch_ratio: 1.0,
            });
            // 2-step bending brace for smooth curvature
            let next2 = (i + 2) % n;
            let len2 = nodes[i].pos.distance(nodes[next2].pos);
            constraints.push(DistanceConstraint {
                a: i,
                b: next2,
                rest_length: len2,
                compliance: edge_compliance * 6.0 + 0.002,
                tear_ratio: None,
                broken: false,
                stretch_ratio: 1.0,
            });
        }

        let rest_area = signed_polygon_area(&nodes).abs().max(1e-4);
        Self {
            nodes,
            constraints,
            rest_area,
            pressure: pressure.max(0.2),
            area_compliance: 0.0005,
            damping: 0.992,
            substeps: 6,
            bounds,
            color_argb,
        }
    }

    pub fn center(&self) -> Vec2 {
        if self.nodes.is_empty() {
            return Vec2::ZERO;
        }
        let sum = self.nodes.iter().fold(Vec2::ZERO, |acc, n| acc + n.pos);
        sum / self.nodes.len() as f32
    }

    pub fn current_area(&self) -> f32 {
        signed_polygon_area(&self.nodes).abs()
    }

    pub fn apply_impulse(&mut self, impulse: Vec2) {
        for n in &mut self.nodes {
            if n.inv_mass > 0.0 {
                n.vel += impulse;
            }
        }
    }

    pub fn step(&mut self, dt: f32, gravity: Vec2, bodies: &[&RigidBody]) {
        if dt <= 0.0 || self.nodes.len() < 3 {
            return;
        }
        let substeps = self.substeps.max(1);
        let h = dt / substeps as f32;
        let damp = self.damping.powf(1.0 / substeps as f32);
        let n_len = self.nodes.len();
        let mut grads = vec![Vec2::ZERO; n_len];

        for _ in 0..substeps {
            // 1. Predict
            for node in &mut self.nodes {
                node.prev_pos = node.pos;
                if node.inv_mass > 0.0 {
                    node.vel = (node.vel + gravity * h) * damp;
                    node.pos += node.vel * h;
                }
            }

            // 2. Edge & bending distance constraints
            for c in &mut self.constraints {
                if c.broken {
                    continue;
                }
                let w1 = self.nodes[c.a].inv_mass;
                let w2 = self.nodes[c.b].inv_mass;
                let w_sum = w1 + w2;
                if w_sum <= 0.0 {
                    continue;
                }
                let delta = self.nodes[c.b].pos - self.nodes[c.a].pos;
                let len = delta.length();
                if len < 1e-6 {
                    continue;
                }
                let alpha = c.compliance / (h * h);
                let lambda = -(len - c.rest_length) / (w_sum + alpha);
                let corr = delta * (lambda / len);
                self.nodes[c.a].pos -= corr * w1;
                self.nodes[c.b].pos += corr * w2;
            }

            // 3. Global 2D Area / Pressure conservation constraint:
            //    C(p) = 0.5 * sum(p_i x p_{i+1}) - target_area
            //    grad_{p_i} C = 0.5 * (p_{i+1} - p_{i-1}).perp_cw()
            let cur_area = signed_polygon_area(&self.nodes);
            let target_area = self.rest_area * self.pressure;
            let c_area = cur_area - target_area;
            let mut denom = self.area_compliance / (h * h);
            for i in 0..n_len {
                let prev = self.nodes[(i + n_len - 1) % n_len].pos;
                let next = self.nodes[(i + 1) % n_len].pos;
                // Derivative of 0.5 * (prev x cur + cur x next) wrt cur is 0.5 * (next.y - prev.y, prev.x - next.x)
                let g = Vec2::new(next.y - prev.y, prev.x - next.x) * 0.5;
                grads[i] = g;
                denom += self.nodes[i].inv_mass * g.length_squared();
            }
            if denom > 1e-6 {
                let lambda = -c_area / denom;
                for i in 0..n_len {
                    let w = self.nodes[i].inv_mass;
                    if w > 0.0 {
                        self.nodes[i].pos += grads[i] * (lambda * w);
                    }
                }
            }

            // 4. Collide with rigid bodies & bounds + velocity recovery
            for node in &mut self.nodes {
                if node.inv_mass <= 0.0 {
                    continue;
                }
                collide_soft_node_with_bodies(&mut node.pos, bodies, 0.05);
                let clamped = node.pos.clamp(self.bounds.min, self.bounds.max);
                let hit_floor = (clamped.y - node.pos.y).abs() > 1e-5;
                node.pos = clamped;
                node.vel = (node.pos - node.prev_pos) / h;
                if hit_floor {
                    node.vel.x *= 0.92; // ground friction
                }
            }
        }
    }
}

fn signed_polygon_area(nodes: &[SoftNode]) -> f32 {
    let n = nodes.len();
    let mut sum = 0.0_f32;
    for i in 0..n {
        sum += nodes[i].pos.cross(nodes[(i + 1) % n].pos);
    }
    sum * 0.5
}

fn collide_soft_node_with_bodies(pos: &mut Vec2, bodies: &[&RigidBody], margin: f32) {
    for &body in bodies {
        if body.is_sensor {
            continue;
        }
        let tf = body.transform();
        match &body.shape {
            Shape2D::Circle { radius } => {
                let r = *radius + margin;
                let d = *pos - body.position;
                let d2 = d.length_squared();
                if d2 < r * r && d2 > 1e-8 {
                    let dist = d2.sqrt();
                    *pos = body.position + d * (r / dist);
                }
            }
            Shape2D::Box { half_extents } => {
                let local = tf.inv_transform_point(*pos);
                let ext = *half_extents + Vec2::splat(margin);
                if local.x.abs() < ext.x && local.y.abs() < ext.y {
                    let dx = ext.x - local.x.abs();
                    let dy = ext.y - local.y.abs();
                    let pushed = if dx < dy {
                        Vec2::new(local.x.signum() * ext.x, local.y)
                    } else {
                        Vec2::new(local.x, local.y.signum() * ext.y)
                    };
                    *pos = tf.transform_point(pushed);
                }
            }
            Shape2D::Capsule { half_height, radius } => {
                let local = tf.inv_transform_point(*pos);
                let clamped = Vec2::new(0.0, local.y.clamp(-*half_height, *half_height));
                let d = local - clamped;
                let r = *radius + margin;
                let d2 = d.length_squared();
                if d2 < r * r && d2 > 1e-8 {
                    let pushed = clamped + d * (r / d2.sqrt());
                    *pos = tf.transform_point(pushed);
                }
            }
            Shape2D::Polygon(poly) => {
                let local = tf.inv_transform_point(*pos);
                let mut best_sep = f32::NEG_INFINITY;
                let mut best_n = Vec2::Y;
                let mut inside = true;
                for i in 0..poly.vertices.len() {
                    let sep = poly.normals[i].dot(local - poly.vertices[i]) - margin;
                    if sep > 0.0 {
                        inside = false;
                        break;
                    }
                    if sep > best_sep {
                        best_sep = sep;
                        best_n = poly.normals[i];
                    }
                }
                if inside {
                    *pos = tf.transform_point(local - best_n * best_sep);
                }
            }
            Shape2D::Segment { .. } => {}
        }
    }
}
