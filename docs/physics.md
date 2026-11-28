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
| `particles`, `gpu` | Shared particle interchange layout for CPU SIMD and `wgpu` GPU compute shaders (Metal / Vulkan / DX12 / WebGPU): 3-pass atomic spatial-hash particle solver (`clear_grid` → `integrate_and_hash` → `resolve_collisions`) plus persistent-buffer `GpuRigidBatchIntegrator`. Neighbor work depends on local density; heavily clustered inputs can approach quadratic work. |
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

A simulation claims only the UI visual properties it writes, so ordinary control
repaints preserve those values while unrelated properties continue to synchronize.
Releasing a UI target restores its saved handoff values and then its current
layout, styled properties, and composition facades. Ownership is per visual:
use one owning simulation for each UI visual, or use nested visual hosts when
independent simulations need separate lifetimes. `reset_to_identity` releases
all simulation property overrides on that UI visual, including claims made
through another `SimulationTarget` wrapping the same visual. Pure composition
targets keep identity-reset semantics.

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
scroller.animate_to(400.0); // explicit, bounded paging/drawer target
scroller.jump_to(0.0);     // reset immediately and cancel drag/inertia
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

Run the Fluent motion and simulation showcase (`samples/PhysicsSandbox`):

```sh
cargo run --profile fast-release -p physics-sandbox
```

## World lifecycle and spatial queries

`BodyHandle` and `JointHandle` are generational: removing a body invalidates its
handle and reclaims its attached joints. `PhysicsWorld::clear()` also advances
slot generations, so a stale handle cannot target an unrelated body created
after a reset. Keep handles rather than caching slot indices.

Mutable queries automatically synchronize the BVH after direct edits through
`body_mut()` or `bodies_mut()`, including static-body teleports. Immutable
`raycast()` remains correct between an edit and the next simulation tick: it
falls back to a body scan while the acceleration structure is dirty. For a batch
of edits followed by many rays, call `world.synchronize_queries()` once to restore
BVH acceleration. The final integrated positions are synchronized before `step()`
returns.

Collision and trigger events observe every substep. A short trigger crossing can
therefore emit both `Entered` and `Exited` in one frame, and `Started` reports the
closing speed before collision impulses are applied. Sleeping overlaps remain
contacts, and static/kinematic sensors still emit lifecycle events. Event buffers
are replaced by each valid `step()`; consume them before advancing again.

Bullet CCD uses conservative advancement of convex separating planes rather than
a fixed number of positional samples. This catches thin obstacles during long
translations and also supports short shape casts. Angular sweeps follow the
shortest arc; split full revolutions into smaller steps. CCD currently sweeps each
bullet against the candidate obstacle's current pose, so it is not a simultaneous
relative-motion solver for two fast moving bodies.

## Rewind and rollback

```rust
use ferroui_physics::prelude::*;

let mut world = PhysicsWorld::default();
let original = world.add_body(BodyBuilder::dynamic(Shape2D::rect(2.0, 2.0)));
let before = WorldSnapshot::capture(&world, 0);
let shards = fracture_body(&mut world, original, Vec2::ZERO, 3.0);
before.restore(&mut world);
assert!(world.body(original).is_some());
assert!(shards.iter().all(|handle| world.body(*handle).is_none()));
```

A snapshot owns complete rigid-body and joint slot state, generations and free
lists, pending forces, gravity/solver configuration, force fields, active contacts,
and warm-start impulses. Restore removes objects created in the abandoned future
and reinstates objects removed or fractured since capture. The public `bodies` and
`joint_broken` arrays provide compact inspector summaries. Particle systems,
cloth/soft-body objects, and application-owned state live outside `PhysicsWorld`
and must be recorded separately if the application needs to rewind them.

BVH nodes and per-step scratch storage are rebuilt/reused instead of copied into
every snapshot. Stored memory is proportional to body slots, joint slots, and
contacts per frame, including shape/material data. `WorldHistory::new(capacity)`
retains at most the requested number of frames (minimum one), drops the oldest
frame when full, and truncates the redo branch when recording after a rewind.
Choose capacity according to the scene's population and desired history duration.

Broadphase pairs use a stable index order, and warm-start state is restored for
repeatable stepping on the same build and target. Bit-identical replay across
different architectures or CPU/GPU solvers is not guaranteed.

## Hot-path storage and numeric behavior

The world retains contact preparation buffers across substeps; box/box SAT uses
inline polygon storage. CPU particles retain their neighbor scratch and recycle
spatial-hash bucket storage without retaining every coordinate ever visited.
Ray traversal normally uses an inline stack, spilling only for unusually deep
trees. These changes remove recurring allocations without changing public world
or particle stepping APIs.

GPU neighbor traversal checks exact cell coordinates in each hash bucket and
visits the complete chain. Hash collisions cannot count the same neighbor more
than once, and dense cells no longer silently discard all entries beyond a fixed
48-particle limit. CPU and GPU share data layouts and physical parameters, but the
CPU sequential contact pass and GPU parallel pass are different numerical
methods; compare invariants and tolerances rather than exact particle positions.

UI inertia uses the analytical integral of exponentially decaying velocity, so
free-motion distance agrees across frame rates. Invalid/non-finite elapsed times
are ignored by springs, inertia, world stepping, and particle stepping; an invalid
fixed-step duration resets the accumulator without entering a loop.

### Reproducing allocation measurements

```sh
cargo bench -p ferroui-physics --bench physics-step-allocations
```

The benchmark has no additional dependencies and runs five rounds of two seeded,
single-threaded workloads at a fixed 1/60-second step: 256 awake boxes in a 16×16
stack (60 warmup steps, then 240 measured steps), and 2,048 CPU particles (30
warmup steps, then 120 measured steps). Only the measured stepping loops are
counted. `alloc` and `realloc` each count as one allocation request; byte totals
sum requested sizes and **are not peak or retained RAM**.

The review comparison used original PR head
`a31daffcb452a67c1325722954e698c92e8438f2` and the revised engine, release mode on
x86_64 Linux with AVX2 enabled. All five rounds produced the same allocation
counts:

| Workload | Original allocation requests | Revised allocation requests | Original requested bytes | Revised requested bytes |
|---|---:|---:|---:|---:|
| 256 awake boxes, 240 steps | 1,287,499 | 3 | 90,147,680 | 67,616 |
| 2,048 CPU particles, 120 steps | 3,210 | 906 | 381,552 | 31,616 |

The remaining requests grow buffers as the contact/neighborhood population
changes. Counts can vary with compiler, architecture, dependencies, and scene
evolution. To repeat the baseline comparison, copy this benchmark and its explicit
manifest target into a separate checkout of the baseline commit. Timing was
approximately unchanged in the initial uninstrumented comparison and noisy during
parallel builds; these results support lower allocation pressure, not a general
throughput speedup claim.

Joint motor and pointer-target force limits apply to the total impulse within a
substep. Increasing velocity iterations improves convergence without multiplying
the configured motor torque or pointer force. Compliant constraints also retain
their softness across iterations by including accumulated impulses in the gamma
term.

Visual physics, spring-driver and draggable-visual bindings restore the previous
render transform when dropped or explicitly detached. If application code has
installed a newer transform in the meantime, cleanup preserves that override.
`VisualPhysicsHost::detach_visual(handle)` and `VisualSpringDriver::detach(id)`
support page/component teardown; repeat attachment of the same visual is
idempotent. After layout changes, call `refresh_rest_layout()` to refresh collider
sizes and home anchors. Pointer drag samples use input-event elapsed time rather
than render-frame timestamps.
