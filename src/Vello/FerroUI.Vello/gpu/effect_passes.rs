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
//! - The passes of the runtime shader effects (#455, E5) are one more
//!   program on that route: the WGSL of a custom effect's description,
//!   wrapped with the bindings of the pass, checked by naga against the
//!   uniform convention of its parameters once per type and compiled once
//!   per device and type; a content-sampling program reads the content, an
//!   overlay is composed over it with its blend mode. The images an overlay
//!   samples (its children, E6) are textures of the device, uploaded once
//!   per device and image and kept by the identity of the image's handle
//!   while a handle lives ([`release_dropped_effect_images`]).
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
// The uniform blocks of the passes, written once for this route and the WebGL2 one.
pub(crate) use crate::effect_uniforms::LightingOutput;
use crate::effect_uniforms::{self as blocks, MAX_LIGHTS};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::media::effects::{
    EffectImage, EffectImageWeak, ImmutableCustomEffect, ShaderUniformType, MAX_SHADER_CHILDREN,
    SHADER_WGSL_CHILDREN_BINDING, SHADER_WGSL_CONTENT_BINDING, SHADER_WGSL_PASS_BINDING, SHADER_WGSL_UNIFORMS_BINDING,
};
use ferroui_base::media::{ColorMatrix, ColorMatrixAlphaMode, ConvolveMatrix};
use ferroui_base::platform::{LightingParameters, PixelOperation};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::sync::{Arc, Weak};

/// The bytes of the uniform block of the lighting pass.
const UNIFORM_BYTES: u64 = (blocks::LIGHTING_FLOATS * 4) as u64;

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
    if width == 0 || height == 0 {
        return None;
    }
    // The sizes of the pool are multiples of 32 pixels: an area that moves
    // by a fraction of a pixel covers one pixel more or less, and takes the
    // same texture (the scene paints the part the area covers).
    let (width, height) = (width.div_ceil(32) * 32, height.div_ceil(32) * 32);
    if lighting.lights.len() > MAX_LIGHTS {
        let lights = blocks::many_lighting(lighting, output, origin, to_local);
        return many_lights_pass(device, None, (width, height), &lights);
    }
    let floats = blocks::lighting(lighting, output, origin, to_local);
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

/// The BT.1886 reference curve of Rec. 709 video turned into sRGB values (the curve of the Skia
/// backend's colour filter, `leased_gpu_bitmap.rs`).
const REC709_TO_SRGB_WGSL: &str = r#"
fn rec709_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let v = pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.4));
    let low = 12.92 * v;
    let high = 1.055 * pow(v, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, v < vec3<f32>(0.0031308));
}
"#;

/// A frame of NV12 in two textures of the device (`Nv12Conversion`, #575): the luma plane and the
/// chroma plane at half the size, its samples beside the left luma column and between the rows
/// (the siting of Rec. 709 video), read bilinearly by texel; the BT.709 matrix in full or limited
/// range; then the curve of `FrameConversion`. The result is opaque.
const NV12_CONVERSION_WGSL: &str = r#"
struct Params {
    // x: 1 for limited range, y: 1 when the first row is the bottom one.
    mode: vec4<f32>,
    // x, y: the size of the luma plane in pixels; z, w: that of the chroma plane.
    size: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var luma: texture_2d<f32>;
@group(0) @binding(2) var chroma: texture_2d<f32>;

fn chroma_texel(texel: vec2<i32>) -> vec2<f32> {
    let last = vec2<i32>(params.size.zw) - vec2<i32>(1);
    return textureLoad(chroma, clamp(texel, vec2<i32>(0), last), 0).rg;
}

fn chroma_at(pixel: vec2<i32>) -> vec2<f32> {
    let place = vec2<f32>(f32(pixel.x) * 0.5, (f32(pixel.y) + 0.5) * 0.5 - 0.5);
    let base = floor(place);
    let t = place - base;
    let texel = vec2<i32>(base);
    let top = mix(chroma_texel(texel), chroma_texel(texel + vec2<i32>(1, 0)), t.x);
    let bottom = mix(chroma_texel(texel + vec2<i32>(0, 1)), chroma_texel(texel + vec2<i32>(1, 1)), t.x);
    return mix(top, bottom, t.y);
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.size.x || f32(pixel.y) >= params.size.y) {
        return vec4<f32>(0.0);
    }
    var source = pixel;
    if (params.mode.y > 0.5) {
        source.y = i32(params.size.y) - 1 - pixel.y;
    }
    var y = textureLoad(luma, source, 0).r;
    var c = chroma_at(source) - vec2<f32>(128.0 / 255.0);
    if (params.mode.x > 0.5) {
        y = (y - 16.0 / 255.0) * (255.0 / 219.0);
        c = c * (255.0 / 224.0);
    }
    let rgb = vec3<f32>(y + 1.5748 * c.y, y - 0.1873 * c.x - 0.4681 * c.y, y + 1.8556 * c.x);
    return vec4<f32>(rec709_to_srgb(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0))), 1.0);
}
"#;

/// A frame of a producer (a video or a camera) as premultiplied RGBA of the canvas
/// (`FrameConversion`): read in whatever format its storage has (BGRA or RGBA), turned upright when
/// its first row is the bottom one, made opaque when the frame is, and, for full-range Rec. 709
/// video, the BT.1886 reference curve turned into sRGB values (the curve of the Skia backend's
/// colour filter, `leased_gpu_bitmap.rs`).
const FRAME_CONVERSION_WGSL: &str = r#"
struct Params {
    // x: 1 for full-range Rec. 709, y: 1 when the first row is the bottom one, z: 1 when the
    // frame is opaque.
    mode: vec4<f32>,
    // x, y: the size of the frame in pixels.
    size: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var frame: texture_2d<f32>;

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.size.x || f32(pixel.y) >= params.size.y) {
        return vec4<f32>(0.0);
    }
    var source = pixel;
    if (params.mode.y > 0.5) {
        source.y = i32(params.size.y) - 1 - pixel.y;
    }
    var c = textureLoad(frame, source, 0);
    if (params.mode.z > 0.5) {
        c.a = 1.0;
    }
    if (params.mode.x > 0.5) {
        c = vec4<f32>(rec709_to_srgb(c.rgb), c.a);
    }
    return c;
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
/// A multiply layer of factors: each colour channel of the content times the factor of its point,
/// sampled bilinearly between the centres of the samples (`MultiplyLayerFactors::factor_at`),
/// clamped to the alpha; a pixel outside the unit square of the factors is cleared.
const MULTIPLY_FACTORS_WGSL: &str = r#"
struct Params {
    // The rows of the transform from the pixels of the area to the unit square of the factors.
    row_u: vec4<f32>,
    row_v: vec4<f32>,
    // x, y: the size of the samples; z: what a sample of 1 stands for.
    samples: vec4<f32>,
    // x, y: the size of the area in pixels.
    extent: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var content: texture_2d<f32>;
@group(0) @binding(2) var factors: texture_2d<f32>;

fn sample(p: vec2<i32>) -> vec3<f32> {
    return textureLoad(factors, p, 0).rgb;
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.extent.x || f32(pixel.y) >= params.extent.y) {
        return vec4<f32>(0.0);
    }
    let c = textureLoad(content, pixel, 0);
    if (c.a <= 0.0) {
        return vec4<f32>(0.0);
    }
    let at = vec3<f32>(vec2<f32>(pixel) + 0.5, 1.0);
    let unit = vec2<f32>(dot(params.row_u.xyz, at), dot(params.row_v.xyz, at));
    if (any(unit < vec2<f32>(0.0)) || any(unit > vec2<f32>(1.0))) {
        return vec4<f32>(0.0);
    }
    let size = params.samples.xy;
    let p = clamp(unit * size - 0.5, vec2<f32>(0.0), size - 1.0);
    let a = vec2<i32>(floor(p));
    let b = min(a + vec2<i32>(1), vec2<i32>(size) - vec2<i32>(1));
    let f = p - vec2<f32>(a);
    let top = mix(sample(a), sample(vec2<i32>(b.x, a.y)), f.x);
    let bottom = mix(sample(vec2<i32>(a.x, b.y)), sample(b), f.x);
    let factor = mix(top, bottom, f.y) * params.samples.z;
    return vec4<f32>(min(c.rgb * factor, vec3<f32>(c.a)), c.a);
}
"#;

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
    FrameConversion,
    MultiplyFactors,
    Nv12Conversion,
}

impl LayerProgram {
    fn label(self) -> &'static str {
        match self {
            LayerProgram::ColorMatrix => "FerroUI colour matrix",
            LayerProgram::LightMultiply => "FerroUI light multiply",
            LayerProgram::PixelOperation => "FerroUI pixel operation",
            LayerProgram::Convolution => "FerroUI convolution",
            LayerProgram::FrameConversion => "FerroUI frame conversion",
            LayerProgram::MultiplyFactors => "FerroUI multiply factors",
            LayerProgram::Nv12Conversion => "FerroUI NV12 conversion",
        }
    }

    /// The WGSL of the program.
    fn source(self) -> String {
        match self {
            LayerProgram::ColorMatrix => format!("{LAYER_VERTEX_WGSL}{COLOR_MATRIX_WGSL}"),
            LayerProgram::LightMultiply => format!("{LIGHT_COMMON_WGSL}{LIGHT_MULTIPLY_FRAGMENT_WGSL}"),
            LayerProgram::PixelOperation => format!("{LAYER_VERTEX_WGSL}{PIXEL_OPERATION_WGSL}"),
            LayerProgram::Convolution => format!("{LAYER_VERTEX_WGSL}{CONVOLUTION_WGSL}"),
            LayerProgram::FrameConversion => {
                format!("{LAYER_VERTEX_WGSL}{REC709_TO_SRGB_WGSL}{FRAME_CONVERSION_WGSL}")
            }
            LayerProgram::Nv12Conversion => format!("{LAYER_VERTEX_WGSL}{REC709_TO_SRGB_WGSL}{NV12_CONVERSION_WGSL}"),
            LayerProgram::MultiplyFactors => format!("{LAYER_VERTEX_WGSL}{MULTIPLY_FACTORS_WGSL}"),
        }
    }

    /// The textures the program reads.
    fn inputs(self) -> usize {
        match self {
            LayerProgram::PixelOperation | LayerProgram::MultiplyFactors | LayerProgram::Nv12Conversion => 2,
            _ => 1,
        }
    }

    /// The floats of the uniform block.
    fn uniform_floats(self) -> usize {
        match self {
            LayerProgram::ColorMatrix => blocks::COLOR_MATRIX_FLOATS,
            LayerProgram::LightMultiply => blocks::LIGHTING_FLOATS,
            LayerProgram::PixelOperation => blocks::PIXEL_OPERATION_FLOATS,
            LayerProgram::FrameConversion | LayerProgram::Nv12Conversion => 2 * 4,
            LayerProgram::Convolution => blocks::CONVOLUTION_FLOATS,
            LayerProgram::MultiplyFactors => blocks::MULTIPLY_FACTORS_FLOATS,
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

/// The layout entry of a uniform buffer of the fragment stage at `binding`.
fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// The layout entry of a texture of the pool the fragment stage reads with
/// `textureLoad` at `binding`.
fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

/// The pipeline of a pass into a texture of the pool: the entry points
/// `vs_main` and `fs_main` of the WGSL `source`, the bind group `entries`
/// (group 0), writing `Rgba8Unorm` without blending.
fn create_pass_pipeline(
    wgpu_device: &wgpu::Device,
    label: Option<&str>,
    source: String,
    entries: &[wgpu::BindGroupLayoutEntry],
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let module = wgpu_device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label, entries });
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
    (pipeline, layout)
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
                let mut entries = vec![uniform_entry(0)];
                for input in 0..program.inputs() {
                    entries.push(texture_entry(1 + input as u32));
                }
                let (pipeline, layout) = create_pass_pipeline(wgpu_device, label, program.source(), &entries);
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
    // The chroma plane of NV12 and the factors of a multiply layer are inputs of their own size.
    let same_size = matches!(program, LayerProgram::Nv12Conversion | LayerProgram::MultiplyFactors)
        || inputs.iter().all(|input| input.width() == first.width() && input.height() == first.height());
    if inputs.len() != program.inputs() || uniforms.len() != program.uniform_floats() || !same_size {
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
        if matches!(program, LayerProgram::FrameConversion | LayerProgram::Nv12Conversion) {
            // Each frame of a producer is converted once: its bind group would only keep the
            // frame's storage alive.
            pipeline.bind_groups.remove(index);
        }
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

/// `frame` (a texture wrapping a producer's storage) as a texture of the pool of its size, in
/// premultiplied RGBA: see [`FRAME_CONVERSION_WGSL`]. The pass is submitted: once the queue has
/// done it, the producer's storage is no longer read.
pub(crate) fn frame_conversion_texture(
    device: &Arc<VelloWgpuDevice>,
    frame: &Arc<VelloDeviceTexture>,
    rec709: bool,
    bottom_up: bool,
    opaque: bool,
) -> Option<Arc<VelloDeviceTexture>> {
    let flag = |value: bool| if value { 1.0 } else { 0.0 };
    let floats = [flag(rec709), flag(bottom_up), flag(opaque), 0.0, frame.width() as f32, frame.height() as f32, 0.0, 0.0];
    run_layer_pass(device, LayerProgram::FrameConversion, &floats, &[frame])
}

/// A frame of NV12 (`luma` and `chroma` wrap a producer's storage) as a texture of the pool of its
/// size, in opaque RGBA: see [`NV12_CONVERSION_WGSL`]. As for
/// [`frame_conversion_texture`], the storage is no longer read once the queue has done the pass.
pub(crate) fn nv12_conversion_texture(
    device: &Arc<VelloWgpuDevice>,
    luma: &Arc<VelloDeviceTexture>,
    chroma: &Arc<VelloDeviceTexture>,
    limited_range: bool,
    bottom_up: bool,
) -> Option<Arc<VelloDeviceTexture>> {
    let flag = |value: bool| if value { 1.0 } else { 0.0 };
    let floats = [
        flag(limited_range),
        flag(bottom_up),
        0.0,
        0.0,
        luma.width() as f32,
        luma.height() as f32,
        chroma.width() as f32,
        chroma.height() as f32,
    ];
    run_layer_pass(device, LayerProgram::Nv12Conversion, &floats, &[luma, chroma])
}

/// `content` through `matrix` in `alpha_mode`.
pub(crate) fn color_matrix_texture(
    device: &Arc<VelloWgpuDevice>,
    content: &Arc<VelloDeviceTexture>,
    area: LayerArea,
    matrix: &ColorMatrix,
    alpha_mode: ColorMatrixAlphaMode,
) -> Option<Arc<VelloDeviceTexture>> {
    let floats = blocks::color_matrix(matrix, alpha_mode, (area.width, area.height));
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
        let lights = blocks::many_light_multiply(lighting, (area.width, area.height), to_local);
        return many_lights_pass(device, Some(content), (content.width(), content.height()), &lights);
    }
    let floats = blocks::light_multiply(lighting, (area.width, area.height), to_local)?;
    run_layer_pass(device, LayerProgram::LightMultiply, &floats, &[content])
}

// --- more lights than the uniform block holds (#1158) ------------------------------------

/// The binding of the image of the lights.
const LIGHT_ROWS_BINDING: u32 = 3;
/// The texels of a light.
const LIGHT_ROW_TEXELS: u32 = (blocks::LIGHT_FLOATS / 4) as u32;

/// [`LIGHT_COMMON_WGSL`] with the lights read from an image, one row per light, so that their
/// number is not bound by the size of a uniform block.
fn many_lights_common_wgsl() -> String {
    let replacements = [
        ("    lights: array<Light, 8>,\n", ""),
        (
            "@group(0) @binding(0) var<uniform> params: Params;\n",
            "@group(0) @binding(0) var<uniform> params: Params;\n\
             @group(0) @binding(3) var light_rows: texture_2d<f32>;\n\
             fn light_field(i: i32, field: i32) -> vec4<f32> {\n\
                 return textureLoad(light_rows, vec2<i32>(field, i), 0);\n\
             }\n\
             fn light_of(i: i32) -> Light {\n\
                 return Light(light_field(i, 0), light_field(i, 1), light_field(i, 2), light_field(i, 3),\n\
                     light_field(i, 4), light_field(i, 5), light_field(i, 6), light_field(i, 7),\n\
                     light_field(i, 8), light_field(i, 9));\n\
             }\n",
        ),
        ("for (var i = 0; i < 8; i = i + 1) {", "for (var i = 0; i < count; i = i + 1) {"),
        ("let l = params.lights[i];", "let l = light_of(i);"),
    ];
    let mut source = LIGHT_COMMON_WGSL.to_string();
    for (from, to) in replacements {
        debug_assert_eq!(1, source.matches(from).count(), "{from}");
        source = source.replace(from, to);
    }
    source
}

/// The pipeline of the lights read from an image, with or without content to multiply.
struct ManyLightsPipeline {
    device: Weak<VelloWgpuDevice>,
    multiply: bool,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    /// The image of the lights and the rows it holds; it grows and is written each pass.
    rows: Option<(wgpu::Texture, wgpu::TextureView, u32)>,
    /// By the id of the content (0 for none).
    bind_groups: Vec<(u64, wgpu::BindGroup)>,
}

ferroui_base::ferro_thread_local! {
    // As `PIPELINES`: not destroyed when the thread ends.
    static MANY_LIGHTS_PIPELINES: ManuallyDrop<RefCell<Vec<ManyLightsPipeline>>> =
        ManuallyDrop::new(RefCell::new(Vec::new()));
}

/// Whether `lights` point and spot lights fit the image of the lights of `device`.
pub(crate) fn holds_lights(device: &VelloWgpuDevice, lights: usize) -> bool {
    lights <= MAX_LIGHTS || lights <= device.device().limits().max_texture_dimension_2d as usize
}

/// The lights of `lights` evaluated per pixel into a texture of the pool of `size`: over `content`
/// when there is one (the multiply layer), else the light itself (the lighting fill). The cost per
/// pixel grows with the number of lights, as in the uniform route; what a frame uploads is the one
/// image of the lights.
fn many_lights_pass(
    device: &Arc<VelloWgpuDevice>,
    content: Option<&Arc<VelloDeviceTexture>>,
    size: (u32, u32),
    lights: &blocks::ManyLights,
) -> Option<Arc<VelloDeviceTexture>> {
    let rows = lights.row_count();
    if rows == 0 || !holds_lights(device, rows as usize) {
        return None;
    }
    let output = pooled_texture(device, size.0, size.1);
    let multiply = content.is_some();
    MANY_LIGHTS_PIPELINES.with(|pipelines| {
        let mut pipelines = pipelines.borrow_mut();
        pipelines.retain(|pipeline| pipeline.device.strong_count() > 0);
        let found = pipelines.iter().position(|pipeline| {
            pipeline.multiply == multiply && std::ptr::eq(pipeline.device.as_ptr(), Arc::as_ptr(device))
        });
        let index = found.unwrap_or_else(|| {
            let wgpu_device = device.device();
            let label = Some(if multiply { "FerroUI many lights multiply" } else { "FerroUI many lights" });
            let fragment = if multiply { LIGHT_MULTIPLY_FRAGMENT_WGSL } else { LIGHTING_FRAGMENT_WGSL };
            let mut entries = vec![uniform_entry(0)];
            if multiply {
                entries.push(texture_entry(1));
            }
            entries.push(texture_entry(LIGHT_ROWS_BINDING));
            let source = format!("{}{fragment}", many_lights_common_wgsl());
            let (pipeline, layout) = create_pass_pipeline(wgpu_device, label, source, &entries);
            let buffer = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
                label,
                size: (blocks::HEAD_FLOATS * 4) as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            pipelines.push(ManyLightsPipeline {
                device: Arc::downgrade(device),
                multiply,
                pipeline,
                layout,
                buffer,
                rows: None,
                bind_groups: Vec::new(),
            });
            pipelines.len() - 1
        });
        let pipeline = &mut pipelines[index];
        if pipeline.rows.as_ref().is_none_or(|(_, _, held)| *held < rows) {
            let held = rows.next_power_of_two().min(device.device().limits().max_texture_dimension_2d);
            let texture = device.device().create_texture(&wgpu::TextureDescriptor {
                label: Some("FerroUI light rows"),
                size: wgpu::Extent3d { width: LIGHT_ROW_TEXELS, height: held, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            pipeline.rows = Some((texture, view, held));
            pipeline.bind_groups.clear();
            ALLOCATIONS.with(|count| count.set(count.get() + 1));
        }
        let (texture, view, _) = pipeline.rows.as_ref()?;
        let row_bytes: Vec<u8> = lights.rows.iter().flat_map(|value| value.to_ne_bytes()).collect();
        device.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &row_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(LIGHT_ROW_TEXELS * 16),
                rows_per_image: Some(rows),
            },
            wgpu::Extent3d { width: LIGHT_ROW_TEXELS, height: rows, depth_or_array_layers: 1 },
        );
        let id = content.map_or(0, |content| content.id());
        let group = match pipeline.bind_groups.iter().position(|(key, _)| *key == id) {
            Some(group) => group,
            None => {
                if pipeline.bind_groups.len() >= MAX_BIND_GROUPS {
                    pipeline.bind_groups.clear();
                }
                let mut entries =
                    vec![wgpu::BindGroupEntry { binding: 0, resource: pipeline.buffer.as_entire_binding() }];
                if let Some(content) = content {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(content.view()),
                    });
                }
                entries.push(wgpu::BindGroupEntry {
                    binding: LIGHT_ROWS_BINDING,
                    resource: wgpu::BindingResource::TextureView(view),
                });
                let bind_group = device.device().create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("FerroUI many lights"),
                    layout: &pipeline.layout,
                    entries: &entries,
                });
                pipeline.bind_groups.push((id, bind_group));
                pipeline.bind_groups.len() - 1
            }
        };
        let head: Vec<u8> = lights.head.iter().flat_map(|value| value.to_ne_bytes()).collect();
        device.queue().write_buffer(&pipeline.buffer, 0, &head);
        let mut encoder = device
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("FerroUI many lights") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FerroUI many lights"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, &pipeline.bind_groups[group].1, &[]);
            pass.draw(0..3, 0..1);
        }
        device.queue().submit(Some(encoder.finish()));
        Some(())
    })?;
    Some(output)
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
    let floats = blocks::pixel_operation(operation, (area.width, area.height), to_local)?;
    run_layer_pass(device, LayerProgram::PixelOperation, &floats, &[destination, source])
}

/// `content` convolved with `convolution`.
/// `content` multiplied by `factors`, whose samples are uploaded into a texture of the pool for
/// the pass. `to_unit` maps the pixels of the area to the unit square of the factors.
pub(crate) fn multiply_factors_texture(
    device: &Arc<VelloWgpuDevice>,
    content: &Arc<VelloDeviceTexture>,
    area: LayerArea,
    factors: &ferroui_base::platform::MultiplyLayerFactors,
    to_unit: kurbo::Affine,
) -> Option<Arc<VelloDeviceTexture>> {
    let (width, height, samples, scale) = blocks::factor_samples(factors);
    let largest = device.device().limits().max_texture_dimension_2d;
    if width > largest || height > largest {
        return None;
    }
    let (texture_width, texture_height) = pooled_size(device, width, height);
    let texture = pooled_texture(device, texture_width, texture_height);
    device.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: texture.texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &samples[..width as usize * height as usize * 4],
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    let floats = blocks::multiply_factors((width, height, scale), to_unit, (area.width, area.height));
    run_layer_pass(device, LayerProgram::MultiplyFactors, &floats, &[content, &texture])
}

pub(crate) fn convolution_texture(
    device: &Arc<VelloWgpuDevice>,
    content: &Arc<VelloDeviceTexture>,
    area: LayerArea,
    convolution: &ConvolveMatrix,
) -> Option<Arc<VelloDeviceTexture>> {
    let floats = blocks::convolution(convolution, (area.width, area.height));
    run_layer_pass(device, LayerProgram::Convolution, &floats, &[content])
}

// --- shader effects ---------------------------------------------------------------------

/// The WGSL module of the pass of a shader effect type (#455, E5): the
/// program of its description wrapped by
/// [`ShaderEffectDescription::wgsl_module`](ferroui_base::media::effects::ShaderEffectDescription::wgsl_module),
/// checked by naga against the uniform convention of the parameters of the
/// type, with the layout of its uniform block.
#[derive(Debug)]
pub(crate) struct ShaderModule {
    /// The module the pipelines are compiled from.
    source: String,
    /// The byte offset of each uniform of the convention in the block, in
    /// the order of the convention.
    offsets: Vec<u32>,
    /// The size of the block in bytes: a multiple of 16.
    size: usize,
    /// The number of children (images) the module binds.
    children: usize,
}

#[cfg(test)]
impl ShaderModule {
    /// The byte offset of each uniform of the convention in the block.
    pub(crate) fn offsets(&self) -> &[u32] {
        &self.offsets
    }

    /// The size of the block in bytes.
    pub(crate) fn size(&self) -> usize {
        self.size
    }
}

/// Whether a member of `struct Uniforms` has the type of the convention.
fn uniform_type_matches(inner: &wgpu::naga::TypeInner, expected: ShaderUniformType) -> bool {
    use wgpu::naga::{Scalar, TypeInner, VectorSize};
    match (inner, expected) {
        (TypeInner::Scalar(scalar), ShaderUniformType::Float) => *scalar == Scalar::F32,
        (TypeInner::Scalar(scalar), ShaderUniformType::Int) => *scalar == Scalar::I32,
        (TypeInner::Vector { size: VectorSize::Bi, scalar }, ShaderUniformType::Float2) => *scalar == Scalar::F32,
        (TypeInner::Vector { size: VectorSize::Quad, scalar }, ShaderUniformType::Float4) => *scalar == Scalar::F32,
        _ => false,
    }
}

/// The offsets of the members of the uniform block `uniforms` of `module`
/// and the size of the block, when its members are the uniforms of `layout`
/// (names, order and types, none an array or a struct).
fn uniform_block_layout(
    module: &wgpu::naga::Module,
    layout: &[(String, ShaderUniformType)],
) -> Result<(Vec<u32>, usize), String> {
    let expected = || {
        let members: Vec<String> = layout.iter().map(|(name, ty)| format!("{name}: {}", ty.wgsl())).collect();
        members.join(", ")
    };
    let Some((_, variable)) = module.global_variables.iter().find(|(_, variable)| {
        variable.name.as_deref() == Some("uniforms") && variable.space == wgpu::naga::AddressSpace::Uniform
    }) else {
        return Err("the module has no uniform block `uniforms`".to_owned());
    };
    let wgpu::naga::TypeInner::Struct { members, span } = &module.types[variable.ty].inner else {
        return Err(format!("Uniforms is not a struct ({})", expected()));
    };
    if members.len() != layout.len() {
        return Err(format!(
            "Uniforms has {} members, the convention has {} ({})",
            members.len(),
            layout.len(),
            expected()
        ));
    }
    let mut offsets = Vec::with_capacity(members.len());
    for (index, (member, (name, ty))) in members.iter().zip(layout).enumerate() {
        let declared = member.name.as_deref().unwrap_or("");
        if declared != name.as_str() {
            return Err(format!("member {index} of Uniforms is {declared}, the convention has {name} ({})", expected()));
        }
        if !uniform_type_matches(&module.types[member.ty].inner, *ty) {
            return Err(format!("the member {name} of Uniforms is not a {} ({})", ty.wgsl(), expected()));
        }
        offsets.push(member.offset);
    }
    let size = (*span as usize).max(16).div_ceil(16) * 16;
    Ok((offsets, size))
}

/// The module of the pass of the shader of `effect`, checked: the children
/// of the description are image parameters of an overlay
/// ([`ImmutableCustomEffect::check_shader_children`], E6), the module parses
/// and validates with naga (the version wgpu compiles it with, without
/// optional capabilities, so every device takes it), and its block
/// `uniforms` declares the uniforms of the convention
/// ([`ImmutableCustomEffect::shader_uniform_layout`]). The error says why
/// not. No device is needed.
pub(crate) fn check_shader_module(effect: &ImmutableCustomEffect) -> Result<ShaderModule, String> {
    use wgpu::naga;
    let Some(description) = effect.shader() else {
        return Err("the effect is not a shader effect".to_owned());
    };
    effect.check_shader_children()?;
    let source = description.wgsl_module();
    let module = naga::front::wgsl::parse_str(&source)
        .map_err(|error| format!("its WGSL does not parse: {}", error.emit_to_string(&source)))?;
    let mut validator = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty());
    validator
        .validate(&module)
        .map_err(|error| format!("its WGSL is not valid: {}", error.emit_to_string(&source)))?;
    let (offsets, size) = uniform_block_layout(&module, &effect.shader_uniform_layout())?;
    Ok(ShaderModule { source, offsets, size, children: description.children.len() })
}

/// The names of the shader effects whose module failed, each reported once.
static SHADER_REPORTED: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());

/// Reports that the shader effect `name` is not drawn by its WGSL (`reason`)
/// and that its content is drawn unchanged: logged the first time for a
/// name (an error in the log area `Vello`), when it returns `true`.
pub(crate) fn report_shader_failure(name: &'static str, reason: &str) -> bool {
    {
        let mut reported = SHADER_REPORTED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if reported.contains(&name) {
            return false;
        }
        reported.push(name);
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Vello") {
        logger.log(
            None,
            &format!("The shader of the custom effect {name} is not drawn ({reason}); its content is drawn unchanged."),
        );
    }
    true
}

/// How many times the failure of the shader effect `name` was reported (0
/// or 1).
#[cfg(test)]
pub(crate) fn shader_failure_reports(name: &str) -> usize {
    SHADER_REPORTED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .filter(|reported| **reported == name)
        .count()
}

/// The compiled pipeline of a shader effect type on a device, with its
/// buffers: the block of the effect and the uniforms of the pass, written
/// for each pass (every pass is submitted on its own, as the layer passes
/// are).
struct ShaderPipeline {
    device: Weak<VelloWgpuDevice>,
    type_id: TypeId,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniforms: wgpu::Buffer,
    pass: wgpu::Buffer,
    /// The bind groups the pipeline was run with, by the identities of the
    /// textures they bind: the content (a texture of the pool) and the
    /// images of the children ([`ImageTexture::id`]), 0 for no child.
    bind_groups: Vec<(BindGroupKey, wgpu::BindGroup)>,
    /// The bytes of the block, kept for the next pass.
    block: Vec<u8>,
}

/// The identities of the textures a bind group of a shader pipeline binds:
/// the content, then the children.
type BindGroupKey = [u64; 1 + MAX_SHADER_CHILDREN];

ferroui_base::ferro_thread_local! {
    // The modules of the shader effect types of this thread, checked once
    // per type; `None` for a type whose module failed (reported once).
    static SHADER_MODULES: RefCell<HashMap<TypeId, Option<Rc<ShaderModule>>>> = RefCell::new(HashMap::new());
    // Kept beyond the end of the thread, as the objects of the lighting
    // pass are (see there).
    static SHADER_PIPELINES: ManuallyDrop<RefCell<Vec<ShaderPipeline>>> = ManuallyDrop::new(RefCell::new(Vec::new()));
    // How many shader pipelines this thread compiled (#455, E7): the tests
    // hold frames that change only uniforms to compiling none.
    static SHADER_PIPELINES_CREATED: Cell<u64> = const { Cell::new(0) };
}

/// How many pipelines of shader effects this thread compiled: a frame of an
/// effect whose pipeline exists for the device compiles none.
#[allow(dead_code)]
pub(crate) fn shader_pipelines_created() -> u64 {
    SHADER_PIPELINES_CREATED.with(Cell::get)
}

/// The checked module of the shader of `effect`, checked on the first call
/// of this thread for its type; `None` when it failed (reported once, see
/// [`check_shader_module`]).
pub(crate) fn shader_module(effect: &ImmutableCustomEffect) -> Option<Rc<ShaderModule>> {
    let type_id = effect.value_type_id();
    if let Some(found) = SHADER_MODULES.with(|modules| modules.borrow().get(&type_id).cloned()) {
        return found;
    }
    let module = match check_shader_module(effect) {
        Ok(module) => Some(Rc::new(module)),
        Err(reason) => {
            report_shader_failure(effect.name(), &reason);
            None
        }
    };
    SHADER_MODULES.with(|modules| modules.borrow_mut().insert(type_id, module.clone()));
    module
}

fn with_shader_pipeline<R>(
    device: &Arc<VelloWgpuDevice>,
    type_id: TypeId,
    module: &ShaderModule,
    f: impl FnOnce(&mut ShaderPipeline) -> R,
) -> R {
    SHADER_PIPELINES.with(|pipelines| {
        let mut pipelines = pipelines.borrow_mut();
        pipelines.retain(|pipeline| pipeline.device.strong_count() > 0);
        let found = pipelines.iter().position(|pipeline| {
            pipeline.type_id == type_id && std::ptr::eq(pipeline.device.as_ptr(), Arc::as_ptr(device))
        });
        let index = match found {
            Some(index) => index,
            None => {
                SHADER_PIPELINES_CREATED.with(|count| count.set(count.get() + 1));
                let wgpu_device = device.device();
                let label = Some("FerroUI shader effect");
                let mut entries = vec![
                    uniform_entry(SHADER_WGSL_UNIFORMS_BINDING),
                    uniform_entry(SHADER_WGSL_PASS_BINDING),
                    texture_entry(SHADER_WGSL_CONTENT_BINDING),
                ];
                for child in 0..module.children {
                    entries.push(texture_entry(SHADER_WGSL_CHILDREN_BINDING + child as u32));
                }
                let (pipeline, layout) = create_pass_pipeline(wgpu_device, label, module.source.clone(), &entries);
                let buffer = |size: usize| {
                    wgpu_device.create_buffer(&wgpu::BufferDescriptor {
                        label,
                        size: size as u64,
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    })
                };
                let (uniforms, pass) = (buffer(module.size), buffer(16));
                pipelines.push(ShaderPipeline {
                    device: Arc::downgrade(device),
                    type_id,
                    pipeline,
                    layout,
                    uniforms,
                    pass,
                    bind_groups: Vec::new(),
                    block: Vec::with_capacity(module.size),
                });
                pipelines.len() - 1
            }
        };
        f(&mut pipelines[index])
    })
}

/// Where a shader effect is drawn: the area of the effect, in device pixels,
/// and the part of it the textures of its pass hold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShaderArea {
    /// The device pixel of the area at the top left texel of the textures
    /// (whole pixels).
    pub(crate) origin: [f32; 2],
    /// The width and the height of the area (`layer_size`).
    pub(crate) size: [f32; 2],
    /// Device pixels per device independent pixel (`layer_scale`).
    pub(crate) scale: f32,
}

/// The shader of `effect` over `content` (a texture of the pool holding the
/// part `area` of the area of the effect, `shader`) into a texture of the
/// pool of the same size: the program compiled once per device and type,
/// the uniforms of this frame written into its buffers, its images bound as
/// the textures of [`effect_image_view`] (uploaded once). `None` when its
/// module failed (reported once).
pub(crate) fn shader_effect_texture(
    device: &Arc<VelloWgpuDevice>,
    content: &Arc<VelloDeviceTexture>,
    area: LayerArea,
    effect: &ImmutableCustomEffect,
    shader: ShaderArea,
) -> Option<Arc<VelloDeviceTexture>> {
    let module = shader_module(effect)?;
    release_dropped_effect_images();
    // The textures of the images, in the order of the children; no
    // allocation.
    let mut children: [(u64, Option<wgpu::TextureView>); MAX_SHADER_CHILDREN] = std::array::from_fn(|_| (0, None));
    let mut child_count = 0;
    effect.for_each_shader_child(&mut |_, image| {
        if let Some(slot) = children.get_mut(child_count) {
            let (id, view) = effect_image_view(device, image);
            *slot = (id, Some(view));
        }
        child_count += 1;
    });
    if child_count != module.children {
        report_shader_failure(effect.name(), "its images do not match its module");
        return None;
    }
    let mut key: BindGroupKey = [0; 1 + MAX_SHADER_CHILDREN];
    key[0] = content.id();
    for (slot, (id, _)) in key[1..].iter_mut().zip(&children) {
        *slot = *id;
    }
    let mut pass_bytes = [0u8; 16];
    let pass_floats = [shader.origin[0], shader.origin[1], area.width as f32, area.height as f32];
    for (slot, value) in pass_bytes.chunks_exact_mut(4).zip(pass_floats) {
        slot.copy_from_slice(&value.to_ne_bytes());
    }
    let output = pooled_texture(device, content.width(), content.height());
    let drawn = with_shader_pipeline(device, effect.value_type_id(), &module, |pipeline| {
        let mut block = std::mem::take(&mut pipeline.block);
        block.clear();
        block.resize(module.size, 0);
        let (size, scale) = (shader.size, shader.scale);
        let complete = crate::effect_uniforms::fill_shader_uniforms(&mut block, &module.offsets, effect, size, scale);
        if complete {
            device.queue().write_buffer(&pipeline.uniforms, 0, &block);
        }
        pipeline.block = block;
        if !complete {
            report_shader_failure(effect.name(), "its uniforms do not match its module");
            return false;
        }
        device.queue().write_buffer(&pipeline.pass, 0, &pass_bytes);
        let index = match pipeline.bind_groups.iter().position(|(bound, _)| *bound == key) {
            Some(index) => index,
            None => {
                if pipeline.bind_groups.len() >= MAX_BIND_GROUPS {
                    pipeline.bind_groups.clear();
                }
                let mut entries = vec![
                    wgpu::BindGroupEntry { binding: SHADER_WGSL_UNIFORMS_BINDING, resource: pipeline.uniforms.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: SHADER_WGSL_PASS_BINDING, resource: pipeline.pass.as_entire_binding() },
                    wgpu::BindGroupEntry {
                        binding: SHADER_WGSL_CONTENT_BINDING,
                        resource: wgpu::BindingResource::TextureView(content.view()),
                    },
                ];
                for (child, (_, view)) in children.iter().take(module.children).enumerate() {
                    if let Some(view) = view {
                        entries.push(wgpu::BindGroupEntry {
                            binding: SHADER_WGSL_CHILDREN_BINDING + child as u32,
                            resource: wgpu::BindingResource::TextureView(view),
                        });
                    }
                }
                let bind_group = device.device().create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("FerroUI shader effect"),
                    layout: &pipeline.layout,
                    entries: &entries,
                });
                pipeline.bind_groups.push((key, bind_group));
                pipeline.bind_groups.len() - 1
            }
        };
        let mut encoder =
            device.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("FerroUI shader effect") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FerroUI shader effect"),
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
        true
    });
    drawn.then_some(output)
}

// --- the images of the shader effects (E6) ---------------------------------------------

/// The texture of an image of a shader effect on a device: the pixels of the
/// image uploaded once, kept while a handle of the image lives.
struct ImageTexture {
    device: Weak<VelloWgpuDevice>,
    /// Tells when the last handle of the image is dropped; the empty
    /// texture of a device has a handle that never lives.
    handle: EffectImageWeak,
    /// The identity of the image ([`EffectImage::id`]); 0 for the empty
    /// texture of a device.
    image: u64,
    /// The version of the bitmap the pixels were uploaded at.
    version: i32,
    /// The identity of this texture among the textures of the images, what
    /// the bind groups are kept by.
    id: u64,
    view: wgpu::TextureView,
    _texture: wgpu::Texture,
}

ferroui_base::ferro_thread_local! {
    // Kept beyond the end of the thread, as the objects of the lighting
    // pass are (see there).
    static IMAGE_TEXTURES: ManuallyDrop<RefCell<Vec<ImageTexture>>> = ManuallyDrop::new(RefCell::new(Vec::new()));
    // The generation of releases of effect images this thread last looked at
    // its textures for (`EffectImage::release_generation`).
    static IMAGE_RELEASES_SEEN: Cell<u64> = const { Cell::new(0) };
    static IMAGE_UPLOADS: Cell<u64> = const { Cell::new(0) };
}

/// How many images the passes of this thread uploaded into textures:
/// constant over frames that only change uniforms.
#[allow(dead_code)]
pub(crate) fn image_uploads() -> u64 {
    IMAGE_UPLOADS.with(Cell::get)
}

/// How many textures of images (the empty textures of the devices aside)
/// this thread keeps.
#[allow(dead_code)]
pub(crate) fn image_texture_count() -> usize {
    IMAGE_TEXTURES.with(|textures| textures.borrow().iter().filter(|texture| texture.image != 0).count())
}

/// The next identity of a texture of an image.
fn next_image_texture_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Releases the textures of the images whose last handle was dropped since
/// this thread last looked (on any thread; one atomic read when none was),
/// and the bind groups that bind them, and the textures of devices that are
/// gone. Called by the passes of the shader effects and by the drawing
/// contexts of this thread when they are created, so the texture of a
/// dropped image goes with the next frame.
pub(crate) fn release_dropped_effect_images() {
    let generation = EffectImage::release_generation();
    if IMAGE_RELEASES_SEEN.with(|seen| seen.replace(generation)) == generation {
        return;
    }
    let mut released = Vec::new();
    IMAGE_TEXTURES.with(|textures| {
        textures.borrow_mut().retain(|texture| {
            let device_lives = texture.device.strong_count() > 0;
            let keep = device_lives && (texture.image == 0 || texture.handle.is_alive());
            if !keep {
                released.push(texture.id);
            }
            keep
        });
    });
    if released.is_empty() {
        return;
    }
    // A bind group holds the views it binds: the bind groups of the
    // released textures go with them.
    SHADER_PIPELINES.with(|pipelines| {
        for pipeline in pipelines.borrow_mut().iter_mut() {
            pipeline.bind_groups.retain(|(key, _)| !key[1..].iter().any(|id| released.contains(id)));
        }
    });
}

/// The texture of `image` on `device` and its identity: the pixels of the
/// image ([`EffectImage::pixels`]) uploaded the first time the device draws
/// the image (or a new version of its bitmap), kept while a handle of the
/// image lives; a transparent texture of one pixel for the empty image, an
/// image whose pixels cannot be read or that is larger than the largest
/// texture of the device.
pub(crate) fn effect_image_view(device: &Arc<VelloWgpuDevice>, image: &EffectImage) -> (u64, wgpu::TextureView) {
    let uploaded = if image.is_empty() { None } else { image_texture(device, image) };
    uploaded.unwrap_or_else(|| empty_image_view(device))
}

fn image_texture(device: &Arc<VelloWgpuDevice>, image: &EffectImage) -> Option<(u64, wgpu::TextureView)> {
    let (image_id, version) = (image.id(), image.version());
    let on_device =
        |texture: &ImageTexture| texture.image == image_id && std::ptr::eq(texture.device.as_ptr(), Arc::as_ptr(device));
    let found = IMAGE_TEXTURES.with(|textures| {
        let mut textures = textures.borrow_mut();
        if let Some(texture) = textures.iter().find(|texture| on_device(*texture)) {
            if texture.version == version {
                return Some((texture.id, texture.view.clone()));
            }
        }
        // A new version of the bitmap: the old pixels go.
        textures.retain(|texture| !on_device(texture));
        None
    });
    if found.is_some() {
        return found;
    }
    let pixels = image.pixels()?;
    let largest = device.device().limits().max_texture_dimension_2d as usize;
    let (width, height) = (pixels.width(), pixels.height());
    if width == 0 || height == 0 || width > largest || height > largest {
        return None;
    }
    let texture = device.create_image_texture(width as u32, height as u32, pixels.rgba());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let id = next_image_texture_id();
    IMAGE_UPLOADS.with(|count| count.set(count.get() + 1));
    IMAGE_TEXTURES.with(|textures| {
        textures.borrow_mut().push(ImageTexture {
            device: Arc::downgrade(device),
            handle: image.downgrade(),
            image: image_id,
            version,
            id,
            view: view.clone(),
            _texture: texture,
        })
    });
    Some((id, view))
}

/// The transparent texture of one pixel of `device`, made once.
fn empty_image_view(device: &Arc<VelloWgpuDevice>) -> (u64, wgpu::TextureView) {
    let found = IMAGE_TEXTURES.with(|textures| {
        textures
            .borrow()
            .iter()
            .find(|texture| texture.image == 0 && std::ptr::eq(texture.device.as_ptr(), Arc::as_ptr(device)))
            .map(|texture| (texture.id, texture.view.clone()))
    });
    if let Some(found) = found {
        return found;
    }
    let texture = device.create_image_texture(1, 1, &[0; 4]);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let id = next_image_texture_id();
    IMAGE_TEXTURES.with(|textures| {
        textures.borrow_mut().push(ImageTexture {
            device: Arc::downgrade(device),
            handle: EffectImageWeak::default(),
            image: 0,
            version: 0,
            id,
            view: view.clone(),
            _texture: texture,
        })
    });
    (id, view)
}
