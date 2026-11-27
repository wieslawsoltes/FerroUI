//! # `ferroui-physics`
//!
//! A professional-grade, hardware-accelerated physics engine built as an independent crate for
//! games, interactive user interfaces, animations, particle/fluid effects, and general-purpose
//! 2D simulation.
//!
//! ## Architecture
//! - **Math & Shapes** ([`math`], [`shapes`], [`material`]): `#[repr(C)]` SIMD/GPU-friendly
//!   primitives (`Vec2`, `Vec3`, `Rot2`, `Transform2D`, `Aabb2`, `Ray2`), exact mass/inertia
//!   properties for `Circle`, `Box` (OBB), `Capsule`, `ConvexPolygon`, and `Segment`.
//! - **CPU SIMD Acceleration** ([`simd`]): Architecture-native ARM NEON (`aarch64`), AVX2/SSE2
//!   (`x86_64`), and scalar fallback Structure-of-Arrays integration and force-field kernels.
//! - **Broadphase & Narrowphase** ([`broadphase`], [`narrowphase`]): SAH Dynamic AABB Tree (BVH)
//!   with fat velocity-predicted AABBs + AVL rotations, Uniform Spatial Hash Grid, analytic SAT +
//!   Sutherland-Hodgman 2-point face clipping with persistent `FeatureId`s, GJK/EPA fallback, and
//!   swept Continuous Collision Detection (CCD).
//! - **Substepped Solver & Joints** ([`solver`], [`joints`], [`world`]): Substepped Sequential
//!   Impulse / Soft-Step solver with warm-starting, Coulomb friction, positional stabilization,
//!   sleeping, deterministic [`FixedTimestep`], and breakable `Distance`, `Spring`, `Revolute`,
//!   `Prismatic`, `Weld`, `Wheel`, `Rope`, and `Target` joints.
//! - **GPU Compute Particles** ([`particles`], [`gpu`]): Lock-free atomic O(N) spatial-hash GPU
//!   compute particle solver written in WGSL and executed on Metal, Vulkan, DX12, and WebGPU via
//!   `wgpu`, paired with an identical CPU SIMD fallback.
//! - **UI Animation & Interaction Physics** ([`ui`]): Exact closed-form analytical harmonic
//!   springs (`Spring1D`, `Spring2D`, `Spring3D`), iOS/macOS-style `InertiaScroller` with
//!   rubber-banding & detents, `MagneticSnap`, `GestureTracker`, and `DraggableCard`.

pub mod body;
pub mod broadphase;
pub mod controller;
pub mod fracture;
pub mod joints;
pub mod material;
pub mod math;
pub mod narrowphase;
pub mod particles;
pub mod queries;
pub mod shapes;
pub mod simd;
pub mod snapshot;
pub mod softbody;
pub mod solver;
pub mod ui;
pub mod world;

#[cfg(feature = "gpu")]
pub mod gpu;

#[cfg(feature = "ferroui")]
pub mod ferroui_bridge;

#[cfg(feature = "ferroui")]
pub mod compositor_integration;

/// Alias for [`ferroui_bridge`] providing zero-layout-thrash FerroUI Visual & Logical tree physics.
#[cfg(feature = "ferroui")]
pub use ferroui_bridge as ferroui;

/// Alias for [`compositor_integration`] providing native FerroUI `Compositor` + GPU shader physics integration.
#[cfg(feature = "ferroui")]
pub use compositor_integration as compositor;

/// Frequently used types for building games, simulations, and physics-driven UIs.
pub mod prelude {
    pub use crate::body::{BodyBuilder, BodyHandle, BodyType, RigidBody};
    pub use crate::broadphase::{DynamicBvh, SpatialHashGrid};
    pub use crate::controller::{CharacterController2D, CharacterMoveResult};
    pub use crate::fracture::fracture_body;
    pub use crate::joints::{Joint, JointHandle, JointKind};
    pub use crate::material::{CollisionFilter, CombineMode, PhysicsMaterial};
    pub use crate::math::{Aabb2, Mat2, Ray2, Rot2, Transform2D, Vec2, Vec3};
    pub use crate::narrowphase::{ContactManifold, ContactPoint, FeatureId};
    pub use crate::particles::{GpuParticle, ParticleConfig, ParticleObstacle, ParticleStepStats, ParticleSystem};
    pub use crate::queries::{ClosestPointHit, CollisionEvent, ForceField, RayHit, ShapeCastHit, TriggerEvent};
    pub use crate::shapes::{ConvexPolygon, MassProperties, Shape2D};
    pub use crate::simd::SimdBackend;
    pub use crate::snapshot::{BodySnapshot, WorldHistory, WorldSnapshot};
    pub use crate::softbody::{ClothMesh, DistanceConstraint, SoftBody2D, SoftNode};
    pub use crate::solver::{FixedTimestep, SolverConfig};
    pub use crate::ui::{
        DraggableCard, GestureTracker, InertiaScroller, LayoutSpringGroup, MagneticSnap, Spring1D, Spring2D, Spring3D,
        SpringChain, SpringConfig,
    };
    pub use crate::world::{PhysicsWorld, StepStats};

    #[cfg(feature = "gpu")]
    pub use crate::gpu::{
        GpuAabbRecord, GpuDeviceInfo, GpuParticleSolver, GpuPhysicsContext, GpuRigidBatchIntegrator,
        GpuRigidBodyRecord,
    };

    #[cfg(feature = "ferroui")]
    pub use crate::ferroui_bridge::{
        start_ui_physics_timer, DraggableVisual, SpringVisualId, VisualBodyOptions, VisualPhysicsHost,
        VisualSpringDriver, VisualSyncStats,
    };

    #[cfg(feature = "ferroui")]
    pub use crate::compositor_integration::{
        CompositorPhysicsController, CompositorPhysicsSimulation, CompositorPhysicsTelemetry,
        PhysicsCompositorBehavior, PhysicsEnergyFieldEffect, PhysicsImpactRippleEffect,
        PhysicsKineticGlassEffect, PhysicsShaderMode,
    };
}

pub use body::{BodyBuilder, BodyHandle, BodyType, RigidBody};
pub use controller::{CharacterController2D, CharacterMoveResult};
pub use fracture::fracture_body;
pub use joints::{Joint, JointHandle, JointKind};
pub use material::{CollisionFilter, PhysicsMaterial};
pub use math::{Aabb2, Ray2, Rot2, Transform2D, Vec2, Vec3};
pub use particles::{GpuParticle, ParticleConfig, ParticleObstacle, ParticleStepStats, ParticleSystem};
pub use queries::{ClosestPointHit, CollisionEvent, ForceField, RayHit, ShapeCastHit, TriggerEvent};
pub use shapes::{ConvexPolygon, Shape2D};
pub use simd::SimdBackend;
pub use snapshot::{WorldHistory, WorldSnapshot};
pub use softbody::{ClothMesh, SoftBody2D};
pub use solver::{FixedTimestep, SolverConfig};
pub use world::{PhysicsWorld, StepStats};

#[cfg(test)]
mod tests;
