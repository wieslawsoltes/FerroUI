//! Stable scene identities, descriptions and user-selectable presets.

use ferroui_physics::{ui::SpringConfig, SolverConfig};

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

    /// Stable key for reproducible startup and automated smoke runs.
    pub fn key(self) -> &'static str {
        match self {
            Self::Playground => "playground",
            Self::GpuParticles => "particles",
            Self::ClothSoftBody => "cloth",
            Self::UiSprings => "springs",
            Self::RagdollWrecking => "destruction",
            Self::VehicleTerrain => "vehicle",
            Self::CharacterDemo => "character",
            Self::LaserGravity => "orbit",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Playground => "Build, drag and break a structure. Inspect contacts, joints and collision bounds.",
            Self::GpuParticles => "Stir a particle pool and compare CPU and GPU compute at different particle counts.",
            Self::ClothSoftBody => "Pull a curtain, cut its fibers, and compare soft-body responses to wind and gravity.",
            Self::UiSprings => "Tune a spring response, snap draggable cards, and inspect inertial scrolling.",
            Self::RagdollWrecking => "Test breakable beams, fracture crates and observe an articulated ragdoll.",
            Self::VehicleTerrain => "Follow a suspended vehicle over ramps, bumps and a buoyancy pool.",
            Self::CharacterDemo => "Guide a character across slopes and moving platforms; click to jump.",
            Self::LaserGravity => "Trace reflected rays through an orbital scene and inspect proximity queries.",
        }
    }

    pub fn supports_history(self) -> bool {
        matches!(
            self,
            Self::Playground | Self::RagdollWrecking | Self::VehicleTerrain | Self::LaserGravity
        )
    }

    pub fn supports_body_tools(self) -> bool {
        !matches!(
            self,
            Self::GpuParticles | Self::UiSprings | Self::CharacterDemo
        )
    }

    /// Tools offered by a workspace; scenes without a tool strip keep the neutral Drag mode.
    pub fn supports_tool(self, tool: ToolMode) -> bool {
        match self {
            Self::ClothSoftBody => matches!(tool, ToolMode::Drag | ToolMode::SpawnJelly),
            Self::Playground
            | Self::RagdollWrecking
            | Self::VehicleTerrain
            | Self::LaserGravity => tool != ToolMode::SpawnJelly,
            _ => tool == ToolMode::Drag,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Playground => "Playground",
            Self::GpuParticles => "Particle pool",
            Self::ClothSoftBody => "Cloth & Jelly",
            Self::UiSprings => "Spring laboratory",
            Self::RagdollWrecking => "Destruction",
            Self::VehicleTerrain => "Vehicle",
            Self::CharacterDemo => "Character",
            Self::LaserGravity => "Ray & Orbit",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::Playground => "Drag bodies, spawn shapes, use Shatter tool, or right-click for shockwaves",
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
