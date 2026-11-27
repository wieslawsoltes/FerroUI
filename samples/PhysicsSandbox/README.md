# FerroUI Physics Studio (`PhysicsSandbox`)

Professional dark-mode FerroUI desktop physics studio showcasing [`ferroui-physics`](../../src/FerroUI.Physics)
across rigid-body games, `wgpu` GPU compute particles, XPBD cloth & soft bodies, kinematic character
control, destructible fracture, time-travel timeline scrubbing, and physics-driven UI micro-interactions.

## Running

```sh
# macOS native Quartz + Metal (fast pure-Rust build, hardware Metal rendering + Metal GPU physics)
cargo run --profile fast-release -p physics-sandbox --no-default-features --features quartz-metal

# Default Skia backend (Linux / Windows / macOS)
cargo run --profile fast-release -p physics-sandbox

# Windows native Direct2D backend
cargo run --profile fast-release -p physics-sandbox --no-default-features --features direct2d
```

## Included Studio Scenes

1. **Playground**: Rigid-body pyramid, convex polygons, motorized windmill (`RevoluteJoint`), hanging chain bridge, drag/spawn/shatter tools, and shockwave blasts.
2. **GPU Fluid**: Switchable 2K / 4K / 8K colliding fluid particles running on Metal / Vulkan / DX12 via `wgpu` compute shaders with live switching between GPU Compute and CPU SIMD (NEON/AVX2).
3. **Cloth & Jelly**: Real-time XPBD tearable wind-blown cloth curtain (with stress heatmap coloring & pointer slicing) + pressure-inflated area-preserving jelly soft bodies.
4. **UI Studio**: Velocity-coupled `DraggableCard` physics, `MagneticSnap` dock slots, 12-node `SpringChain` cursor ribbon, live spring presets, and rubber-band `InertiaScroller` carousel.
5. **Destruction**: Heavy wrecking ball smashing breakable welded tower beams, auto-shattering fracture crates into convex shards, and launching articulated ragdolls.
6. **Vehicle**: 2D car with spring `WheelJoint` suspension and motorized drive wheels crossing ramps into a shimmering buoyant water pool.
7. **Character**: `CharacterController2D` swept move-and-slide across walkable slopes, moving kinematic platforms, and pushable crates.
8. **Ray & Orbit**: Orbital vortex gravity well with a 6-bounce live `raycast` laser and `closest_point` proximity tether.
