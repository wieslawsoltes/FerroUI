//! The blur of a shape on the device for the compute mode (#1139), which has no filter layers:
//! the compute renderer draws the coverage of the shape into a texture of the pool, and two
//! separable Gaussian passes blur it and give it its colour. A large deviation is blurred at a
//! reduced scale, so a pass never reads more than 25 texels for a pixel, and the result is scaled
//! back with a bilinear filter, which a blur has no detail to lose to.

use super::effect_passes::{pooled_size, pooled_texture};
use super::{log_render_failure, VelloDeviceTexture, VelloGpuTexture, VelloWgpuDevice};
use crate::scene::{IVelloSceneSink, VelloGpuSceneSink, VelloScenePaint};
use crate::shadow_gaussian::{gaussian_weights, reduction_for, REACH, WEIGHTS};
use kurbo::{Affine, BezPath};
use peniko::color::{AlphaColor, Srgb};
use peniko::{BlendMode, Fill};
use std::cell::RefCell;
use std::sync::{Arc, Weak};

const BLUR_WGSL: &str = r#"
struct Params {
    weights: array<vec4<f32>, 4>,
    // What the blurred coverage is multiplied by: a premultiplied colour.
    color: vec4<f32>,
    // x, y: the direction of the pass; z: how many texels it reads at each side.
    step: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var content: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn coverage(p: vec2<i32>) -> f32 {
    let size = vec2<i32>(textureDimensions(content));
    if (any(p < vec2<i32>(0)) || any(p >= size)) {
        return 0.0;
    }
    return textureLoad(content, p, 0).a;
}

fn weight(index: i32) -> f32 {
    let quad = params.weights[index / 4];
    return quad[index % 4];
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    let direction = vec2<i32>(i32(params.step.x), i32(params.step.y));
    let radius = i32(params.step.z);
    var sum = weight(0) * coverage(pixel);
    for (var i = 1; i <= radius; i = i + 1) {
        let offset = direction * i;
        sum = sum + weight(i) * (coverage(pixel + offset) + coverage(pixel - offset));
    }
    return params.color * sum;
}
"#;

struct BlurPipeline {
    device: Weak<VelloWgpuDevice>,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
}

ferroui_base::ferro_thread_local! {
    static PIPELINES: RefCell<Vec<BlurPipeline>> = const { RefCell::new(Vec::new()) };
}

fn create_pipeline(device: &Arc<VelloWgpuDevice>) -> BlurPipeline {
    let wgpu_device = device.device();
    let label = Some("shadow blur");
    let module = wgpu_device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label,
        source: wgpu::ShaderSource::Wgsl(BLUR_WGSL.into()),
    });
    let layout = wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    });
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
        size: (24 * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    BlurPipeline { device: Arc::downgrade(device), pipeline, layout, buffer }
}

/// One pass over `content` into a texture of the pool of its size.
fn blur_pass(
    device: &Arc<VelloWgpuDevice>,
    content: &Arc<VelloDeviceTexture>,
    direction: (f32, f32),
    deviation: f64,
    color: [f32; 4],
) -> Arc<VelloDeviceTexture> {
    let output = pooled_texture(device, content.width(), content.height());
    let (weights, radius) = gaussian_weights(deviation);
    let mut floats = [0.0f32; 24];
    floats[..WEIGHTS].copy_from_slice(&weights);
    floats[16..20].copy_from_slice(&color);
    floats[20..24].copy_from_slice(&[direction.0, direction.1, radius as f32, 0.0]);
    let bytes: Vec<u8> = floats.iter().flat_map(|value| value.to_ne_bytes()).collect();
    PIPELINES.with(|pipelines| {
        let mut pipelines = pipelines.borrow_mut();
        pipelines.retain(|pipeline| pipeline.device.strong_count() > 0);
        let found = pipelines.iter().position(|pipeline| std::ptr::eq(pipeline.device.as_ptr(), Arc::as_ptr(device)));
        let index = found.unwrap_or_else(|| {
            pipelines.push(create_pipeline(device));
            pipelines.len() - 1
        });
        let pipeline = &pipelines[index];
        let bind_group = device.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow blur"),
            layout: &pipeline.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: pipeline.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(content.view()) },
            ],
        });
        device.queue().write_buffer(&pipeline.buffer, 0, &bytes);
        let mut encoder =
            device.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("shadow blur") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow blur"),
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
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        device.queue().submit(Some(encoder.finish()));
    });
    output
}

/// A blurred shape in a texture of the device.
pub(crate) struct BlurredShape {
    pub(crate) texture: Arc<VelloDeviceTexture>,
    /// From the texels of the texture to the pixels of the area.
    pub(crate) to_area: Affine,
    /// Whether the texture is at a reduced scale, and so is sampled with a filter.
    pub(crate) reduced: bool,
}

/// The fill of `path` blurred by a Gaussian of `std_deviation` (in the units of the path) in
/// `color`, for an area of `width` by `height` pixels that `to_area` maps the path to. The texture
/// also holds what lies a reach around the area, which blurs into it. `None` when the device
/// failed to render it, which is logged.
pub(crate) fn blurred_shape(
    device: &Arc<VelloWgpuDevice>,
    path: &BezPath,
    fill_rule: Fill,
    to_area: Affine,
    std_deviation: f64,
    color: AlphaColor<Srgb>,
    width: u16,
    height: u16,
) -> Option<BlurredShape> {
    let [a, b, c, d, _, _] = to_area.as_coeffs();
    let (deviation_x, deviation_y) = (std_deviation * a.hypot(b), std_deviation * c.hypot(d));
    let reduction = reduction_for(deviation_x.max(deviation_y));
    let margin = (deviation_x.max(deviation_y) / reduction * REACH).ceil();
    let texels = |pixels: u16| (f64::from(pixels) / reduction).ceil() as u32 + 2 * margin as u32 + 1;
    let (texture_width, texture_height) = pooled_size(device, texels(width), texels(height));
    let limit = u32::from(u16::MAX);
    let (texture_width, texture_height) = (texture_width.min(limit), texture_height.min(limit));

    let to_texels = Affine::translate((margin, margin)) * Affine::scale(1.0 / reduction) * to_area;
    let mut scene = VelloGpuSceneSink::new(device.clone(), texture_width as u16, texture_height as u16);
    let opaque = VelloScenePaint::solid(AlphaColor::from_rgba8(255, 255, 255, 255));
    scene.fill(path, fill_rule, to_texels, &opaque, BlendMode::default(), true);
    let content = pooled_texture(device, texture_width, texture_height);
    if let Err(error) = scene.render_to_texture(&VelloGpuTexture { texture: content.texture() }) {
        log_render_failure("compute", &error);
        return None;
    }

    let across = blur_pass(device, &content, (1.0, 0.0), deviation_x / reduction, [1.0; 4]);
    let [red, green, blue, alpha] = color.components;
    let premultiplied = [red * alpha, green * alpha, blue * alpha, alpha];
    let texture = blur_pass(device, &across, (0.0, 1.0), deviation_y / reduction, premultiplied);
    Some(BlurredShape {
        texture,
        to_area: Affine::scale(reduction) * Affine::translate((-margin, -margin)),
        reduced: reduction > 1.0,
    })
}
