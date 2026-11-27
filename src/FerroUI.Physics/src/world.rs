//! High-level [`PhysicsWorld`] combining rigid bodies, joints, force fields, BVH broadphase,
//! narrowphase + warm-starting, CCD anti-tunneling, SIMD velocity integration, spatial queries,
//! collision events, and real-time step profiling telemetry.

use crate::body::{BodyBuilder, BodyHandle, BodyType, RigidBody};
use crate::broadphase::DynamicBvh;
use crate::joints::{Joint, JointHandle, JointKind};
use crate::material::{CollisionFilter, PhysicsMaterial};
use crate::math::{Aabb2, Ray2, Rot2, Transform2D, Vec2};
use crate::narrowphase::{collide_shapes, closest_point_on_segment, swept_toi, ContactManifold};
use crate::queries::{raycast_shape, ClosestPointHit, CollisionEvent, ForceField, RayHit, ShapeCastHit, TriggerEvent};
use crate::shapes::Shape2D;
use crate::simd::{integrate_rigid_velocities_soa, SimdBackend};
use crate::solver::{solve_constraints, SolverConfig, WarmStartCache};
use rustc_hash::FxHashSet;
use std::time::Instant;

/// Per-step performance and diagnostic telemetry returned by [`PhysicsWorld::step`].
#[derive(Clone, Copy, Debug, Default)]
pub struct StepStats {
    pub total_us: u64,
    pub broadphase_us: u64,
    pub narrowphase_us: u64,
    pub solver_us: u64,
    pub integrate_us: u64,
    pub total_bodies: usize,
    pub active_bodies: usize,
    pub sleeping_bodies: usize,
    pub contact_manifolds: usize,
    pub contact_points: usize,
    pub active_joints: usize,
    pub broadphase_pairs: usize,
    pub ccd_events: usize,
    pub simd_backend: Option<SimdBackend>,
}

/// Complete 2D rigid-body physics world suitable for games, UI interactions, robotics, and tools.
pub struct PhysicsWorld {
    /// Global gravity vector in m/s^2 (default `(0.0, 9.81)` for screen/UI Y-down or configurable to `(0.0, -9.81)` for Y-up).
    pub gravity: Vec2,
    pub config: SolverConfig,

    bodies: Vec<Option<RigidBody>>,
    body_generations: Vec<u32>,
    free_body_slots: Vec<u32>,

    joints: Vec<Option<Joint>>,
    joint_generations: Vec<u32>,
    free_joint_slots: Vec<u32>,

    force_fields: Vec<ForceField>,

    bvh: DynamicBvh,
    broadphase_pairs: Vec<(usize, usize)>,
    manifolds: Vec<ContactManifold>,
    warm_start: WarmStartCache,

    prev_contact_pairs: FxHashSet<(BodyHandle, BodyHandle)>,
    prev_sensor_pairs: FxHashSet<(BodyHandle, BodyHandle)>,
    collision_events: Vec<CollisionEvent>,
    trigger_events: Vec<TriggerEvent>,

    // Reusable SoA scratch buffers for SIMD velocity integration
    soa_indices: Vec<usize>,
    soa_vx: Vec<f32>,
    soa_vy: Vec<f32>,
    soa_fx: Vec<f32>,
    soa_fy: Vec<f32>,
    soa_inv_mass: Vec<f32>,
    soa_gravity_scale: Vec<f32>,
    soa_damping: Vec<f32>,

    last_stats: StepStats,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new(Vec2::new(0.0, 9.81))
    }
}

impl PhysicsWorld {
    /// Creates a new physics world with the given gravity vector.
    pub fn new(gravity: Vec2) -> Self {
        Self {
            gravity,
            config: SolverConfig::game_default(),
            bodies: Vec::new(),
            body_generations: Vec::new(),
            free_body_slots: Vec::new(),
            joints: Vec::new(),
            joint_generations: Vec::new(),
            free_joint_slots: Vec::new(),
            force_fields: Vec::new(),
            bvh: DynamicBvh::new(),
            broadphase_pairs: Vec::with_capacity(256),
            manifolds: Vec::with_capacity(128),
            warm_start: WarmStartCache::default(),
            prev_contact_pairs: FxHashSet::default(),
            prev_sensor_pairs: FxHashSet::default(),
            collision_events: Vec::new(),
            trigger_events: Vec::new(),
            soa_indices: Vec::new(),
            soa_vx: Vec::new(),
            soa_vy: Vec::new(),
            soa_fx: Vec::new(),
            soa_fy: Vec::new(),
            soa_inv_mass: Vec::new(),
            soa_gravity_scale: Vec::new(),
            soa_damping: Vec::new(),
            last_stats: StepStats::default(),
        }
    }

    pub fn with_config(mut self, config: SolverConfig) -> Self {
        self.config = config;
        self
    }

    /// Removes all bodies, joints, force fields, and cached state.
    pub fn clear(&mut self) {
        self.bodies.clear();
        self.body_generations.clear();
        self.free_body_slots.clear();
        self.joints.clear();
        self.joint_generations.clear();
        self.free_joint_slots.clear();
        self.force_fields.clear();
        self.bvh.clear();
        self.broadphase_pairs.clear();
        self.manifolds.clear();
        self.warm_start.clear();
        self.prev_contact_pairs.clear();
        self.prev_sensor_pairs.clear();
        self.collision_events.clear();
        self.trigger_events.clear();
    }

    /// Adds a rigid body from a [`BodyBuilder`] and returns its generational [`BodyHandle`].
    pub fn add_body(&mut self, builder: BodyBuilder) -> BodyHandle {
        let (index, generation) = if let Some(idx) = self.free_body_slots.pop() {
            let gen = self.body_generations[idx as usize];
            (idx, gen)
        } else {
            let idx = self.bodies.len() as u32;
            self.bodies.push(None);
            self.body_generations.push(1);
            (idx, 1)
        };
        let handle = BodyHandle { index, generation };
        let mut body = builder.build(handle);
        let proxy = self.bvh.insert(index as usize, body.aabb);
        body.bvh_proxy = Some(proxy);
        self.bodies[index as usize] = Some(body);
        handle
    }

    /// Removes a rigid body and any joints attached to it.
    pub fn remove_body(&mut self, handle: BodyHandle) -> bool {
        let idx = handle.index();
        if idx >= self.bodies.len() {
            return false;
        }
        let matches = self.bodies[idx].as_ref().is_some_and(|b| b.handle == handle);
        if !matches {
            return false;
        }
        if let Some(body) = self.bodies[idx].take() {
            if let Some(proxy) = body.bvh_proxy {
                self.bvh.remove(proxy);
            }
        }
        self.body_generations[idx] = self.body_generations[idx].wrapping_add(1).max(1);
        self.free_body_slots.push(handle.index);

        // Detach any joints referencing this body
        for slot in &mut self.joints {
            if let Some(j) = slot {
                if j.body_a == handle || j.body_b == handle {
                    j.broken = true;
                }
            }
        }
        true
    }

    #[inline]
    pub fn body(&self, handle: BodyHandle) -> Option<&RigidBody> {
        self.bodies.get(handle.index())?.as_ref().filter(|b| b.handle == handle)
    }

    #[inline]
    pub fn body_mut(&mut self, handle: BodyHandle) -> Option<&mut RigidBody> {
        self.bodies.get_mut(handle.index())?.as_mut().filter(|b| b.handle == handle)
    }

    /// Iterates over all live rigid bodies.
    pub fn bodies(&self) -> impl Iterator<Item = &RigidBody> {
        self.bodies.iter().filter_map(|s| s.as_ref())
    }

    /// Mutably iterates over all live rigid bodies.
    pub fn bodies_mut(&mut self) -> impl Iterator<Item = &mut RigidBody> {
        self.bodies.iter_mut().filter_map(|s| s.as_mut())
    }

    pub fn body_count(&self) -> usize {
        self.bodies.iter().filter(|s| s.is_some()).count()
    }

    /// Adds a mechanical joint and returns its [`JointHandle`].
    pub fn add_joint(&mut self, mut joint: Joint) -> JointHandle {
        let (index, generation) = if let Some(idx) = self.free_joint_slots.pop() {
            let gen = self.joint_generations[idx as usize];
            (idx, gen)
        } else {
            let idx = self.joints.len() as u32;
            self.joints.push(None);
            self.joint_generations.push(1);
            (idx, 1)
        };
        let handle = JointHandle { index, generation };
        joint.handle = handle;
        // Wake connected bodies
        if let Some(a) = self.body_mut(joint.body_a) {
            a.wake_up();
        }
        if let Some(b) = self.body_mut(joint.body_b) {
            b.wake_up();
        }
        self.joints[index as usize] = Some(joint);
        handle
    }

    pub fn remove_joint(&mut self, handle: JointHandle) -> bool {
        let idx = handle.index as usize;
        if idx >= self.joints.len() {
            return false;
        }
        let matches = self.joints[idx].as_ref().is_some_and(|j| j.handle == handle);
        if !matches {
            return false;
        }
        self.joints[idx] = None;
        self.joint_generations[idx] = self.joint_generations[idx].wrapping_add(1).max(1);
        self.free_joint_slots.push(handle.index);
        true
    }

    #[inline]
    pub fn joint(&self, handle: JointHandle) -> Option<&Joint> {
        self.joints.get(handle.index as usize)?.as_ref().filter(|j| j.handle == handle)
    }

    #[inline]
    pub fn joint_mut(&mut self, handle: JointHandle) -> Option<&mut Joint> {
        self.joints.get_mut(handle.index as usize)?.as_mut().filter(|j| j.handle == handle)
    }

    pub fn joints(&self) -> impl Iterator<Item = &Joint> {
        self.joints.iter().filter_map(|j| j.as_ref()).filter(|j| !j.broken)
    }

    /// Iterates over all live joints including broken ones (used for snapshot/rollback).
    pub fn all_joints(&self) -> impl Iterator<Item = &Joint> {
        self.joints.iter().filter_map(|j| j.as_ref())
    }

    /// Updates the world target of a [`JointKind::Target`] (e.g., mouse/touch dragging).
    pub fn set_target_joint_position(&mut self, handle: JointHandle, target: Vec2) {
        if let Some(j) = self.joint_mut(handle) {
            let body_a = j.body_a;
            if let JointKind::Target { target: ref mut t, .. } = j.kind {
                *t = target;
            }
            if let Some(b) = self.body_mut(body_a) {
                b.wake_up();
            }
        }
    }

    pub fn add_force_field(&mut self, field: ForceField) -> usize {
        let id = self.force_fields.len();
        self.force_fields.push(field);
        id
    }

    pub fn clear_force_fields(&mut self) {
        self.force_fields.clear();
    }

    pub fn force_fields(&self) -> &[ForceField] {
        &self.force_fields
    }

    /// Applies an instantaneous radial explosion blast impulse to all dynamic bodies within `radius`.
    pub fn apply_explosion(&mut self, center: Vec2, radius: f32, strength: f32) {
        let r2 = radius * radius;
        for body in self.bodies.iter_mut().filter_map(|s| s.as_mut()) {
            if !body.is_dynamic() {
                continue;
            }
            let delta = body.position - center;
            let d2 = delta.length_squared();
            if d2 < r2 {
                let dist = d2.sqrt();
                let dir = if dist > 1e-4 { delta * (1.0 / dist) } else { Vec2::new(0.0, -1.0) };
                let falloff = (1.0 - dist / radius).clamp(0.0, 1.0);
                let impulse = dir * (strength * falloff * body.mass);
                body.apply_impulse(impulse);
            }
        }
    }

    /// Wakes every dynamic body in the world (useful when changing gravity).
    pub fn wake_all(&mut self) {
        for body in self.bodies.iter_mut().filter_map(|s| s.as_mut()) {
            body.wake_up();
        }
    }

    /// Returns the contact manifolds generated in the most recent substep (for debug rendering & queries).
    pub fn contacts(&self) -> &[ContactManifold] {
        &self.manifolds
    }

    /// Returns collision lifecycle events from the most recent [`Self::step`] call.
    pub fn collision_events(&self) -> &[CollisionEvent] {
        &self.collision_events
    }

    /// Returns sensor trigger events from the most recent [`Self::step`] call.
    pub fn trigger_events(&self) -> &[TriggerEvent] {
        &self.trigger_events
    }

    /// Returns performance and diagnostic telemetry from the most recent [`Self::step`] call.
    pub fn last_stats(&self) -> StepStats {
        self.last_stats
    }

    /// Advances the entire physics simulation by `dt` seconds using substepped TGS/Sequential Impulse.
    pub fn step(&mut self, dt: f32) -> StepStats {
        let total_start = Instant::now();
        if dt <= 0.0 || !dt.is_finite() {
            return self.last_stats;
        }

        // Snapshot previous transforms for smooth render interpolation
        for body in self.bodies.iter_mut().filter_map(|s| s.as_mut()) {
            body.prev_position = body.position;
            body.prev_rotation = body.rotation;
        }

        let substeps = self.config.substeps.max(1);
        let sub_dt = dt / substeps as f32;

        let mut broadphase_us = 0u64;
        let mut narrowphase_us = 0u64;
        let mut solver_us = 0u64;
        let mut integrate_us = 0u64;
        let mut ccd_events = 0usize;

        for _ in 0..substeps {
            // 1. Force fields & SIMD velocity integration
            let t_int = Instant::now();
            self.integrate_velocities_simd(sub_dt);
            integrate_us += t_int.elapsed().as_micros() as u64;

            // 2. Broadphase (BVH update + pair collection)
            let t_bp = Instant::now();
            self.update_broadphase(sub_dt);
            broadphase_us += t_bp.elapsed().as_micros() as u64;

            // 3. Narrowphase (exact manifolds + warm-start lookup)
            let t_np = Instant::now();
            self.run_narrowphase();
            narrowphase_us += t_np.elapsed().as_micros() as u64;

            // 4. Constraint solver (warm-start, iterative velocity, positional stabilization)
            let t_sol = Instant::now();
            solve_constraints(&mut self.bodies, &mut self.manifolds, &mut self.joints, &self.config, sub_dt);
            self.warm_start.store_from(&self.manifolds);
            solver_us += t_sol.elapsed().as_micros() as u64;

            // 5. Integrate positions + optional CCD anti-tunneling
            let t_pos = Instant::now();
            ccd_events += self.integrate_positions_and_ccd(sub_dt);
            integrate_us += t_pos.elapsed().as_micros() as u64;
        }

        // Clear accumulated per-step user forces & torques
        for body in self.bodies.iter_mut().filter_map(|s| s.as_mut()) {
            body.force = Vec2::ZERO;
            body.torque = 0.0;
        }

        // Update sleeping state
        self.update_sleeping(dt);

        // Emit collision & trigger lifecycle events
        self.emit_events();

        // Collect telemetry
        let mut total_bodies = 0;
        let mut active_bodies = 0;
        let mut sleeping_bodies = 0;
        for body in self.bodies.iter().filter_map(|s| s.as_ref()) {
            total_bodies += 1;
            if body.is_dynamic() {
                if body.is_sleeping {
                    sleeping_bodies += 1;
                } else {
                    active_bodies += 1;
                }
            }
        }
        let contact_points: usize = self.manifolds.iter().map(|m| m.points.len()).sum();
        let active_joints = self.joints.iter().filter(|j| j.as_ref().is_some_and(|x| !x.broken)).count();

        self.last_stats = StepStats {
            total_us: total_start.elapsed().as_micros() as u64,
            broadphase_us,
            narrowphase_us,
            solver_us,
            integrate_us,
            total_bodies,
            active_bodies,
            sleeping_bodies,
            contact_manifolds: self.manifolds.len(),
            contact_points,
            active_joints,
            broadphase_pairs: self.broadphase_pairs.len(),
            ccd_events,
            simd_backend: Some(SimdBackend::active()),
        };
        self.last_stats
    }

    fn integrate_velocities_simd(&mut self, dt: f32) {
        self.soa_indices.clear();
        self.soa_vx.clear();
        self.soa_vy.clear();
        self.soa_fx.clear();
        self.soa_fy.clear();
        self.soa_inv_mass.clear();
        self.soa_gravity_scale.clear();
        self.soa_damping.clear();

        let gravity = self.gravity;
        for (idx, slot) in self.bodies.iter_mut().enumerate() {
            let Some(body) = slot.as_mut() else { continue };
            if !body.is_dynamic() || body.is_sleeping {
                continue;
            }
            // Apply spatial force fields for THIS substep only (do not accumulate across substeps!)
            let user_force = body.force;
            let user_torque = body.torque;
            for field in &self.force_fields {
                field.apply_to_body(body, gravity);
            }
            let total_force = body.force;
            let total_torque = body.torque;
            body.force = user_force;
            body.torque = user_torque;

            // Angular velocity update
            if !body.fixed_rotation {
                let ang_damp = 1.0 / (1.0 + body.angular_damping * dt);
                body.angular_velocity = (body.angular_velocity + total_torque * body.inv_inertia * dt) * ang_damp;
            }
            self.soa_indices.push(idx);
            self.soa_vx.push(body.linear_velocity.x);
            self.soa_vy.push(body.linear_velocity.y);
            self.soa_fx.push(total_force.x);
            self.soa_fy.push(total_force.y);
            self.soa_inv_mass.push(body.inv_mass);
            self.soa_gravity_scale.push(body.gravity_scale);
            self.soa_damping.push(body.linear_damping);
        }

        // Batch SIMD kernel (NEON / AVX2 / SSE2 / Scalar)
        integrate_rigid_velocities_soa(
            &mut self.soa_vx,
            &mut self.soa_vy,
            &self.soa_fx,
            &self.soa_fy,
            &self.soa_inv_mass,
            &self.soa_gravity_scale,
            &self.soa_damping,
            gravity.x,
            gravity.y,
            dt,
        );

        for (k, &idx) in self.soa_indices.iter().enumerate() {
            if let Some(body) = self.bodies[idx].as_mut() {
                body.linear_velocity = Vec2::new(self.soa_vx[k], self.soa_vy[k]);
            }
        }
    }

    fn update_broadphase(&mut self, dt: f32) {
        let mut active_proxies = Vec::with_capacity(self.bodies.len());
        for body in self.bodies.iter_mut().filter_map(|s| s.as_mut()) {
            let tight = body.shape.compute_aabb(body.transform());
            body.aabb = tight;
            if let Some(proxy) = body.bvh_proxy {
                if body.body_type != BodyType::Static && !body.is_sleeping {
                    self.bvh.update(proxy, tight, body.linear_velocity * dt);
                }
                active_proxies.push(proxy);
            }
        }
        self.bvh.collect_pairs(&active_proxies, &mut self.broadphase_pairs);
    }

    fn run_narrowphase(&mut self) {
        self.manifolds.clear();

        // Build non-colliding joint pair exclusion set
        let mut ignored_pairs: FxHashSet<(BodyHandle, BodyHandle)> = FxHashSet::default();
        for joint in self.joints.iter().filter_map(|j| j.as_ref()) {
            if !joint.broken && !joint.collide_connected && joint.body_b != BodyHandle::INVALID {
                let pair = ordered_pair(joint.body_a, joint.body_b);
                ignored_pairs.insert(pair);
            }
        }

        for &(idx_a, idx_b) in &self.broadphase_pairs {
            let (Some(a), Some(b)) = (self.bodies[idx_a].as_ref(), self.bodies[idx_b].as_ref()) else {
                continue;
            };
            if !a.is_dynamic() && !b.is_dynamic() {
                continue;
            }
            if (a.is_sleeping || !a.is_dynamic()) && (b.is_sleeping || !b.is_dynamic()) {
                continue;
            }
            if !a.filter.should_collide(b.filter) {
                continue;
            }
            if !a.aabb.intersects(b.aabb) {
                continue;
            }
            if ignored_pairs.contains(&ordered_pair(a.handle, b.handle)) {
                continue;
            }

            if let Some(mut manifold) =
                collide_shapes(a.handle, &a.shape, a.transform(), b.handle, &b.shape, b.transform())
            {
                manifold.friction = PhysicsMaterial::combine_friction(a.material, b.material);
                manifold.restitution = PhysicsMaterial::combine_restitution(a.material, b.material);
                manifold.is_sensor = a.is_sensor || b.is_sensor;
                self.manifolds.push(manifold);
            }
        }

        // Wake sleeping bodies touched by moving bodies
        for m in &self.manifolds {
            if m.is_sensor {
                continue;
            }
            let wake_a = self.bodies[m.body_b.index()]
                .as_ref()
                .is_some_and(|b| !b.is_sleeping && b.linear_velocity.length_squared() > 0.04);
            let wake_b = self.bodies[m.body_a.index()]
                .as_ref()
                .is_some_and(|a| !a.is_sleeping && a.linear_velocity.length_squared() > 0.04);
            if wake_a {
                if let Some(a) = self.bodies[m.body_a.index()].as_mut() {
                    a.wake_up();
                }
            }
            if wake_b {
                if let Some(b) = self.bodies[m.body_b.index()].as_mut() {
                    b.wake_up();
                }
            }
        }

        self.warm_start.apply_to(&mut self.manifolds, self.config.warm_start_factor);
    }

    fn integrate_positions_and_ccd(&mut self, dt: f32) -> usize {
        let mut ccd_hits = 0usize;

        // Collect fast bullet indices first so we can query static/solid obstacles cleanly
        let mut bullets: Vec<(usize, Transform2D, Transform2D)> = Vec::new();
        for (idx, slot) in self.bodies.iter_mut().enumerate() {
            let Some(body) = slot.as_mut() else { continue };
            if body.body_type == BodyType::Static || body.is_sleeping {
                continue;
            }
            let tf0 = body.transform();
            let new_pos = body.position + body.linear_velocity * dt;
            let new_angle = body.angle + body.angular_velocity * dt;
            let tf1 = Transform2D::new(new_pos, new_angle);

            let travel = (new_pos - body.position).length();
            if self.config.enable_ccd && body.is_bullet && body.is_dynamic() && !body.is_sensor
                && travel > body.shape.bounding_radius() * 0.5
            {
                bullets.push((idx, tf0, tf1));
            } else {
                body.position = new_pos;
                body.angle = new_angle;
                body.rotation = Rot2::from_radians(new_angle);
            }
        }

        // Resolve CCD bullets against candidate obstacles in the BVH
        for (idx, tf0, tf1) in bullets {
            let (shape_a, filter_a) = match self.bodies[idx].as_ref() {
                Some(b) => (b.shape.clone(), b.filter),
                None => continue,
            };
            let sweep_aabb = shape_a.compute_aabb(tf0).union(shape_a.compute_aabb(tf1));
            let mut candidates = Vec::new();
            self.bvh.query_aabb(sweep_aabb, |other_idx| {
                if other_idx != idx {
                    candidates.push(other_idx);
                }
            });

            let mut min_toi = 1.0_f32;
            let mut hit_other: Option<usize> = None;
            for other_idx in candidates {
                let Some(other) = self.bodies[other_idx].as_ref() else { continue };
                if other.is_sensor || !filter_a.should_collide(other.filter) {
                    continue;
                }
                if let Some(toi) = swept_toi(&shape_a, tf0, tf1, &other.shape, other.transform()) {
                    if toi < min_toi {
                        min_toi = toi;
                        hit_other = Some(other_idx);
                    }
                }
            }

            if let Some(other_idx) = hit_other {
                ccd_hits += 1;
                let impact_tf = tf0.interpolate(tf1, min_toi);
                // Place bullet at impact TOI and reflect/impulse off obstacle normal
                let (other_shape, other_tf, other_rest) = match self.bodies[other_idx].as_ref() {
                    Some(o) => (o.shape.clone(), o.transform(), o.material.restitution),
                    None => continue,
                };
                if let Some(body) = self.bodies[idx].as_mut() {
                    body.position = impact_tf.position;
                    body.angle = impact_tf.rotation.radians();
                    body.rotation = impact_tf.rotation;

                    let slightly_past = tf0.interpolate(tf1, (min_toi + 0.02).min(1.0));
                    if let Some(m) = collide_shapes(
                        body.handle,
                        &body.shape,
                        slightly_past,
                        BodyHandle::INVALID,
                        &other_shape,
                        other_tf,
                    ) {
                        let vn = body.linear_velocity.dot(m.normal);
                        if vn > 0.0 {
                            let e = body.material.restitution.max(other_rest);
                            body.linear_velocity -= m.normal * ((1.0 + e) * vn);
                        }
                    }
                }
            } else if let Some(body) = self.bodies[idx].as_mut() {
                body.position = tf1.position;
                body.angle = tf1.rotation.radians();
                body.rotation = tf1.rotation;
            }
        }

        ccd_hits
    }

    fn update_sleeping(&mut self, dt: f32) {
        let lin_sq = self.config.sleep_linear_threshold * self.config.sleep_linear_threshold;
        let ang_thr = self.config.sleep_angular_threshold;
        let time_thr = self.config.sleep_time_threshold;

        for body in self.bodies.iter_mut().filter_map(|s| s.as_mut()) {
            if !body.is_dynamic() || !body.can_sleep {
                continue;
            }
            if body.linear_velocity.length_squared() < lin_sq && body.angular_velocity.abs() < ang_thr {
                body.sleep_time += dt;
                if body.sleep_time >= time_thr {
                    body.put_to_sleep();
                }
            } else {
                body.sleep_time = 0.0;
                body.is_sleeping = false;
            }
        }
    }

    fn emit_events(&mut self) {
        self.collision_events.clear();
        self.trigger_events.clear();

        let mut curr_contacts = FxHashSet::default();
        let mut curr_sensors = FxHashSet::default();

        for m in &self.manifolds {
            let pair = ordered_pair(m.body_a, m.body_b);
            if m.is_sensor {
                curr_sensors.insert(pair);
                if !self.prev_sensor_pairs.contains(&pair) {
                    let a_is_sensor = self.body(m.body_a).is_some_and(|b| b.is_sensor);
                    let (sensor, other) = if a_is_sensor { (m.body_a, m.body_b) } else { (m.body_b, m.body_a) };
                    self.trigger_events.push(TriggerEvent::Entered { sensor, other });
                }
            } else {
                curr_contacts.insert(pair);
                let pt = m.points.first().map_or(Vec2::ZERO, |p| p.position);
                if !self.prev_contact_pairs.contains(&pair) {
                    let va = self.body(m.body_a).map_or(Vec2::ZERO, |b| b.linear_velocity);
                    let vb = self.body(m.body_b).map_or(Vec2::ZERO, |b| b.linear_velocity);
                    let impact_speed = (va - vb).dot(m.normal).abs();
                    self.collision_events.push(CollisionEvent::Started {
                        body_a: m.body_a,
                        body_b: m.body_b,
                        normal: m.normal,
                        point: pt,
                        impact_speed,
                    });
                } else {
                    self.collision_events.push(CollisionEvent::Persisted {
                        body_a: m.body_a,
                        body_b: m.body_b,
                        normal: m.normal,
                        point: pt,
                    });
                }
            }
        }

        for &pair in &self.prev_contact_pairs {
            if !curr_contacts.contains(&pair) {
                self.collision_events.push(CollisionEvent::Ended { body_a: pair.0, body_b: pair.1 });
            }
        }
        for &pair in &self.prev_sensor_pairs {
            if !curr_sensors.contains(&pair) {
                let a_is_sensor = self.body(pair.0).is_some_and(|b| b.is_sensor);
                let (sensor, other) = if a_is_sensor { (pair.0, pair.1) } else { (pair.1, pair.0) };
                self.trigger_events.push(TriggerEvent::Exited { sensor, other });
            }
        }

        self.prev_contact_pairs = curr_contacts;
        self.prev_sensor_pairs = curr_sensors;
    }

    // -------------------------------------------------------------------------
    // Spatial Queries
    // -------------------------------------------------------------------------

    /// Casts a ray through the BVH and returns the closest hit within `max_toi`.
    pub fn raycast(&self, ray: Ray2, max_toi: f32, filter: CollisionFilter) -> Option<RayHit> {
        let mut best: Option<RayHit> = None;
        self.bvh.query_ray(ray, max_toi, |idx, curr_max| {
            let Some(body) = self.bodies.get(idx).and_then(|s| s.as_ref()) else {
                return curr_max;
            };
            if !filter.should_collide(body.filter) {
                return curr_max;
            }
            if let Some((toi, normal)) = raycast_shape(&body.shape, body.transform(), ray, curr_max) {
                if toi < curr_max {
                    best = Some(RayHit { body: body.handle, toi, point: ray.point_at(toi), normal });
                    return toi;
                }
            }
            curr_max
        });
        best
    }

    /// Returns all bodies containing `world_point` matching `filter`.
    pub fn point_query(&mut self, world_point: Vec2, filter: CollisionFilter) -> Vec<BodyHandle> {
        let query_aabb = Aabb2::from_center_half_extents(world_point, Vec2::splat(1e-3));
        let mut candidates = Vec::new();
        self.bvh.query_aabb(query_aabb, |idx| candidates.push(idx));

        let mut hits = Vec::new();
        for idx in candidates {
            if let Some(body) = self.bodies.get(idx).and_then(|s| s.as_ref()) {
                if filter.should_collide(body.filter) && body.shape.contains_point(body.transform(), world_point) {
                    hits.push(body.handle);
                }
            }
        }
        hits
    }

    /// Picks the top-most dynamic (or any) body at `world_point`.
    pub fn pick_body(&mut self, world_point: Vec2, dynamic_only: bool) -> Option<BodyHandle> {
        let hits = self.point_query(world_point, CollisionFilter::ALL);
        for handle in hits {
            if let Some(b) = self.body(handle) {
                if !dynamic_only || b.is_dynamic() {
                    return Some(handle);
                }
            }
        }
        None
    }

    /// Queries all bodies whose AABB intersects `region`.
    pub fn aabb_query(&mut self, region: Aabb2, filter: CollisionFilter) -> Vec<BodyHandle> {
        let mut candidates = Vec::new();
        self.bvh.query_aabb(region, |idx| candidates.push(idx));
        let mut hits = Vec::new();
        for idx in candidates {
            if let Some(body) = self.bodies.get(idx).and_then(|s| s.as_ref()) {
                if filter.should_collide(body.filter) && body.aabb.intersects(region) {
                    hits.push(body.handle);
                }
            }
        }
        hits
    }

    /// Sweeps `shape` from `start_tf` along unit `direction` up to `max_distance` and returns
    /// the closest solid impact.
    pub fn shape_cast(
        &mut self,
        shape: &Shape2D,
        start_tf: Transform2D,
        direction: Vec2,
        max_distance: f32,
        filter: CollisionFilter,
        ignore_body: Option<BodyHandle>,
    ) -> Option<ShapeCastHit> {
        let dir = direction.normalize_or_zero();
        if max_distance <= 0.0 || dir.length_squared() < 0.5 {
            return None;
        }
        let end_tf = Transform2D { position: start_tf.position + dir * max_distance, rotation: start_tf.rotation };
        let sweep_aabb = shape.compute_aabb(start_tf).union(shape.compute_aabb(end_tf)).expanded(0.05);

        let mut candidates = Vec::new();
        self.bvh.query_aabb(sweep_aabb, |idx| candidates.push(idx));

        let mut best: Option<ShapeCastHit> = None;
        let mut best_toi = max_distance;

        for idx in candidates {
            let Some(body) = self.bodies.get(idx).and_then(|s| s.as_ref()) else { continue };
            if body.is_sensor || Some(body.handle) == ignore_body || !filter.should_collide(body.filter) {
                continue;
            }
            let body_tf = body.transform();
            // Already overlapping at t = 0
            if let Some(m) = collide_shapes(BodyHandle::INVALID, shape, start_tf, body.handle, &body.shape, body_tf) {
                let pt = m.points.first().map_or(start_tf.position, |p| p.position);
                // Normal points from cast shape -> hit body; outward from hit body is -m.normal
                let outward = -m.normal;
                if outward.dot(dir) < 0.0 {
                    return Some(ShapeCastHit { body: body.handle, toi: 0.0, point: pt, normal: outward });
                }
                continue;
            }

            let cur_end = Transform2D { position: start_tf.position + dir * best_toi, rotation: start_tf.rotation };
            let frac = swept_toi(shape, start_tf, cur_end, &body.shape, body_tf).or_else(|| {
                // Fine linear sweep for short casts (< 0.5 * bounding_radius)
                let steps = 8;
                let mut prev = 0.0_f32;
                for s in 1..=steps {
                    let t = s as f32 / steps as f32;
                    let tf_t = start_tf.interpolate(cur_end, t);
                    if collide_shapes(BodyHandle::INVALID, shape, tf_t, body.handle, &body.shape, body_tf).is_some() {
                        let mut lo = prev;
                        let mut hi = t;
                        for _ in 0..6 {
                            let mid = 0.5 * (lo + hi);
                            if collide_shapes(BodyHandle::INVALID, shape, start_tf.interpolate(cur_end, mid), body.handle, &body.shape, body_tf).is_some() {
                                hi = mid;
                            } else {
                                lo = mid;
                            }
                        }
                        return Some(lo);
                    }
                    prev = t;
                }
                None
            });

            if let Some(t_frac) = frac {
                let hit_dist = t_frac * best_toi;
                let probe = start_tf.interpolate(cur_end, (t_frac + 0.01).min(1.0));
                if let Some(m) = collide_shapes(BodyHandle::INVALID, shape, probe, body.handle, &body.shape, body_tf) {
                    if hit_dist < best_toi {
                        best_toi = hit_dist;
                        let pt = m.points.first().map_or(start_tf.position + dir * hit_dist, |p| p.position);
                        best = Some(ShapeCastHit { body: body.handle, toi: hit_dist, point: pt, normal: -m.normal });
                    }
                }
            }
        }
        best
    }

    /// Finds the nearest surface point on any solid body within `max_radius` of `query_point`.
    pub fn closest_point(
        &mut self,
        query_point: Vec2,
        max_radius: f32,
        filter: CollisionFilter,
    ) -> Option<ClosestPointHit> {
        let region = Aabb2::from_center_half_extents(query_point, Vec2::splat(max_radius.max(0.01)));
        let mut candidates = Vec::new();
        self.bvh.query_aabb(region, |idx| candidates.push(idx));

        let mut best: Option<ClosestPointHit> = None;
        let mut best_d2 = max_radius * max_radius;

        for idx in candidates {
            let Some(body) = self.bodies.get(idx).and_then(|s| s.as_ref()) else { continue };
            if body.is_sensor || !filter.should_collide(body.filter) {
                continue;
            }
            let tf = body.transform();
            let closest = closest_point_on_shape(&body.shape, tf, query_point);
            let d2 = closest.distance_squared(query_point);
            if d2 < best_d2 {
                best_d2 = d2;
                best = Some(ClosestPointHit { body: body.handle, point: closest, distance: d2.sqrt() });
            }
        }
        best
    }
}

fn closest_point_on_shape(shape: &Shape2D, tf: Transform2D, world_point: Vec2) -> Vec2 {
    let local = tf.inv_transform_point(world_point);
    let local_closest = match shape {
        Shape2D::Circle { radius } => {
            let len = local.length();
            if len > *radius { local * (*radius / len) } else { local }
        }
        Shape2D::Box { half_extents } => local.clamp(-*half_extents, *half_extents),
        Shape2D::Capsule { half_height, radius } => {
            let seg_pt = Vec2::new(0.0, local.y.clamp(-*half_height, *half_height));
            let d = local - seg_pt;
            let len = d.length();
            if len > *radius { seg_pt + d * (*radius / len) } else { local }
        }
        Shape2D::Segment { a, b } => closest_point_on_segment(local, *a, *b),
        Shape2D::Polygon(poly) => {
            if shape.contains_point(Transform2D::IDENTITY, local) {
                local
            } else {
                let n = poly.vertices.len();
                let mut best_p = poly.vertices[0];
                let mut best_d2 = f32::INFINITY;
                for i in 0..n {
                    let c = closest_point_on_segment(local, poly.vertices[i], poly.vertices[(i + 1) % n]);
                    let d2 = c.distance_squared(local);
                    if d2 < best_d2 {
                        best_d2 = d2;
                        best_p = c;
                    }
                }
                best_p
            }
        }
    };
    tf.transform_point(local_closest)
}

#[inline]
fn ordered_pair(a: BodyHandle, b: BodyHandle) -> (BodyHandle, BodyHandle) {
    if a <= b { (a, b) } else { (b, a) }
}
