//! Six interactive physics showcase scenes built on `ferroui-physics`:
//! 1. Rigid Body Playground (Games: pyramid, polygons, motorized windmill, chain bridge, spawning)
//! 2. GPU Compute Particles (3,072 colliding fluid/sand particles on Metal/Vulkan/DX12 wgpu or CPU SIMD)
//! 3. UI Physics & Micro-Interactions (DraggableCard springs + MagneticSnap + rubber-band InertiaScroller)
//! 4. Ragdoll & Wrecking Ball (Articulated ragdolls, swinging pendulum, breakable welded tower)
//! 5. Suspension Vehicle & Terrain (2D car with WheelJoint spring suspension + motorized drive)
//! 6. Laser Raycasting & Orbital Gravity Well (Multi-bounce raycast + Vortex/Radial force fields)

use ferroui_physics::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

/// Logical world dimensions in meters (16:10 widescreen coordinate space, Y-down).
pub const WORLD_WIDTH: f32 = 16.0;
pub const WORLD_HEIGHT: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    Playground,
    GpuParticles,
    UiSprings,
    RagdollWrecking,
    VehicleTerrain,
    LaserGravity,
}

impl SceneKind {
    pub const ALL: [SceneKind; 6] = [
        Self::Playground,
        Self::GpuParticles,
        Self::UiSprings,
        Self::RagdollWrecking,
        Self::VehicleTerrain,
        Self::LaserGravity,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Playground => "Playground",
            Self::GpuParticles => "GPU Particles",
            Self::UiSprings => "UI Springs",
            Self::RagdollWrecking => "Ragdoll",
            Self::VehicleTerrain => "Vehicle",
            Self::LaserGravity => "Ray & Orbit",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::Playground => "Drag bodies, spawn shapes, or right-click for shockwave blasts",
            Self::GpuParticles => "Move/hold pointer to swirl 3,072 particles; toggle GPU vs CPU SIMD",
            Self::UiSprings => "Drag & fling cards into magnetic dock slots, or flick the bottom scroller",
            Self::RagdollWrecking => "Wrecking ball smashes breakable welded beams & articulated ragdolls",
            Self::VehicleTerrain => "Motorized suspension car drives over ramps; click to drop obstacles",
            Self::LaserGravity => "6-bounce live raycast + orbital vortex well; drag or spawn satellites",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolMode {
    Drag,
    SpawnBox,
    SpawnCircle,
    SpawnPolygon,
    Explosion,
}

impl ToolMode {
    pub const ALL: [ToolMode; 5] = [
        Self::Drag,
        Self::SpawnBox,
        Self::SpawnCircle,
        Self::SpawnPolygon,
        Self::Explosion,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Drag => "Drag",
            Self::SpawnBox => "Box",
            Self::SpawnCircle => "Circle",
            Self::SpawnPolygon => "Hex",
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

/// Visual metadata for UI spring cards in `SceneKind::UiSprings`.
#[derive(Clone, Debug)]
pub struct UiDemoCard {
    pub label: &'static str,
    pub accent_argb: u32,
    pub half_size: Vec2,
    pub card: DraggableCard,
    pub slot_index: usize,
}

/// Live telemetry snapshot surfaced in the HUD bar.
#[derive(Clone, Debug, Default)]
pub struct TelemetrySnapshot {
    pub backend_label: String,
    pub step_ms: f32,
    pub entity_summary: String,
    pub gpu_available: bool,
}

/// Self-contained simulation engine state powering the `PhysicsSandbox` viewport.
pub struct SandboxEngine {
    pub scene: SceneKind,
    pub tool: ToolMode,
    pub spring_preset: SpringPreset,
    pub paused: bool,
    pub time_scale: f32,
    pub gravity_y: f32,
    pub restitution: f32,
    pub show_aabbs: bool,
    pub show_contacts: bool,
    pub show_velocities: bool,
    pub prefer_gpu: bool,

    // Core rigid-body simulation
    pub world: PhysicsWorld,
    drag_joint: Option<JointHandle>,

    // Particle / GPU compute simulation
    pub particles: ParticleSystem,
    pub gpu_particles_view: Vec<GpuParticle>,
    gpu_solver: Option<GpuParticleSolver>,
    gpu_synced: bool,

    // UI spring & inertia simulation
    pub ui_cards: Vec<UiDemoCard>,
    pub ui_snap: MagneticSnap,
    pub ui_scroller: InertiaScroller,
    active_card_idx: Option<usize>,
    dragging_scroller: bool,
    last_pointer_world: Vec2,
    pointer_down: bool,
    pointer_right: bool,
    gesture_tracker: GestureTracker,

    // Vehicle & laser state
    pub sim_time: f32,
    pub laser_segments: Vec<(Vec2, Vec2, Option<Vec2>)>,
    pub blast_ring: Option<(Vec2, f32)>,
    pub telemetry: TelemetrySnapshot,

    spawn_counter: u64,
}

impl SandboxEngine {
    pub fn new() -> Self {
        let gpu_solver = GpuParticleSolver::try_new(4096);
        let gpu_available = gpu_solver.is_some();
        let mut engine = Self {
            scene: SceneKind::Playground,
            tool: ToolMode::Drag,
            spring_preset: SpringPreset::Bouncy,
            paused: false,
            time_scale: 1.0,
            gravity_y: 9.81,
            restitution: 0.35,
            show_aabbs: false,
            show_contacts: false,
            show_velocities: false,
            prefer_gpu: gpu_available,
            world: PhysicsWorld::new(Vec2::new(0.0, 9.81)),
            drag_joint: None,
            particles: ParticleSystem::new(ParticleConfig::default()),
            gpu_particles_view: Vec::new(),
            gpu_solver,
            gpu_synced: false,
            ui_cards: Vec::new(),
            ui_snap: MagneticSnap::new(Vec::new()),
            ui_scroller: InertiaScroller::new(0.0, 12.0, 4.0),
            active_card_idx: None,
            dragging_scroller: false,
            last_pointer_world: Vec2::new(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5),
            pointer_down: false,
            pointer_right: false,
            gesture_tracker: GestureTracker::default(),
            sim_time: 0.0,
            laser_segments: Vec::new(),
            blast_ring: None,
            telemetry: TelemetrySnapshot {
                backend_label: SimdBackend::active().name().to_string(),
                step_ms: 0.0,
                entity_summary: String::new(),
                gpu_available,
            },
            spawn_counter: 1,
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
    }

    pub fn load_scene(&mut self, kind: SceneKind) {
        self.scene = kind;
        self.world.clear();
        self.drag_joint = None;
        self.active_card_idx = None;
        self.dragging_scroller = false;
        self.laser_segments.clear();
        self.blast_ring = None;
        self.sim_time = 0.0;
        self.gpu_synced = false;

        match kind {
            SceneKind::Playground => self.build_playground_scene(),
            SceneKind::GpuParticles => self.build_gpu_particles_scene(),
            SceneKind::UiSprings => self.build_ui_springs_scene(),
            SceneKind::RagdollWrecking => self.build_ragdoll_scene(),
            SceneKind::VehicleTerrain => self.build_vehicle_scene(),
            SceneKind::LaserGravity => self.build_laser_gravity_scene(),
        }
    }

    fn add_arena_walls(&mut self) {
        let wall_mat = PhysicsMaterial::new(0.0, 0.2, 0.65);
        // Floor
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::rect(WORLD_WIDTH + 1.0, 0.8))
                .position((WORLD_WIDTH * 0.5, WORLD_HEIGHT - 0.35))
                .material(wall_mat)
                .user_data(0xFF263042),
        );
        // Ceiling
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::rect(WORLD_WIDTH + 1.0, 0.8))
                .position((WORLD_WIDTH * 0.5, -0.4))
                .material(wall_mat)
                .user_data(0xFF263042),
        );
        // Left & Right walls
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::rect(0.8, WORLD_HEIGHT + 1.0))
                .position((-0.35, WORLD_HEIGHT * 0.5))
                .material(wall_mat)
                .user_data(0xFF263042),
        );
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::rect(0.8, WORLD_HEIGHT + 1.0))
                .position((WORLD_WIDTH + 0.35, WORLD_HEIGHT * 0.5))
                .material(wall_mat)
                .user_data(0xFF263042),
        );
    }

    fn build_playground_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = SolverConfig::game_default();
        self.add_arena_walls();

        // 1. Pyramid stack of boxes on the left
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

        // 2. Motorized windmill spinner in center-right
        let hub = self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.18))
                .position((10.8, 5.2))
                .user_data(0xFF94A3B8),
        );
        let blade = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::rect(3.4, 0.24))
                .position((10.8, 5.2))
                .density(2.0)
                .can_sleep(false)
                .user_data(0xFFF59E0B),
        );
        self.world.add_joint(Joint::revolute(hub, blade, Vec2::ZERO, Vec2::ZERO).with_motor(2.2, 85.0));

        // 3. Hanging suspension chain bridge at top
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

        // 4. Bouncy circles & convex polygons dropping onto the bridge and windmill
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
        let config = ParticleConfig {
            bounds: Aabb2::new(Vec2::new(0.25, 0.25), Vec2::new(WORLD_WIDTH - 0.25, WORLD_HEIGHT - 0.25)),
            gravity: Vec2::new(0.0, self.gravity_y),
            particle_radius: 0.078,
            damping: 0.996,
            restitution: 0.3,
            stiffness: 0.58,
            viscosity: 0.045,
            substeps: 2,
            pointer_center: Vec2::new(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5),
            pointer_strength: 0.0,
            pointer_radius: 3.2,
        };
        self.particles = ParticleSystem::new(config);
        self.particles.spawn_grid(Aabb2::new(Vec2::new(0.6, 0.6), Vec2::new(9.2, 8.8)), 3072);
        self.particles.obstacles = vec![
            ParticleObstacle { center: [5.5, 5.2], radius: 0.85, restitution: 0.5 },
            ParticleObstacle { center: [10.6, 6.4], radius: 1.05, restitution: 0.5 },
            ParticleObstacle { center: [12.8, 3.8], radius: 0.72, restitution: 0.5 },
        ];
        self.gpu_particles_view = self.particles.to_gpu_particles();
        if let Some(gpu) = self.gpu_solver.as_mut() {
            gpu.upload_particles(&self.gpu_particles_view);
            self.gpu_synced = true;
        }
    }

    fn build_ui_springs_scene(&mut self) {
        let anchors = vec![
            Vec2::new(3.4, 2.6),
            Vec2::new(8.0, 2.6),
            Vec2::new(12.6, 2.6),
            Vec2::new(5.7, 5.5),
            Vec2::new(10.3, 5.5),
        ];
        self.ui_snap = MagneticSnap::new(anchors.clone());
        let cfg = self.spring_preset.config();

        let defs = [
            ("Spring Card A", 0xFF38BDF8, 0),
            ("Tilt Physics B", 0xFFA78BFA, 1),
            ("Magnetic C", 0xFFF472B6, 2),
            ("Critically D", 0xFF34D399, 3),
        ];
        self.ui_cards = defs
            .into_iter()
            .map(|(label, accent_argb, slot_index)| {
                let mut card = DraggableCard::new(anchors[slot_index] + Vec2::new(0.0, -1.8), cfg);
                card.position.set_target(anchors[slot_index]);
                card.tilt_sensitivity = 0.035;
                UiDemoCard { label, accent_argb, half_size: Vec2::new(1.55, 0.92), card, slot_index }
            })
            .collect();

        self.ui_scroller = InertiaScroller::new(0.0, 12.0, 5.0);
        self.ui_scroller.detents = vec![0.0, 3.0, 6.0, 9.0, 12.0];
        self.ui_scroller.position = 3.0;
    }

    fn build_ragdoll_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = SolverConfig::high_precision();
        self.add_arena_walls();

        // Breakable tower of welded beams
        let beam_x = 10.6;
        let floor_y = WORLD_HEIGHT - 0.75;
        let mut prev_left: Option<BodyHandle> = None;
        let mut prev_right: Option<BodyHandle> = None;
        for level in 0..4 {
            let y_pillar = floor_y - 0.7 - level as f32 * 1.55;
            let left = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.26, 1.35))
                    .position((beam_x - 1.1, y_pillar))
                    .density(1.4)
                    .friction(0.7)
                    .user_data(0xFF60A5FA),
            );
            let right = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.26, 1.35))
                    .position((beam_x + 1.1, y_pillar))
                    .density(1.4)
                    .friction(0.7)
                    .user_data(0xFF60A5FA),
            );
            let top = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(2.7, 0.24))
                    .position((beam_x, y_pillar - 0.78))
                    .density(1.2)
                    .friction(0.7)
                    .user_data(0xFFFBBF24),
            );
            // Breakable welds hold the frame until the wrecking ball strikes!
            self.world.add_joint(
                Joint::weld(left, top, Vec2::new(0.0, -0.68), Vec2::new(-1.1, 0.12), 0.0).with_break_force(180.0),
            );
            self.world.add_joint(
                Joint::weld(right, top, Vec2::new(0.0, -0.68), Vec2::new(1.1, 0.12), 0.0).with_break_force(180.0),
            );
            if let (Some(pl), Some(pr)) = (prev_left, prev_right) {
                self.world.add_joint(
                    Joint::weld(pl, left, Vec2::new(0.0, -0.68), Vec2::new(0.0, 0.68), 0.0).with_break_force(160.0),
                );
                self.world.add_joint(
                    Joint::weld(pr, right, Vec2::new(0.0, -0.68), Vec2::new(0.0, 0.68), 0.0).with_break_force(160.0),
                );
            }
            prev_left = Some(left);
            prev_right = Some(right);
        }

        // Two articulated ragdolls standing near/on the tower
        self.spawn_ragdoll(Vec2::new(10.6, 2.2), -10);
        self.spawn_ragdoll(Vec2::new(13.4, 6.2), -11);

        // Heavy swinging wrecking ball raised high on the left!
        let crane = self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.22))
                .position((6.8, 0.9))
                .user_data(0xFF94A3B8),
        );
        let ball = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.72))
                .position((1.5, 2.1))
                .density(9.5)
                .restitution(0.35)
                .bullet(true)
                .user_data(0xFFEF4444),
        );
        self.world.add_joint(Joint::distance(crane, ball, Vec2::ZERO, Vec2::ZERO, 5.45));
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

        // Sculpted ramps and speed bumps across the arena
        let terrain_mat = PhysicsMaterial::new(0.0, 0.1, 0.95);
        for (x, y, w, h, ang) in [
            (4.8, 9.1, 3.2, 0.28, -0.22),
            (8.2, 8.95, 2.8, 0.28, 0.18),
            (11.6, 9.15, 3.0, 0.28, -0.25),
        ] {
            self.world.add_body(
                BodyBuilder::static_body(Shape2D::rect(w, h))
                    .position((x, y))
                    .angle(ang)
                    .material(terrain_mat)
                    .user_data(0xFF334155),
            );
        }
        // Small speed-bump rollers
        for i in 0..4 {
            self.world.add_body(
                BodyBuilder::static_body(Shape2D::circle(0.24))
                    .position((5.6 + i as f32 * 1.9, 9.32))
                    .material(terrain_mat)
                    .user_data(0xFF475569),
            );
        }

        // Vehicle chassis + two spring-suspended motorized wheels
        let car_filter = CollisionFilter::ALL.with_group(-20);
        let chassis = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::rect(2.3, 0.56))
                .position((2.4, 7.2))
                .density(1.8)
                .filter(car_filter)
                .can_sleep(false)
                .user_data(0xFF38BDF8),
        );
        let wheel_mat = PhysicsMaterial::new(2.2, 0.2, 1.4);
        let wheel_l = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.42))
                .position((1.55, 7.85))
                .material(wheel_mat)
                .filter(car_filter)
                .can_sleep(false)
                .user_data(0xFFF59E0B),
        );
        let wheel_r = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.42))
                .position((3.25, 7.85))
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

        // Stack of light crates at the right end for the car to crash through!
        for row in 0..4 {
            for col in 0..2 {
                self.world.add_body(
                    BodyBuilder::dynamic(Shape2D::rect(0.55, 0.55))
                        .position((13.6 + col as f32 * 0.62, 8.95 - row as f32 * 0.57))
                        .density(0.45)
                        .user_data(0xFF34D399),
                );
            }
        }
    }

    fn build_laser_gravity_scene(&mut self) {
        self.world.gravity = Vec2::ZERO;
        self.world.config = SolverConfig::game_default();
        self.add_arena_walls();

        let center = Vec2::new(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5);
        self.world.add_force_field(ForceField::Vortex {
            center,
            tangent_strength: 6.5,
            inward_strength: 16.0,
            radius: 7.5,
        });

        // Central static glowing star core
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.65))
                .position(center)
                .restitution(0.95)
                .user_data(0xFFFBBF24),
        );

        // Orbiting mirror satellites & convex crystals
        let colors = [0xFF38BDF8, 0xFFA78BFA, 0xFF34D399, 0xFFF472B6, 0xFFFB923C];
        for i in 0..14 {
            let ang = i as f32 * (2.0 * PI / 14.0);
            let dist = 2.4 + (i % 3) as f32 * 1.15;
            let pos = center + Vec2::new(ang.cos() * dist, ang.sin() * dist);
            let tangent = Vec2::new(-ang.sin(), ang.cos());
            let shape = match i % 3 {
                0 => Shape2D::rect(0.95, 0.24),
                1 => Shape2D::regular_polygon(6, 0.42),
                _ => Shape2D::circle(0.34),
            };
            self.world.add_body(
                BodyBuilder::dynamic(shape)
                    .position(pos)
                    .angle(ang)
                    .linear_velocity(tangent * 4.2)
                    .angular_velocity(0.9)
                    .linear_damping(0.02)
                    .angular_damping(0.02)
                    .restitution(0.92)
                    .can_sleep(false)
                    .user_data(colors[i % colors.len()]),
            );
        }
    }

    /// Advances the active scene by `dt` seconds.
    pub fn step(&mut self, raw_dt: f32) {
        if self.paused {
            return;
        }
        let dt = (raw_dt * self.time_scale).clamp(1e-4, 1.0 / 20.0);
        self.sim_time += dt;

        if let Some((_, ref mut r)) = self.blast_ring {
            *r += dt * 14.0;
            if *r > 5.0 {
                self.blast_ring = None;
            }
        }

        match self.scene {
            SceneKind::GpuParticles => self.step_particles(dt),
            SceneKind::UiSprings => self.step_ui_springs(dt),
            _ => self.step_rigid_world(dt),
        }
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

        // Update 6-bounce live sweeping laser in LaserGravity scene
        if self.scene == SceneKind::LaserGravity {
            self.update_laser_raycast();
        }

        self.telemetry = TelemetrySnapshot {
            backend_label: stats.simd_backend.map_or("CPU", |b| b.name()).to_string(),
            step_ms: stats.total_us as f32 / 1000.0,
            entity_summary: format!(
                "{} bodies ({} awake) · {} contacts · {} joints",
                stats.total_bodies, stats.active_bodies, stats.contact_points, stats.active_joints
            ),
            gpu_available: self.gpu_solver.is_some(),
        };
    }

    fn update_laser_raycast(&mut self) {
        self.laser_segments.clear();
        let mut origin = Vec2::new(0.8, 1.0 + (self.sim_time * 0.7).sin() * 0.5);
        let aim = self.last_pointer_world - origin;
        let mut dir = if aim.length_squared() > 0.25 {
            aim.normalize_or_zero()
        } else {
            let sweep = 0.45 + 0.35 * (self.sim_time * 0.6).sin();
            Vec2::new(sweep.cos(), sweep.sin())
        };

        for _bounce in 0..6 {
            let ray = Ray2::new(origin, dir);
            if let Some(hit) = self.world.raycast(ray, 25.0, CollisionFilter::ALL) {
                self.laser_segments.push((origin, hit.point, Some(hit.normal)));
                // Reflect off surface normal
                dir = (dir - hit.normal * (2.0 * dir.dot(hit.normal))).normalize_or_zero();
                origin = hit.point + hit.normal * 0.02;
            } else {
                self.laser_segments.push((origin, origin + dir * 22.0, None));
                break;
            }
        }
    }

    fn step_particles(&mut self, dt: f32) {
        self.particles.config.gravity = Vec2::new(0.0, self.gravity_y);
        self.particles.config.restitution = self.restitution;
        self.particles.config.pointer_center = self.last_pointer_world;
        self.particles.config.pointer_strength = if self.pointer_down {
            if self.pointer_right { -90.0 } else { 95.0 }
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
                // Transfer live GPU positions back into CPU SoA arrays seamlessly!
                self.particles.load_from_gpu_particles(&self.gpu_particles_view);
                self.gpu_synced = false;
            }
            let s = self.particles.step_cpu(dt);
            self.gpu_particles_view = self.particles.to_gpu_particles();
            s
        };

        self.telemetry = TelemetrySnapshot {
            backend_label: stats.backend_name.to_string(),
            step_ms: stats.step_us as f32 / 1000.0,
            entity_summary: format!("{} particles · 3 obstacles", stats.particle_count),
            gpu_available: self.gpu_solver.is_some(),
        };
    }

    fn step_ui_springs(&mut self, dt: f32) {
        let start = std::time::Instant::now();
        for c in &mut self.ui_cards {
            c.card.step(dt);
        }
        self.ui_scroller.step(dt);
        let us = start.elapsed().as_micros();
        self.telemetry = TelemetrySnapshot {
            backend_label: format!("Analytical UI Springs ({})", self.spring_preset.label()),
            step_ms: us as f32 / 1000.0,
            entity_summary: format!(
                "{} cards · scroll={:.2} · vel={:.1}",
                self.ui_cards.len(),
                self.ui_scroller.position,
                self.ui_scroller.velocity
            ),
            gpu_available: self.gpu_solver.is_some(),
        };
    }

    // -------------------------------------------------------------------------
    // Pointer Interaction Handlers (world coordinates)
    // -------------------------------------------------------------------------

    pub fn on_pointer_pressed(&mut self, world_pos: Vec2, right_button: bool) {
        self.pointer_down = true;
        self.pointer_right = right_button;
        self.last_pointer_world = world_pos;
        self.gesture_tracker.clear();
        self.gesture_tracker.push(self.sim_time as f64, world_pos);

        if self.scene == SceneKind::GpuParticles {
            return;
        }

        if self.scene == SceneKind::UiSprings {
            // Check bottom scroller band (y > 7.4)
            if world_pos.y > 7.4 {
                self.dragging_scroller = true;
                self.ui_scroller.begin_drag();
                return;
            }
            // Check cards top-to-bottom
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

        // Rigid-body scenes
        if right_button || self.tool == ToolMode::Explosion {
            self.world.apply_explosion(world_pos, 4.8, 16.0);
            self.blast_ring = Some((world_pos, 0.2));
            return;
        }

        match self.tool {
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
        if self.pointer_down {
            self.gesture_tracker.push(self.sim_time as f64, world_pos);
        }

        if self.scene == SceneKind::UiSprings {
            if self.dragging_scroller {
                self.ui_scroller.drag_by(-delta.x * 1.15);
            } else if let Some(idx) = self.active_card_idx {
                self.ui_cards[idx].card.drag_to(world_pos, 1.0 / 60.0);
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

    fn spawn_shape_at(&mut self, pos: Vec2, kind: ToolMode) {
        let colors = [0xFF38BDF8, 0xFFA78BFA, 0xFFF472B6, 0xFF34D399, 0xFFFBBF24, 0xFFFB7185];
        let color = colors[(self.spawn_counter as usize) % colors.len()];
        self.spawn_counter += 1;

        let shape = match kind {
            ToolMode::SpawnBox => Shape2D::rect(0.62, 0.62),
            ToolMode::SpawnPolygon => Shape2D::regular_polygon(6, 0.38),
            _ => Shape2D::circle(0.34),
        };
        let mut builder = BodyBuilder::dynamic(shape)
            .position(pos)
            .restitution(self.restitution)
            .friction(0.45)
            .user_data(color);
        if self.scene == SceneKind::LaserGravity {
            let center = Vec2::new(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5);
            let tangent = (pos - center).perp().normalize_or_zero();
            builder = builder.linear_velocity(tangent * 4.5).can_sleep(false);
        }
        self.world.add_body(builder);
    }

    pub fn spawn_quick_burst(&mut self) {
        if self.scene == SceneKind::GpuParticles {
            self.load_scene(SceneKind::GpuParticles);
            return;
        }
        if self.scene == SceneKind::UiSprings {
            // Shuffle cards across magnetic slots with energetic spring fling!
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
