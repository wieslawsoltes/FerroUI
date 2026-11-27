# Hardware-Accelerated Physics Engine (`ferroui-physics`)

`ferroui-physics` (`src/FerroUI.Physics`) is a standalone, hardware-accelerated physics engine
crate designed for games, interactive user interfaces, spring/inertia animations, particle and
fluid simulations, robotics, and general 2D simulation.

## Architecture

| Module | Responsibility |
|---|---|
| `math` | `#[repr(C)]` SIMD/GPU-compatible `Vec2`, `Vec3`, `Rot2`, `Transform2D`, `Mat2`, `Aabb2`, `Ray2`. |
| `simd` | Hardware CPU SIMD kernels (ARM64 NEON, x86_64 AVX2/SSE2, portable scalar fallback) over Structure-of-Arrays (SoA) buffers. |
| `shapes`, `material` | `Circle`, `Box` (OBB), `Capsule`, `ConvexPolygon` (with monotone-chain hull), `Segment`, exact mass/inertia tensors, material combine modes, and 32-bit layer/group collision filtering. |
| `broadphase` | SAH Dynamic AABB Tree (BVH) with fat velocity-predicted AABBs and AVL rotations, plus an O(N) Uniform Spatial Hash Grid. |
| `narrowphase` | Exact analytic contact manifolds, SAT + Sutherland-Hodgman 2-point face clipping with persistent `FeatureId`s, GJK/EPA general convex fallback, and swept Continuous Collision Detection (CCD). |
| `joints` | `Distance`, `Spring`, `Revolute` (limits + motor), `Prismatic` (limits + motor), `Weld`, `Wheel` (suspension + drive motor), `Rope`, and `Target` (pointer drag), all supporting breakable force thresholds. |
| `solver`, `world` | Substepped Sequential Impulse / Soft-Step solver with warm-starting, Coulomb friction, positional stabilization, automatic sleeping/waking, deterministic `FixedTimestep`, spatial queries, force fields, and per-step profiling telemetry (`StepStats`). |
| `particles`, `gpu` | Identical SoA particle layout for CPU SIMD and `wgpu` GPU compute shaders (Metal / Vulkan / DX12 / WebGPU): 3-pass lock-free atomic O(N) spatial-hash particle solver (`clear_grid` → `integrate_and_hash` → `resolve_collisions`) plus persistent-buffer `GpuRigidBatchIntegrator`. |
| `softbody` | Extended Position-Based Dynamics (XPBD): tearable `ClothMesh` with structural/shear/bending constraints, stress heatmaps, and wind gusts, plus pressure-inflated area-preserving `SoftBody2D` jelly meshes. |
| `controller` | Production `CharacterController2D` with swept `shape_cast` move-and-slide, slope limits, coyote time, jump buffering, and dynamic box pushing. |
| `fracture`, `snapshot` | Radial convex shard fracturing (`fracture_body`) conserving linear/angular velocity, and deterministic ring-buffer `WorldHistory` / `WorldSnapshot` for rollback and studio timeline scrubbing. |
| `ui` | Unconditionally stable closed-form analytical harmonic springs (`Spring1D`, `Spring2D`, `Spring3D`), `SpringChain`, `LayoutSpringGroup`, native-feel `InertiaScroller` (exponential decay, rubber-banding, bounce-back, magnetic detents), `MagneticSnap`, `GestureTracker`, and `DraggableCard`. |
| `ferroui` *(feature `ferroui`)* | Zero-layout-thrash FerroUI Visual & Logical tree physics bridge (`VisualPhysicsHost`, `VisualSpringDriver`, `DraggableVisual`, `start_ui_physics_timer`) updating persistent `MatrixTransform` render transforms (`O(active)`, zero layout passes, automatic weak cleanup). |
| `compositor` *(feature `ferroui`)* | Native integration into `ferroui-base`'s public `ICompositionSimulation` contract (`CompositorPhysicsSimulation`, `CompositorPhysicsController`) driving `CompositionVisual` transforms directly on `Compositor` (`MediaContext` clock, auto-idle) plus physics-driven WGSL/SkSL GPU shader effects (`PhysicsKineticGlassEffect`, `PhysicsImpactRippleEffect`). |

## Composition Engine & Multi-Mode Simulation Contract

`ferroui-base::rendering::composition` defines a domain-agnostic public simulation contract so any simulation mode — physics, springs, harmonic waves, audio-reactive, particles, or custom modes — plugs directly into FerroUI's `Compositor` and per-visual `Effect` / GPU shader pipeline without coupling `ferroui-base` to `ferroui-physics`:

- `ICompositionSimulation` + `SimulationMode` (`Physics`, `SpringDynamics`, `Harmonic`, `AudioReactive`, `Particles`, `Custom`)
- `Compositor::register_simulation(...) -> CompositionSimulationHandle` (pause/resume/wake/RAII detach, synced to `MediaContext` animation frames with automatic sleep on `SimulationStepResult::Idle`)
- `SimulationVisualPatch` & `SimulationTarget` (`Composition(Rc<CompositionVisual>)` or `Visual(Ref<Visual>)`) applying composition transforms (`translation`, `rotation_angle`, `scale`, `center_point`, `opacity`) and immutable GPU shader effects (`ImmutableCustomEffect`) with zero layout passes
- Built-in non-physics reference mode `HarmonicWaveSimulation` in `ferroui-base`, alongside `CompositorPhysicsSimulation`, `PhysicsKineticGlassEffect`, and `PhysicsImpactRippleEffect` in `ferroui-physics` (WGSL compiled to Metal MSL via `ferroui-quartz` / Vello, SkSL for Skia, and CPU reference).

## Features

- `simd` (default): enables architecture-native NEON / AVX2 / SSE2 kernels.
- `gpu`: enables `wgpu` compute shaders (`GpuPhysicsContext`, `GpuParticleSolver`, `GpuRigidBatchIntegrator`) with automatic graceful fallback to CPU SIMD when no GPU adapter is available.
- `ferroui`: enables zero-layout-thrash bindings to apply rigid bodies, elastic home-springs, shockwaves, magnetic hover, tactile pops, error-shake, and drag-and-throw directly to any FerroUI `Visual` or `StyledElement` subtree.

## Quick Examples

### Rigid-Body World (Games & Interactive Canvases)

```rust
use ferroui_physics::prelude::*;

let mut world = PhysicsWorld::new(Vec2::new(0.0, 9.81));
world.add_body(BodyBuilder::static_body(Shape2D::rect(20.0, 1.0)).position((0.0, 10.0)));
let ball = world.add_body(
    BodyBuilder::dynamic(Shape2D::circle(0.5))
        .position((0.0, 0.0))
        .material(PhysicsMaterial::rubber()),
);
let stats = world.step(1.0 / 60.0);
let pos = world.body(ball).unwrap().position;
```

### UI Springs & Rubber-Band Inertia Scrolling

```rust
use ferroui_physics::ui::{InertiaScroller, Spring2D, SpringConfig};
use ferroui_physics::Vec2;

let mut card_pos = Spring2D::new(Vec2::ZERO, SpringConfig::bouncy());
card_pos.set_target(Vec2::new(240.0, 120.0));
let next = card_pos.step(1.0 / 60.0);

let mut scroller = InertiaScroller::new(0.0, 1200.0, 400.0);
scroller.begin_drag();
scroller.drag_by(-60.0); // rubber-band overscroll resistance
scroller.end_drag(0.0);  // springs smoothly back to 0.0
```

### Hardware-Accelerated GPU Particles

```rust
use ferroui_physics::prelude::*;

let config = ParticleConfig::default();
let mut ps = ParticleSystem::new(config);
ps.spawn_grid(Aabb2::new(Vec2::new(1.0, 1.0), Vec2::new(8.0, 6.0)), 4096);

if let Some(mut gpu) = GpuParticleSolver::try_new(4096) {
    gpu.upload_particles(&ps.to_gpu_particles());
    let mut out = Vec::new();
    let stats = gpu.step_and_readback(&config, &[], 1.0 / 60.0, &mut out);
}
```

## Interactive Sample App

Run the interactive 6-scene FerroUI showcase (`samples/PhysicsSandbox`):

```sh
cargo run --profile fast-release -p physics-sandbox
```
