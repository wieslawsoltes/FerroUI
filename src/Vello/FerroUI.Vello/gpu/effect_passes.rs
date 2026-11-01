//! Render passes FerroUI owns on the device of a GPU mode, for what vello's
//! renderers cannot express: the result is a texture of the device that the
//! scene paints with (a `VelloSceneTexture`), so nothing is drawn on the
//! processor or uploaded (docs/porting/gpu-rendering.md, "Vello").
//!
//! - The lighting pass evaluates the lights of WinUI's composition per pixel
//!   into a texture: a WGSL port of the light math of the compositor
//!   (`server_composition_light.rs`), the same as the SkSL program of the
//!   Skia backend (`ferroui-skia/composition_effects.rs`). Each pixel of the
//!   texture is a device pixel; the inverse of the transform of the drawing
//!   maps it to the point of the lit visual it shows.
//!
//! The pipelines are compiled once per device and thread. The textures,
//! with their uniform buffers and bind groups, are kept in a pool per size
//! (rounded up to 32 pixels):
//! a texture the scene of a frame paints with is in use until the scene is
//! rendered, and is reused by the frames after it, so steady frames
//! allocate nothing.

use super::{VelloDeviceTexture, VelloTextureAlpha, VelloWgpuDevice};
use ferroui_base::platform::LightingParameters;
use ferroui_base::rendering::composition::server::LightSource;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Weak};

/// The most point and spot lights the pass evaluates.
const MAX_LIGHTS: usize = LightingParameters::MAX_LIGHTS;

const _: () = assert!(MAX_LIGHTS == 8, "the array of lights of the WGSL program has 8 elements");

/// Floats of a light in the uniform block: 10 `vec4<f32>`.
const LIGHT_FLOATS: usize = 40;
/// Floats of the uniform block before the lights: 6 `vec4<f32>`.
const HEAD_FLOATS: usize = 24;
const UNIFORM_BYTES: u64 = ((HEAD_FLOATS + LIGHT_FLOATS * MAX_LIGHTS) * 4) as u64;

const LIGHTING_WGSL: &str = r#"
struct Light {
    // x: 1 point, 2 spot; y, z: the cosines of the inner and outer cones.
    kind: vec4<f32>,
    // The colour of a point light, the inner colour of a spot light.
    color: vec4<f32>,
    outer_color: vec4<f32>,
    position: vec4<f32>,
    direction: vec4<f32>,
    // constant, linear, quadratic, minimum cutoff
    attenuation: vec4<f32>,
    // x: the maximum cutoff, y: 1 when there is one
    cutoff: vec4<f32>,
    // The rows of the transform into the space of the light.
    to_x: vec4<f32>,
    to_y: vec4<f32>,
    to_w: vec4<f32>,
}

struct Params {
    constant_light: vec4<f32>,
    // ambient, diffuse, specular amounts and the shine
    material: vec4<f32>,
    // x: 1 for factors (the light over y), 0 for the light clamped to 0..1;
    // z: the number of lights
    output_mode: vec4<f32>,
    // x, y: the device pixel of the top left of the texture
    origin: vec4<f32>,
    // The rows of the transform from device pixels to the drawing.
    to_local_x: vec4<f32>,
    to_local_y: vec4<f32>,
    lights: array<Light, 8>,
}

@group(0) @binding(0) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn attenuation_factor(a: vec4<f32>, c: vec4<f32>, d: f32) -> f32 {
    if (c.y > 0.5 && d > c.x) {
        return 0.0;
    }
    let dd = max(max(d, a.w), 0.0);
    let denominator = a.x + a.y * dd + a.z * dd * dd;
    if (denominator <= 0.0) {
        return 1.0;
    }
    return 1.0 / denominator;
}

fn specular_factor(to_light: vec3<f32>, shine: f32) -> f32 {
    let len = length(to_light);
    if (len <= 0.0 || to_light.z <= 0.0) {
        return 0.0;
    }
    let h = to_light / len + vec3<f32>(0.0, 0.0, 1.0);
    let half_length = length(h);
    if (half_length <= 0.0) {
        return 0.0;
    }
    let x = max(h.z / half_length, 0.0);
    if (x <= 0.0) {
        return 0.0;
    }
    return pow(x, max(shine, 1.0));
}

fn spot_cone(pos: vec3<f32>, dir: vec3<f32>, cos_inner: f32, cos_outer: f32, p: vec2<f32>) -> f32 {
    let to_point = vec3<f32>(p - pos.xy, -pos.z);
    let to_point_length = length(to_point);
    let direction_length = length(dir);
    if (to_point_length <= 0.0 || direction_length <= 0.0) {
        return 0.0;
    }
    let c = dot(to_point, dir) / (to_point_length * direction_length);
    if (c >= cos_inner) {
        return 1.0;
    }
    if (c <= cos_outer || cos_inner <= cos_outer) {
        return 0.0;
    }
    let t = clamp((c - cos_outer) / (cos_inner - cos_outer), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let device = vec3<f32>(params.origin.xy + fragment.xy, 1.0);
    let coord = vec2<f32>(dot(params.to_local_x.xyz, device), dot(params.to_local_y.xyz, device));
    var light = params.constant_light.rgb;
    let count = i32(params.output_mode.z);
    let material = params.material;
    for (var i = 0; i < 8; i = i + 1) {
        if (i >= count) {
            break;
        }
        let l = params.lights[i];
        let point = vec3<f32>(coord, 1.0);
        let p = vec2<f32>(dot(l.to_x.xyz, point), dot(l.to_y.xyz, point)) / dot(l.to_w.xyz, point);
        let pos = l.position.xyz;
        let to_light = vec3<f32>(pos.xy - p, pos.z);
        let d = length(to_light);
        var incidence = 0.0;
        if (d > 0.0 && pos.z > 0.0) {
            incidence = pos.z / d;
        }
        let att = attenuation_factor(l.attenuation, l.cutoff, d);
        var c = l.color.rgb;
        var cone = 1.0;
        if (l.kind.x > 1.5) {
            cone = spot_cone(pos, l.direction.xyz, l.kind.y, l.kind.z, p);
            if (cone <= 0.0) {
                continue;
            }
            c = mix(l.outer_color.rgb, l.color.rgb, cone);
        }
        var lit = vec3<f32>(0.0);
        if (material.y != 0.0) {
            lit = c * (cone * incidence * att) * material.y;
        }
        if (material.z != 0.0) {
            lit = lit + c * (cone * specular_factor(to_light, material.w) * att) * material.z;
        }
        light = light + lit;
    }
    if (params.output_mode.x > 0.5) {
        let scale = params.output_mode.y;
        return vec4<f32>(clamp(light, vec3<f32>(0.0), vec3<f32>(scale)) / scale, 1.0);
    }
    return vec4<f32>(clamp(light, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
"#;

/// What the lighting pass writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum LightingOutput {
    /// The light clamped to 0..1, opaque.
    Light,
    /// The light clamped to 0..`scale`, over it.
    Factors { scale_bits: u32 },
}

/// The uniform block of the lighting pass.
fn uniforms(lighting: &LightingParameters, output: LightingOutput, origin: (u32, u32), to_local: kurbo::Affine) -> Vec<f32> {
    let mut floats = vec![0f32; HEAD_FLOATS + LIGHT_FLOATS * MAX_LIGHTS];
    let constant = lighting.constant;
    let material = lighting.material;
    let (mode, scale) = match output {
        LightingOutput::Light => (0.0, 1.0),
        LightingOutput::Factors { scale_bits } => (1.0, f32::from_bits(scale_bits)),
    };
    let [a, b, c, d, e, f] = to_local.as_coeffs();
    floats[0..4].copy_from_slice(&[constant.r as f32, constant.g as f32, constant.b as f32, 0.0]);
    floats[4..8].copy_from_slice(&[material.ambient as f32, material.diffuse as f32, material.specular as f32, material.shine as f32]);
    floats[8..12].copy_from_slice(&[mode, scale, lighting.lights.len().min(MAX_LIGHTS) as f32, 0.0]);
    floats[12..16].copy_from_slice(&[origin.0 as f32, origin.1 as f32, 0.0, 0.0]);
    // x' = a x + c y + e, y' = b x + d y + f
    floats[16..20].copy_from_slice(&[a as f32, c as f32, e as f32, 0.0]);
    floats[20..24].copy_from_slice(&[b as f32, d as f32, f as f32, 0.0]);
    let max_cutoff = |value: f64| -> [f32; 2] {
        if value.is_nan() || value >= f64::from(f32::MAX) {
            [0.0, 0.0]
        } else if value <= -f64::from(f32::MAX) {
            [-f32::MAX, 1.0]
        } else {
            [value as f32, 1.0]
        }
    };
    for (index, light) in lighting.lights.iter().take(MAX_LIGHTS).enumerate() {
        let at = HEAD_FLOATS + index * LIGHT_FLOATS;
        let slot = |field: usize| at + field * 4;
        let m = light.to_light_space;
        floats[slot(7)..slot(7) + 3].copy_from_slice(&[m.m11 as f32, m.m21 as f32, m.m31 as f32]);
        floats[slot(8)..slot(8) + 3].copy_from_slice(&[m.m12 as f32, m.m22 as f32, m.m32 as f32]);
        floats[slot(9)..slot(9) + 3].copy_from_slice(&[m.m13 as f32, m.m23 as f32, m.m33 as f32]);
        let (attenuation, position) = match light.source {
            LightSource::Point { color, position, attenuation } => {
                floats[slot(0)] = 1.0;
                floats[slot(1)..slot(1) + 3].copy_from_slice(&[color.r as f32, color.g as f32, color.b as f32]);
                (attenuation, position)
            }
            LightSource::Spot { inner_color, outer_color, position, direction, inner_cone_angle, outer_cone_angle, attenuation } => {
                let outer = outer_cone_angle.clamp(0.0, std::f64::consts::PI);
                let inner = inner_cone_angle.clamp(0.0, outer);
                floats[slot(0)..slot(0) + 3].copy_from_slice(&[2.0, inner.cos() as f32, outer.cos() as f32]);
                floats[slot(1)..slot(1) + 3].copy_from_slice(&[inner_color.r as f32, inner_color.g as f32, inner_color.b as f32]);
                floats[slot(2)..slot(2) + 3].copy_from_slice(&[outer_color.r as f32, outer_color.g as f32, outer_color.b as f32]);
                floats[slot(4)..slot(4) + 3].copy_from_slice(&[direction.x as f32, direction.y as f32, direction.z as f32]);
                (attenuation, position)
            }
            // The parameters list only point and spot lights.
            LightSource::Ambient { .. } | LightSource::Distant { .. } => continue,
        };
        floats[slot(3)..slot(3) + 3].copy_from_slice(&[position.x as f32, position.y as f32, position.z as f32]);
        let min_cutoff = attenuation.min_cutoff.clamp(-f64::from(f32::MAX), f64::from(f32::MAX));
        floats[slot(5)..slot(5) + 4].copy_from_slice(&[
            attenuation.constant as f32,
            attenuation.linear as f32,
            attenuation.quadratic as f32,
            min_cutoff as f32,
        ]);
        floats[slot(6)..slot(6) + 2].copy_from_slice(&max_cutoff(attenuation.max_cutoff));
    }
    floats
}

/// The compiled lighting pipeline of a device.
struct Pipeline {
    device: Weak<VelloWgpuDevice>,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

/// A texture of the pool with what the pass binds to draw into it.
struct PooledTexture {
    texture: Arc<VelloDeviceTexture>,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

ferroui_base::ferro_thread_local! {
    // The objects of the device are not destroyed when the thread ends: the
    // bindings of the drivers use thread-local state of their own to
    // release them, which is gone by then. The thread that renders lives as
    // long as the application.
    static PIPELINES: ManuallyDrop<RefCell<Vec<Pipeline>>> = ManuallyDrop::new(RefCell::new(Vec::new()));
    static POOL: ManuallyDrop<RefCell<HashMap<(usize, u32, u32), Vec<PooledTexture>>>> =
        ManuallyDrop::new(RefCell::new(HashMap::new()));
    static ALLOCATIONS: Cell<u64> = Cell::new(0);
}

/// How many textures the passes of this thread allocated.
#[allow(dead_code)]
pub(crate) fn texture_allocations() -> u64 {
    ALLOCATIONS.with(Cell::get)
}

fn with_pipeline<R>(device: &Arc<VelloWgpuDevice>, f: impl FnOnce(&Pipeline) -> R) -> R {
    PIPELINES.with(|pipelines| {
        let mut pipelines = pipelines.borrow_mut();
        pipelines.retain(|pipeline| pipeline.device.strong_count() > 0);
        if let Some(pipeline) = pipelines.iter().find(|pipeline| std::ptr::eq(pipeline.device.as_ptr(), Arc::as_ptr(device))) {
            return f(pipeline);
        }
        let wgpu_device = device.device();
        let module = wgpu_device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("FerroUI lighting"),
            source: wgpu::ShaderSource::Wgsl(LIGHTING_WGSL.into()),
        });
        let layout = wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("FerroUI lighting"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = wgpu_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("FerroUI lighting"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = wgpu_device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FerroUI lighting"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        pipelines.push(Pipeline { device: Arc::downgrade(device), pipeline, layout });
        f(pipelines.last().unwrap())
    })
}

/// Evaluates the lights of `lighting` into a texture of at least `width` by
/// `height` device pixels whose top left is the device pixel `origin`;
/// `to_local` maps a device pixel to the coordinates of the drawing. The
/// texture is the pool's: it is reused once nothing paints with it.
pub(crate) fn lighting_texture(
    device: &Arc<VelloWgpuDevice>,
    width: u32,
    height: u32,
    origin: (u32, u32),
    to_local: kurbo::Affine,
    lighting: &LightingParameters,
    output: LightingOutput,
) -> Option<Arc<VelloDeviceTexture>> {
    if lighting.lights.len() > MAX_LIGHTS || width == 0 || height == 0 {
        return None;
    }
    // The sizes of the pool are multiples of 32 pixels: an area that moves
    // by a fraction of a pixel covers one pixel more or less, and takes the
    // same texture (the scene paints the part the area covers).
    let (width, height) = (width.div_ceil(32) * 32, height.div_ceil(32) * 32);
    let floats = uniforms(lighting, output, origin, to_local);
    let bytes: Vec<u8> = floats.iter().flat_map(|value| value.to_ne_bytes()).collect();
    with_pipeline(device, |pipeline| {
        POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            // The textures of a lost device are of no use.
            pool.retain(|_, entries| entries.first().is_none_or(|entry| !entry.texture.device().is_lost()));
            let entries = pool.entry((Arc::as_ptr(device) as usize, width, height)).or_default();
            // A texture nothing else holds: its last scene was rendered.
            let index = match entries.iter().position(|entry| Arc::strong_count(&entry.texture) == 1) {
                Some(index) => index,
                None => {
                    let texture = VelloDeviceTexture::new(device, width, height, VelloTextureAlpha::Premultiplied);
                    let buffer = device.device().create_buffer(&wgpu::BufferDescriptor {
                        label: Some("FerroUI lighting"),
                        size: UNIFORM_BYTES,
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    let bind_group = device.device().create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("FerroUI lighting"),
                        layout: &pipeline.layout,
                        entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
                    });
                    ALLOCATIONS.with(|count| count.set(count.get() + 1));
                    entries.push(PooledTexture { texture, buffer, bind_group });
                    entries.len() - 1
                }
            };
            let entry = &entries[index];
            device.queue().write_buffer(&entry.buffer, 0, &bytes);
            let mut encoder = device.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("FerroUI lighting") });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("FerroUI lighting"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: entry.texture.view(),
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline.pipeline);
                pass.set_bind_group(0, &entry.bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
            device.queue().submit(Some(encoder.finish()));
            Some(entry.texture.clone())
        })
    })
}
