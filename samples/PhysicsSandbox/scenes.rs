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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    Playground,
    GpuParticles,
    ClothSoftBody,
    UiSprings,
    RagdollWrecking,
    VehicleTerrain,
    CharacterDemo,
    LaserGravity,
}

impl SceneKind {
    pub const ALL: [SceneKind; 8] = [
        Self::Playground,
        Self::GpuParticles,
        Self::ClothSoftBody,
        Self::UiSprings,
        Self::RagdollWrecking,
        Self::VehicleTerrain,
        Self::CharacterDemo,
        Self::LaserGravity,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Playground => "Playground",
            Self::GpuParticles => "GPU Fluid",
            Self::ClothSoftBody => "Cloth & Jelly",
            Self::UiSprings => "UI Studio",
            Self::RagdollWrecking => "Destruction",
            Self::VehicleTerrain => "Vehicle",
            Self::CharacterDemo => "Character",
            Self::LaserGravity => "Ray & Orbit",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::Playground => "Drag bodies, spawn shapes/jelly, use Shatter tool, or right-click for shockwaves",
            Self::GpuParticles => "Hold left/right mouse to swirl or blast particles; switch 2K/4K/8K & GPU vs CPU SIMD",
            Self::ClothSoftBody => "Left-drag to pull cloth & drop jelly blobs; right-drag to slice/tear the cloth mesh!",
            Self::UiSprings => "Drag & fling cards into magnetic slots, sweep cursor for SpringChain ribbon, flick scroller",
            Self::RagdollWrecking => "Wrecking ball shatters crates, snaps welded beams & launches articulated ragdolls",
            Self::VehicleTerrain => "Spring-suspended car drives over ramps & splashes into a buoyant water pool",
            Self::CharacterDemo => "Move pointer to guide CharacterController2D (click to jump) across slopes & platforms",
            Self::LaserGravity => "6-bounce live laser raycast + orbital vortex + closest_point proximity tether",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolMode {
    Drag,
    SpawnBox,
    SpawnCircle,
    SpawnPolygon,
    SpawnJelly,
    Shatter,
    Explosion,
}

impl ToolMode {
    pub const ALL: [ToolMode; 7] = [
        Self::Drag,
        Self::SpawnBox,
        Self::SpawnCircle,
        Self::SpawnPolygon,
        Self::SpawnJelly,
        Self::Shatter,
        Self::Explosion,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Drag => "Drag",
            Self::SpawnBox => "Box",
            Self::SpawnCircle => "Circle",
            Self::SpawnPolygon => "Hex",
            Self::SpawnJelly => "Jelly",
            Self::Shatter => "Shatter",
            Self::Explosion => "Blast",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpringPreset {
    Snappy,
    Bouncy,
    Gentle,
    Wobbly,
    Critical,
}

impl SpringPreset {
    pub const ALL: [SpringPreset; 5] = [
        Self::Snappy,
        Self::Bouncy,
        Self::Gentle,
        Self::Wobbly,
        Self::Critical,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Snappy => "Snappy",
            Self::Bouncy => "Bouncy",
            Self::Gentle => "Gentle",
            Self::Wobbly => "Wobbly",
            Self::Critical => "Critical",
        }
    }

    pub fn config(self) -> SpringConfig {
        match self {
            Self::Snappy => SpringConfig::snappy(),
            Self::Bouncy => SpringConfig::bouncy(),
            Self::Gentle => SpringConfig::gentle(),
            Self::Wobbly => SpringConfig::wobbly(),
            Self::Critical => SpringConfig::critically_damped(0.32),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolverProfile {
    Game,
    Precision,
    UiFast,
}

impl SolverProfile {
    pub const ALL: [SolverProfile; 3] = [Self::Game, Self::Precision, Self::UiFast];

    pub fn label(self) -> &'static str {
        match self {
            Self::Game => "Game",
            Self::Precision => "Precision",
            Self::UiFast => "UI Fast",
        }
    }

    pub fn config(self) -> SolverConfig {
        match self {
            Self::Game => SolverConfig::game_default(),
            Self::Precision => SolverConfig::high_precision(),
            Self::UiFast => SolverConfig::ui_realtime(),
        }
    }
}

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
}

impl SandboxEngine {
    pub fn new() -> Self {
        let gpu_solver = GpuParticleSolver::try_new(8192);
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
            ui_ribbon: SpringChain::new(12, Vec2::new(8.0, 5.0), SpringConfig::from_response_damping(0.18, 0.78)),
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
        };
        engine.load_scene(SceneKind::Playground);
        engine
    }

    pub fn gpu_available(&self) -> bool {
        self.gpu_solver.is_some()
    }

    pub fn gpu_adapter_name(&self) -> Option<&str> {
        self.gpu_solver.as_ref().map(|g| g.device_info().adapter_name.as_str())
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
        self.telemetry.history_frame = self.history.cursor();
        self.telemetry.history_len = self.history.len();
    }

    pub fn step_forward_one(&mut self) {
        if self.history.step_forward(&mut self.world) {
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
            self.telemetry.history_frame = self.history.cursor();
            self.telemetry.history_len = self.history.len();
        }
    }

    pub fn load_scene(&mut self, kind: SceneKind) {
        self.scene = kind;
        self.world.clear();
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
        self.history.record(&self.world);
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

    fn build_playground_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = self.solver_profile.config();
        self.add_arena_walls();

        let palette = [0xFF38BDF8, 0xFF818CF8, 0xFFF472B6, 0xFF34D399, 0xFFFBBF24, 0xFFFB7185];
        let base_x = 3.8;
        let base_y = WORLD_HEIGHT - 1.05;
        let size = 0.56;
        let mut color_i = 0usize;
        for row in 0..6 {
            let cols = 6 - row;
            let start_x = base_x - (cols as f32 - 1.0) * (size * 0.52);
            for col in 0..cols {
                self.world.add_body(
                    BodyBuilder::dynamic(Shape2D::rect(size, size))
                        .position((start_x + col as f32 * (size * 1.04), base_y - row as f32 * (size + 0.01)))
                        .friction(0.6)
                        .restitution(self.restitution * 0.4)
                        .user_data(palette[color_i % palette.len()]),
                );
                color_i += 1;
            }
        }

        // Motorized windmill spinner
        let hub = self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.18))
                .position((10.8, 5.4))
                .user_data(0xFF94A3B8),
        );
        let blade = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::rect(3.5, 0.24))
                .position((10.8, 5.4))
                .density(2.0)
                .can_sleep(false)
                .user_data(0xFFF59E0B),
        );
        self.world.add_joint(Joint::revolute(hub, blade, Vec2::ZERO, Vec2::ZERO).with_motor(2.2, 85.0));

        // Hanging chain bridge
        let left_anchor = self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.15))
                .position((7.0, 2.0))
                .user_data(0xFF94A3B8),
        );
        let right_anchor = self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.15))
                .position((14.2, 2.0))
                .user_data(0xFF94A3B8),
        );
        let links = 8;
        let link_w = 0.78;
        let mut prev = left_anchor;
        let mut prev_local = Vec2::ZERO;
        for i in 0..links {
            let x = 7.45 + i as f32 * 0.85;
            let plank = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(link_w, 0.16))
                    .position((x, 2.1))
                    .density(1.2)
                    .user_data(0xFF2DD4BF),
            );
            self.world.add_joint(Joint::revolute(prev, plank, prev_local, Vec2::new(-link_w * 0.46, 0.0)));
            prev = plank;
            prev_local = Vec2::new(link_w * 0.46, 0.0);
        }
        self.world.add_joint(Joint::revolute(prev, right_anchor, prev_local, Vec2::ZERO));

        for i in 0..7 {
            let x = 8.0 + i as f32 * 0.95;
            let shape = match i % 3 {
                0 => Shape2D::circle(0.32),
                1 => Shape2D::regular_polygon(5, 0.36),
                _ => Shape2D::regular_polygon(6, 0.35),
            };
            self.world.add_body(
                BodyBuilder::dynamic(shape)
                    .position((x, 0.9 + (i % 2) as f32 * 0.35))
                    .restitution(0.65)
                    .friction(0.35)
                    .user_data(palette[(i + 2) % palette.len()]),
            );
        }
    }

    fn build_gpu_particles_scene(&mut self) {
        let radius = if self.particle_target_count >= 8000 {
            0.056
        } else if self.particle_target_count >= 4000 {
            0.070
        } else {
            0.088
        };
        let config = ParticleConfig {
            bounds: Aabb2::new(Vec2::new(0.25, 0.25), Vec2::new(WORLD_WIDTH - 0.25, WORLD_HEIGHT - 0.25)),
            gravity: Vec2::new(0.0, self.gravity_y),
            particle_radius: radius,
            damping: 0.996,
            restitution: 0.3,
            stiffness: 0.58,
            viscosity: 0.045,
            substeps: 2,
            pointer_center: Vec2::new(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5),
            pointer_strength: 0.0,
            pointer_radius: 3.4,
        };
        self.particles = ParticleSystem::new(config);
        self.particles
            .spawn_grid(Aabb2::new(Vec2::new(0.55, 0.55), Vec2::new(10.0, 9.0)), self.particle_target_count);
        self.particles.obstacles = vec![
            ParticleObstacle { center: [5.4, 5.1], radius: 0.85, restitution: 0.5 },
            ParticleObstacle { center: [10.4, 6.4], radius: 1.05, restitution: 0.5 },
            ParticleObstacle { center: [12.9, 3.8], radius: 0.72, restitution: 0.5 },
        ];
        self.gpu_particles_view = self.particles.to_gpu_particles();
        if let Some(gpu) = self.gpu_solver.as_mut() {
            gpu.upload_particles(&self.gpu_particles_view);
            self.gpu_synced = true;
        }
    }

    fn build_cloth_softbody_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = SolverConfig::game_default();
        self.add_arena_walls();

        let bounds = Aabb2::new(Vec2::new(0.2, 0.2), Vec2::new(WORLD_WIDTH - 0.2, WORLD_HEIGHT - 0.78));
        // Tearable XPBD hanging cloth curtain on the left
        self.cloth = Some(ClothMesh::grid(
            Vec2::new(1.2, 1.0),
            16,
            11,
            0.36,
            0.00018,
            Some(2.05),
            bounds,
        ));

        // Rigid bumper pegs & ramps on the right for jelly soft bodies to squish over!
        for &(x, y, r) in &[(10.2, 5.2, 0.65), (12.8, 6.4, 0.72), (8.6, 7.6, 0.55)] {
            self.world.add_body(
                BodyBuilder::static_body(Shape2D::circle(r))
                    .position((x, y))
                    .user_data(0xFF334155),
            );
        }
        // Slanted ramp under the jelly drop zone
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::rect(3.4, 0.22))
                .position((11.6, 8.2))
                .angle(-0.22)
                .user_data(0xFF475569),
        );
        // Dynamic ball swinging into cloth
        let ball = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.52))
                .position((3.6, 6.8))
                .linear_velocity((3.2, -2.0))
                .restitution(0.75)
                .user_data(0xFFF59E0B),
        );
        let _ = ball;

        // Three colorful pressure-inflated XPBD jelly blobs!
        self.soft_bodies = vec![
            SoftBody2D::circle_blob(Vec2::new(10.0, 2.2), 0.92, 18, 1.15, 0.00035, bounds, 0xCC34D399),
            SoftBody2D::circle_blob(Vec2::new(12.5, 2.6), 0.85, 18, 1.12, 0.0004, bounds, 0xCCF472B6),
            SoftBody2D::circle_blob(Vec2::new(11.3, 1.1), 0.78, 16, 1.18, 0.0003, bounds, 0xCC38BDF8),
        ];
    }

    fn build_ui_springs_scene(&mut self) {
        let anchors = vec![
            Vec2::new(2.7, 2.55),
            Vec2::new(6.3, 2.55),
            Vec2::new(9.9, 2.55),
            Vec2::new(4.5, 5.45),
            Vec2::new(8.1, 5.45),
        ];
        self.ui_snap = MagneticSnap::new(anchors.clone());
        let cfg = self.spring_preset.config();

        let defs = [
            ("PiP Player", "Magnetic Corner", 0xFF0284C7, 0),
            ("Kanban Card", "Velocity Tilt", 0xFF7C3AED, 1),
            ("Wallet Pass", "Spring Reflow", 0xFFDB2777, 2),
            ("Quick Action", "Analytical ODE", 0xFF059669, 3),
        ];
        self.ui_cards = defs
            .into_iter()
            .map(|(label, subtitle, accent_argb, slot_index)| {
                let mut card = DraggableCard::new(anchors[slot_index] + Vec2::new(0.0, -1.5), cfg);
                card.position.set_target(anchors[slot_index]);
                card.tilt_sensitivity = 0.032;
                UiDemoCard { label, subtitle, accent_argb, half_size: Vec2::new(1.48, 0.88), card, slot_index }
            })
            .collect();

        // Expandable Radial Spring FAB Menu on the right side
        self.ui_fab_center = Vec2::new(13.6, 5.45);
        self.ui_fab_open = true;
        let fab_defs = [
            ("Cam", 0xFF38BDF8),
            ("Mic", 0xFFA78BFA),
            ("Pin", 0xFFF472B6),
            ("Tag", 0xFF34D399),
            ("Zap", 0xFFF59E0B),
        ];
        self.ui_fab_items = fab_defs
            .into_iter()
            .enumerate()
            .map(|(i, (lbl, col))| {
                let ang = -PI + (i as f32) * (PI / 4.0);
                let target = self.ui_fab_center + Vec2::new(ang.cos() * 1.45, ang.sin() * 1.45);
                let mut sp = Spring2D::new(self.ui_fab_center, cfg);
                sp.set_target(target);
                (sp, lbl, col)
            })
            .collect();
        self.recompute_spring_curve();

        self.ui_scroller = InertiaScroller::new(0.0, 12.0, 5.0);
        self.ui_scroller.detents = vec![0.0, 3.0, 6.0, 9.0, 12.0];
        self.ui_scroller.position = 3.0;
        self.ui_ribbon = SpringChain::new(10, Vec2::new(6.3, 4.0), SpringConfig::from_response_damping(0.15, 0.78));
    }

    fn build_ragdoll_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = SolverConfig::high_precision();
        self.add_arena_walls();

        let beam_x = 10.6;
        let floor_y = WORLD_HEIGHT - 0.75;
        let mut prev_left: Option<BodyHandle> = None;
        let mut prev_right: Option<BodyHandle> = None;
        for level in 0..4 {
            let y_pillar = floor_y - 0.7 - level as f32 * 1.55;
            let left = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.28, 1.35))
                    .position((beam_x - 1.15, y_pillar))
                    .density(1.4)
                    .friction(0.7)
                    .user_data(0xFF60A5FA),
            );
            let right = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.28, 1.35))
                    .position((beam_x + 1.15, y_pillar))
                    .density(1.4)
                    .friction(0.7)
                    .user_data(0xFF60A5FA),
            );
            let top = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(2.8, 0.26))
                    .position((beam_x, y_pillar - 0.78))
                    .density(1.2)
                    .friction(0.7)
                    .user_data(0xFFFBBF24),
            );
            // Glass-like shatterable core crate inside each bay!
            let crate_h = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.95, 0.95))
                    .position((beam_x, y_pillar + 0.12))
                    .density(0.9)
                    .user_data(0xFF38BDF8),
            );
            self.shatter_candidates.push(crate_h);

            self.world.add_joint(
                Joint::weld(left, top, Vec2::new(0.0, -0.68), Vec2::new(-1.15, 0.13), 0.0).with_break_force(175.0),
            );
            self.world.add_joint(
                Joint::weld(right, top, Vec2::new(0.0, -0.68), Vec2::new(1.15, 0.13), 0.0).with_break_force(175.0),
            );
            if let (Some(pl), Some(pr)) = (prev_left, prev_right) {
                self.world.add_joint(
                    Joint::weld(pl, left, Vec2::new(0.0, -0.68), Vec2::new(0.0, 0.68), 0.0).with_break_force(155.0),
                );
                self.world.add_joint(
                    Joint::weld(pr, right, Vec2::new(0.0, -0.68), Vec2::new(0.0, 0.68), 0.0).with_break_force(155.0),
                );
            }
            prev_left = Some(left);
            prev_right = Some(right);
        }

        self.spawn_ragdoll(Vec2::new(10.6, 2.0), -10);
        self.spawn_ragdoll(Vec2::new(13.6, 6.2), -11);

        let crane = self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.22))
                .position((6.8, 0.9))
                .user_data(0xFF94A3B8),
        );
        let ball = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.75))
                .position((1.4, 2.0))
                .density(10.5)
                .restitution(0.35)
                .bullet(true)
                .user_data(0xFFEF4444),
        );
        self.world.add_joint(Joint::distance(crane, ball, Vec2::ZERO, Vec2::ZERO, 5.5));
    }

    fn spawn_ragdoll(&mut self, origin: Vec2, group: i16) {
        let filter = CollisionFilter::ALL.with_group(group);
        let head = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.24))
                .position(origin + Vec2::new(0.0, -0.68))
                .filter(filter)
                .user_data(0xFFFDE68A),
        );
        let torso = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::capsule(0.32, 0.22))
                .position(origin)
                .filter(filter)
                .user_data(0xFFEC4899),
        );
        let arm_l = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::capsule(0.24, 0.1))
                .position(origin + Vec2::new(-0.42, -0.15))
                .angle(0.4)
                .filter(filter)
                .user_data(0xFFF472B6),
        );
        let arm_r = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::capsule(0.24, 0.1))
                .position(origin + Vec2::new(0.42, -0.15))
                .angle(-0.4)
                .filter(filter)
                .user_data(0xFFF472B6),
        );
        let leg_l = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::capsule(0.30, 0.11))
                .position(origin + Vec2::new(-0.18, 0.72))
                .filter(filter)
                .user_data(0xFF818CF8),
        );
        let leg_r = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::capsule(0.30, 0.11))
                .position(origin + Vec2::new(0.18, 0.72))
                .filter(filter)
                .user_data(0xFF818CF8),
        );

        self.world.add_joint(
            Joint::revolute(torso, head, Vec2::new(0.0, -0.45), Vec2::new(0.0, 0.22)).with_limits(-0.5, 0.5),
        );
        self.world.add_joint(
            Joint::revolute(torso, arm_l, Vec2::new(-0.22, -0.32), Vec2::new(0.0, -0.22)).with_limits(-1.8, 1.8),
        );
        self.world.add_joint(
            Joint::revolute(torso, arm_r, Vec2::new(0.22, -0.32), Vec2::new(0.0, -0.22)).with_limits(-1.8, 1.8),
        );
        self.world.add_joint(
            Joint::revolute(torso, leg_l, Vec2::new(-0.15, 0.42), Vec2::new(0.0, -0.28)).with_limits(-1.1, 1.1),
        );
        self.world.add_joint(
            Joint::revolute(torso, leg_r, Vec2::new(0.15, 0.42), Vec2::new(0.0, -0.28)).with_limits(-1.1, 1.1),
        );
    }

    fn build_vehicle_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = SolverConfig::high_precision();
        self.add_arena_walls();

        // Buoyant water pool on the right side!
        self.world.add_force_field(ForceField::Buoyancy {
            region: Aabb2::new(Vec2::new(10.8, 7.2), Vec2::new(15.6, 9.6)),
            fluid_density: 2.2,
            linear_drag: 2.4,
            angular_drag: 2.0,
        });

        let terrain_mat = PhysicsMaterial::new(0.0, 0.1, 0.95);
        for (x, y, w, h, ang) in [
            (4.8, 9.1, 3.2, 0.28, -0.22),
            (8.2, 8.95, 2.8, 0.28, 0.18),
        ] {
            self.world.add_body(
                BodyBuilder::static_body(Shape2D::rect(w, h))
                    .position((x, y))
                    .angle(ang)
                    .material(terrain_mat)
                    .user_data(0xFF334155),
            );
        }
        for i in 0..3 {
            self.world.add_body(
                BodyBuilder::static_body(Shape2D::circle(0.24))
                    .position((5.4 + i as f32 * 1.9, 9.32))
                    .material(terrain_mat)
                    .user_data(0xFF475569),
            );
        }

        let car_filter = CollisionFilter::ALL.with_group(-20);
        let chassis = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::rect(2.3, 0.56))
                .position((2.3, 7.1))
                .density(1.5)
                .filter(car_filter)
                .can_sleep(false)
                .user_data(0xFF38BDF8),
        );
        let wheel_mat = PhysicsMaterial::new(2.2, 0.2, 1.4);
        let wheel_l = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.42))
                .position((1.45, 7.75))
                .material(wheel_mat)
                .filter(car_filter)
                .can_sleep(false)
                .user_data(0xFFF59E0B),
        );
        let wheel_r = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.42))
                .position((3.15, 7.75))
                .material(wheel_mat)
                .filter(car_filter)
                .can_sleep(false)
                .user_data(0xFFF59E0B),
        );

        self.world.add_joint(
            Joint::wheel(chassis, wheel_l, Vec2::new(-0.85, 0.55), Vec2::ZERO, Vec2::Y, 5.5, 0.75)
                .with_motor(11.0, 55.0),
        );
        self.world.add_joint(
            Joint::wheel(chassis, wheel_r, Vec2::new(0.85, 0.55), Vec2::ZERO, Vec2::Y, 5.5, 0.75)
                .with_motor(11.0, 55.0),
        );

        // Floating wooden crates bobbing in the water pool!
        for i in 0..4 {
            self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.62, 0.62))
                    .position((11.6 + i as f32 * 0.95, 7.4 - (i % 2) as f32 * 0.6))
                    .density(0.65)
                    .can_sleep(false)
                    .user_data(0xFF34D399),
            );
        }
    }

    fn build_character_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = SolverConfig::game_default();
        self.add_arena_walls();

        // Walkable slope ramps + floating platforms
        for (x, y, w, h, ang) in [
            (4.4, 8.6, 4.2, 0.35, -0.32),
            (9.2, 7.8, 3.2, 0.35, 0.0),
            (13.2, 6.4, 3.4, 0.35, -0.25),
            (3.2, 5.2, 3.2, 0.3, 0.0),
        ] {
            self.world.add_body(
                BodyBuilder::static_body(Shape2D::rect(w, h))
                    .position((x, y))
                    .angle(ang)
                    .user_data(0xFF334155),
            );
        }

        // Moving kinematic elevator platform
        let plat_origin = Vec2::new(7.4, 5.4);
        let plat = self.world.add_body(
            BodyBuilder::kinematic(Shape2D::rect(2.4, 0.28))
                .position(plat_origin)
                .user_data(0xFF0EA5E9),
        );
        self.moving_platform = Some((plat, plat_origin));

        // Pushable dynamic physics crates on platforms
        for (x, y, col) in [
            (9.4, 7.1, 0xFFF59E0B),
            (10.1, 7.1, 0xFFFB7185),
            (9.75, 6.4, 0xFFA78BFA),
            (2.8, 4.6, 0xFF34D399),
        ] {
            self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.58, 0.58))
                    .position((x, y))
                    .density(0.8)
                    .user_data(col),
            );
        }

        self.character = Some(CharacterController2D::new(Shape2D::capsule(0.36, 0.26), Vec2::new(2.0, 8.0)));
    }

    fn build_laser_gravity_scene(&mut self) {
        self.world.gravity = Vec2::ZERO;
        self.world.config = SolverConfig::high_precision();
        self.add_arena_walls();

        let center = Vec2::new(WORLD_WIDTH * 0.52, WORLD_HEIGHT * 0.5);
        let mu = 52.0_f32;
        self.world.add_force_field(ForceField::NewtonianGravity {
            center,
            mu,
            min_radius: 1.0,
            max_radius: 4.75,
        });

        // Glowing central star
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.68))
                .position(center)
                .restitution(0.95)
                .user_data(0xFFFBBF24),
        );

        // Planets & reflective crystal satellites on exact Keplerian circular & elliptical orbits!
        let specs: &[(f32, f32, f32, u8, u32)] = &[
            // (orbit_radius, start_angle, ecc_scale, shape_kind, color)
            (1.65, 0.2, 1.0, 0, 0xFF38BDF8),
            (2.25, 2.1, 1.0, 1, 0xFFA78BFA),
            (2.85, 4.0, 0.96, 2, 0xFF34D399),
            (3.40, 0.9, 1.02, 0, 0xFFF472B6),
            (3.95, 3.3, 0.98, 1, 0xFFFB923C),
            (4.40, 5.2, 1.0, 2, 0xFF2DD4BF),
            (2.55, 5.5, 1.06, 0, 0xFFFDE047),
            (3.70, 1.8, 0.93, 1, 0xFF818CF8),
        ];

        for &(r, ang, ecc, kind, color) in specs {
            let pos = center + Vec2::new(ang.cos() * r, ang.sin() * r);
            let tangent = Vec2::new(-ang.sin(), ang.cos());
            // Exact circular orbital speed v = sqrt(mu / r)
            let v_orbit = (mu / r).sqrt() * ecc;
            let shape = match kind {
                0 => Shape2D::circle(0.28),
                1 => Shape2D::regular_polygon(6, 0.34),
                _ => Shape2D::rect(0.76, 0.20),
            };
            let h = self.world.add_body(
                BodyBuilder::dynamic(shape)
                    .position(pos)
                    .angle(ang)
                    .linear_velocity(tangent * v_orbit)
                    .angular_velocity(1.1)
                    .linear_damping(0.0)
                    .angular_damping(0.0)
                    .restitution(0.95)
                    .can_sleep(false)
                    .user_data(color as u64),
            );
            let mut trail = VecDeque::with_capacity(56);
            trail.push_back(pos);
            self.orbit_trails.push((h, trail, color as u32));
        }

        self.laser_origin = Vec2::new(0.45, WORLD_HEIGHT * 0.5);
    }

    pub fn step(&mut self, raw_dt: f32) {
        if self.paused {
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
        let inst_fps = if raw_dt > 1e-4 { (1.0 / raw_dt).clamp(1.0, 240.0) } else { 60.0 };
        self.telemetry.fps = self.telemetry.fps * 0.9 + inst_fps * 0.1;
    }

    fn step_rigid_world(&mut self, dt: f32) {
        if self.scene != SceneKind::LaserGravity {
            let target_g = Vec2::new(0.0, self.gravity_y);
            if (self.world.gravity - target_g).length_squared() > 1e-4 {
                self.world.gravity = target_g;
                self.world.wake_all();
            }
        }

        let stats = self.world.step(dt);

        // Automatic impact shattering in Destruction scene!
        if self.scene == SceneKind::RagdollWrecking && !self.shatter_candidates.is_empty() {
            let mut to_shatter: Vec<(BodyHandle, Vec2)> = Vec::new();
            for ev in self.world.collision_events() {
                if let CollisionEvent::Started { body_a, body_b, point, impact_speed, .. } = *ev {
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
                self.shatter_candidates.retain(|&x| x != h);
                fracture_body(&mut self.world, h, pt, 2.8);
            }
        }

        if self.scene == SceneKind::LaserGravity {
            // Update Keplerian orbital trails
            for (h, trail, _) in &mut self.orbit_trails {
                if let Some(b) = self.world.body(*h) {
                    if trail.back().map_or(true, |p| p.distance_squared(b.position) > 0.015) {
                        if trail.len() >= 56 {
                            trail.pop_front();
                        }
                        trail.push_back(b.position);
                    }
                }
            }
            self.update_laser_raycast();
            self.closest_tether = if self.pointer_in_arena {
                self.world
                    .closest_point(self.last_pointer_world, 6.0, CollisionFilter::ALL)
                    .map(|hit| (self.last_pointer_world, hit.point, hit.distance))
            } else {
                None
            };
        }

        self.history.record(&self.world);

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

        let bodies_snapshot: Vec<RigidBody> = self.world.bodies().cloned().collect();
        let body_refs: Vec<&RigidBody> = bodies_snapshot.iter().collect();
        let g = Vec2::new(0.0, self.gravity_y);
        let wind = Vec2::new(self.wind_x, 0.0);

        if let Some(cloth) = self.cloth.as_mut() {
            cloth.step(dt, g, wind, self.sim_time, &body_refs);
        }
        for sb in &mut self.soft_bodies {
            sb.step(dt, g, &body_refs);
        }

        let us = start.elapsed().as_micros();
        let cloth_links = self.cloth.as_ref().map_or(0, ClothMesh::active_constraint_count);
        self.telemetry = TelemetrySnapshot {
            backend_label: format!("XPBD Cloth + SoftBody ({})", SimdBackend::active().name()),
            step_ms: us as f32 / 1000.0,
            fps: self.telemetry.fps,
            breakdown: format!("XPBD 6 substeps · wind {:.1} m/s²", self.wind_x),
            entity_summary: format!("{} cloth links · {} jelly blobs", cloth_links, self.soft_bodies.len()),
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
                p.linear_velocity = (new_pos - p.position) / dt;
                p.set_position(new_pos);
            }
        }

        let stats = self.world.step(dt);

        let mut char_info = String::new();
        if let Some(ctrl) = self.character.as_mut() {
            // Guide character smoothly toward pointer X (or patrol gently if idle)
            let target_x = self.last_pointer_world.x.clamp(1.0, WORLD_WIDTH - 1.0);
            let dx = target_x - ctrl.position.x;
            let desired_vx = if dx.abs() > 0.25 { (dx * 3.5).clamp(-6.5, 6.5) } else { 0.0 };
            ctrl.velocity.x = ctrl.velocity.x + (desired_vx - ctrl.velocity.x) * (10.0 * dt).min(1.0);
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
                if res.is_grounded { "Grounded" } else { "Airborne" },
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
            entity_summary: format!("{} bodies · {} contacts", stats.total_bodies, stats.contact_points),
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
                self.laser_segments.push((origin, hit.point, Some(hit.normal)));
                dir = (dir - hit.normal * (2.0 * dir.dot(hit.normal))).normalize_or_zero();
                origin = hit.point + hit.normal * 0.03;
            } else {
                self.laser_segments.push((origin, origin + dir * 20.0, None));
                break;
            }
        }
    }

    fn step_particles(&mut self, dt: f32) {
        self.particles.config.gravity = Vec2::new(self.wind_x * 0.35, self.gravity_y);
        self.particles.config.restitution = self.restitution;
        self.particles.config.pointer_center = self.last_pointer_world;
        self.particles.config.pointer_strength = if self.pointer_down {
            if self.pointer_right { -105.0 } else { 110.0 }
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
                self.gpu_particles_view = self.particles.to_gpu_particles();
                s
            }
        } else {
            if self.gpu_synced {
                self.particles.load_from_gpu_particles(&self.gpu_particles_view);
                self.gpu_synced = false;
            }
            let s = self.particles.step_cpu(dt);
            self.gpu_particles_view = self.particles.to_gpu_particles();
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
            breakdown: format!("Throughput {:.2}M particles/s · O(N) atomic spatial hash", m_per_sec),
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
            Vec2::new(6.3 + (self.sim_time * 1.4).cos() * 2.2, 4.0 + (self.sim_time * 2.1).sin() * 0.8)
        };
        self.ui_ribbon.set_leader_target(ribbon_target);
        self.ui_ribbon.step(dt);
        let us = start.elapsed().as_micros();
        self.telemetry = TelemetrySnapshot {
            backend_label: format!("UI Physics Lab ({} · ζ={:.2})", self.spring_preset.label(), self.spring_preset.config().damping_ratio()),
            step_ms: us as f32 / 1000.0,
            fps: self.telemetry.fps,
            breakdown: format!("scroll={:.2} · vel={:.1} · FAB {}", self.ui_scroller.position, self.ui_scroller.velocity, if self.ui_fab_open { "Open" } else { "Closed" }),
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
        self.gesture_tracker.push(self.sim_time as f64, world_pos);

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
                        sp.set_target(self.ui_fab_center + Vec2::new(ang.cos() * 1.45, ang.sin() * 1.45));
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
                        let jh = self.world.add_joint(Joint::target(handle, local, world_pos, 450.0 * body.mass));
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
        let delta = world_pos - self.last_pointer_world;
        self.last_pointer_world = world_pos;
        self.pointer_in_arena = world_pos.x >= 0.0
            && world_pos.x <= WORLD_WIDTH
            && world_pos.y >= 0.0
            && world_pos.y <= WORLD_HEIGHT;
        if self.pointer_down {
            self.gesture_tracker.push(self.sim_time as f64, world_pos);
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
                self.ui_cards[idx].card.drag_to(world_pos, 1.0 / 60.0);
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
        let bounds = Aabb2::new(Vec2::new(0.2, 0.2), Vec2::new(WORLD_WIDTH - 0.2, WORLD_HEIGHT - 0.78));
        if self.soft_bodies.len() < 12 {
            self.soft_bodies.push(SoftBody2D::circle_blob(pos, 0.76, 16, 1.14, 0.00035, bounds, col));
        }
    }

    fn spawn_shape_at(&mut self, pos: Vec2, kind: ToolMode) {
        let colors = [0xFF38BDF8, 0xFFA78BFA, 0xFFF472B6, 0xFF34D399, 0xFFFBBF24, 0xFFFB7185];
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
            let center = Vec2::new(WORLD_WIDTH * 0.52, WORLD_HEIGHT * 0.5);
            let r = (pos - center).length().max(1.2);
            let tangent = (pos - center).perp().normalize_or_zero();
            let v_orbit = (52.0 / r).sqrt();
            builder = builder
                .linear_velocity(tangent * v_orbit)
                .linear_damping(0.0)
                .angular_damping(0.0)
                .can_sleep(false);
        }
        let h = self.world.add_body(builder);
        if self.scene == SceneKind::LaserGravity {
            let mut trail = VecDeque::with_capacity(56);
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
                c.card.position.set_velocity(Vec2::new((i as f32 - 1.5) * 8.0, -6.0));
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
