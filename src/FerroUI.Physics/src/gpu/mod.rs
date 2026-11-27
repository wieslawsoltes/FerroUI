//! Hardware-accelerated GPU compute solver powered by `wgpu` (Metal, Vulkan, DirectX 12, WebGPU).
//!
//! Provides:
//! - [`GpuPhysicsContext`]: manages the `wgpu::Device`, `wgpu::Queue`, and adapter capabilities.
//! - [`GpuParticleSolver`]: executes the 3-pass atomic spatial-hash particle solver entirely on
//!   the GPU (`clear_grid` -> `integrate_and_hash` -> `resolve_collisions`) with double-buffered
//!   storage buffers and zero per-frame pipeline or buffer allocations.

pub mod shaders;

use crate::particles::{GpuParticle, ParticleConfig, ParticleObstacle, ParticleStepStats};
use shaders::{GPU_GRID_BUCKETS, PARTICLE_PHYSICS_WGSL, RIGID_INTEGRATE_WGSL, WORKGROUP_SIZE};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Metadata describing the active GPU compute device.
#[derive(Clone, Debug)]
pub struct GpuDeviceInfo {
    pub adapter_name: String,
    pub backend: String,
}

/// Shared `wgpu` compute context for hardware-accelerated physics solvers.
pub struct GpuPhysicsContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub info: GpuDeviceInfo,
}

impl GpuPhysicsContext {
    /// Initializes a headless `wgpu` compute device on the fastest available hardware GPU
    /// (Apple Metal, Vulkan, DX12). Returns `None` if no compatible GPU adapter is present.
    pub fn try_new() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            ..wgpu::RequestAdapterOptions::default()
        }))
        .ok()?;

        let adapter_info = adapter.get_info();
        let info = GpuDeviceInfo {
            adapter_name: adapter_info.name,
            backend: format!("{:?}", adapter_info.backend),
        };

        let descriptor = wgpu::DeviceDescriptor {
            label: Some("ferroui-physics compute device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..wgpu::DeviceDescriptor::default()
        };
        let (device, queue) = block_on(adapter.request_device(&descriptor)).ok()?;
        Some(Self { device, queue, info })
    }

    /// Creates a context from an existing application-owned `wgpu::Device` and `wgpu::Queue`.
    pub fn from_existing(device: wgpu::Device, queue: wgpu::Queue, info: GpuDeviceInfo) -> Self {
        Self { device, queue, info }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct SimUniforms {
    bounds_min: [f32; 2],
    bounds_max: [f32; 2],
    gravity: [f32; 2],
    dt: f32,
    damping: f32,
    radius: f32,
    restitution: f32,
    stiffness: f32,
    viscosity: f32,
    pointer_pos: [f32; 2],
    pointer_strength: f32,
    pointer_radius: f32,
    particle_count: u32,
    obstacle_count: u32,
    grid_mask: u32,
    cell_size_inv: f32,
}

const MAX_GPU_OBSTACLES: usize = 64;

/// GPU-accelerated spatial-hash particle solver executing natively on Metal/Vulkan/DX12 via `wgpu`.
pub struct GpuParticleSolver {
    ctx: GpuPhysicsContext,
    clear_pipeline: wgpu::ComputePipeline,
    integrate_pipeline: wgpu::ComputePipeline,
    resolve_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    cell_head_buffer: wgpu::Buffer,
    obstacle_buffer: wgpu::Buffer,

    // Ping-pong particle storage buffers + linked-list next buffer + readback staging buffer
    capacity: usize,
    count: usize,
    particles_a: wgpu::Buffer,
    particles_b: wgpu::Buffer,
    next_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    bind_group_a_to_b: wgpu::BindGroup,
    bind_group_b_to_a: wgpu::BindGroup,
    ping_a_is_input: bool,
    last_stats: ParticleStepStats,
    backend_label: &'static str,
}

impl GpuParticleSolver {
    /// Creates a new GPU particle solver with initial buffer capacity `initial_capacity`.
    pub fn try_new(initial_capacity: usize) -> Option<Self> {
        let ctx = GpuPhysicsContext::try_new()?;
        Some(Self::with_context(ctx, initial_capacity))
    }

    pub fn with_context(ctx: GpuPhysicsContext, initial_capacity: usize) -> Self {
        let cap = initial_capacity.max(256);
        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ferroui-physics particle compute shader"),
            source: wgpu::ShaderSource::Wgsl(PARTICLE_PHYSICS_WGSL.into()),
        });

        let bind_group_layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ferroui-physics particle bgl"),
            entries: &[
                bgl_entry(0, wgpu::BufferBindingType::Uniform),
                bgl_entry(1, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(2, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(3, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(4, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(5, wgpu::BufferBindingType::Storage { read_only: true }),
            ],
        });

        let pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ferroui-physics particle pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let make_pipeline = |entry: &str| {
            ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };

        let clear_pipeline = make_pipeline("clear_grid");
        let integrate_pipeline = make_pipeline("integrate_and_hash");
        let resolve_pipeline = make_pipeline("resolve_collisions");

        let uniform_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("physics uniforms"),
            size: std::mem::size_of::<SimUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cell_head_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("physics grid cell heads"),
            size: (GPU_GRID_BUCKETS as usize * std::mem::size_of::<i32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let obstacle_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("physics obstacles"),
            size: (MAX_GPU_OBSTACLES * std::mem::size_of::<ParticleObstacle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (particles_a, particles_b, next_buffer, readback_buffer) =
            create_particle_buffers(&ctx.device, cap);
        let bind_group_a_to_b = create_bind_group(
            &ctx.device,
            &bind_group_layout,
            &uniform_buffer,
            &particles_a,
            &particles_b,
            &cell_head_buffer,
            &next_buffer,
            &obstacle_buffer,
        );
        let bind_group_b_to_a = create_bind_group(
            &ctx.device,
            &bind_group_layout,
            &uniform_buffer,
            &particles_b,
            &particles_a,
            &cell_head_buffer,
            &next_buffer,
            &obstacle_buffer,
        );

        let backend_label = match ctx.info.backend.as_str() {
            "Metal" => "GPU Metal (wgpu)",
            "Vulkan" => "GPU Vulkan (wgpu)",
            "Dx12" => "GPU DX12 (wgpu)",
            "Gl" => "GPU OpenGL (wgpu)",
            "BrowserWebGpu" => "GPU WebGPU (wgpu)",
            _ => "GPU Compute (wgpu)",
        };

        Self {
            ctx,
            clear_pipeline,
            integrate_pipeline,
            resolve_pipeline,
            bind_group_layout,
            uniform_buffer,
            cell_head_buffer,
            obstacle_buffer,
            capacity: cap,
            count: 0,
            particles_a,
            particles_b,
            next_buffer,
            readback_buffer,
            bind_group_a_to_b,
            bind_group_b_to_a,
            ping_a_is_input: true,
            last_stats: ParticleStepStats::default(),
            backend_label,
        }
    }

    pub fn device_info(&self) -> &GpuDeviceInfo {
        &self.ctx.info
    }

    pub fn particle_count(&self) -> usize {
        self.count
    }

    pub fn last_stats(&self) -> ParticleStepStats {
        self.last_stats
    }

    /// Uploads a fresh set of particles to GPU memory, resizing buffers only if capacity grows.
    pub fn upload_particles(&mut self, particles: &[GpuParticle]) {
        self.count = particles.len();
        if self.count > self.capacity {
            let new_cap = self.count.next_power_of_two().max(256);
            let (pa, pb, nb, rb) = create_particle_buffers(&self.ctx.device, new_cap);
            self.bind_group_a_to_b = create_bind_group(
                &self.ctx.device,
                &self.bind_group_layout,
                &self.uniform_buffer,
                &pa,
                &pb,
                &self.cell_head_buffer,
                &nb,
                &self.obstacle_buffer,
            );
            self.bind_group_b_to_a = create_bind_group(
                &self.ctx.device,
                &self.bind_group_layout,
                &self.uniform_buffer,
                &pb,
                &pa,
                &self.cell_head_buffer,
                &nb,
                &self.obstacle_buffer,
            );
            self.particles_a = pa;
            self.particles_b = pb;
            self.next_buffer = nb;
            self.readback_buffer = rb;
            self.capacity = new_cap;
        }
        self.ping_a_is_input = true;
        if !particles.is_empty() {
            self.ctx.queue.write_buffer(&self.particles_a, 0, slice_as_bytes(particles));
        }
    }

    /// Dispatches all substeps of the 3-pass spatial-hash compute pipeline on the GPU and reads
    /// back the updated particle states into `out_particles`.
    pub fn step_and_readback(
        &mut self,
        config: &ParticleConfig,
        obstacles: &[ParticleObstacle],
        dt: f32,
        out_particles: &mut Vec<GpuParticle>,
    ) -> ParticleStepStats {
        let start = Instant::now();
        if self.count == 0 || dt <= 0.0 {
            out_particles.clear();
            return ParticleStepStats {
                step_us: 0,
                particle_count: 0,
                backend_name: self.backend_label,
                is_gpu: true,
            };
        }

        let substeps = config.substeps.max(1);
        let sub_dt = dt / substeps as f32;
        let damp = config.damping.powf(1.0 / substeps as f32);
        let obs_count = obstacles.len().min(MAX_GPU_OBSTACLES);
        if obs_count > 0 {
            self.ctx.queue.write_buffer(&self.obstacle_buffer, 0, slice_as_bytes(&obstacles[..obs_count]));
        }

        let cell_size = (config.particle_radius * 2.1).max(0.05);
        let uniforms = SimUniforms {
            bounds_min: [config.bounds.min.x, config.bounds.min.y],
            bounds_max: [config.bounds.max.x, config.bounds.max.y],
            gravity: [config.gravity.x, config.gravity.y],
            dt: sub_dt,
            damping: damp,
            radius: config.particle_radius,
            restitution: config.restitution,
            stiffness: config.stiffness,
            viscosity: config.viscosity,
            pointer_pos: [config.pointer_center.x, config.pointer_center.y],
            pointer_strength: config.pointer_strength,
            pointer_radius: config.pointer_radius,
            particle_count: self.count as u32,
            obstacle_count: obs_count as u32,
            grid_mask: GPU_GRID_BUCKETS - 1,
            cell_size_inv: 1.0 / cell_size,
        };
        self.ctx.queue.write_buffer(&self.uniform_buffer, 0, struct_as_bytes(&uniforms));

        let grid_groups = (GPU_GRID_BUCKETS + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let particle_groups = (self.count as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

        let mut encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ferroui-physics particle step encoder"),
        });

        for _ in 0..substeps {
            let bg = if self.ping_a_is_input { &self.bind_group_a_to_b } else { &self.bind_group_b_to_a };
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("physics particle substep"),
                    timestamp_writes: None,
                });
                pass.set_bind_group(0, bg, &[]);

                pass.set_pipeline(&self.clear_pipeline);
                pass.dispatch_workgroups(grid_groups, 1, 1);

                pass.set_pipeline(&self.integrate_pipeline);
                pass.dispatch_workgroups(particle_groups, 1, 1);

                pass.set_pipeline(&self.resolve_pipeline);
                pass.dispatch_workgroups(particle_groups, 1, 1);
            }
            self.ping_a_is_input = !self.ping_a_is_input;
        }

        // Copy latest output buffer to readback staging buffer
        let latest_buf = if self.ping_a_is_input { &self.particles_a } else { &self.particles_b };
        let byte_size = (self.count * std::mem::size_of::<GpuParticle>()) as u64;
        encoder.copy_buffer_to_buffer(latest_buf, 0, &self.readback_buffer, 0, byte_size);

        self.ctx.queue.submit(Some(encoder.finish()));

        // Map readback buffer synchronously
        let slice = self.readback_buffer.slice(0..byte_size);
        let mapped = Arc::new(AtomicBool::new(false));
        let mapped_flag = Arc::clone(&mapped);
        slice.map_async(wgpu::MapMode::Read, move |res| {
            mapped_flag.store(res.is_ok(), Ordering::SeqCst);
        });
        let _ = self.ctx.device.poll(wgpu::PollType::wait_indefinitely());

        if mapped.load(Ordering::SeqCst) {
            if let Ok(data) = slice.get_mapped_range() {
                out_particles.resize(self.count, GpuParticle::default());
                // SAFETY: GpuParticle is #[repr(C)] plain old data matching WGSL Particle (32 bytes).
                let dst_bytes = unsafe {
                    std::slice::from_raw_parts_mut(
                        out_particles.as_mut_ptr() as *mut u8,
                        self.count * std::mem::size_of::<GpuParticle>(),
                    )
                };
                dst_bytes.copy_from_slice(&data[..dst_bytes.len()]);
            }
            self.readback_buffer.unmap();
        }

        self.last_stats = ParticleStepStats {
            step_us: start.elapsed().as_micros() as u64,
            particle_count: self.count,
            backend_name: self.backend_label,
            is_gpu: true,
        };
        self.last_stats
    }
}

fn bgl_entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None },
        count: None,
    }
}

fn create_particle_buffers(
    device: &wgpu::Device,
    capacity: usize,
) -> (wgpu::Buffer, wgpu::Buffer, wgpu::Buffer, wgpu::Buffer) {
    let p_bytes = (capacity * std::mem::size_of::<GpuParticle>()) as u64;
    let next_bytes = (capacity * std::mem::size_of::<i32>()) as u64;

    let make_storage = |label| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: p_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    };

    let pa = make_storage("particles_a");
    let pb = make_storage("particles_b");
    let next = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("next_particle"),
        size: next_bytes,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("particles_readback"),
        size: p_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    (pa, pb, next, readback)
}

#[allow(clippy::too_many_arguments)]
fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniforms: &wgpu::Buffer,
    in_buf: &wgpu::Buffer,
    out_buf: &wgpu::Buffer,
    cell_head: &wgpu::Buffer,
    next_buf: &wgpu::Buffer,
    obstacles: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ferroui-physics particle bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: in_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: out_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: cell_head.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: next_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: obstacles.as_entire_binding() },
        ],
    })
}

fn slice_as_bytes<T>(slice: &[T]) -> &[u8] {
    // SAFETY: Caller only passes #[repr(C)] POD structs (GpuParticle, ParticleObstacle).
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

fn struct_as_bytes<T>(value: &T) -> &[u8] {
    // SAFETY: Caller only passes #[repr(C)] POD structs (SimUniforms).
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of::<T>()) }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    let mut future = std::pin::pin!(future);
    for _ in 0..2_000_000 {
        if let std::task::Poll::Ready(out) = future.as_mut().poll(&mut context) {
            return out;
        }
        std::thread::yield_now();
    }
    panic!("wgpu future did not complete");
}

/// GPU-compatible rigid body record matching `RIGID_INTEGRATE_WGSL` (48 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuRigidBodyRecord {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub force: [f32; 2],
    pub angle: f32,
    pub ang_vel: f32,
    pub inv_mass: f32,
    pub inv_inertia: f32,
    pub damping: f32,
    pub gravity_scale: f32,
}

/// GPU-computed axis-aligned bounding box output matching `RIGID_INTEGRATE_WGSL`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuAabbRecord {
    pub min_xy: [f32; 2],
    pub max_xy: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct RigidUniforms {
    gravity: [f32; 2],
    dt: f32,
    body_count: u32,
}

/// Hardware-accelerated GPU batch integrator for large rigid-body swarms, computing updated
/// velocities, positions, rotations, and broadphase AABBs in parallel on the GPU with
/// persistent zero-allocation buffer reuse in steady state.
pub struct GpuRigidBatchIntegrator {
    ctx: GpuPhysicsContext,
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    capacity: usize,
    body_buf: wgpu::Buffer,
    radii_buf: wgpu::Buffer,
    aabb_buf: wgpu::Buffer,
    body_rb: wgpu::Buffer,
    aabb_rb: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl GpuRigidBatchIntegrator {
    pub fn try_new() -> Option<Self> {
        let ctx = GpuPhysicsContext::try_new()?;
        Some(Self::with_context(ctx))
    }

    pub fn with_context(ctx: GpuPhysicsContext) -> Self {
        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ferroui-physics rigid integrate shader"),
            source: wgpu::ShaderSource::Wgsl(RIGID_INTEGRATE_WGSL.into()),
        });
        let bgl = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ferroui-physics rigid bgl"),
            entries: &[
                bgl_entry(0, wgpu::BufferBindingType::Uniform),
                bgl_entry(1, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(2, wgpu::BufferBindingType::Storage { read_only: true }),
                bgl_entry(3, wgpu::BufferBindingType::Storage { read_only: false }),
            ],
        });
        let layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ferroui-physics rigid layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("integrate_bodies"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("integrate_bodies"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let uniform_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rigid uniforms"),
            size: std::mem::size_of::<RigidUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let initial_cap = 256usize;
        let (body_buf, radii_buf, aabb_buf, body_rb, aabb_rb, bind_group) =
            create_rigid_buffers(&ctx.device, &bgl, &uniform_buffer, initial_cap);
        Self {
            ctx,
            pipeline,
            bgl,
            uniform_buffer,
            capacity: initial_cap,
            body_buf,
            radii_buf,
            aabb_buf,
            body_rb,
            aabb_rb,
            bind_group,
        }
    }

    /// Integrates `bodies` on the GPU for `dt` seconds under `gravity` and writes back updated
    /// body states and computed world-space AABBs.
    pub fn integrate_batch(
        &mut self,
        bodies: &mut [GpuRigidBodyRecord],
        radii: &[f32],
        gravity: [f32; 2],
        dt: f32,
    ) -> Vec<GpuAabbRecord> {
        let n = bodies.len().min(radii.len());
        if n == 0 || dt <= 0.0 {
            return Vec::new();
        }
        if n > self.capacity {
            let new_cap = n.next_power_of_two().max(256);
            let (bb, rb, ab, brb, arb, bg) =
                create_rigid_buffers(&self.ctx.device, &self.bgl, &self.uniform_buffer, new_cap);
            self.body_buf = bb;
            self.radii_buf = rb;
            self.aabb_buf = ab;
            self.body_rb = brb;
            self.aabb_rb = arb;
            self.bind_group = bg;
            self.capacity = new_cap;
        }
        let body_bytes = (n * std::mem::size_of::<GpuRigidBodyRecord>()) as u64;
        let aabb_bytes = (n * std::mem::size_of::<GpuAabbRecord>()) as u64;

        let uniforms = RigidUniforms { gravity, dt, body_count: n as u32 };
        self.ctx.queue.write_buffer(&self.uniform_buffer, 0, struct_as_bytes(&uniforms));
        self.ctx.queue.write_buffer(&self.body_buf, 0, slice_as_bytes(&bodies[..n]));
        self.ctx.queue.write_buffer(&self.radii_buf, 0, slice_as_bytes(&radii[..n]));

        let mut encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rigid integrate encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("rigid integrate pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            let groups = (n as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            pass.dispatch_workgroups(groups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&self.body_buf, 0, &self.body_rb, 0, body_bytes);
        encoder.copy_buffer_to_buffer(&self.aabb_buf, 0, &self.aabb_rb, 0, aabb_bytes);
        self.ctx.queue.submit(Some(encoder.finish()));

        let body_done = Arc::new(AtomicBool::new(false));
        let aabb_done = Arc::new(AtomicBool::new(false));
        let bd = Arc::clone(&body_done);
        let ad = Arc::clone(&aabb_done);
        self.body_rb.slice(0..body_bytes).map_async(wgpu::MapMode::Read, move |r| bd.store(r.is_ok(), Ordering::SeqCst));
        self.aabb_rb.slice(0..aabb_bytes).map_async(wgpu::MapMode::Read, move |r| ad.store(r.is_ok(), Ordering::SeqCst));
        let _ = self.ctx.device.poll(wgpu::PollType::wait_indefinitely());

        if body_done.load(Ordering::SeqCst) {
            if let Ok(view) = self.body_rb.slice(0..body_bytes).get_mapped_range() {
                // SAFETY: GpuRigidBodyRecord is #[repr(C)] POD matching WGSL GpuRigidBody.
                let dst = unsafe {
                    std::slice::from_raw_parts_mut(bodies.as_mut_ptr() as *mut u8, body_bytes as usize)
                };
                dst.copy_from_slice(&view[..dst.len()]);
            }
            self.body_rb.unmap();
        }

        let mut aabbs = vec![GpuAabbRecord::default(); n];
        if aabb_done.load(Ordering::SeqCst) {
            if let Ok(view) = self.aabb_rb.slice(0..aabb_bytes).get_mapped_range() {
                // SAFETY: GpuAabbRecord is #[repr(C)] POD matching WGSL GpuAabb.
                let dst = unsafe {
                    std::slice::from_raw_parts_mut(aabbs.as_mut_ptr() as *mut u8, aabb_bytes as usize)
                };
                dst.copy_from_slice(&view[..dst.len()]);
            }
            self.aabb_rb.unmap();
        }
        aabbs
    }
}

fn create_rigid_buffers(
    device: &wgpu::Device,
    bgl: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
    capacity: usize,
) -> (wgpu::Buffer, wgpu::Buffer, wgpu::Buffer, wgpu::Buffer, wgpu::Buffer, wgpu::BindGroup) {
    let body_bytes = (capacity * std::mem::size_of::<GpuRigidBodyRecord>()) as u64;
    let radii_bytes = (capacity * std::mem::size_of::<f32>()) as u64;
    let aabb_bytes = (capacity * std::mem::size_of::<GpuAabbRecord>()) as u64;

    let body_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rigid bodies"),
        size: body_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let radii_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rigid radii"),
        size: radii_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let aabb_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rigid aabbs"),
        size: aabb_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let body_rb = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rigid bodies readback"),
        size: body_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let aabb_rb = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rigid aabbs readback"),
        size: aabb_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("rigid integrate bg"),
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: body_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: radii_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: aabb_buf.as_entire_binding() },
        ],
    });
    (body_buf, radii_buf, aabb_buf, body_rb, aabb_rb, bind_group)
}
