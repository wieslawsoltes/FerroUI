//! Eight interactive physics studio scenes built on `ferroui-physics`:
//! 1. Playground — Rigid-body pyramid, polygons, motorized windmill, chain bridge, shatterable boxes
//! 2. GPU Fluid — 2K/4K/8K colliding particles on Metal/Vulkan/DX12 wgpu compute vs CPU SIMD
//! 3. Cloth & Jelly — XPBD tearable wind-blown cloth curtain + pressure-inflated jelly soft bodies
//! 4. UI Studio — Velocity-tilting DraggableCards, MagneticSnap, SpringChain cursor trail, InertiaScroller
//! 5. Destruction — Swinging wrecking ball smashing breakable welded beams, shattering crates & ragdolls
//! 6. Vehicle — 2D suspension car crossing ramps, speed bumps, and a buoyant water pool
//! 7. Character — Kinematic CharacterController2D with swept shape_cast, slopes, moving platforms & crates
//! 8. Ray & Orbit — Orbital vortex well, 6-bounce sweeping laser raycast, and closest_point tether

use ferroui_physics::prelude::*;
use std::collections::VecDeque;
use std::f32::consts::{FRAC_PI_2, PI};

/// Logical studio world dimensions in meters (16:10 widescreen, Y-down).
pub const WORLD_WIDTH: f32 = 16.0;
pub const WORLD_HEIGHT: f32 = 10.0;

/// Sentinel `user_data` value marking invisible outer arena containment walls so they are not
/// drawn as thick overlapping slabs outside the crisp studio grid border.
pub const ARENA_WALL_TAG: u64 = 0xFFFF_0000_0000_0001;

mod catalog;
mod constructors;
pub use catalog::{SceneKind, SolverProfile, SpringPreset, ToolMode};

#[derive(Clone, Debug)]
pub struct UiDemoCard {
    pub label: &'static str,
    pub subtitle: &'static str,
    pub accent_argb: u32,
    pub half_size: Vec2,
    pub card: DraggableCard,
    pub slot_index: usize,
}

#[derive(Clone, Debug, Default)]
pub struct TelemetrySnapshot {
    pub backend_label: String,
    pub step_ms: f32,
    pub fps: f32,
    pub breakdown: String,
    pub entity_summary: String,
    pub gpu_available: bool,
    pub history_frame: usize,
    pub history_len: usize,
}

/// Complete studio simulation engine managing all 8 showcase scenes, history rollback, and profiler metrics.
pub struct SandboxEngine {
    pub scene: SceneKind,
    pub tool: ToolMode,
    pub spring_preset: SpringPreset,
    pub solver_profile: SolverProfile,
    pub paused: bool,
    pub time_scale: f32,
    pub gravity_y: f32,
    pub restitution: f32,
    pub wind_x: f32,
    pub show_aabbs: bool,
    pub show_contacts: bool,
    pub show_velocities: bool,
    pub show_joints: bool,
    pub show_hud_graph: bool,
    pub prefer_gpu: bool,
    pub particle_target_count: usize,

    // Rigid-body world + history rollback
    pub world: PhysicsWorld,
    pub history: WorldHistory,
    drag_joint: Option<JointHandle>,

    // Particle / GPU compute simulation
    pub particles: ParticleSystem,
    pub gpu_particles_view: Vec<GpuParticle>,
    gpu_solver: Option<GpuParticleSolver>,
    gpu_synced: bool,

    // XPBD Cloth & Soft-body jelly
    pub cloth: Option<ClothMesh>,
    pub soft_bodies: Vec<SoftBody2D>,

    // Character controller demo
    pub character: Option<CharacterController2D>,
    moving_platform: Option<(BodyHandle, Vec2)>,

    // UI spring, chain & inertia simulation
    pub ui_cards: Vec<UiDemoCard>,
    pub ui_snap: MagneticSnap,
    pub ui_scroller: InertiaScroller,
    pub ui_ribbon: SpringChain,
    pub ui_projected_slot: Option<usize>,
    pub ui_fab_center: Vec2,
    pub ui_fab_open: bool,
    pub ui_fab_items: Vec<(Spring2D, &'static str, u32)>,
    pub ui_curve_samples: Vec<f32>,
    active_card_idx: Option<usize>,
    dragging_scroller: bool,
    last_pointer_world: Vec2,
    pointer_in_arena: bool,
    pointer_down: bool,
    pointer_right: bool,
    gesture_tracker: GestureTracker,

    // Visual effects & telemetry
    pub sim_time: f32,
    pub laser_origin: Vec2,
    pub laser_segments: Vec<(Vec2, Vec2, Option<Vec2>)>,
    pub orbit_trails: Vec<(BodyHandle, VecDeque<Vec2>, u32)>,
    pub closest_tether: Option<(Vec2, Vec2, f32)>,
    pub blast_ring: Option<(Vec2, f32)>,
    pub step_ms_history: VecDeque<f32>,
    pub telemetry: TelemetrySnapshot,

    spawn_counter: u64,
    shatter_candidates: Vec<BodyHandle>,
    pub blueprint_driven: bool,
    frame_clock: FixedTimestep,
    input_clock: std::time::Instant,
    last_pointer_sample: f64,
}

impl SandboxEngine {
    pub fn new() -> Self {
        if std::env::var_os("FERROUI_PHYSICS_CPU_ONLY").is_some() {
            Self::new_cpu()
        } else {
            Self::with_gpu_solver(GpuParticleSolver::try_new(8192))
        }
    }

    /// CPU-only setup for repeatable tests, benchmarks, and machines without a GPU adapter.
    pub fn new_cpu() -> Self {
        Self::with_gpu_solver(None)
    }

    fn with_gpu_solver(gpu_solver: Option<GpuParticleSolver>) -> Self {
        let gpu_available = gpu_solver.is_some();
        let mut engine = Self {
            scene: SceneKind::Playground,
            tool: ToolMode::Drag,
            spring_preset: SpringPreset::Bouncy,
            solver_profile: SolverProfile::Game,
            paused: false,
            time_scale: 1.0,
            gravity_y: 9.81,
            restitution: 0.35,
            wind_x: 3.2,
            show_aabbs: false,
            show_contacts: false,
            show_velocities: false,
            show_joints: true,
            show_hud_graph: true,
            prefer_gpu: gpu_available,
            particle_target_count: 4096,
            world: PhysicsWorld::new(Vec2::new(0.0, 9.81)),
            history: WorldHistory::new(240),
            drag_joint: None,
            particles: ParticleSystem::new(ParticleConfig::default()),
            gpu_particles_view: Vec::new(),
            gpu_solver,
            gpu_synced: false,
            cloth: None,
            soft_bodies: Vec::new(),
            character: None,
            moving_platform: None,
            ui_cards: Vec::new(),
            ui_snap: MagneticSnap::new(Vec::new()),
            ui_scroller: InertiaScroller::new(0.0, 12.0, 4.0),
            ui_ribbon: SpringChain::new(
                12,
                Vec2::new(8.0, 5.0),
                SpringConfig::from_response_damping(0.18, 0.78),
            ),
            ui_projected_slot: None,
            ui_fab_center: Vec2::new(13.6, 5.5),
            ui_fab_open: true,
            ui_fab_items: Vec::new(),
            ui_curve_samples: Vec::new(),
            active_card_idx: None,
            dragging_scroller: false,
            last_pointer_world: Vec2::new(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5),
            pointer_in_arena: false,
            pointer_down: false,
            pointer_right: false,
            gesture_tracker: GestureTracker::default(),
            sim_time: 0.0,
            laser_origin: Vec2::new(0.55, 5.0),
            laser_segments: Vec::new(),
            orbit_trails: Vec::new(),
            closest_tether: None,
            blast_ring: None,
            step_ms_history: VecDeque::with_capacity(96),
            telemetry: TelemetrySnapshot {
                backend_label: SimdBackend::active().name().to_string(),
                fps: 60.0,
                gpu_available,
                ..Default::default()
            },
            spawn_counter: 1,
            shatter_candidates: Vec::new(),
            blueprint_driven: false,
            frame_clock: FixedTimestep::default(),
            input_clock: std::time::Instant::now(),
            last_pointer_sample: 0.0,
        };
        engine.load_scene(SceneKind::Playground);
        engine
    }

    pub fn gpu_available(&self) -> bool {
        self.gpu_solver.is_some()
    }

    pub fn gpu_adapter_name(&self) -> Option<&str> {
        self.gpu_solver
            .as_ref()
            .map(|g| g.device_info().adapter_name.as_str())
    }

    pub fn set_spring_preset(&mut self, preset: SpringPreset) {
        self.spring_preset = preset;
        let cfg = preset.config();
        for c in &mut self.ui_cards {
            c.card.set_spring_config(cfg);
        }
        for (sp, _, _) in &mut self.ui_fab_items {
            sp.set_config(cfg);
        }
        self.recompute_spring_curve();
    }

    fn recompute_spring_curve(&mut self) {
        self.ui_curve_samples.clear();
        let mut s = Spring1D::new(0.0, self.spring_preset.config());
        s.set_target(1.0);
        for _ in 0..80 {
            self.ui_curve_samples.push(s.step(1.0 / 60.0));
        }
    }

    pub fn set_solver_profile(&mut self, profile: SolverProfile) {
        self.solver_profile = profile;
        self.world.config = profile.config();
    }

    pub fn set_particle_count(&mut self, count: usize) {
        self.particle_target_count = count;
        if self.scene == SceneKind::GpuParticles {
            self.build_gpu_particles_scene();
        }
    }

    pub fn step_back_history(&mut self) {
        self.paused = true;
        self.history.step_back(&mut self.world);
        self.cancel_restored_interaction();
        self.telemetry.history_frame = self.history.cursor();
        self.telemetry.history_len = self.history.len();
    }

    pub fn step_forward_one(&mut self) {
        if self.history.step_forward(&mut self.world) {
            self.cancel_restored_interaction();
            self.telemetry.history_frame = self.history.cursor();
            self.telemetry.history_len = self.history.len();
        } else {
            let was_paused = self.paused;
            self.paused = false;
            self.step(1.0 / 60.0);
            self.paused = was_paused;
        }
    }

    pub fn scrub_history(&mut self, normalized_t: f32) {
        self.paused = true;
        if self.history.scrub(&mut self.world, normalized_t) {
            self.cancel_restored_interaction();
            self.telemetry.history_frame = self.history.cursor();
            self.telemetry.history_len = self.history.len();
        }
    }

    fn cancel_restored_interaction(&mut self) {
        // A snapshot can contain a temporary pointer joint from a previous drag.
        // It must never survive a scrub without a live pointer able to release it.
        self.on_pointer_released();
        let targets: Vec<_> = self
            .world
            .all_joints()
            .filter(|joint| matches!(joint.kind, JointKind::Target { .. }))
            .map(|joint| joint.handle)
            .collect();
        for handle in targets {
            self.world.remove_joint(handle);
        }
        // Playback restores body motion while the workspace keeps its selected settings.
        // Parametric scenes retain the solver configuration authored in their blueprint.
        if !self.blueprint_driven {
            self.world.config = self.solver_profile.config();
        }
        self.frame_clock.reset();
    }

    pub fn load_scene(&mut self, kind: SceneKind) {
        if self.scene != kind {
            self.solver_profile = if matches!(
                kind,
                SceneKind::RagdollWrecking | SceneKind::VehicleTerrain | SceneKind::LaserGravity
            ) {
                SolverProfile::Precision
            } else {
                SolverProfile::Game
            };
        }
        self.scene = kind;
        if !kind.supports_tool(self.tool) {
            self.tool = ToolMode::Drag;
        }
        self.blueprint_driven = false;
        self.paused = false;
        self.frame_clock.reset();
        self.world.clear();
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.history.clear();
        self.drag_joint = None;
        self.active_card_idx = None;
        self.dragging_scroller = false;
        self.cloth = None;
        self.soft_bodies.clear();
        self.character = None;
        self.moving_platform = None;
        self.laser_segments.clear();
        self.orbit_trails.clear();
        self.closest_tether = None;
        self.ui_projected_slot = None;
        self.blast_ring = None;
        self.shatter_candidates.clear();
        self.sim_time = 0.0;
        self.gpu_synced = false;
        self.pointer_down = false;
        self.pointer_right = false;
        self.pointer_in_arena = false;
        self.step_ms_history.clear();
        self.telemetry = TelemetrySnapshot::default();

        match kind {
            SceneKind::Playground => self.build_playground_scene(),
            SceneKind::GpuParticles => self.build_gpu_particles_scene(),
            SceneKind::ClothSoftBody => self.build_cloth_softbody_scene(),
            SceneKind::UiSprings => self.build_ui_springs_scene(),
            SceneKind::RagdollWrecking => self.build_ragdoll_scene(),
            SceneKind::VehicleTerrain => self.build_vehicle_scene(),
            SceneKind::CharacterDemo => self.build_character_scene(),
            SceneKind::LaserGravity => self.build_laser_gravity_scene(),
        }
        if kind.supports_history() {
            self.history.record(&self.world);
        }
        self.telemetry = TelemetrySnapshot {
            backend_label: SimdBackend::active().name().to_owned(),
            fps: 60.0,
            gpu_available: self.gpu_available(),
            history_len: self.history.len(),
            entity_summary: format!("{} bodies", self.world.bodies().count()),
            ..Default::default()
        };
    }

    fn add_arena_walls(&mut self) {
        let wall_mat = PhysicsMaterial::new(0.0, 0.2, 0.65);
        // Placed cleanly outside [0..WORLD_WIDTH] x [0..WORLD_HEIGHT] so inner faces sit flush on the studio grid border.
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::rect(WORLD_WIDTH + 2.0, 0.8))
                .position((WORLD_WIDTH * 0.5, WORLD_HEIGHT + 0.4))
                .material(wall_mat)
                .user_data(ARENA_WALL_TAG),
        );
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::rect(WORLD_WIDTH + 2.0, 0.8))
                .position((WORLD_WIDTH * 0.5, -0.4))
                .material(wall_mat)
                .user_data(ARENA_WALL_TAG),
        );
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::rect(0.8, WORLD_HEIGHT + 2.0))
                .position((-0.4, WORLD_HEIGHT * 0.5))
                .material(wall_mat)
                .user_data(ARENA_WALL_TAG),
        );
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::rect(0.8, WORLD_HEIGHT + 2.0))
                .position((WORLD_WIDTH + 0.4, WORLD_HEIGHT * 0.5))
                .material(wall_mat)
                .user_data(ARENA_WALL_TAG),
        );
    }

    /// Advances using fixed simulation steps, independent of display/dispatcher cadence.
    pub fn advance_frame(&mut self, frame_dt: f32) -> bool {
        if self.paused {
            self.frame_clock.reset();
            return false;
        }
        let mut clock = self.frame_clock;
        let mut stepped = false;
        clock.advance(frame_dt, |dt| {
            self.step(dt);
            stepped = true;
        });
        self.frame_clock = clock;
        if stepped && frame_dt.is_finite() && frame_dt > 0.0 {
            self.telemetry.fps = self.telemetry.fps * 0.9 + (1.0 / frame_dt).min(1000.0) * 0.1;
        }
        stepped
    }

    pub fn step(&mut self, raw_dt: f32) {
        if self.paused
            || !raw_dt.is_finite()
            || raw_dt <= 0.0
            || !self.time_scale.is_finite()
            || self.time_scale <= 0.0
        {
            return;
        }
        let dt = (raw_dt * self.time_scale).clamp(1e-4, 1.0 / 20.0);
        self.sim_time += dt;

        if let Some((_, ref mut r)) = self.blast_ring {
            *r += dt * 14.0;
            if *r > 5.2 {
                self.blast_ring = None;
            }
        }

        match self.scene {
            SceneKind::GpuParticles => self.step_particles(dt),
            SceneKind::ClothSoftBody => self.step_cloth_softbody(dt),
            SceneKind::UiSprings => self.step_ui_springs(dt),
            SceneKind::CharacterDemo => self.step_character(dt),
            _ => self.step_rigid_world(dt),
        }

        // Push sparkline sample
        if self.step_ms_history.len() >= 90 {
            self.step_ms_history.pop_front();
        }
        self.step_ms_history.push_back(self.telemetry.step_ms);
    }

    /// Loads a live physics scene compiled by a Rhino Grasshopper `ferroui-nodegraph` definition!
    pub fn load_from_blueprint(
        &mut self,
        bp: &ferroui_nodegraph::physics_nodes::CompiledPhysicsBlueprint,
    ) {
        self.scene = SceneKind::Playground;
        self.blueprint_driven = true;
        self.paused = false;
        self.frame_clock.reset();
        self.sim_time = 0.0;
        self.pointer_down = false;
        self.pointer_right = false;
        self.step_ms_history.clear();
        self.drag_joint = None;
        self.character = None;
        self.moving_platform = None;
        self.laser_segments.clear();
        self.orbit_trails.clear();
        self.closest_tether = None;
        self.blast_ring = None;
        self.shatter_candidates.clear();
        self.history.clear();

        let mut built = bp.build(WORLD_WIDTH, WORLD_HEIGHT);
        // Normalize arena wall sentinel tag so PhysicsViewport hides containment slabs cleanly
        for b in built.world.bodies_mut() {
            if b.user_data == ferroui_nodegraph::physics_nodes::GRAPH_ARENA_WALL_TAG {
                b.user_data = ARENA_WALL_TAG;
            }
        }
        self.world = built.world;
        self.gravity_y = self.world.gravity.y;
        self.soft_bodies = built.soft_bodies.into_iter().map(|(sb, _)| sb).collect();
        self.cloth = built.cloth;
        for (h, col) in built.trail_bodies {
            let mut trail = VecDeque::with_capacity(72);
            if let Some(b) = self.world.body(h) {
                trail.push_back(b.position);
            }
            self.orbit_trails.push((h, trail, col));
        }
        // Graph previews have no timeline. Avoid cloning complete world snapshots
        // on every preview frame, particularly for large procedural definitions.
    }

    fn step_rigid_world(&mut self, dt: f32) {
        if self.scene != SceneKind::LaserGravity && !self.blueprint_driven {
            let target_g = Vec2::new(0.0, self.gravity_y);
            if (self.world.gravity - target_g).length_squared() > 1e-4 {
                self.world.gravity = target_g;
                self.world.wake_all();
            }
        }

        let stats = self.world.step(dt);

        // Step any cloth & soft bodies generated by a Grasshopper parametric physics graph!
        if self.cloth.is_some() || !self.soft_bodies.is_empty() {
            let mut wind = Vec2::ZERO;
            for ff in self.world.force_fields() {
                if let ForceField::Wind { acceleration, .. } = *ff {
                    wind += acceleration;
                }
            }
            let bodies: Vec<&RigidBody> = self
                .world
                .bodies()
                .filter(|b| b.user_data != ARENA_WALL_TAG)
                .collect();
            if let Some(cloth) = self.cloth.as_mut() {
                cloth.step(dt, self.world.gravity, wind, self.sim_time, &bodies);
            }
            for sb in &mut self.soft_bodies {
                sb.step(dt, self.world.gravity + wind * 0.25, &bodies);
            }
        }

        // Update orbital trails for both Ray & Orbit and Grasshopper Solar System graphs
        if !self.orbit_trails.is_empty() {
            for (h, trail, _) in &mut self.orbit_trails {
                if let Some(b) = self.world.body(*h) {
                    if trail
                        .back()
                        .map_or(true, |p| p.distance_squared(b.position) > 0.012)
                    {
                        if trail.len() >= 72 {
                            trail.pop_front();
                        }
                        trail.push_back(b.position);
                    }
                }
            }
        }

        // Automatic impact shattering in Destruction scene!
        if self.scene == SceneKind::RagdollWrecking && !self.shatter_candidates.is_empty() {
            let mut to_shatter: Vec<(BodyHandle, Vec2)> = Vec::new();
            for ev in self.world.collision_events() {
                if let CollisionEvent::Started {
                    body_a,
                    body_b,
                    point,
                    impact_speed,
                    ..
                } = *ev
                {
                    if impact_speed > 5.5 {
                        if self.shatter_candidates.contains(&body_a) {
                            to_shatter.push((body_a, point));
                        }
                        if self.shatter_candidates.contains(&body_b) {
                            to_shatter.push((body_b, point));
                        }
                    }
                }
            }
            for (h, pt) in to_shatter {
                // Keep original generation-tagged handles: rewinding can restore a crate,
                // while a removed crate's handle cannot match any newly spawned debris.
                fracture_body(&mut self.world, h, pt, 2.8);
            }
        }

        if self.scene == SceneKind::LaserGravity {
            self.update_laser_raycast();
            self.closest_tether = if self.pointer_in_arena {
                self.world
                    .closest_point(self.last_pointer_world, 6.0, CollisionFilter::ALL)
                    .map(|hit| (self.last_pointer_world, hit.point, hit.distance))
            } else {
                None
            };
        }

        if !self.blueprint_driven && self.scene.supports_history() {
            self.history.record(&self.world);
        }

        self.telemetry = TelemetrySnapshot {
            backend_label: stats.simd_backend.map_or("CPU", |b| b.name()).to_string(),
            step_ms: stats.total_us as f32 / 1000.0,
            fps: self.telemetry.fps,
            breakdown: format!(
                "BP {:.2}ms · NP {:.2}ms · Solve {:.2}ms · Int {:.2}ms",
                stats.broadphase_us as f32 / 1000.0,
                stats.narrowphase_us as f32 / 1000.0,
                stats.solver_us as f32 / 1000.0,
                stats.integrate_us as f32 / 1000.0,
            ),
            entity_summary: format!(
                "{} bodies ({} awake) · {} contacts · {} joints",
                stats.total_bodies, stats.active_bodies, stats.contact_points, stats.active_joints
            ),
            gpu_available: self.gpu_solver.is_some(),
            history_frame: self.history.cursor(),
            history_len: self.history.len(),
        };
    }

    fn step_cloth_softbody(&mut self, dt: f32) {
        let start = std::time::Instant::now();
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.step(dt);

        // Borrow immutable colliders directly; cloning shapes/vertices every frame is unnecessary.
        let body_refs: Vec<&RigidBody> = self.world.bodies().collect();
        let g = Vec2::new(0.0, self.gravity_y);
        let wind = Vec2::new(self.wind_x, 0.0);

        if let Some(cloth) = self.cloth.as_mut() {
            cloth.step(dt, g, wind, self.sim_time, &body_refs);
        }
        for sb in &mut self.soft_bodies {
            sb.step(dt, g, &body_refs);
        }

        let us = start.elapsed().as_micros();
        let cloth_links = self
            .cloth
            .as_ref()
            .map_or(0, ClothMesh::active_constraint_count);
        self.telemetry = TelemetrySnapshot {
            backend_label: format!("XPBD Cloth + SoftBody ({})", SimdBackend::active().name()),
            step_ms: us as f32 / 1000.0,
            fps: self.telemetry.fps,
            breakdown: format!("XPBD 6 substeps · wind {:.1} m/s²", self.wind_x),
            entity_summary: format!(
                "{} cloth links · {} jelly blobs",
                cloth_links,
                self.soft_bodies.len()
            ),
            gpu_available: self.gpu_solver.is_some(),
            history_frame: 0,
            history_len: 0,
        };
    }

    fn step_character(&mut self, dt: f32) {
        let start = std::time::Instant::now();
        self.world.gravity = Vec2::new(0.0, self.gravity_y);

        // Animate kinematic elevator smoothly
        if let Some((plat_h, origin)) = self.moving_platform {
            let offset_y = (self.sim_time * 1.4).sin() * 1.4;
            let new_pos = origin + Vec2::new(0.0, offset_y);
            if let Some(p) = self.world.body_mut(plat_h) {
                // The world integrates kinematic velocity. Teleporting as well would
                // apply this movement twice and corrupt platform contact velocities.
                p.linear_velocity = (new_pos - p.position) / dt;
            }
        }

        let stats = self.world.step(dt);

        let mut char_info = String::new();
        if let Some(ctrl) = self.character.as_mut() {
            // Guide character smoothly toward pointer X (or patrol gently if idle)
            let target_x = self.last_pointer_world.x.clamp(1.0, WORLD_WIDTH - 1.0);
            let dx = target_x - ctrl.position.x;
            let desired_vx = if dx.abs() > 0.25 {
                (dx * 3.5).clamp(-6.5, 6.5)
            } else {
                0.0
            };
            ctrl.velocity.x =
                ctrl.velocity.x + (desired_vx - ctrl.velocity.x) * (10.0 * dt).min(1.0);
            ctrl.velocity.y += self.gravity_y * dt;

            // Auto-hop onto high targets if pointer is significantly above character
            if self.last_pointer_world.y + 1.4 < ctrl.position.y && ctrl.last_result.is_grounded {
                ctrl.request_jump();
            }
            ctrl.try_consume_jump(7.8);

            let res = ctrl.move_and_slide(&mut self.world, dt, None);
            let slope_deg = res.slope_angle_radians.to_degrees();
            char_info = format!(
                "{} · slope {:.0}° · pos ({:.1}, {:.1})",
                if res.is_grounded {
                    "Grounded"
                } else {
                    "Airborne"
                },
                slope_deg,
                res.position.x,
                res.position.y
            );
        }

        let total_us = start.elapsed().as_micros();
        self.telemetry = TelemetrySnapshot {
            backend_label: format!("CharacterController2D + {}", SimdBackend::active().name()),
            step_ms: total_us as f32 / 1000.0,
            fps: self.telemetry.fps,
            breakdown: char_info,
            entity_summary: format!(
                "{} bodies · {} contacts",
                stats.total_bodies, stats.contact_points
            ),
            gpu_available: self.gpu_solver.is_some(),
            history_frame: 0,
            history_len: 0,
        };
    }

    fn update_laser_raycast(&mut self) {
        self.laser_segments.clear();
        let mut origin = self.laser_origin;
        // Aim smoothly through the solar system (follow pointer only while cursor is well inside arena)
        let mut dir = if self.pointer_in_arena
            && self.last_pointer_world.x > 1.2
            && self.last_pointer_world.x < WORLD_WIDTH - 0.5
            && self.last_pointer_world.y > 0.6
            && self.last_pointer_world.y < WORLD_HEIGHT - 0.6
        {
            (self.last_pointer_world - origin).normalize_or_zero()
        } else {
            let sweep = (self.sim_time * 0.55).sin() * 0.42;
            Vec2::new(sweep.cos(), sweep.sin())
        };

        for _bounce in 0..6 {
            let ray = Ray2::new(origin, dir);
            if let Some(hit) = self.world.raycast(ray, 22.0, CollisionFilter::ALL) {
                let is_wall = self
                    .world
                    .body(hit.body)
                    .is_some_and(|b| b.user_data == ARENA_WALL_TAG);
                self.laser_segments.push((
                    origin,
                    hit.point,
                    if is_wall { None } else { Some(hit.normal) },
                ));
                // Outer arena walls absorb the beam so it only reflects dramatically off planets, star & mirrors!
                if is_wall {
                    break;
                }
                dir = (dir - hit.normal * (2.0 * dir.dot(hit.normal))).normalize_or_zero();
                origin = hit.point + hit.normal * 0.03;
            } else {
                self.laser_segments
                    .push((origin, origin + dir * 20.0, None));
                break;
            }
        }
    }

    fn step_particles(&mut self, dt: f32) {
        self.particles.config.gravity = Vec2::new(self.wind_x * 0.35, self.gravity_y);
        self.particles.config.restitution = self.restitution;
        self.particles.config.pointer_center = self.last_pointer_world;
        self.particles.config.pointer_strength = if self.pointer_down {
            if self.pointer_right {
                -105.0
            } else {
                110.0
            }
        } else {
            0.0
        };

        let stats = if self.prefer_gpu {
            if let Some(gpu) = self.gpu_solver.as_mut() {
                if !self.gpu_synced {
                    let buf = self.particles.to_gpu_particles();
                    gpu.upload_particles(&buf);
                    self.gpu_synced = true;
                }
                gpu.step_and_readback(
                    &self.particles.config,
                    &self.particles.obstacles,
                    dt,
                    &mut self.gpu_particles_view,
                )
            } else {
                let s = self.particles.step_cpu(dt);
                self.particles
                    .write_gpu_particles(&mut self.gpu_particles_view);
                s
            }
        } else {
            if self.gpu_synced {
                self.particles
                    .load_from_gpu_particles(&self.gpu_particles_view);
                self.gpu_synced = false;
            }
            let s = self.particles.step_cpu(dt);
            self.particles
                .write_gpu_particles(&mut self.gpu_particles_view);
            s
        };

        let m_per_sec = if stats.step_us > 0 {
            (stats.particle_count as f32 / (stats.step_us as f32 * 1e-6)) / 1_000_000.0
        } else {
            0.0
        };
        self.telemetry = TelemetrySnapshot {
            backend_label: stats.backend_name.to_string(),
            step_ms: stats.step_us as f32 / 1000.0,
            fps: self.telemetry.fps,
            breakdown: format!(
                "Throughput {:.2}M particles/s · O(N) atomic spatial hash",
                m_per_sec
            ),
            entity_summary: format!("{} particles · 3 obstacles", stats.particle_count),
            gpu_available: self.gpu_solver.is_some(),
            history_frame: 0,
            history_len: 0,
        };
    }

    fn step_ui_springs(&mut self, dt: f32) {
        let start = std::time::Instant::now();
        for c in &mut self.ui_cards {
            c.card.step(dt);
        }
        for (sp, _, _) in &mut self.ui_fab_items {
            sp.step(dt);
        }
        self.ui_scroller.step(dt);
        let ribbon_target = if self.pointer_in_arena {
            self.last_pointer_world
        } else {
            Vec2::new(
                6.3 + (self.sim_time * 1.4).cos() * 2.2,
                4.0 + (self.sim_time * 2.1).sin() * 0.8,
            )
        };
        self.ui_ribbon.set_leader_target(ribbon_target);
        self.ui_ribbon.step(dt);
        let us = start.elapsed().as_micros();
        self.telemetry = TelemetrySnapshot {
            backend_label: format!(
                "UI Physics Lab ({} · ζ={:.2})",
                self.spring_preset.label(),
                self.spring_preset.config().damping_ratio()
            ),
            step_ms: us as f32 / 1000.0,
            fps: self.telemetry.fps,
            breakdown: format!(
                "scroll={:.2} · vel={:.1} · FAB {}",
                self.ui_scroller.position,
                self.ui_scroller.velocity,
                if self.ui_fab_open { "Open" } else { "Closed" }
            ),
            entity_summary: format!("{} cards · 5 radial orbs · 5 detents", self.ui_cards.len()),
            gpu_available: self.gpu_solver.is_some(),
            history_frame: 0,
            history_len: 0,
        };
    }

    pub fn on_pointer_pressed(&mut self, world_pos: Vec2, right_button: bool) {
        self.pointer_down = true;
        self.pointer_right = right_button;
        self.last_pointer_world = world_pos;
        self.gesture_tracker.clear();
        self.last_pointer_sample = self.input_clock.elapsed().as_secs_f64();
        self.gesture_tracker
            .push(self.last_pointer_sample, world_pos);

        if self.scene == SceneKind::GpuParticles {
            return;
        }

        if self.scene == SceneKind::CharacterDemo {
            if let Some(ctrl) = self.character.as_mut() {
                ctrl.request_jump();
                ctrl.try_consume_jump(8.5);
            }
            return;
        }

        if self.scene == SceneKind::ClothSoftBody {
            if right_button {
                if let Some(cloth) = self.cloth.as_mut() {
                    cloth.tear_at(world_pos, 0.55);
                }
            } else if self.tool == ToolMode::SpawnJelly {
                self.spawn_jelly_at(world_pos);
            } else if let Some(cloth) = self.cloth.as_mut() {
                if let Some(node_idx) = cloth.pick_node(world_pos, 0.9) {
                    cloth.grab_node = Some((node_idx, world_pos));
                } else {
                    self.spawn_jelly_at(world_pos);
                }
            }
            return;
        }

        if self.scene == SceneKind::UiSprings {
            // Check expandable radial FAB orb click
            if world_pos.distance(self.ui_fab_center) <= 0.65 {
                self.ui_fab_open = !self.ui_fab_open;
                for (i, (sp, _, _)) in self.ui_fab_items.iter_mut().enumerate() {
                    if self.ui_fab_open {
                        let ang = -PI + (i as f32) * (PI / 4.0);
                        sp.set_target(
                            self.ui_fab_center + Vec2::new(ang.cos() * 1.45, ang.sin() * 1.45),
                        );
                    } else {
                        sp.set_target(self.ui_fab_center);
                    }
                }
                return;
            }
            if world_pos.y > 7.4 {
                self.dragging_scroller = true;
                self.ui_scroller.begin_drag();
                return;
            }
            for i in (0..self.ui_cards.len()).rev() {
                let c = &self.ui_cards[i];
                let d = (world_pos - c.card.position.value()).abs();
                if d.x <= c.half_size.x && d.y <= c.half_size.y {
                    self.active_card_idx = Some(i);
                    self.ui_cards[i].card.begin_drag(world_pos);
                    return;
                }
            }
            return;
        }

        if right_button || self.tool == ToolMode::Explosion {
            self.world.apply_explosion(world_pos, 5.0, 17.0);
            self.blast_ring = Some((world_pos, 0.2));
            return;
        }

        match self.tool {
            ToolMode::Shatter => {
                if let Some(handle) = self.world.pick_body(world_pos, true) {
                    fracture_body(&mut self.world, handle, world_pos, 3.6);
                    self.blast_ring = Some((world_pos, 0.15));
                }
            }
            ToolMode::SpawnJelly => self.spawn_jelly_at(world_pos),
            ToolMode::Drag => {
                if let Some(handle) = self.world.pick_body(world_pos, true) {
                    if let Some(body) = self.world.body(handle) {
                        let local = body.transform().inv_transform_point(world_pos);
                        let jh = self.world.add_joint(Joint::target(
                            handle,
                            local,
                            world_pos,
                            450.0 * body.mass,
                        ));
                        self.drag_joint = Some(jh);
                    }
                } else {
                    self.spawn_shape_at(world_pos, ToolMode::SpawnCircle);
                }
            }
            ToolMode::SpawnBox | ToolMode::SpawnCircle | ToolMode::SpawnPolygon => {
                self.spawn_shape_at(world_pos, self.tool);
            }
            ToolMode::Explosion => {}
        }
    }

    pub fn on_pointer_moved(&mut self, world_pos: Vec2) {
        let now = self.input_clock.elapsed().as_secs_f64();
        let input_dt = (now - self.last_pointer_sample).clamp(1.0 / 240.0, 0.1) as f32;
        self.last_pointer_sample = now;
        let delta = world_pos - self.last_pointer_world;
        self.last_pointer_world = world_pos;
        self.pointer_in_arena = world_pos.x >= 0.0
            && world_pos.x <= WORLD_WIDTH
            && world_pos.y >= 0.0
            && world_pos.y <= WORLD_HEIGHT;
        if self.pointer_down {
            self.gesture_tracker.push(now, world_pos);
        }

        if self.scene == SceneKind::ClothSoftBody {
            if self.pointer_down {
                if self.pointer_right {
                    if let Some(cloth) = self.cloth.as_mut() {
                        cloth.tear_at(world_pos, 0.5);
                    }
                } else if let Some(cloth) = self.cloth.as_mut() {
                    if let Some((_, ref mut target)) = cloth.grab_node {
                        *target = world_pos;
                    }
                }
            }
            return;
        }

        if self.scene == SceneKind::UiSprings {
            if self.dragging_scroller {
                self.ui_scroller.drag_by(-delta.x * 1.15);
            } else if let Some(idx) = self.active_card_idx {
                self.ui_cards[idx].card.drag_to(world_pos, input_dt);
                let vel = self.gesture_tracker.velocity();
                self.ui_projected_slot = self.ui_snap.choose_anchor(world_pos, vel).map(|(s, _)| s);
            }
            return;
        }

        if let Some(jh) = self.drag_joint {
            self.world.set_target_joint_position(jh, world_pos);
        }
    }

    pub fn on_pointer_released(&mut self) {
        self.pointer_down = false;
        self.pointer_right = false;
        self.ui_projected_slot = None;

        if let Some(cloth) = self.cloth.as_mut() {
            cloth.grab_node = None;
        }

        if self.scene == SceneKind::UiSprings {
            let vel = self.gesture_tracker.velocity();
            if self.dragging_scroller {
                self.dragging_scroller = false;
                self.ui_scroller.end_drag(-vel.x * 1.15);
            } else if let Some(idx) = self.active_card_idx.take() {
                let pos = self.ui_cards[idx].card.position.value();
                if let Some((slot, target)) = self.ui_snap.choose_anchor(pos, vel) {
                    self.ui_cards[idx].slot_index = slot;
                    self.ui_cards[idx].card.end_drag(vel, target);
                }
            }
            return;
        }

        if let Some(jh) = self.drag_joint.take() {
            self.world.remove_joint(jh);
        }
    }

    fn spawn_jelly_at(&mut self, pos: Vec2) {
        let colors = [0xCC34D399, 0xCCF472B6, 0xCC38BDF8, 0xCCFBBF24, 0xCCA78BFA];
        let col = colors[(self.spawn_counter as usize) % colors.len()];
        self.spawn_counter += 1;
        let bounds = Aabb2::new(
            Vec2::new(0.2, 0.2),
            Vec2::new(WORLD_WIDTH - 0.2, WORLD_HEIGHT - 0.78),
        );
        if self.soft_bodies.len() < 12 {
            self.soft_bodies.push(SoftBody2D::circle_blob(
                pos, 0.76, 16, 1.14, 0.00035, bounds, col,
            ));
        }
    }

    fn spawn_shape_at(&mut self, pos: Vec2, kind: ToolMode) {
        let colors = [
            0xFF38BDF8, 0xFFA78BFA, 0xFFF472B6, 0xFF34D399, 0xFFFBBF24, 0xFFFB7185,
        ];
        let color = colors[(self.spawn_counter as usize) % colors.len()];
        self.spawn_counter += 1;

        let shape = match kind {
            ToolMode::SpawnBox => Shape2D::rect(0.64, 0.64),
            ToolMode::SpawnPolygon => Shape2D::regular_polygon(6, 0.38),
            _ => Shape2D::circle(0.34),
        };
        let mut builder = BodyBuilder::dynamic(shape)
            .position(pos)
            .restitution(self.restitution)
            .friction(0.45)
            .user_data(color);
        if self.scene == SceneKind::LaserGravity {
            let center = Vec2::new(WORLD_WIDTH * 0.54, WORLD_HEIGHT * 0.5);
            let r = (pos - center).length().max(1.1);
            let tangent = (pos - center).perp().normalize_or_zero();
            let v_orbit = (48.0 / r).sqrt();
            builder = builder
                .linear_velocity(tangent * v_orbit)
                .linear_damping(0.0)
                .angular_damping(0.0)
                .can_sleep(false);
        }
        let h = self.world.add_body(builder);
        if self.scene == SceneKind::LaserGravity {
            let mut trail = VecDeque::with_capacity(72);
            trail.push_back(pos);
            self.orbit_trails.push((h, trail, color as u32));
        }
    }

    pub fn spawn_quick_burst(&mut self) {
        if self.scene == SceneKind::GpuParticles {
            self.load_scene(SceneKind::GpuParticles);
            return;
        }
        if self.scene == SceneKind::ClothSoftBody {
            self.spawn_jelly_at(Vec2::new(11.0, 1.5));
            return;
        }
        if self.scene == SceneKind::UiSprings {
            let anchors = self.ui_snap.anchors.clone();
            for (i, c) in self.ui_cards.iter_mut().enumerate() {
                let next_slot = (c.slot_index + 1) % anchors.len();
                c.slot_index = next_slot;
                c.card
                    .position
                    .set_velocity(Vec2::new((i as f32 - 1.5) * 8.0, -6.0));
                c.card.position.set_target(anchors[next_slot]);
            }
            self.ui_scroller.end_drag(18.0);
            return;
        }
        for i in 0..5 {
            let x = 3.0 + i as f32 * 2.3;
            let tool = match i % 3 {
                0 => ToolMode::SpawnBox,
                1 => ToolMode::SpawnCircle,
                _ => ToolMode::SpawnPolygon,
            };
            self.spawn_shape_at(Vec2::new(x, 1.2 + (i % 2) as f32 * FRAC_PI_2 * 0.2), tool);
        }
    }
}

impl Default for SandboxEngine {
    fn default() -> Self {
        Self::new()
    }
}
