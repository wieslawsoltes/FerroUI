//! WGSL GPU compute shaders for hardware-accelerated particle & rigid-body physics.
//!
//! Validated at test time with `naga` and executed natively on Metal, Vulkan, DX12, and WebGPU via `wgpu`.

/// Workgroup size for 1D compute dispatches (matches Apple Metal SIMD-group & Vulkan/DX12 wave multiples).
pub const WORKGROUP_SIZE: u32 = 64;

/// Number of hash buckets in the GPU spatial hash grid (power of two for fast bitmasking).
pub const GPU_GRID_BUCKETS: u32 = 16384;

/// Multi-pass GPU particle physics shader with lock-free atomic O(N) spatial hashing:
/// - `clear_grid`: resets `cell_head` linked-list heads to `-1`.
/// - `integrate_and_hash`: symplectic Euler integration, pointer attractor/repulsor force field,
///   obstacle collisions, world-box collisions, and atomic linked-list insertion into spatial buckets.
/// - `resolve_collisions`: 3x3 neighborhood spatial-hash lookup computing elastic repulsion,
///   viscosity smoothing, density accumulation, and boundary clamping into double-buffered output.
pub const PARTICLE_PHYSICS_WGSL: &str = r#"
struct SimUniforms {
    bounds_min: vec2<f32>,
    bounds_max: vec2<f32>,
    gravity: vec2<f32>,
    dt: f32,
    damping: f32,
    radius: f32,
    restitution: f32,
    stiffness: f32,
    viscosity: f32,
    pointer_pos: vec2<f32>,
    pointer_strength: f32,
    pointer_radius: f32,
    particle_count: u32,
    obstacle_count: u32,
    grid_mask: u32,
    cell_size_inv: f32,
};

struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    color_hint: f32,
    density: f32,
    _pad0: f32,
    _pad1: f32,
};

struct Obstacle {
    center: vec2<f32>,
    radius: f32,
    restitution: f32,
};

@group(0) @binding(0) var<uniform> params: SimUniforms;
@group(0) @binding(1) var<storage, read_write> particles_in: array<Particle>;
@group(0) @binding(2) var<storage, read_write> particles_out: array<Particle>;
@group(0) @binding(3) var<storage, read_write> cell_head: array<atomic<i32>>;
@group(0) @binding(4) var<storage, read_write> next_particle: array<i32>;
@group(0) @binding(5) var<storage, read> obstacles: array<Obstacle>;

fn hash_cell(cx: i32, cy: i32) -> u32 {
    let h = u32(cx) * 73856093u ^ u32(cy) * 19349663u;
    return h & params.grid_mask;
}

@compute @workgroup_size(64)
fn clear_grid(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx <= params.grid_mask) {
        atomicStore(&cell_head[idx], -1);
    }
}

@compute @workgroup_size(64)
fn integrate_and_hash(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.particle_count) {
        return;
    }

    var p = particles_in[idx];

    // Interactive radial force field (pointer attract / repel)
    if (abs(params.pointer_strength) > 0.0001) {
        let delta = params.pointer_pos - p.pos;
        let d2 = dot(delta, delta);
        let r2 = params.pointer_radius * params.pointer_radius;
        if (d2 > 0.0001 && d2 < r2) {
            let inv_d = inverseSqrt(d2);
            let falloff = 1.0 - (d2 / r2);
            p.vel = p.vel + delta * (params.pointer_strength * params.dt * falloff * inv_d);
        }
    }

    // Symplectic Euler integration
    p.vel = (p.vel + params.gravity * params.dt) * params.damping;
    p.pos = p.pos + p.vel * params.dt;

    // Obstacle collisions
    for (var k: u32 = 0u; k < params.obstacle_count; k = k + 1u) {
        let obs = obstacles[k];
        let d = p.pos - obs.center;
        let r_sum = params.radius + obs.radius;
        let d2 = dot(d, d);
        if (d2 < r_sum * r_sum && d2 > 0.000001) {
            let dist = sqrt(d2);
            let n = d / dist;
            p.pos = obs.center + n * r_sum;
            let vn = dot(p.vel, n);
            if (vn < 0.0) {
                p.vel = p.vel - n * ((1.0 + obs.restitution) * vn);
            }
        }
    }

    // Clamp to simulation bounds
    let min_p = params.bounds_min + vec2<f32>(params.radius, params.radius);
    let max_p = params.bounds_max - vec2<f32>(params.radius, params.radius);
    if (p.pos.x < min_p.x) {
        p.pos.x = min_p.x;
        if (p.vel.x < 0.0) { p.vel.x = -p.vel.x * params.restitution; }
    } else if (p.pos.x > max_p.x) {
        p.pos.x = max_p.x;
        if (p.vel.x > 0.0) { p.vel.x = -p.vel.x * params.restitution; }
    }
    if (p.pos.y < min_p.y) {
        p.pos.y = min_p.y;
        if (p.vel.y < 0.0) { p.vel.y = -p.vel.y * params.restitution; }
    } else if (p.pos.y > max_p.y) {
        p.pos.y = max_p.y;
        if (p.vel.y > 0.0) { p.vel.y = -p.vel.y * params.restitution; }
    }

    particles_in[idx] = p;

    // Insert into lock-free atomic spatial hash linked list
    let cx = i32(floor(p.pos.x * params.cell_size_inv));
    let cy = i32(floor(p.pos.y * params.cell_size_inv));
    let bucket = hash_cell(cx, cy);
    let prev_head = atomicExchange(&cell_head[bucket], i32(idx));
    next_particle[idx] = prev_head;
}

@compute @workgroup_size(64)
fn resolve_collisions(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.particle_count) {
        return;
    }

    var p = particles_in[idx];
    let cx = i32(floor(p.pos.x * params.cell_size_inv));
    let cy = i32(floor(p.pos.y * params.cell_size_inv));
    let min_dist = params.radius * 2.0;
    let min_dist_sq = min_dist * min_dist;

    var pos_corr = vec2<f32>(0.0, 0.0);
    var vel_corr = vec2<f32>(0.0, 0.0);
    var density = 0.0;

    for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
        for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
            let bucket = hash_cell(cx + dx, cy + dy);
            var j = atomicLoad(&cell_head[bucket]);
            var guard: u32 = 0u;
            while (j >= 0 && guard < 48u) {
                let ju = u32(j);
                if (ju != idx) {
                    let other = particles_in[ju];
                    let diff = p.pos - other.pos;
                    let d2 = dot(diff, diff);
                    if (d2 > 0.0000001 && d2 < min_dist_sq) {
                        let d = sqrt(d2);
                        let n = diff / d;
                        let overlap = (min_dist - d) * 0.5 * params.stiffness;
                        pos_corr = pos_corr + n * overlap;

                        let rv = p.vel - other.vel;
                        let vn = dot(rv, n);
                        if (vn < 0.0) {
                            vel_corr = vel_corr - n * ((1.0 + params.restitution) * vn * 0.5);
                        }
                        vel_corr = vel_corr - rv * params.viscosity;
                        density = density + (1.0 - d / min_dist);
                    }
                }
                j = next_particle[ju];
                guard = guard + 1u;
            }
        }
    }

    p.pos = p.pos + pos_corr;
    p.vel = p.vel + vel_corr;
    p.density = density;

    // Final world bounds enforcement
    let min_p = params.bounds_min + vec2<f32>(params.radius, params.radius);
    let max_p = params.bounds_max - vec2<f32>(params.radius, params.radius);
    p.pos = clamp(p.pos, min_p, max_p);

    particles_out[idx] = p;
}
"#;

/// Parallel GPU compute shader for batch rigid-body integration and AABB generation.
pub const RIGID_INTEGRATE_WGSL: &str = r#"
struct RigidUniforms {
    gravity: vec2<f32>,
    dt: f32,
    body_count: u32,
};

struct GpuRigidBody {
    pos: vec2<f32>,
    vel: vec2<f32>,
    force: vec2<f32>,
    angle: f32,
    ang_vel: f32,
    inv_mass: f32,
    inv_inertia: f32,
    damping: f32,
    gravity_scale: f32,
};

struct GpuAabb {
    min_xy: vec2<f32>,
    max_xy: vec2<f32>,
};

@group(0) @binding(0) var<uniform> params: RigidUniforms;
@group(0) @binding(1) var<storage, read_write> bodies: array<GpuRigidBody>;
@group(0) @binding(2) var<storage, read> radii: array<f32>;
@group(0) @binding(3) var<storage, read_write> aabbs: array<GpuAabb>;

@compute @workgroup_size(64)
fn integrate_bodies(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.body_count) {
        return;
    }

    var b = bodies[idx];
    if (b.inv_mass > 0.0) {
        let accel = params.gravity * b.gravity_scale + b.force * b.inv_mass;
        let damp = 1.0 / (1.0 + b.damping * params.dt);
        b.vel = (b.vel + accel * params.dt) * damp;
        b.pos = b.pos + b.vel * params.dt;
        b.angle = b.angle + b.ang_vel * params.dt;
        b.force = vec2<f32>(0.0, 0.0);
        bodies[idx] = b;
    }

    let r = radii[idx];
    aabbs[idx] = GpuAabb(b.pos - vec2<f32>(r, r), b.pos + vec2<f32>(r, r));
}
"#;
