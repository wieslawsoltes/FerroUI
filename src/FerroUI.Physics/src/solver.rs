//! Substepped Sequential Impulse / Soft-Step contact & constraint solver with warm-starting,
//! split/Baumgarte positional stabilization, sleeping, and fixed-timestep accumulator.

use crate::body::{BodyHandle, RigidBody};
use crate::joints::{solve_joint_velocity, Joint};
use crate::math::Vec2;
use crate::narrowphase::{ContactManifold, FeatureId};
use rustc_hash::FxHashMap;

/// Configuration for the rigid-body solver.
#[derive(Clone, Copy, Debug)]
pub struct SolverConfig {
    /// Substeps per `step(dt)` call (Box2D v3 / XPBD style substepping dramatically improves stack & joint stability).
    pub substeps: u32,
    /// Velocity constraint iterations per substep.
    pub velocity_iterations: u32,
    /// Positional penetration correction iterations per substep.
    pub position_iterations: u32,
    /// Allowed overlap slop before positional stabilization activates (meters).
    pub penetration_slop: f32,
    /// Baumgarte positional correction stiffness factor (`0.15..=0.4`).
    pub baumgarte: f32,
    /// Minimum relative closing speed (m/s) required to trigger elastic bounce (prevents micro-bounce jitter).
    pub restitution_threshold: f32,
    /// Fraction of previous frame's impulse reused to warm-start contacts (`0.8..=1.0`).
    pub warm_start_factor: f32,
    /// Linear speed below which a body accumulates sleep time.
    pub sleep_linear_threshold: f32,
    /// Angular speed (rad/s) below which a body accumulates sleep time.
    pub sleep_angular_threshold: f32,
    /// Seconds of stillness before a body sleeps.
    pub sleep_time_threshold: f32,
    /// Enables swept Continuous Collision Detection for `is_bullet` bodies.
    pub enable_ccd: bool,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self::game_default()
    }
}

impl SolverConfig {
    /// Balanced high-stability preset for 2D games (60Hz/120Hz).
    pub const fn game_default() -> Self {
        Self {
            substeps: 4,
            velocity_iterations: 6,
            position_iterations: 2,
            penetration_slop: 0.005,
            baumgarte: 0.25,
            restitution_threshold: 0.65,
            warm_start_factor: 0.85,
            sleep_linear_threshold: 0.06,
            sleep_angular_threshold: 0.08,
            sleep_time_threshold: 0.6,
            enable_ccd: true,
        }
    }

    /// Extra-stiff preset for tall stacks, ragdolls, and robotic chains.
    pub const fn high_precision() -> Self {
        Self {
            substeps: 8,
            velocity_iterations: 10,
            position_iterations: 4,
            penetration_slop: 0.002,
            baumgarte: 0.3,
            restitution_threshold: 0.5,
            warm_start_factor: 0.9,
            sleep_linear_threshold: 0.04,
            sleep_angular_threshold: 0.05,
            sleep_time_threshold: 0.75,
            enable_ccd: true,
        }
    }

    /// Ultra-low-latency preset tuned for responsive UI interactions.
    pub const fn ui_realtime() -> Self {
        Self {
            substeps: 2,
            velocity_iterations: 4,
            position_iterations: 2,
            penetration_slop: 0.01,
            baumgarte: 0.28,
            restitution_threshold: 0.8,
            warm_start_factor: 0.8,
            sleep_linear_threshold: 0.1,
            sleep_angular_threshold: 0.1,
            sleep_time_threshold: 0.4,
            enable_ccd: false,
        }
    }
}

/// Persistent warm-start impulse cache keyed by `(body_a, body_b, feature_id)`.
#[derive(Clone, Debug, Default)]
pub(crate) struct WarmStartCache {
    impulses: FxHashMap<(BodyHandle, BodyHandle, FeatureId), (f32, f32)>,
}

impl WarmStartCache {
    pub fn apply_to(&self, manifolds: &mut [ContactManifold], factor: f32) {
        if factor <= 0.0 {
            return;
        }
        for m in manifolds {
            for pt in &mut m.points {
                if let Some(&(jn, jt)) = self.impulses.get(&(m.body_a, m.body_b, pt.feature_id)) {
                    pt.normal_impulse = jn * factor;
                    pt.tangent_impulse = jt * factor;
                }
            }
        }
    }

    pub fn store_from(&mut self, manifolds: &[ContactManifold]) {
        self.impulses.clear();
        for m in manifolds {
            if m.is_sensor {
                continue;
            }
            for pt in &m.points {
                self.impulses.insert(
                    (m.body_a, m.body_b, pt.feature_id),
                    (pt.normal_impulse, pt.tangent_impulse),
                );
            }
        }
    }

    pub fn clear(&mut self) {
        self.impulses.clear();
    }
}

/// Precomputed per-contact-point constraint data for cache-friendly inner solver loops.
struct PreparedPoint {
    ra: Vec2,
    rb: Vec2,
    normal_mass: f32,
    tangent_mass: f32,
    velocity_bias: f32,
}

struct PreparedManifold {
    idx_a: usize,
    idx_b: usize,
    normal: Vec2,
    tangent: Vec2,
    friction: f32,
    points: smallvec::SmallVec<[PreparedPoint; 2]>,
}

/// Solves all active contact manifolds and joints for one substep `dt`.
pub(crate) fn solve_constraints(
    bodies: &mut [Option<RigidBody>],
    manifolds: &mut [ContactManifold],
    joints: &mut [Option<Joint>],
    config: &SolverConfig,
    dt: f32,
) {
    let mut prepared = Vec::with_capacity(manifolds.len());

    // 1. Prepare contact constraints & apply warm-start impulses
    for m in manifolds.iter_mut() {
        if m.is_sensor || m.points.is_empty() {
            continue;
        }
        let idx_a = m.body_a.index();
        let idx_b = m.body_b.index();
        if idx_a == idx_b || idx_a >= bodies.len() || idx_b >= bodies.len() {
            continue;
        }
        let (Some(a), Some(b)) = get_pair_mut(bodies, idx_a, idx_b) else { continue };
        if !a.is_dynamic() && !b.is_dynamic() {
            continue;
        }

        let normal = m.normal;
        let tangent = normal.perp();
        let mut pts = smallvec::SmallVec::new();

        for pt in &m.points {
            let ra = pt.position - a.position;
            let rb = pt.position - b.position;

            let rxa_n = ra.cross(normal);
            let rxb_n = rb.cross(normal);
            let k_n = a.inv_mass + b.inv_mass + a.inv_inertia * rxa_n * rxa_n + b.inv_inertia * rxb_n * rxb_n;
            let normal_mass = if k_n > 1e-8 { 1.0 / k_n } else { 0.0 };

            let rxa_t = ra.cross(tangent);
            let rxb_t = rb.cross(tangent);
            let k_t = a.inv_mass + b.inv_mass + a.inv_inertia * rxa_t * rxa_t + b.inv_inertia * rxb_t * rxb_t;
            let tangent_mass = if k_t > 1e-8 { 1.0 / k_t } else { 0.0 };

            // Relative velocity at contact point for restitution bias
            let va = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, ra);
            let vb = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, rb);
            let vn = (vb - va).dot(normal);
            let velocity_bias = if vn < -config.restitution_threshold {
                -m.restitution * vn
            } else {
                0.0
            };

            // Warm start
            let warm = normal * pt.normal_impulse + tangent * pt.tangent_impulse;
            a.linear_velocity -= warm * a.inv_mass;
            a.angular_velocity -= ra.cross(warm) * a.inv_inertia;
            b.linear_velocity += warm * b.inv_mass;
            b.angular_velocity += rb.cross(warm) * b.inv_inertia;

            pts.push(PreparedPoint { ra, rb, normal_mass, tangent_mass, velocity_bias });
        }

        prepared.push(PreparedManifold {
            idx_a,
            idx_b,
            normal,
            tangent,
            friction: m.friction,
            points: pts,
        });
    }

    // 2. Iterative velocity solver (joints + contacts)
    let active_manifolds: Vec<usize> = manifolds
        .iter()
        .enumerate()
        .filter_map(|(i, m)| if !m.is_sensor && !m.points.is_empty() { Some(i) } else { None })
        .collect();

    for _ in 0..config.velocity_iterations {
        // Solve joints
        for slot in joints.iter_mut() {
            let Some(joint) = slot.as_mut() else { continue };
            if joint.broken {
                continue;
            }
            let idx_a = joint.body_a.index();
            if idx_a >= bodies.len() {
                continue;
            }
            if joint.body_b == BodyHandle::INVALID {
                if let Some(a) = bodies[idx_a].as_mut() {
                    solve_joint_velocity(joint, a, None, dt);
                }
            } else {
                let idx_b = joint.body_b.index();
                if idx_b < bodies.len() && idx_a != idx_b {
                    if let (Some(a), Some(b)) = get_pair_mut(bodies, idx_a, idx_b) {
                        solve_joint_velocity(joint, a, Some(b), dt);
                    }
                }
            }
        }

        // Solve contacts
        for (p_idx, pm) in prepared.iter().enumerate() {
            let m = &mut manifolds[active_manifolds[p_idx]];
            let (Some(a), Some(b)) = get_pair_mut(bodies, pm.idx_a, pm.idx_b) else { continue };

            for (pt_i, pp) in pm.points.iter().enumerate() {
                let cp = &mut m.points[pt_i];

                // Tangent (Coulomb friction) impulse first
                let va = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, pp.ra);
                let vb = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, pp.rb);
                let vt = (vb - va).dot(pm.tangent);
                let mut lambda_t = -vt * pp.tangent_mass;
                let max_friction = pm.friction * cp.normal_impulse;
                let old_t = cp.tangent_impulse;
                cp.tangent_impulse = (old_t + lambda_t).clamp(-max_friction, max_friction);
                lambda_t = cp.tangent_impulse - old_t;

                let imp_t = pm.tangent * lambda_t;
                a.linear_velocity -= imp_t * a.inv_mass;
                a.angular_velocity -= pp.ra.cross(imp_t) * a.inv_inertia;
                b.linear_velocity += imp_t * b.inv_mass;
                b.angular_velocity += pp.rb.cross(imp_t) * b.inv_inertia;

                // Normal impulse
                let va = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, pp.ra);
                let vb = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, pp.rb);
                let vn = (vb - va).dot(pm.normal);
                let mut lambda_n = (-vn + pp.velocity_bias) * pp.normal_mass;
                let old_n = cp.normal_impulse;
                cp.normal_impulse = (old_n + lambda_n).max(0.0);
                lambda_n = cp.normal_impulse - old_n;

                let imp_n = pm.normal * lambda_n;
                a.linear_velocity -= imp_n * a.inv_mass;
                a.angular_velocity -= pp.ra.cross(imp_n) * a.inv_inertia;
                b.linear_velocity += imp_n * b.inv_mass;
                b.angular_velocity += pp.rb.cross(imp_n) * b.inv_inertia;
            }
        }
    }

    // 3. Direct positional stabilization (eliminates residual overlap without injecting phantom bounce velocity)
    for _ in 0..config.position_iterations {
        for m in manifolds.iter() {
            if m.is_sensor || m.points.is_empty() {
                continue;
            }
            let idx_a = m.body_a.index();
            let idx_b = m.body_b.index();
            if idx_a == idx_b || idx_a >= bodies.len() || idx_b >= bodies.len() {
                continue;
            }
            let (Some(a), Some(b)) = get_pair_mut(bodies, idx_a, idx_b) else { continue };
            let inv_mass_sum = a.inv_mass + b.inv_mass;
            if inv_mass_sum <= 1e-8 {
                continue;
            }
            let max_depth = m.points.iter().map(|p| p.depth).fold(0.0_f32, f32::max);
            let correction_mag = ((max_depth - config.penetration_slop).max(0.0) * config.baumgarte) / inv_mass_sum;
            if correction_mag > 0.0 {
                let corr = m.normal * correction_mag;
                a.position -= corr * a.inv_mass;
                b.position += corr * b.inv_mass;
            }
        }
    }
}

/// Safe mutable borrow of two distinct indices in a slice.
#[inline]
pub(crate) fn get_pair_mut<T>(slice: &mut [Option<T>], i: usize, j: usize) -> (Option<&mut T>, Option<&mut T>) {
    if i < j {
        let (left, right) = slice.split_at_mut(j);
        (left[i].as_mut(), right[0].as_mut())
    } else if j < i {
        let (left, right) = slice.split_at_mut(i);
        (right[0].as_mut(), left[j].as_mut())
    } else {
        (None, None)
    }
}

/// Deterministic fixed-timestep accumulator for frame-rate independent simulation and smooth interpolation.
#[derive(Clone, Copy, Debug)]
pub struct FixedTimestep {
    /// Fixed simulation step duration in seconds (default `1.0 / 60.0`).
    pub dt: f32,
    /// Maximum accumulated frame time per update to prevent spiral of death (default `0.1` s).
    pub max_frame_time: f32,
    accumulator: f32,
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self::new(1.0 / 60.0)
    }
}

impl FixedTimestep {
    pub const fn new(dt: f32) -> Self {
        Self { dt, max_frame_time: 0.1, accumulator: 0.0 }
    }

    /// Advances the accumulator by `frame_dt`, invoking `step_fn(self.dt)` zero or more times,
    /// and returns the interpolation factor `alpha` in `[0.0, 1.0)` for rendering.
    pub fn advance(&mut self, frame_dt: f32, mut step_fn: impl FnMut(f32)) -> f32 {
        let clamped = frame_dt.clamp(0.0, self.max_frame_time);
        self.accumulator += clamped;
        while self.accumulator >= self.dt {
            step_fn(self.dt);
            self.accumulator -= self.dt;
        }
        (self.accumulator / self.dt).clamp(0.0, 1.0)
    }

    pub fn reset(&mut self) {
        self.accumulator = 0.0;
    }
}
