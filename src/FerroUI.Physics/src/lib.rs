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
pub mod joints;
pub mod material;
pub mod math;
pub mod narrowphase;
pub mod particles;
pub mod queries;
pub mod shapes;
pub mod simd;
pub mod solver;
pub mod ui;
pub mod world;

#[cfg(feature = "gpu")]
pub mod gpu;

/// Frequently used types for building games, simulations, and physics-driven UIs.
pub mod prelude {
    pub use crate::body::{BodyBuilder, BodyHandle, BodyType, RigidBody};
    pub use crate::broadphase::{DynamicBvh, SpatialHashGrid};
    pub use crate::joints::{Joint, JointHandle, JointKind};
    pub use crate::material::{CollisionFilter, CombineMode, PhysicsMaterial};
    pub use crate::math::{Aabb2, Mat2, Ray2, Rot2, Transform2D, Vec2, Vec3};
    pub use crate::narrowphase::{ContactManifold, ContactPoint, FeatureId};
    pub use crate::particles::{GpuParticle, ParticleConfig, ParticleObstacle, ParticleStepStats, ParticleSystem};
    pub use crate::queries::{CollisionEvent, ForceField, RayHit, TriggerEvent};
    pub use crate::shapes::{ConvexPolygon, MassProperties, Shape2D};
    pub use crate::simd::SimdBackend;
    pub use crate::solver::{FixedTimestep, SolverConfig};
    pub use crate::ui::{
        DraggableCard, GestureTracker, InertiaScroller, MagneticSnap, Spring1D, Spring2D, Spring3D, SpringConfig,
    };
    pub use crate::world::{PhysicsWorld, StepStats};

    #[cfg(feature = "gpu")]
    pub use crate::gpu::{
        GpuAabbRecord, GpuDeviceInfo, GpuParticleSolver, GpuPhysicsContext, GpuRigidBatchIntegrator,
        GpuRigidBodyRecord,
    };
}

pub use body::{BodyBuilder, BodyHandle, BodyType, RigidBody};
pub use joints::{Joint, JointHandle, JointKind};
pub use material::{CollisionFilter, PhysicsMaterial};
pub use math::{Aabb2, Ray2, Rot2, Transform2D, Vec2, Vec3};
pub use particles::{GpuParticle, ParticleConfig, ParticleObstacle, ParticleStepStats, ParticleSystem};
pub use queries::{CollisionEvent, ForceField, RayHit, TriggerEvent};
pub use shapes::{ConvexPolygon, Shape2D};
pub use simd::SimdBackend;
pub use solver::{FixedTimestep, SolverConfig};
pub use world::{PhysicsWorld, StepStats};

#[cfg(test)]
mod tests;
