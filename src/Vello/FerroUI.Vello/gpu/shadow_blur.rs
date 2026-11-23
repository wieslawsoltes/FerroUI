//! The blur of a shape on the device for the compute mode (#1139), which has no filter layers:
//! the compute renderer draws the coverage of the shape into a texture of the pool, and two
//! separable Gaussian passes blur it and give it its colour. A large deviation is blurred at a
//! reduced scale, so a pass never reads more than 25 texels for a pixel, and the result is scaled
//! back with a bilinear filter, which a blur has no detail to lose to.

use super::effect_passes::{pooled_size, pooled_texture};
use super::{log_render_failure, VelloDeviceTexture, VelloGpuTexture, VelloWgpuDevice};
use crate::drawing_context_impl::box_shadows::ShadowKey;
use crate::scene::{IVelloSceneSink, VelloScenePaint};
#[cfg(feature = "gpu")]
use crate::scene::VelloGpuSceneSink;
#[cfg(feature = "hybrid")]
use crate::scene::VelloHybridSceneSink;
use crate::shadow_gaussian::{gaussian_weights, ShadowRaster, WEIGHTS};
#[cfg(feature = "gpu")]
use crate::shadow_gaussian::{reduction_for, REACH};
use crate::vello_options::VelloRenderingMode;
#[cfg(feature = "gpu")]
use kurbo::Affine;
use kurbo::BezPath;
use peniko::color::{AlphaColor, Srgb};
use peniko::{BlendMode, Fill};
use std::cell::RefCell;
use std::sync::{Arc, Weak};

const BLUR_WGSL: &str = r#"
struct Params {
    weights: array<vec4<f32>, 4>,
    // What the blurred coverage is multiplied by: a premultiplied colour.
    color: vec4<f32>,
    // Gaussian: x, y the direction, z how many texels it reads at each side, w 1 to keep the
    // colours of the content rather than give its alpha `color`. Reduction: x the texels of the
    // content per texel of the result. Scaling up: x the texels of the content per pixel.
    step: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var content: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn texel(p: vec2<i32>) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(content));
    if (any(p < vec2<i32>(0)) || any(p >= size)) {
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
    let direction = vec2<i32>(i32(params.step.x), i32(params.step.y));
    let radius = i32(params.step.z);
    var sum = weight(0) * texel(pixel);
    for (var i = 1; i <= radius; i = i + 1) {
        let offset = direction * i;
        sum = sum + weight(i) * (texel(pixel + offset) + texel(pixel - offset));
    }
    if (params.step.w > 0.5) {
        return sum;
    }
    return params.color * sum.a;
}

// The mean of the texels of the content a texel of the result covers.
@fragment
fn fs_reduce(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let side = i32(params.step.x);
    let first = vec2<i32>(floor(fragment.xy)) * side;
    var sum = vec4<f32>(0.0);
    for (var y = 0; y < side; y = y + 1) {
        for (var x = 0; x < side; x = x + 1) {
            sum = sum + texel(first + vec2<i32>(x, y));
        }
    }
    return sum / f32(side * side);
}

// The content at a reduced scale, read with a bilinear filter.
@fragment
fn fs_scale_up(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let at = (floor(fragment.xy) + 0.5) * params.step.x - 0.5;
    let corner = vec2<i32>(floor(at));
    let f = at - vec2<f32>(corner);
    let top = mix(texel(corner), texel(corner + vec2<i32>(1, 0)), f.x);
    let bottom = mix(texel(corner + vec2<i32>(0, 1)), texel(corner + vec2<i32>(1, 1)), f.x);
    return mix(top, bottom, f.y);
}
"#;

/// The fragment stages of the module.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PassKind {
    Gaussian,
    Reduce,
    ScaleUp,
}

impl PassKind {
    fn entry_point(self) -> &'static str {
        match self {
            PassKind::Gaussian => "fs_main",
            PassKind::Reduce => "fs_reduce",
            PassKind::ScaleUp => "fs_scale_up",
        }
    }
}

struct BlurPipeline {
    device: Weak<VelloWgpuDevice>,
    kind: PassKind,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
}

ferroui_base::ferro_thread_local! {
    static PIPELINES: RefCell<Vec<BlurPipeline>> = const { RefCell::new(Vec::new()) };
}

fn create_pipeline(device: &Arc<VelloWgpuDevice>, kind: PassKind) -> BlurPipeline {
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
            entry_point: Some(kind.entry_point()),
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
    BlurPipeline { device: Arc::downgrade(device), kind, pipeline, layout, buffer }
}

/// One Gaussian pass over `content` into a texture of the pool of its size: the alpha given
/// `color`, or the colours kept when `color` is `None`.
fn blur_pass(
    device: &Arc<VelloWgpuDevice>,
    content: &Arc<VelloDeviceTexture>,
    direction: (f32, f32),
    deviation: f64,
    color: Option<[f32; 4]>,
) -> Arc<VelloDeviceTexture> {
    let (weights, radius) = gaussian_weights(deviation);
    let mut floats = [0.0f32; 24];
    floats[..WEIGHTS].copy_from_slice(&weights);
    floats[16..20].copy_from_slice(&color.unwrap_or([1.0; 4]));
    let keep_colours = if color.is_none() { 1.0 } else { 0.0 };
    floats[20..24].copy_from_slice(&[direction.0, direction.1, radius as f32, keep_colours]);
    run_pass(device, PassKind::Gaussian, content, (content.width(), content.height()), &floats)
}

/// A pass of `kind` over `content` into a texture of the pool of `size`.
fn run_pass(
    device: &Arc<VelloWgpuDevice>,
    kind: PassKind,
    content: &Arc<VelloDeviceTexture>,
    size: (u32, u32),
    floats: &[f32; 24],
) -> Arc<VelloDeviceTexture> {
    let output = pooled_texture(device, size.0, size.1);
    let bytes: Vec<u8> = floats.iter().flat_map(|value| value.to_ne_bytes()).collect();
    PIPELINES.with(|pipelines| {
        let mut pipelines = pipelines.borrow_mut();
        pipelines.retain(|pipeline| pipeline.device.strong_count() > 0);
        let found = pipelines
            .iter()
            .position(|pipeline| pipeline.kind == kind && std::ptr::eq(pipeline.device.as_ptr(), Arc::as_ptr(device)));
        let index = found.unwrap_or_else(|| {
            pipelines.push(create_pipeline(device, kind));
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
#[cfg(feature = "gpu")]
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
#[cfg(feature = "gpu")]
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

    let across = blur_pass(device, &content, (1.0, 0.0), deviation_x / reduction, Some([1.0; 4]));
    let [red, green, blue, alpha] = color.components;
    let premultiplied = [red * alpha, green * alpha, blue * alpha, alpha];
    let texture = blur_pass(device, &across, (0.0, 1.0), deviation_y / reduction, Some(premultiplied));
    Some(BlurredShape {
        texture,
        to_area: Affine::scale(reduction) * Affine::translate((-margin, -margin)),
        reduced: reduction > 1.0,
    })
}

/// `content` (premultiplied colours in a texture of the pool) blurred by a Gaussian of these
/// deviations in texels, for the blur and drop shadow effects of the compute mode, which has no
/// filter layers (#1146). With `tint`, a premultiplied colour, the result is that colour through
/// the blurred alpha, the shadow of the content. A large deviation is blurred at a reduced scale;
/// the box of the reduction and the bilinear filter that scales back blur too, by a variance of
/// a quarter of a reduced texel, which the Gaussian leaves out.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn blurred_content(
    device: &Arc<VelloWgpuDevice>,
    content: &Arc<VelloDeviceTexture>,
    (deviation_x, deviation_y): (f64, f64),
    tint: Option<[f32; 4]>,
) -> Arc<VelloDeviceTexture> {
    let reduction = crate::shadow_gaussian::reduction_for(deviation_x.max(deviation_y));
    if reduction <= 1.0 {
        let across = blur_pass(device, content, (1.0, 0.0), deviation_x, None);
        return blur_pass(device, &across, (0.0, 1.0), deviation_y, tint);
    }
    let reduced_deviation = |deviation: f64| ((deviation / reduction).powi(2) - 0.25).max(0.0).sqrt();
    let side = |length: u32| (f64::from(length) / reduction).ceil() as u32;
    let size = pooled_size(device, side(content.width()), side(content.height()));
    let mut floats = [0.0f32; 24];
    floats[20] = reduction as f32;
    let reduced = run_pass(device, PassKind::Reduce, content, size, &floats);
    let across = blur_pass(device, &reduced, (1.0, 0.0), reduced_deviation(deviation_x), None);
    let blurred = blur_pass(device, &across, (0.0, 1.0), reduced_deviation(deviation_y), tint);
    floats[20] = (1.0 / reduction) as f32;
    run_pass(device, PassKind::ScaleUp, &blurred, (content.width(), content.height()), &floats)
}

/// The most pictures a thread keeps, and the most bytes of their textures.
const MOST_PICTURES: usize = 64;
const MOST_BYTES: usize = 32 * 1024 * 1024;

struct KeptPicture {
    device: Weak<VelloWgpuDevice>,
    key: ShadowKey,
    texture: Arc<VelloDeviceTexture>,
    used: u64,
    bytes: usize,
}

#[derive(Default)]
struct KeptPictures {
    kept: Vec<KeptPicture>,
    uses: u64,
    bytes: usize,
}

impl KeptPictures {
    /// Lets go of the pictures of devices that are gone or lost.
    fn trim(&mut self) {
        let mut bytes = 0;
        self.kept.retain(|picture| {
            let alive = picture.device.upgrade().is_some_and(|device| !device.is_lost());
            bytes += if alive { picture.bytes } else { 0 };
            alive
        });
        self.bytes = bytes;
    }

    fn get(&mut self, device: &Arc<VelloWgpuDevice>, key: &ShadowKey) -> Option<Arc<VelloDeviceTexture>> {
        self.trim();
        self.uses += 1;
        let uses = self.uses;
        let found = self
            .kept
            .iter_mut()
            .find(|picture| std::ptr::eq(picture.device.as_ptr(), Arc::as_ptr(device)) && picture.key == *key)?;
        found.used = uses;
        Some(found.texture.clone())
    }

    fn insert(&mut self, device: &Arc<VelloWgpuDevice>, key: ShadowKey, texture: Arc<VelloDeviceTexture>) {
        let bytes = texture.width() as usize * texture.height() as usize * 4;
        if bytes > MOST_BYTES {
            return;
        }
        while self.kept.len() >= MOST_PICTURES || self.bytes + bytes > MOST_BYTES {
            let Some(oldest) = (0..self.kept.len()).min_by_key(|&index| self.kept[index].used) else {
                break;
            };
            self.bytes -= self.kept.swap_remove(oldest).bytes;
        }
        self.uses += 1;
        self.bytes += bytes;
        self.kept.push(KeptPicture { device: Arc::downgrade(device), key, texture, used: self.uses, bytes });
    }
}

ferroui_base::ferro_thread_local! {
    static KEPT_PICTURES: RefCell<Option<KeptPictures>> = const { RefCell::new(None) };
}

#[cfg(test)]
ferroui_base::ferro_thread_local! {
    static PICTURES_MADE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Lets go of the pictures this thread keeps, so that a test sees a mode make its own.
#[cfg(test)]
pub(crate) fn forget_pictures() {
    KEPT_PICTURES.with(|pictures| pictures.borrow_mut().take());
}

/// The pictures of shadows this thread has blurred so far.
#[cfg(test)]
pub(crate) fn pictures_made() -> usize {
    PICTURES_MADE.with(std::cell::Cell::get)
}

/// A scene of the renderer of `mode` on the device, for the coverage of a shape.
fn coverage_scene(
    device: &Arc<VelloWgpuDevice>,
    mode: VelloRenderingMode,
    width: u16,
    height: u16,
) -> Option<Box<dyn IVelloSceneSink>> {
    let _ = (device, width, height);
    match mode {
        #[cfg(feature = "gpu")]
        VelloRenderingMode::Gpu => Some(Box::new(VelloGpuSceneSink::new(device.clone(), width, height))),
        #[cfg(feature = "hybrid")]
        VelloRenderingMode::Hybrid => Some(Box::new(VelloHybridSceneSink::new(device.clone(), width, height))),
        _ => None,
    }
}

/// The picture of a blurred shape for `raster`, the kept one when there is one, so a shadow under
/// an animated transform is blurred once. `made` is called when a new one is blurred. `None` when
/// the device failed to render it, which is logged.
#[allow(clippy::too_many_arguments)]
pub(crate) fn kept_picture(
    device: &Arc<VelloWgpuDevice>,
    mode: VelloRenderingMode,
    path: &BezPath,
    fill_rule: Fill,
    raster: &ShadowRaster,
    key: ShadowKey,
    color: AlphaColor<Srgb>,
    made: impl FnOnce(),
) -> Option<Arc<VelloDeviceTexture>> {
    let kept = KEPT_PICTURES.with(|pictures| pictures.borrow_mut().get_or_insert_with(Default::default).get(device, &key));
    if kept.is_some() {
        return kept;
    }
    made();
    #[cfg(test)]
    PICTURES_MADE.with(|count| count.set(count.get() + 1));
    let (width, height) = pooled_size(device, raster.size.0, raster.size.1);
    let mut scene = coverage_scene(device, mode, width.min(u32::from(u16::MAX)) as u16, height.min(u32::from(u16::MAX)) as u16)?;
    let opaque = VelloScenePaint::solid(AlphaColor::from_rgba8(255, 255, 255, 255));
    scene.fill(path, fill_rule, raster.to_texels, &opaque, BlendMode::default(), true);
    let content = pooled_texture(device, width, height);
    if let Err(error) = scene.render_to_texture(&VelloGpuTexture { texture: content.texture() }) {
        log_render_failure("shadow", &error);
        return None;
    }
    let across = blur_pass(device, &content, (1.0, 0.0), raster.deviation, Some([1.0; 4]));
    let [red, green, blue, alpha] = color.components;
    let premultiplied = [red * alpha, green * alpha, blue * alpha, alpha];
    let texture = blur_pass(device, &across, (0.0, 1.0), raster.deviation, Some(premultiplied));
    KEPT_PICTURES.with(|pictures| {
        pictures.borrow_mut().get_or_insert_with(Default::default).insert(device, key, texture.clone())
    });
    Some(texture)
}
