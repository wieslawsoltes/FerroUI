# PhysicsSandbox

Interactive FerroUI desktop sample showcasing [`ferroui-physics`](../../src/FerroUI.Physics) across
games, GPU compute particle fluids, and physics-driven UI animations.

## Running

```sh
# macOS native Quartz + Metal (fast pure-Rust build, hardware Metal rendering + Metal GPU physics)
cargo run --profile fast-release -p physics-sandbox --no-default-features --features quartz-metal

# Default Skia backend (Linux / Windows / macOS)
cargo run --profile fast-release -p physics-sandbox

# Windows native Direct2D backend
cargo run --profile fast-release -p physics-sandbox --no-default-features --features direct2d
```

## Included Scenes

1. **Playground**: Rigid-body pyramid, convex polygons, motorized windmill (`RevoluteJoint`), suspension bridge, drag/spawn tools, and radial shockwave blasts.
2. **GPU Particles**: 3,072 colliding fluid/sand particles running on Metal / Vulkan / DX12 via `wgpu` compute shaders with live switching between GPU Compute and CPU SIMD (NEON/AVX2).
3. **UI Springs**: Velocity-coupled `DraggableCard` physics, `MagneticSnap` dock slots, live spring preset selector (`Snappy`, `Bouncy`, `Gentle`, `Wobbly`, `Critical`), and a rubber-band `InertiaScroller` with paging detents.
4. **Ragdoll**: Articulated ragdolls with angle-limited `RevoluteJoint`s and a swinging wrecking ball smashing breakable welded tower beams.
5. **Vehicle**: 2D car with `WheelJoint` spring suspension and motorized drive wheels crossing dynamic ramps and crates.
6. **Ray & Orbit**: Orbital vortex gravity well with a 6-bounce live `raycast` laser reflecting off rotating satellites.
