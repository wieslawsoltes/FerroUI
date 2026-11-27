//! High-throughput SoA particle / granular / fluid-lite physics system with identical memory
//! layout for both CPU SIMD execution and `wgpu` GPU compute shaders.

use crate::broadphase::SpatialHashGrid;
use crate::math::{Aabb2, Vec2};
use crate::simd::{apply_radial_field_soa, integrate_particles_soa, SimdBackend};
use std::time::Instant;

/// GPU-compatible 32-byte particle state (`pos`, `vel`, `color_hint`, `density`, `pad`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuParticle {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    /// Scalar visual property (e.g., speed/pressure heat or hue seed in `0..1`).
    pub color_hint: f32,
    pub density: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

/// Interactive circular obstacle that particles bounce off (e.g. moving balls, UI knobs, cursor brush).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParticleObstacle {
    pub center: [f32; 2],
    pub radius: f32,
    pub restitution: f32,
}

/// Simulation parameters for [`ParticleSystem`] (shared between CPU SIMD and GPU compute).
#[derive(Clone, Copy, Debug)]
pub struct ParticleConfig {
    pub bounds: Aabb2,
    pub gravity: Vec2,
    pub particle_radius: f32,
    pub damping: f32,
    pub restitution: f32,
    /// Repulsive stiffness between overlapping particles (`0.1..=0.9`).
    pub stiffness: f32,
    /// Fluid-like velocity smoothing between close neighbors (`0.0..=0.25`).
    pub viscosity: f32,
    /// Substeps per `step(dt)` call.
    pub substeps: u32,
    /// Optional interactive pointer force field `(center, strength, radius)`.
    pub pointer_center: Vec2,
    pub pointer_strength: f32,
    pub pointer_radius: f32,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self {
            bounds: Aabb2::new(Vec2::new(0.0, 0.0), Vec2::new(16.0, 10.0)),
            gravity: Vec2::new(0.0, 9.81),
            particle_radius: 0.08,
            damping: 0.996,
            restitution: 0.35,
            stiffness: 0.55,
            viscosity: 0.04,
            substeps: 2,
            pointer_center: Vec2::ZERO,
            pointer_strength: 0.0,
            pointer_radius: 2.5,
        }
    }
}

/// Performance telemetry for a particle step.
#[derive(Clone, Copy, Debug, Default)]
pub struct ParticleStepStats {
    pub step_us: u64,
    pub particle_count: usize,
    pub backend_name: &'static str,
    pub is_gpu: bool,
}

/// Structure-of-Arrays (SoA) CPU SIMD particle system for fluids, sand, sparks, and confetti.
pub struct ParticleSystem {
    pub config: ParticleConfig,
    pub px: Vec<f32>,
    pub py: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub color_hint: Vec<f32>,
    pub density: Vec<f32>,
    pub obstacles: Vec<ParticleObstacle>,
    grid: SpatialHashGrid,
    last_stats: ParticleStepStats,
}

impl ParticleSystem {
    pub fn new(config: ParticleConfig) -> Self {
        let cell_size = (config.particle_radius * 2.2).max(0.05);
        Self {
            config,
            px: Vec::new(),
            py: Vec::new(),
            vx: Vec::new(),
            vy: Vec::new(),
            color_hint: Vec::new(),
            density: Vec::new(),
            obstacles: Vec::new(),
            grid: SpatialHashGrid::new(cell_size),
            last_stats: ParticleStepStats::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.px.len()
    }

    pub fn is_empty(&self) -> bool {
        self.px.is_empty()
    }

    pub fn clear(&mut self) {
        self.px.clear();
        self.py.clear();
        self.vx.clear();
        self.vy.clear();
        self.color_hint.clear();
        self.density.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.px.reserve(additional);
        self.py.reserve(additional);
        self.vx.reserve(additional);
        self.vy.reserve(additional);
        self.color_hint.reserve(additional);
        self.density.reserve(additional);
    }

    pub fn push(&mut self, pos: Vec2, vel: Vec2, color_hint: f32) {
        self.px.push(pos.x);
        self.py.push(pos.y);
        self.vx.push(vel.x);
        self.vy.push(vel.y);
        self.color_hint.push(color_hint);
        self.density.push(0.0);
    }

    /// Spawns a neat dam-break grid of `count` particles inside `region`.
    pub fn spawn_grid(&mut self, region: Aabb2, count: usize) {
        if count == 0 {
            return;
        }
        let spacing = self.config.particle_radius * 2.05;
        let width = (region.max.x - region.min.x).max(spacing);
        let cols = ((width / spacing).floor() as usize).max(1);
        self.reserve(count);
        for i in 0..count {
            let col = i % cols;
            let row = i / cols;
            let jitter = ((i.wrapping_mul(2654435761) & 0xFF) as f32 / 255.0 - 0.5) * spacing * 0.15;
            let x = (region.min.x + (col as f32 + 0.5) * spacing + jitter)
                .clamp(region.min.x + self.config.particle_radius, region.max.x - self.config.particle_radius);
            let y = (region.min.y + (row as f32 + 0.5) * spacing)
                .clamp(region.min.y + self.config.particle_radius, region.max.y - self.config.particle_radius);
            let hue = (i as f32 / count as f32).clamp(0.0, 1.0);
            self.push(Vec2::new(x, y), Vec2::ZERO, hue);
        }
    }

    /// Exports particles into the flat GPU-compatible buffer format.
    pub fn to_gpu_particles(&self) -> Vec<GpuParticle> {
        (0..self.len())
            .map(|i| GpuParticle {
                pos: [self.px[i], self.py[i]],
                vel: [self.vx[i], self.vy[i]],
                color_hint: self.color_hint[i],
                density: self.density[i],
                _pad0: 0.0,
                _pad1: 0.0,
            })
            .collect()
    }

    /// Imports particles back from a flat GPU buffer.
    pub fn load_from_gpu_particles(&mut self, src: &[GpuParticle]) {
        self.clear();
        self.reserve(src.len());
        for p in src {
            self.px.push(p.pos[0]);
            self.py.push(p.pos[1]);
            self.vx.push(p.vel[0]);
            self.vy.push(p.vel[1]);
            self.color_hint.push(p.color_hint);
            self.density.push(p.density);
        }
    }

    pub fn last_stats(&self) -> ParticleStepStats {
        self.last_stats
    }

    /// Steps the particle simulation on CPU using hardware SIMD (NEON/AVX2/SSE2) + O(N) spatial hash.
    pub fn step_cpu(&mut self, dt: f32) -> ParticleStepStats {
        let start = Instant::now();
        let n = self.len();
        if n == 0 || dt <= 0.0 {
            self.last_stats = ParticleStepStats {
                step_us: 0,
                particle_count: n,
                backend_name: SimdBackend::active().name(),
                is_gpu: false,
            };
            return self.last_stats;
        }

        let substeps = self.config.substeps.max(1);
        let sub_dt = dt / substeps as f32;
        let r = self.config.particle_radius;
        let min_dist = 2.0 * r;
        let min_dist_sq = min_dist * min_dist;
        let damp = self.config.damping.powf(1.0 / substeps as f32);
        let bounds = self.config.bounds;
        let min_x = bounds.min.x + r;
        let max_x = bounds.max.x - r;
        let min_y = bounds.min.y + r;
        let max_y = bounds.max.y - r;
        let rest = self.config.restitution;
        let stiff = self.config.stiffness;
        let visc = self.config.viscosity;

        // Re-create spatial hash if cell size drifted significantly
        let desired_cell = (min_dist * 1.1).max(0.05);
        if (self.grid.cell_size() - desired_cell).abs() > desired_cell * 0.25 {
            self.grid = SpatialHashGrid::new(desired_cell);
        }

        for _ in 0..substeps {
            // 1. Pointer radial field (SIMD)
            if self.config.pointer_strength.abs() > 1e-4 {
                apply_radial_field_soa(
                    &self.px,
                    &self.py,
                    &mut self.vx,
                    &mut self.vy,
                    self.config.pointer_center.x,
                    self.config.pointer_center.y,
                    self.config.pointer_strength,
                    self.config.pointer_radius,
                    sub_dt,
                );
            }

            // 2. Symplectic integration (SIMD NEON / AVX2 / SSE2)
            integrate_particles_soa(
                &mut self.px,
                &mut self.py,
                &mut self.vx,
                &mut self.vy,
                self.config.gravity.x,
                self.config.gravity.y,
                damp,
                sub_dt,
            );

            // 3. Build O(N) spatial hash grid
            self.grid.clear();
            for i in 0..n {
                self.grid.insert_point(i as u32, self.px[i], self.py[i]);
                self.density[i] = 0.0;
            }

            // 4. Neighbor collision & viscosity pass
            let mut neighbors: Vec<u32> = Vec::with_capacity(24);
            for i in 0..n {
                neighbors.clear();
                self.grid.query_neighbors(self.px[i], self.py[i], |j| {
                    if (j as usize) > i {
                        neighbors.push(j);
                    }
                });

                let mut xi = self.px[i];
                let mut yi = self.py[i];
                let mut vxi = self.vx[i];
                let mut vyi = self.vy[i];

                for &ju in &neighbors {
                    let j = ju as usize;
                    let dx = self.px[j] - xi;
                    let dy = self.py[j] - yi;
                    let d2 = dx * dx + dy * dy;
                    if d2 > 1e-10 && d2 < min_dist_sq {
                        let d = d2.sqrt();
                        let inv_d = 1.0 / d;
                        let nx = dx * inv_d;
                        let ny = dy * inv_d;
                        let overlap = (min_dist - d) * 0.5 * stiff;

                        // Positional push apart
                        xi -= nx * overlap;
                        yi -= ny * overlap;
                        self.px[j] += nx * overlap;
                        self.py[j] += ny * overlap;

                        // Relative normal impulse + viscosity smoothing
                        let rvx = self.vx[j] - vxi;
                        let rvy = self.vy[j] - vyi;
                        let vn = rvx * nx + rvy * ny;
                        if vn < 0.0 {
                            let imp = -(1.0 + rest) * vn * 0.5;
                            vxi -= nx * imp;
                            vyi -= ny * imp;
                            self.vx[j] += nx * imp;
                            self.vy[j] += ny * imp;
                        }
                        if visc > 0.0 {
                            let dvx = rvx * visc;
                            let dvy = rvy * visc;
                            vxi += dvx;
                            vyi += dvy;
                            self.vx[j] -= dvx;
                            self.vy[j] -= dvy;
                        }
                        self.density[i] += 1.0 - d * (1.0 / min_dist);
                        self.density[j] += 1.0 - d * (1.0 / min_dist);
                    }
                }

                // Obstacles
                for obs in &self.obstacles {
                    let dx = xi - obs.center[0];
                    let dy = yi - obs.center[1];
                    let r_sum = r + obs.radius;
                    let d2 = dx * dx + dy * dy;
                    if d2 < r_sum * r_sum && d2 > 1e-10 {
                        let d = d2.sqrt();
                        let nx = dx / d;
                        let ny = dy / d;
                        xi = obs.center[0] + nx * r_sum;
                        yi = obs.center[1] + ny * r_sum;
                        let vn = vxi * nx + vyi * ny;
                        if vn < 0.0 {
                            vxi -= (1.0 + obs.restitution) * vn * nx;
                            vyi -= (1.0 + obs.restitution) * vn * ny;
                        }
                    }
                }

                // World boundaries
                if xi < min_x {
                    xi = min_x;
                    if vxi < 0.0 {
                        vxi = -vxi * rest;
                    }
                } else if xi > max_x {
                    xi = max_x;
                    if vxi > 0.0 {
                        vxi = -vxi * rest;
                    }
                }
                if yi < min_y {
                    yi = min_y;
                    if vyi < 0.0 {
                        vyi = -vyi * rest;
                    }
                } else if yi > max_y {
                    yi = max_y;
                    if vyi > 0.0 {
                        vyi = -vyi * rest;
                    }
                }

                self.px[i] = xi;
                self.py[i] = yi;
                self.vx[i] = vxi;
                self.vy[i] = vyi;
            }
        }

        self.last_stats = ParticleStepStats {
            step_us: start.elapsed().as_micros() as u64,
            particle_count: n,
            backend_name: SimdBackend::active().name(),
            is_gpu: false,
        };
        self.last_stats
    }
}
