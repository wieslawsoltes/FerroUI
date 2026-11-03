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
//! - The passes of the layers with an effect (step 2 of the route) read the
//!   content of a layer, which the drawing context recorded into a scene of
//!   the device and rendered into a texture of the pool, and write another
//!   texture of the pool, which the scene the layer was pushed in paints
//!   with: the colour matrix, the multiply layer of lit visuals (the lights
//!   evaluated per pixel times the content), the pixel operations of two
//!   sources (the blend modes the layers lack, the arithmetic composite,
//!   `MaskInvert`) and the matrix convolution. The order is that of the
//!   queue: the content is submitted first, then the pass, then the scene
//!   that paints with its result.
//!
//! The pipelines are compiled once per device and thread; the parameters
//! are uniforms written for each pass. The textures, kept in pools per size
//! (rounded up to 32 pixels), are in use while something holds them: a
//! texture a scene paints with is held by the scene until it is rendered,
//! and is reused by the frames after it, so steady frames allocate nothing.

// The layer passes are drawn by the hybrid renderer's scenes of the device:
// a build without it has none.
#![cfg_attr(not(feature = "hybrid"), allow(dead_code))]

use super::{VelloDeviceTexture, VelloTextureAlpha, VelloWgpuDevice};
use ferroui_base::media::{ColorMatrix, ColorMatrixAlphaMode, ConvolveMatrix};
use ferroui_base::platform::{LightingParameters, PixelOperation};
use ferroui_base::rendering::composition::effects::BlendEffectMode;
use ferroui_base::rendering::composition::server::{LightSource, LIGHT_MAP_MAX_SCALE};
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

/// The light math of the lighting passes: the uniforms, the vertex stage
/// and `light_at`, followed by the fragment stage of a pass.
const LIGHT_COMMON_WGSL: &str = r#"
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
    // x, y: the device pixel of the top left of the texture; z, w: the size
    // of the area of the content (the multiply pass)
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

// The light reflected at the device pixel of the fragment, not clamped.
fn light_at(fragment: vec4<f32>) -> vec3<f32> {
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
    return light;
}
"#;

/// The fragment stage of the lighting pass: the light, opaque.
const LIGHTING_FRAGMENT_WGSL: &str = r#"
@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let light = light_at(fragment);
    if (params.output_mode.x > 0.5) {
        let scale = params.output_mode.y;
        return vec4<f32>(clamp(light, vec3<f32>(0.0), vec3<f32>(scale)) / scale, 1.0);
    }
    return vec4<f32>(clamp(light, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
"#;

/// The fragment stage of the multiply layer of lit visuals: each colour
/// channel of the content times the light at its pixel, clamped to
/// 0..`scale` (`output_mode.y`), the result clamped to the alpha, the alpha
/// kept (`MultiplyLayerFactors::apply_to_premultiplied_rgba8` with the
/// light evaluated at each pixel).
const LIGHT_MULTIPLY_FRAGMENT_WGSL: &str = r#"
@group(0) @binding(1) var content: texture_2d<f32>;

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.origin.z || f32(pixel.y) >= params.origin.w) {
        return vec4<f32>(0.0);
    }
    let c = textureLoad(content, pixel, 0);
    let factors = clamp(light_at(fragment), vec3<f32>(0.0), vec3<f32>(params.output_mode.y));
    return vec4<f32>(min(c.rgb * factors, vec3<f32>(c.a)), c.a);
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
            source: wgpu::ShaderSource::Wgsl(format!("{LIGHT_COMMON_WGSL}{LIGHTING_FRAGMENT_WGSL}").into()),
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

// --- layers with an effect --------------------------------------------------------------

/// The vertex stage of the layer passes: a triangle over the target.
const LAYER_VERTEX_WGSL: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}
"#;

/// A colour matrix (`ColorMatrix::apply_premultiplied`): the matrix applied
/// to the colour un-premultiplied and the result premultiplied again, or to
/// the premultiplied colour as it is with the colour clamped to the alpha.
const COLOR_MATRIX_WGSL: &str = r#"
struct Params {
    // What red, green, blue and alpha add to each output channel, then the
    // offsets (the rows of `ColorMatrix`).
    rows: array<vec4<f32>, 5>,
    // x: 1 to apply the matrix to the premultiplied colour as it is, 0 to
    // un-premultiply it first; y, z: the size of the area in pixels.
    mode: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var content: texture_2d<f32>;

fn apply_matrix(c: vec4<f32>) -> vec4<f32> {
    let m = params.rows[4] + c.r * params.rows[0] + c.g * params.rows[1] + c.b * params.rows[2] + c.a * params.rows[3];
    return clamp(m, vec4<f32>(0.0), vec4<f32>(1.0));
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.mode.y || f32(pixel.y) >= params.mode.z) {
        return vec4<f32>(0.0);
    }
    let c = textureLoad(content, pixel, 0);
    if (params.mode.x > 0.5) {
        let result = apply_matrix(c);
        return vec4<f32>(min(result.rgb, vec3<f32>(result.a)), result.a);
    }
    var straight = vec3<f32>(0.0);
    if (c.a > 0.0) {
        straight = min(c.rgb / c.a, vec3<f32>(1.0));
    }
    let result = apply_matrix(vec4<f32>(straight, c.a));
    return vec4<f32>(result.rgb * result.a, result.a);
}
"#;

/// The pixel operations of two sources (`PixelOperation`): the formulas of
/// `blend_pixels`, `arithmetic_pixels` and `mask_invert_pixels` of the
/// compositor, the next source (`source`) combined into what the sources
/// before it made (`destination`).
const PIXEL_OPERATION_WGSL: &str = r#"
struct Params {
    // x: 0 a blend, 1 the arithmetic composite, 2 MaskInvert; y: the blend
    // mode (`BlendEffectMode`); z, w: the size of the area in pixels.
    kind: vec4<f32>,
    // The multiply, destination, source and offset amounts.
    k: vec4<f32>,
    // The grid of Dissolve in the coordinates of the drawing (x, y, width,
    // height), and its cells along each side (x, y).
    grid: vec4<f32>,
    cells: vec4<f32>,
    // The rows of the transform from the pixels of the area to the drawing.
    to_local_x: vec4<f32>,
    to_local_y: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
// What the sources before made, and the next source.
@group(0) @binding(1) var first_texture: texture_2d<f32>;
@group(0) @binding(2) var next_texture: texture_2d<f32>;

fn straight(c: vec4<f32>) -> vec3<f32> {
    if (c.a > 0.0) {
        return min(c.rgb / c.a, vec3<f32>(1.0));
    }
    return vec3<f32>(0.0);
}

fn luminosity(c: vec3<f32>) -> f32 {
    return 0.3 * c.r + 0.59 * c.g + 0.11 * c.b;
}

fn color_dodge(b: f32, s: f32) -> f32 {
    if (b <= 0.0) {
        return 0.0;
    }
    if (s >= 1.0) {
        return 1.0;
    }
    return min(b / (1.0 - s), 1.0);
}

fn color_burn(b: f32, s: f32) -> f32 {
    if (b >= 1.0) {
        return 1.0;
    }
    if (s <= 0.0) {
        return 0.0;
    }
    return 1.0 - min((1.0 - b) / s, 1.0);
}

fn blend_channel(mode: i32, b: f32, s: f32) -> f32 {
    if (mode == 6) {
        return max(b + s - 1.0, 0.0);
    }
    if (mode == 14) {
        if (s <= 0.5) {
            return color_burn(b, 2.0 * s);
        }
        return color_dodge(b, 2.0 * (s - 0.5));
    }
    if (mode == 15) {
        return clamp(b + 2.0 * s - 1.0, 0.0, 1.0);
    }
    if (mode == 16) {
        if (s <= 0.5) {
            return min(b, 2.0 * s);
        }
        return max(b, 2.0 * s - 1.0);
    }
    if (mode == 17) {
        if (b + s >= 1.0) {
            return 1.0;
        }
        return 0.0;
    }
    if (mode == 24) {
        return max(b - s, 0.0);
    }
    if (mode == 25) {
        if (s <= 0.0) {
            if (b <= 0.0) {
                return 0.0;
            }
            return 1.0;
        }
        return min(b / s, 1.0);
    }
    return s;
}

// `dissolve_noise` of the compositor: a hash of the number of the cell.
fn dissolve_noise(index: u32) -> f32 {
    var x = index ^ 0x9E3779B9u;
    x = x * 0x85EBCA6Bu;
    x = x ^ (x >> 13u);
    x = x * 0xC2B2AE35u;
    x = x ^ (x >> 16u);
    return f32(x & 0xFFFFu) / 65535.0;
}

fn blend(d: vec4<f32>, s: vec4<f32>, fragment: vec2<f32>) -> vec4<f32> {
    let mode = i32(params.kind.y + 0.5);
    let cb = straight(d);
    let cs = straight(s);
    if (mode == 4) {
        // Dissolve: the foreground, opaque, where the noise of its cell is
        // under its alpha; the background elsewhere.
        let point = vec3<f32>(fragment, 1.0);
        let at = vec2<f32>(dot(params.to_local_x.xyz, point), dot(params.to_local_y.xyz, point));
        let column = clamp(floor((at.x - params.grid.x) * params.cells.x / params.grid.z), 0.0, params.cells.x - 1.0);
        let row = clamp(floor((at.y - params.grid.y) * params.cells.y / params.grid.w), 0.0, params.cells.y - 1.0);
        let index = u32(row) * u32(params.cells.x) + u32(column);
        if (s.a > 0.0 && dissolve_noise(index) < s.a) {
            return vec4<f32>(cs, 1.0);
        }
        return d;
    }
    var blended = vec3<f32>(0.0);
    if (mode == 7) {
        blended = cb;
        if (luminosity(cs) < luminosity(cb)) {
            blended = cs;
        }
    } else if (mode == 8) {
        blended = cb;
        if (luminosity(cs) > luminosity(cb)) {
            blended = cs;
        }
    } else {
        blended = vec3<f32>(blend_channel(mode, cb.r, cs.r), blend_channel(mode, cb.g, cs.g), blend_channel(mode, cb.b, cs.b));
    }
    let alpha = s.a + d.a - s.a * d.a;
    if (alpha <= 0.0) {
        return vec4<f32>(0.0);
    }
    let premultiplied = (1.0 - s.a) * d.a * cb + (1.0 - d.a) * s.a * cs + s.a * d.a * blended;
    let a = clamp(alpha, 0.0, 1.0);
    return vec4<f32>(clamp(premultiplied / alpha, vec3<f32>(0.0), vec3<f32>(1.0)) * a, a);
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.kind.z || f32(pixel.y) >= params.kind.w) {
        return vec4<f32>(0.0);
    }
    let d = textureLoad(first_texture, pixel, 0);
    let s = textureLoad(next_texture, pixel, 0);
    if (params.kind.x < 0.5) {
        return blend(d, s, fragment.xy);
    }
    if (params.kind.x < 1.5) {
        let k = params.k;
        let result = clamp(k.x * d * s + k.y * d + k.z * s + vec4<f32>(k.w), vec4<f32>(0.0), vec4<f32>(1.0));
        return vec4<f32>(min(result.rgb, vec3<f32>(result.a)), result.a);
    }
    let c = s.a * (vec3<f32>(d.a) - d.rgb) + (1.0 - s.a) * d.rgb;
    return vec4<f32>(clamp(c, vec3<f32>(0.0), vec3<f32>(d.a)), d.a);
}
"#;

/// The matrix convolution (`ConvolveMatrix::apply_to_premultiplied_rgba8`):
/// the kernel as a correlation (the weight of row `i`, column `j` at
/// `i * width + j`), the sum times the gain plus the bias, the colours
/// un-premultiplied and the alpha kept, or the four channels convolved.
const CONVOLUTION_WGSL: &str = r#"
struct Params {
    weights: array<vec4<f32>, 7>,
    // x, y: the columns and rows of the kernel; z, w: its target element.
    size: vec4<f32>,
    // x: the gain, y: the bias, z: 1 to keep the alpha, w: 1 to repeat the
    // edge pixels.
    factors: vec4<f32>,
    // x, y: the size of the area in pixels.
    extent: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var content: texture_2d<f32>;

fn fetch(p: vec2<i32>) -> vec4<f32> {
    let last = vec2<i32>(i32(params.extent.x) - 1, i32(params.extent.y) - 1);
    if (params.factors.w > 0.5) {
        return textureLoad(content, clamp(p, vec2<i32>(0), last), 0);
    }
    if (any(p < vec2<i32>(0)) || any(p > last)) {
        return vec4<f32>(0.0);
    }
    return textureLoad(content, p, 0);
}

fn weight(index: i32) -> f32 {
    let quad = params.weights[index / 4];
    return quad[index % 4];
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.extent.x || f32(pixel.y) >= params.extent.y) {
        return vec4<f32>(0.0);
    }
    let columns = i32(params.size.x);
    let rows = i32(params.size.y);
    let anchor = vec2<i32>(i32(params.size.z), i32(params.size.w));
    let keep_alpha = params.factors.z > 0.5;
    var sum = vec4<f32>(0.0);
    for (var row = 0; row < 5; row = row + 1) {
        if (row >= rows) {
            break;
        }
        for (var column = 0; column < 5; column = column + 1) {
            if (column >= columns) {
                break;
            }
            var c = fetch(pixel + vec2<i32>(column, row) - anchor);
            if (keep_alpha) {
                if (c.a > 0.0) {
                    c = vec4<f32>(c.rgb / c.a, c.a);
                } else {
                    c = vec4<f32>(0.0);
                }
            }
            sum = sum + c * weight(row * columns + column);
        }
    }
    var color = sum * params.factors.x + vec4<f32>(params.factors.y);
    if (keep_alpha) {
        let a = textureLoad(content, pixel, 0).a;
        color = vec4<f32>(color.rgb * a, a);
    } else {
        color.a = clamp(color.a, 0.0, 1.0);
    }
    return vec4<f32>(clamp(color.rgb, vec3<f32>(0.0), vec3<f32>(color.a)), color.a);
}
"#;

/// A program of the layers with an effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LayerProgram {
    ColorMatrix,
    LightMultiply,
    PixelOperation,
    Convolution,
}

impl LayerProgram {
    fn label(self) -> &'static str {
        match self {
            LayerProgram::ColorMatrix => "FerroUI colour matrix",
            LayerProgram::LightMultiply => "FerroUI light multiply",
            LayerProgram::PixelOperation => "FerroUI pixel operation",
            LayerProgram::Convolution => "FerroUI convolution",
        }
    }

    /// The WGSL of the program.
    fn source(self) -> String {
        match self {
            LayerProgram::ColorMatrix => format!("{LAYER_VERTEX_WGSL}{COLOR_MATRIX_WGSL}"),
            LayerProgram::LightMultiply => format!("{LIGHT_COMMON_WGSL}{LIGHT_MULTIPLY_FRAGMENT_WGSL}"),
            LayerProgram::PixelOperation => format!("{LAYER_VERTEX_WGSL}{PIXEL_OPERATION_WGSL}"),
            LayerProgram::Convolution => format!("{LAYER_VERTEX_WGSL}{CONVOLUTION_WGSL}"),
        }
    }

    /// The textures the program reads.
    fn inputs(self) -> usize {
        match self {
            LayerProgram::PixelOperation => 2,
            _ => 1,
        }
    }

    /// The floats of the uniform block.
    fn uniform_floats(self) -> usize {
        match self {
            LayerProgram::ColorMatrix => 6 * 4,
            LayerProgram::LightMultiply => HEAD_FLOATS + LIGHT_FLOATS * MAX_LIGHTS,
            LayerProgram::PixelOperation => 6 * 4,
            LayerProgram::Convolution => 10 * 4,
        }
    }
}

/// The compiled pipeline of a layer program on a device, with its uniform
/// buffer: every pass of the program is submitted on its own, so the
/// uniforms written for a pass reach the device before it and after the
/// pass before it.
struct LayerPipeline {
    device: Weak<VelloWgpuDevice>,
    program: LayerProgram,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    /// The bind groups the pipeline was run with, by the identities of the
    /// textures they bind (the textures of the pool, the same in steady
    /// frames).
    bind_groups: Vec<([u64; 2], wgpu::BindGroup)>,
}

/// The most bind groups a layer pipeline keeps.
const MAX_BIND_GROUPS: usize = 64;

ferroui_base::ferro_thread_local! {
    // Kept beyond the end of the thread, as the objects of the lighting
    // pass are (see there).
    static LAYER_PIPELINES: ManuallyDrop<RefCell<Vec<LayerPipeline>>> = ManuallyDrop::new(RefCell::new(Vec::new()));
    static LAYER_POOL: ManuallyDrop<RefCell<HashMap<(usize, u32, u32), Vec<Arc<VelloDeviceTexture>>>>> =
        ManuallyDrop::new(RefCell::new(HashMap::new()));
}

fn with_layer_pipeline<R>(device: &Arc<VelloWgpuDevice>, program: LayerProgram, f: impl FnOnce(&mut LayerPipeline) -> R) -> R {
    LAYER_PIPELINES.with(|pipelines| {
        let mut pipelines = pipelines.borrow_mut();
        pipelines.retain(|pipeline| pipeline.device.strong_count() > 0);
        let found = pipelines.iter().position(|pipeline| {
            pipeline.program == program && std::ptr::eq(pipeline.device.as_ptr(), Arc::as_ptr(device))
        });
        let index = match found {
            Some(index) => index,
            None => {
                let wgpu_device = device.device();
                let label = Some(program.label());
                let module = wgpu_device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label,
                    source: wgpu::ShaderSource::Wgsl(program.source().into()),
                });
                let mut entries = vec![wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }];
                for input in 0..program.inputs() {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding: 1 + input as u32,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    });
                }
                let layout = wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label, entries: &entries });
                let pipeline_layout = wgpu_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label,
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
                let pipeline = wgpu_device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label,
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
                let buffer = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
                    label,
                    size: (program.uniform_floats() * 4) as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                pipelines.push(LayerPipeline {
                    device: Arc::downgrade(device),
                    program,
                    pipeline,
                    layout,
                    buffer,
                    bind_groups: Vec::new(),
                });
                pipelines.len() - 1
            }
        };
        f(&mut pipelines[index])
    })
}

/// The size of the textures of the pool, and of the scenes rendered into
/// them, for an area of `width` by `height` pixels: rounded up to 32 pixels
/// (an area that moves by a fraction of a pixel takes the same texture), at
/// most the largest texture of the device.
pub(crate) fn pooled_size(device: &VelloWgpuDevice, width: u32, height: u32) -> (u32, u32) {
    let largest = device.device().limits().max_texture_dimension_2d;
    let round = |value: u32| (value.max(1).div_ceil(32) * 32).min(largest.max(value.max(1)));
    (round(width), round(height))
}

/// A texture of the pool of `width` by `height` pixels (a size of
/// [`pooled_size`]) that nothing else holds: it holds premultiplied colours,
/// can be rendered to by the hybrid renderer and the passes, and sampled.
/// It is the pool's again once the last holder drops it.
pub(crate) fn pooled_texture(device: &Arc<VelloWgpuDevice>, width: u32, height: u32) -> Arc<VelloDeviceTexture> {
    LAYER_POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        // The textures of a lost device are of no use.
        pool.retain(|_, entries| entries.first().is_none_or(|entry| !entry.device().is_lost()));
        let entries = pool.entry((Arc::as_ptr(device) as usize, width, height)).or_default();
        if let Some(texture) = entries.iter().find(|texture| Arc::strong_count(texture) == 1) {
            return texture.clone();
        }
        let texture = VelloDeviceTexture::new(device, width, height, VelloTextureAlpha::Premultiplied);
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        entries.push(texture.clone());
        texture
    })
}

/// Runs `program` with `uniforms` over `inputs` (textures of the pool of
/// one size) into a texture of the pool of that size, and returns it. The
/// pass is submitted: it reads what was submitted into the inputs before,
/// and a scene that paints with its result is rendered after it.
fn run_layer_pass(
    device: &Arc<VelloWgpuDevice>,
    program: LayerProgram,
    uniforms: &[f32],
    inputs: &[&Arc<VelloDeviceTexture>],
) -> Option<Arc<VelloDeviceTexture>> {
    let first = inputs.first()?;
    if inputs.len() != program.inputs()
        || uniforms.len() != program.uniform_floats()
        || inputs.iter().any(|input| input.width() != first.width() || input.height() != first.height())
    {
        return None;
    }
    let output = pooled_texture(device, first.width(), first.height());
    let bytes: Vec<u8> = uniforms.iter().flat_map(|value| value.to_ne_bytes()).collect();
    with_layer_pipeline(device, program, |pipeline| {
        let ids = [first.id(), inputs.get(1).map_or(0, |input| input.id())];
        let index = match pipeline.bind_groups.iter().position(|(key, _)| *key == ids) {
            Some(index) => index,
            None => {
                if pipeline.bind_groups.len() >= MAX_BIND_GROUPS {
                    pipeline.bind_groups.clear();
                }
                let mut entries = vec![wgpu::BindGroupEntry { binding: 0, resource: pipeline.buffer.as_entire_binding() }];
                for (slot, input) in inputs.iter().enumerate() {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 1 + slot as u32,
                        resource: wgpu::BindingResource::TextureView(input.view()),
                    });
                }
                let bind_group = device.device().create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(program.label()),
                    layout: &pipeline.layout,
                    entries: &entries,
                });
                pipeline.bind_groups.push((ids, bind_group));
                pipeline.bind_groups.len() - 1
            }
        };
        device.queue().write_buffer(&pipeline.buffer, 0, &bytes);
        let mut encoder =
            device.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(program.label()) });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(program.label()),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output.view(),
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
            pass.set_bind_group(0, &pipeline.bind_groups[index].1, &[]);
            pass.draw(0..3, 0..1);
        }
        device.queue().submit(Some(encoder.finish()));
    });
    Some(output)
}

/// The area of the content a pass writes: `width` by `height` pixels at
/// the top left of the textures; the rest of the textures is transparent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayerArea {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// `content` through `matrix` in `alpha_mode`.
pub(crate) fn color_matrix_texture(
    device: &Arc<VelloWgpuDevice>,
    content: &Arc<VelloDeviceTexture>,
    area: LayerArea,
    matrix: &ColorMatrix,
    alpha_mode: ColorMatrixAlphaMode,
) -> Option<Arc<VelloDeviceTexture>> {
    let mut floats = Vec::with_capacity(LayerProgram::ColorMatrix.uniform_floats());
    for row in matrix.rows {
        floats.extend_from_slice(&row);
    }
    let as_it_is = if alpha_mode == ColorMatrixAlphaMode::Straight { 1.0 } else { 0.0 };
    floats.extend_from_slice(&[as_it_is, area.width as f32, area.height as f32, 0.0]);
    run_layer_pass(device, LayerProgram::ColorMatrix, &floats, &[content])
}

/// `content` times the light `lighting` reflects at each of its pixels,
/// clamped to 0..`LIGHT_MAP_MAX_SCALE`; `to_local` maps a pixel of the
/// content to the coordinates of the lit visual.
pub(crate) fn light_multiply_texture(
    device: &Arc<VelloWgpuDevice>,
    content: &Arc<VelloDeviceTexture>,
    area: LayerArea,
    lighting: &LightingParameters,
    to_local: kurbo::Affine,
) -> Option<Arc<VelloDeviceTexture>> {
    if lighting.lights.len() > MAX_LIGHTS {
        return None;
    }
    let scale = LIGHT_MAP_MAX_SCALE as f32;
    let mut floats = uniforms(lighting, LightingOutput::Factors { scale_bits: scale.to_bits() }, (0, 0), to_local);
    floats[14] = area.width as f32;
    floats[15] = area.height as f32;
    run_layer_pass(device, LayerProgram::LightMultiply, &floats, &[content])
}

/// `operation` of `source` onto `destination`; `to_local` maps a pixel of
/// the sources to the coordinates of the drawing (where the grid of
/// `Dissolve` is).
pub(crate) fn pixel_operation_texture(
    device: &Arc<VelloWgpuDevice>,
    destination: &Arc<VelloDeviceTexture>,
    source: &Arc<VelloDeviceTexture>,
    area: LayerArea,
    operation: &PixelOperation,
    to_local: kurbo::Affine,
) -> Option<Arc<VelloDeviceTexture>> {
    let (kind, mode, k, grid, cells) = match *operation {
        PixelOperation::Blend { mode, noise_grid, noise_size } => {
            if mode == BlendEffectMode::Dissolve && (noise_grid.width <= 0.0 || noise_grid.height <= 0.0) {
                return None;
            }
            let grid = [noise_grid.x as f32, noise_grid.y as f32, noise_grid.width as f32, noise_grid.height as f32];
            let cells = [noise_size.width.max(1) as f32, noise_size.height.max(1) as f32, 0.0, 0.0];
            (0.0, mode as i32 as f32, [0.0; 4], grid, cells)
        }
        PixelOperation::Arithmetic { k } => (1.0, 0.0, k, [0.0; 4], [0.0; 4]),
        PixelOperation::MaskInvert => (2.0, 0.0, [0.0; 4], [0.0; 4], [0.0; 4]),
        #[allow(unreachable_patterns)]
        _ => return None,
    };
    let [a, b, c, d, e, f] = to_local.as_coeffs();
    let mut floats = Vec::with_capacity(LayerProgram::PixelOperation.uniform_floats());
    floats.extend_from_slice(&[kind, mode, area.width as f32, area.height as f32]);
    floats.extend_from_slice(&k);
    floats.extend_from_slice(&grid);
    floats.extend_from_slice(&cells);
    // x' = a x + c y + e, y' = b x + d y + f
    floats.extend_from_slice(&[a as f32, c as f32, e as f32, 0.0]);
    floats.extend_from_slice(&[b as f32, d as f32, f as f32, 0.0]);
    run_layer_pass(device, LayerProgram::PixelOperation, &floats, &[destination, source])
}

/// `content` convolved with `convolution`.
pub(crate) fn convolution_texture(
    device: &Arc<VelloWgpuDevice>,
    content: &Arc<VelloDeviceTexture>,
    area: LayerArea,
    convolution: &ConvolveMatrix,
) -> Option<Arc<VelloDeviceTexture>> {
    let mut floats = vec![0.0f32; LayerProgram::Convolution.uniform_floats()];
    for (slot, weight) in floats.iter_mut().zip(convolution.correlation_kernel()) {
        *slot = weight;
    }
    let (target_x, target_y) = convolution.target();
    let columns = convolution.kernel_width.clamp(1, ConvolveMatrix::MAX_SIDE);
    let rows = convolution.kernel_height.clamp(1, ConvolveMatrix::MAX_SIDE);
    floats[28..32].copy_from_slice(&[columns as f32, rows as f32, target_x as f32, target_y as f32]);
    let keep_alpha = if convolution.preserve_alpha { 1.0 } else { 0.0 };
    let repeat_edges = if convolution.clamp_edges { 1.0 } else { 0.0 };
    floats[32..36].copy_from_slice(&[convolution.gain(), convolution.bias, keep_alpha, repeat_edges]);
    floats[36..40].copy_from_slice(&[area.width as f32, area.height as f32, 0.0, 0.0]);
    run_layer_pass(device, LayerProgram::Convolution, &floats, &[content])
}
