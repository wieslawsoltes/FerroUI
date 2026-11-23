//! The wgpu executor of 3D scenes (`docs/porting/media3d.md`, section 3.15, step D4): the
//! renderer of the GPU modes of the Vello backend (hybrid and GPU), and later of the browser's
//! WebGPU and of the ProGPU backend, which draw with wgpu as well.
//!
//! It draws a [`Scene3DSnapshot`] with [`SCENE3D_WGSL`] into a texture of the caller's
//! ([`Scene3DTarget`]), stage for stage as the CPU reference (`cpu/mod.rs`) does:
//!
//! - **Draws.** One draw per material layer of each model, the front layers then the back layers
//!   of a model, in tree order. A draw of the front layers culls the back faces and the other way
//!   round; the first diffuse layer of a side writes depth with `Less`, the others test
//!   `LessEqual` without writing; a diffuse layer is blended premultiplied source-over, a specular
//!   or emissive layer is added.
//! - **Pipelines** are made on first demand, by a [`Scene3DPipelineKey`], once per executor (one
//!   executor per device), never per frame, and counted ([`Scene3DGpuCounters::pipelines_created`]).
//! - **Meshes** are uploaded once per `(id, version)` as an interleaved vertex buffer and a `u32`
//!   index buffer, kept while they are drawn; a new version of a mesh reuses the buffers of the
//!   version it replaces when they are large enough.
//! - **Brushes** (step D6, [`crate::brush`]): the variant of the brush shader of a draw is a bit of
//!   its pipeline key (one fragment entry point each); the uniforms of its brush are written beside
//!   its draw uniforms; the texture of a brush (a gradient's lookup texture, the pixels of an
//!   image) is uploaded once per [`BrushTextureKey`] and device, a new version of a bitmap into the
//!   texture of the version it replaces when it has the same size, and sampled through one
//!   linear, clamped sampler ([`texture_counters`]).
//! - **Uniforms.** A [`Scene3DViewport`] (one per kept output of the caller) holds the uniform
//!   buffers of a viewport: the [`FrameUniforms`] in one, the [`DrawUniforms`] of its draws in
//!   another and the [`BrushUniforms`] of their brushes in a third, at a stride of the device's
//!   `min_uniform_buffer_offset_alignment`, bound with dynamic offsets. A render writes only the
//!   bytes that changed: a camera that moves writes the frame uniforms alone, a brush whose values
//!   change the uniforms of its draws alone.
//! - **Targets.** The multisampled colour, the depth (`Depth32Float`, the CPU reference's `f32`
//!   depth) and the resolved colour of decision N are pooled by the size of the output texture
//!   (the caller rounds it, as the textures of Vello's pool are) and the sample count; depth is
//!   cleared once per render. 4x MSAA where the formats support it, 1x for aliased edges, resolved
//!   on the GPU.
//! - **Resolve.** A scene with specular or emissive layers is resolved into a texture of the pool,
//!   and `fs_resolve` writes it to the output with the alpha of each pixel raised to its largest
//!   colour channel (decision N); a scene of diffuse layers alone is resolved into the output
//!   directly (its colours never exceed their alpha).
//!
//! Errors are caught by error scopes: an executor that cannot be made, or a pipeline that cannot
//! be made with one sample, is an [`Scene3DWgpuError`] the caller reports once before it draws
//! the CPU reference; a pipeline that cannot be made with four samples is reported once, and the
//! executor draws with one from then on.

use crate::brush::DeviceBitmap;
use crate::brush::{
    texture_coordinate_bounds, BrushShader, BrushTextureIdentity, BrushTextureKey, BrushTextureSource, BrushUniforms,
    MaterialBrush,
};
use crate::shader::{
    fragment_entry, samples_texture, write_mesh, BRUSH_BINDING, RESOLVED_BINDING, RESOLVE_FRAGMENT_ENTRY,
    RESOLVE_VERTEX_ENTRY, SCENE3D_WGSL, VERTEX_ATTRIBUTE_OFFSETS, VERTEX_ENTRY, VERTEX_STRIDE,
};
use crate::uniforms::{DrawUniforms, Face, FrameUniforms, LayerKind, Std140};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::SharedBitmapImpl;
use ferroui_base::rendering::scene3d::{MeshData, Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot};
use ferroui_base::Rect;
use std::cell::Cell;
use std::collections::HashMap;
use std::future::Future;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, Ordering};

/// The format of the colour the executor draws: premultiplied RGBA8, the format of the textures
/// of Vello's pool.
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The format of the depth of a render: 32-bit float, the depth of the CPU reference (`z / w`).
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The samples of an antialiased render (decision F).
pub const MULTISAMPLE_COUNT: u32 = 4;

/// The renders a mesh or a target of the pool is kept for without being drawn.
const LIFETIME: u64 = 240;

/// How a draw is blended into the colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scene3DBlend {
    /// A diffuse layer: premultiplied source-over (`One, OneMinusSrcAlpha`).
    SourceOver,
    /// A specular or emissive layer: added (`One, One`), with alpha 0.
    Additive,
}

/// What a pipeline of a draw is made for: a `Copy` key, one pipeline per key, brush shader and
/// device.
///
/// The executor keys its pipelines by this key and the variant of the brush shader of the draw
/// ([`BrushShader`], step D6: the fragment entry point, and whether the pipeline layout has the
/// group of the brush texture); the frame-sampling variant of section 3.5 joins them with the
/// video materials of step D7. Every draw has the one vertex layout of [`crate::shader`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Scene3DPipelineKey {
    /// The blend of the layer kind.
    pub blend: Scene3DBlend,
    /// The faces drawn: the other side is culled.
    pub face: Face,
    /// Whether the draw writes depth (with `Less`) or only tests it (`LessEqual`).
    pub depth_write: bool,
    /// 1, or [`MULTISAMPLE_COUNT`].
    pub sample_count: u32,
    /// The format of the colour attachment.
    pub color_format: wgpu::TextureFormat,
    /// The format of the depth attachment.
    pub depth_format: wgpu::TextureFormat,
}

/// What the executors of this thread did since it started: what the steady-state tests count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scene3DGpuCounters {
    /// Render pipelines made (the draws' and the resolve's).
    pub pipelines_created: u64,
    /// Meshes uploaded (a new `(id, version)`).
    pub mesh_uploads: u64,
    /// The bytes of the vertices and indices uploaded.
    pub mesh_bytes: u64,
    /// Writes of uniform buffers.
    pub uniform_writes: u64,
    /// The bytes of the uniforms written.
    pub uniform_bytes: u64,
    /// Renders of a scene (command buffers with a 3D pass) submitted.
    pub passes: u64,
    /// Textures of the pool made (multisampled colour, depth, resolved colour).
    pub textures_created: u64,
    /// Buffers made (vertex, index and uniform buffers).
    pub buffers_created: u64,
}

/// The key of a pipeline of a draw: [`Scene3DPipelineKey`] and the variant of its brush shader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct DrawPipelineKey {
    layer: Scene3DPipelineKey,
    brush: BrushShader,
}

/// What the executors of this thread did with the textures of brushes since it started (step
/// D6): what the tests of the uploads count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scene3DTextureCounters {
    /// Textures of brushes written: a new key (a gradient's stops, a bitmap's version).
    pub textures_uploaded: u64,
    /// The bytes of those texels.
    pub texture_bytes: u64,
    /// Textures of brushes that could not be uploaded (no readable pixels, larger than the device
    /// takes): their layers are drawn with their colours and counted
    /// ([`crate::layers_with_unsampled_brushes`]).
    pub textures_refused: u64,
    /// Textures of brushes the renderer already had on the device (a frame of a video, step D7):
    /// sampled as they are, with nothing written.
    pub textures_on_device: u64,
}

/// A texture of the device that a renderer gives for a bitmap (step D7): `Rgba8Unorm`,
/// premultiplied, one sample, bindable. The executor samples it and never writes it.
pub struct Scene3DDeviceTexture {
    /// The texture.
    pub texture: wgpu::Texture,
    /// Its view.
    pub view: wgpu::TextureView,
    /// What keeps the texture from being used for something else while the executor keeps it.
    pub owner: std::sync::Arc<dyn std::any::Any + Send + Sync>,
}

/// The bitmaps a renderer has as textures of the device the executor draws on, whose pixels
/// cannot be read: the frames of a video or a camera (step D7).
pub trait Scene3DDeviceTextures {
    /// Whether `bitmap` is a texture of this device. Asked for every layer that has such a
    /// bitmap, so it does no work.
    fn is_on_device(&self, bitmap: &SharedBitmapImpl) -> bool;

    /// The texture of `bitmap`, asked once for each bitmap the executor does not hold. `None`
    /// when it is gone: the layer is drawn with its colour.
    fn device_texture(&self, bitmap: &SharedBitmapImpl) -> Option<Scene3DDeviceTexture>;
}

/// A renderer without such bitmaps.
struct NoDeviceTextures;

impl Scene3DDeviceTextures for NoDeviceTextures {
    fn is_on_device(&self, _: &SharedBitmapImpl) -> bool {
        false
    }

    fn device_texture(&self, _: &SharedBitmapImpl) -> Option<Scene3DDeviceTexture> {
        None
    }
}

impl Scene3DTextureCounters {
    const ZERO: Scene3DTextureCounters =
        Scene3DTextureCounters { textures_uploaded: 0, texture_bytes: 0, textures_refused: 0, textures_on_device: 0 };
}

impl Scene3DGpuCounters {
    const ZERO: Scene3DGpuCounters = Scene3DGpuCounters {
        pipelines_created: 0,
        mesh_uploads: 0,
        mesh_bytes: 0,
        uniform_writes: 0,
        uniform_bytes: 0,
        passes: 0,
        textures_created: 0,
        buffers_created: 0,
    };
}

ferroui_base::ferro_thread_local! {
    /// What the executors of this thread did.
    static COUNTERS: Cell<Scene3DGpuCounters> = const { Cell::new(Scene3DGpuCounters::ZERO) };
    /// What they did with the textures of brushes.
    static TEXTURE_COUNTERS: Cell<Scene3DTextureCounters> = const { Cell::new(Scene3DTextureCounters::ZERO) };
}

/// What the executors of this thread did with the textures of brushes since it started.
pub fn texture_counters() -> Scene3DTextureCounters {
    TEXTURE_COUNTERS.with(Cell::get)
}

fn count_textures(change: impl FnOnce(&mut Scene3DTextureCounters)) {
    TEXTURE_COUNTERS.with(|counters| {
        let mut value = counters.get();
        change(&mut value);
        counters.set(value);
    });
}

/// What the executors of this thread did since it started.
pub fn gpu_counters() -> Scene3DGpuCounters {
    COUNTERS.with(Cell::get)
}

fn count(change: impl FnOnce(&mut Scene3DGpuCounters)) {
    COUNTERS.with(|counters| {
        let mut value = counters.get();
        change(&mut value);
        counters.set(value);
    });
}

/// Why the executor cannot draw a scene: the caller draws the CPU reference instead, and reports
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene3DWgpuError {
    /// What failed, in the words of wgpu.
    pub reason: String,
}

impl std::fmt::Display for Scene3DWgpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The 3D scene cannot be drawn with wgpu: {}", self.reason)
    }
}

impl std::error::Error for Scene3DWgpuError {}

/// The texture a render draws into.
pub struct Scene3DTarget<'a> {
    /// The output: a view of a texture of [`COLOR_FORMAT`] with one sample that can be rendered
    /// to. The scene is drawn at its top left; the rest of it is cleared to transparent.
    pub view: &'a wgpu::TextureView,
    /// The size of the texture of `view`, in pixels.
    pub texture_size: (u32, u32),
    /// The size of the image of the scene, in pixels: at most `texture_size`.
    pub size: (u32, u32),
    /// Whether edges are antialiased (4x MSAA where supported); `false` takes one sample per
    /// pixel (`RenderOptions.EdgeMode` `Aliased`).
    pub antialiased: bool,
}

/// What one render did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scene3DRenderReport {
    /// The samples per pixel it drew with.
    pub sample_count: u32,
    /// The draws it issued.
    pub draws: u32,
    /// The meshes it uploaded.
    pub meshes_uploaded: u32,
    /// The pipelines it made.
    pub pipelines_created: u32,
    /// The textures and buffers it made.
    pub resources_created: u32,
    /// The bytes of uniforms it wrote.
    pub uniform_bytes: u64,
}

impl Scene3DRenderReport {
    /// Whether the render wrote uniforms and nothing else: no mesh uploaded, no pipeline, texture
    /// or buffer made (the steady state of a moving camera).
    pub fn is_uniform_only(&self) -> bool {
        self.meshes_uploaded == 0 && self.pipelines_created == 0 && self.resources_created == 0
    }
}

/// A draw of a render: one layer of one model on one side of its faces.
#[derive(Clone, Copy, Debug)]
struct DrawRecord {
    mesh: (u64, u64),
    index_count: u32,
    blend: Scene3DBlend,
    face: Face,
    depth_write: bool,
    /// The variant of the brush shader.
    brush: BrushShader,
    /// The texture of the brush (`None` for a brush that samples none, or an image brush without
    /// an image, which binds the empty texture).
    texture: Option<BrushTextureKey>,
}

/// The uniform buffers of a viewport, with the bytes they hold, and the draws of its last render.
/// The caller keeps one with each output it keeps (`docs/porting/media3d.md`, section 3.15).
#[derive(Default)]
pub struct Scene3DViewport {
    frame_buffer: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    /// The draw uniforms, with the bind group of the draws (the draw and the brush uniforms).
    draw_buffer: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    /// The brush uniforms of the draws (step D6), bound with `draw_buffer`.
    brush_buffer: Option<wgpu::Buffer>,
    /// The draws `draw_buffer` and `brush_buffer` hold room for.
    draw_capacity: usize,
    /// The bytes in the buffers, as last written.
    frame_bytes: Vec<u8>,
    draw_bytes: Vec<u8>,
    brush_bytes: Vec<u8>,
    /// The bytes of the render being prepared.
    frame_scratch: Vec<u8>,
    draw_scratch: Vec<u8>,
    brush_scratch: Vec<u8>,
    draws: Vec<DrawRecord>,
}

impl Scene3DViewport {
    /// A viewport without buffers yet: they are made by its first render.
    pub fn new() -> Scene3DViewport {
        Scene3DViewport::default()
    }
}

/// The vertex and index buffers of a mesh, with the bytes they hold room for.
struct MeshGpuBuffers {
    vertices: wgpu::Buffer,
    vertex_capacity: u64,
    /// The bytes of the vertices of this version.
    vertex_bytes: u64,
    indices: wgpu::Buffer,
    index_capacity: u64,
}

/// A mesh on the device.
struct MeshBuffers {
    /// `None` for a mesh without triangles.
    buffers: Option<MeshGpuBuffers>,
    index_count: u32,
    /// The box of its texture coordinates, which relative brushes are mapped onto.
    texture_bounds: Rect,
    last_used: u64,
}

/// The texture of a brush on the device, with its bind group (the texture and the sampler).
struct BrushTexture {
    /// Keeps the address of a bitmap from naming another bitmap while the texture is kept.
    identity: BrushTextureIdentity,
    texture: wgpu::Texture,
    size: (u32, u32),
    group: wgpu::BindGroup,
    last_used: u64,
    /// The owner of a texture the renderer gave (step D7), which is never written.
    owner: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
}

/// The textures of the pool for one size and sample count.
struct Targets {
    size: (u32, u32),
    sample_count: u32,
    /// The multisampled colour (four samples only).
    multisampled: Option<wgpu::TextureView>,
    depth: wgpu::TextureView,
    /// The resolved colour `fs_resolve` reads, with its bind group; made on first need.
    resolved: Option<(wgpu::TextureView, wgpu::BindGroup)>,
    last_used: u64,
}

/// The wgpu executor of one device: its shader, layouts, pipelines, meshes and targets.
pub struct Scene3DWgpuExecutor {
    module: wgpu::ShaderModule,
    frame_layout: wgpu::BindGroupLayout,
    draw_layout: wgpu::BindGroupLayout,
    /// The texture of a brush and its sampler (group 2 of the textured pipelines).
    texture_layout: wgpu::BindGroupLayout,
    resolve_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    /// The layout of the pipelines whose brush samples a texture.
    textured_pipeline_layout: wgpu::PipelineLayout,
    resolve_pipeline_layout: wgpu::PipelineLayout,
    /// The one sampler of the brush textures: linear, the edges clamped, no mipmaps.
    sampler: wgpu::Sampler,
    pipelines: HashMap<DrawPipelineKey, wgpu::RenderPipeline>,
    resolve_pipeline: Option<wgpu::RenderPipeline>,
    /// Whether 4x MSAA is drawn: the formats support it, and no pipeline failed with it.
    multisample: bool,
    /// The bytes between the draw uniforms of two draws.
    draw_stride: usize,
    /// The bytes between the brush uniforms of two draws.
    brush_stride: usize,
    meshes: HashMap<(u64, u64), MeshBuffers>,
    /// The textures of brushes, by key.
    textures: HashMap<BrushTextureKey, BrushTexture>,
    /// The keys whose texels could not be read or uploaded, with the render that last asked:
    /// not read again every frame.
    refused_textures: HashMap<BrushTextureKey, u64>,
    /// A transparent texture of one texel: what an image brush without an image samples.
    empty_texture: Option<BrushTexture>,
    targets: Vec<Targets>,
    /// The renders so far: the clock of the pools.
    renders: u64,
    vertex_scratch: Vec<u8>,
    index_scratch: Vec<u32>,
    index_bytes: Vec<u8>,
}

/// The layout of the interleaved vertices.
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: VERTEX_ATTRIBUTE_OFFSETS[0] as u64,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: VERTEX_ATTRIBUTE_OFFSETS[1] as u64,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: VERTEX_ATTRIBUTE_OFFSETS[2] as u64,
        shader_location: 2,
    },
];

/// The stride of the draw uniforms in a buffer bound with dynamic offsets on a device whose
/// offsets are aligned to `alignment` bytes: the size of [`DrawUniforms`] rounded up to it.
pub fn draw_uniforms_stride(alignment: u32) -> usize {
    let alignment = (alignment as usize).max(1);
    std::mem::size_of::<DrawUniforms>().div_ceil(alignment) * alignment
}

/// The stride of the brush uniforms in a buffer bound with dynamic offsets on a device whose
/// offsets are aligned to `alignment` bytes: the size of [`BrushUniforms`] rounded up to it.
pub fn brush_uniforms_stride(alignment: u32) -> usize {
    let alignment = (alignment as usize).max(1);
    std::mem::size_of::<BrushUniforms>().div_ceil(alignment) * alignment
}

/// Whether `format` takes four samples and resolves them by the guarantees of WebGPU (what every
/// adapter of wgpu does without asking for adapter-specific format features).
fn multisamples(format: wgpu::TextureFormat, features: wgpu::Features, resolve: bool) -> bool {
    let flags = format.guaranteed_format_features(features).flags;
    flags.sample_count_supported(MULTISAMPLE_COUNT)
        && (!resolve || flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE))
}

/// The output of a future of wgpu that is complete when it is first asked (an error scope on a
/// native device), or `None` when it is not.
fn ready<F: Future>(future: F) -> Option<F::Output> {
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        std::task::Poll::Ready(output) => Some(output),
        std::task::Poll::Pending => None,
    }
}

/// Runs `f` within error scopes of validation and of memory, and returns the first error they
/// caught. On a device whose scopes resolve later (the browser), the errors reach the device's
/// own handler instead.
fn scoped<R>(device: &wgpu::Device, f: impl FnOnce() -> R) -> (R, Option<wgpu::Error>) {
    let memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let result = f();
    let validation_error = ready(validation.pop()).flatten();
    let memory_error = ready(memory.pop()).flatten();
    (result, validation_error.or(memory_error))
}

static MULTISAMPLE_REPORTED: AtomicBool = AtomicBool::new(false);

/// Reports once that a device draws 3D scenes with one sample per pixel although edges are to be
/// antialiased.
fn report_single_sample(reason: &str) {
    if MULTISAMPLE_REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
        logger.log_with_values(
            None,
            "3D scenes (Viewport3D) are drawn without 4x MSAA on this device: {Reason} (docs/porting/media3d.md, section 3.15).",
            &[&reason],
        );
    }
}

impl Scene3DWgpuExecutor {
    /// The executor of `device`: the shader module and the layouts, made once. The pipelines,
    /// meshes and targets are made on first demand.
    pub fn new(device: &wgpu::Device) -> Result<Scene3DWgpuExecutor, Scene3DWgpuError> {
        let (executor, error) = scoped(device, || Self::create(device));
        match error {
            Some(error) => Err(Scene3DWgpuError { reason: error.to_string() }),
            None => Ok(executor),
        }
    }

    fn create(device: &wgpu::Device) -> Scene3DWgpuExecutor {
        let label = Some("FerroUI 3D scene");
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(SCENE3D_WGSL.into()),
        });
        let uniform_entry = |binding: u32, dynamic: bool, size: usize| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: NonZeroU64::new(size as u64),
            },
            count: None,
        };
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("FerroUI 3D scene frame"),
            entries: &[uniform_entry(0, false, std::mem::size_of::<FrameUniforms>())],
        });
        let draw_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("FerroUI 3D scene draw"),
            entries: &[
                uniform_entry(0, true, std::mem::size_of::<DrawUniforms>()),
                uniform_entry(BRUSH_BINDING, true, std::mem::size_of::<BrushUniforms>()),
            ],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("FerroUI 3D scene brush texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let resolve_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("FerroUI 3D scene resolve"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: RESOLVED_BINDING,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label,
            bind_group_layouts: &[Some(&frame_layout), Some(&draw_layout)],
            immediate_size: 0,
        });
        let textured_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("FerroUI 3D scene textured"),
            bind_group_layouts: &[Some(&frame_layout), Some(&draw_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("FerroUI 3D scene brush"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let resolve_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("FerroUI 3D scene resolve"),
            bind_group_layouts: &[Some(&resolve_layout)],
            immediate_size: 0,
        });
        let features = device.features();
        let multisample =
            multisamples(COLOR_FORMAT, features, true) && multisamples(DEPTH_FORMAT, features, false);
        if !multisample {
            report_single_sample("the formats of the targets take no four samples");
        }
        Scene3DWgpuExecutor {
            module,
            frame_layout,
            draw_layout,
            texture_layout,
            resolve_layout,
            pipeline_layout,
            textured_pipeline_layout,
            resolve_pipeline_layout,
            sampler,
            pipelines: HashMap::new(),
            resolve_pipeline: None,
            multisample,
            draw_stride: draw_uniforms_stride(device.limits().min_uniform_buffer_offset_alignment),
            brush_stride: brush_uniforms_stride(device.limits().min_uniform_buffer_offset_alignment),
            meshes: HashMap::new(),
            textures: HashMap::new(),
            refused_textures: HashMap::new(),
            empty_texture: None,
            targets: Vec::new(),
            renders: 0,
            vertex_scratch: Vec::new(),
            index_scratch: Vec::new(),
            index_bytes: Vec::new(),
        }
    }

    /// Whether antialiased renders take four samples on this device.
    pub fn multisamples(&self) -> bool {
        self.multisample
    }

    /// The pipelines made so far.
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len() + usize::from(self.resolve_pipeline.is_some())
    }

    /// The meshes on the device.
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    /// The textures of brushes on the device.
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Draws `scene` into `target` with the uniform buffers of `viewport`, and submits it to
    /// `queue`. A failure with four samples is reported once and the render is drawn with one;
    /// a failure with one is the error, and nothing is drawn.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport: &mut Scene3DViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DTarget<'_>,
    ) -> Result<Scene3DRenderReport, Scene3DWgpuError> {
        self.render_with_textures(device, queue, viewport, scene, target, &NoDeviceTextures)
    }

    /// [`render`](Self::render) for a renderer that has some bitmaps of the scene as textures of
    /// the device (`textures`, step D7): they are sampled without a copy.
    pub fn render_with_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport: &mut Scene3DViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DTarget<'_>,
        textures: &dyn Scene3DDeviceTextures,
    ) -> Result<Scene3DRenderReport, Scene3DWgpuError> {
        self.renders += 1;
        let mut report = Scene3DRenderReport::default();
        let sample_count = if target.antialiased && self.multisample { MULTISAMPLE_COUNT } else { 1 };
        let (result, error) = scoped(device, || {
            self.render_with(device, queue, viewport, scene, target, textures, sample_count, &mut report)
        });
        let failure = match (result, error) {
            (Ok(()), None) => return Ok(report),
            (Err(error), _) => error,
            (Ok(()), Some(error)) => Scene3DWgpuError { reason: error.to_string() },
        };
        if sample_count == 1 {
            return Err(failure);
        }
        // Four samples failed: one from now on.
        self.multisample = false;
        report_single_sample(&failure.reason);
        let mut report = Scene3DRenderReport::default();
        let (result, error) = scoped(device, || {
            self.render_with(device, queue, viewport, scene, target, textures, 1, &mut report)
        });
        match (result, error) {
            (Ok(()), None) => Ok(report),
            (Err(error), _) => Err(error),
            (Ok(()), Some(error)) => Err(Scene3DWgpuError { reason: error.to_string() }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_with(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport: &mut Scene3DViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DTarget<'_>,
        textures: &dyn Scene3DDeviceTextures,
        sample_count: u32,
        report: &mut Scene3DRenderReport,
    ) -> Result<(), Scene3DWgpuError> {
        report.sample_count = sample_count;
        let width = target.size.0.clamp(1, target.texture_size.0.max(1));
        let height = target.size.1.clamp(1, target.texture_size.1.max(1));

        // The draws and their uniforms, and the meshes they need.
        let frame = FrameUniforms::of(scene);
        viewport.draws.clear();
        viewport.draw_scratch.clear();
        viewport.brush_scratch.clear();
        if frame.is_some() {
            for model in &scene.models {
                if model.front.is_empty() && model.back.is_empty() {
                    continue;
                }
                let (index_count, texture_bounds) = self.mesh(device, queue, &model.mesh, report);
                if index_count == 0 {
                    continue;
                }
                for (face, layers) in [(Face::Front, &model.front), (Face::Back, &model.back)] {
                    let mesh = (index_count, texture_bounds);
                    self.add_draws(device, queue, viewport, model, layers, face, mesh, textures, report);
                }
            }
        }
        report.draws = viewport.draws.len() as u32;
        let additive = viewport.draws.iter().any(|draw| draw.blend == Scene3DBlend::Additive);

        // The pipelines, before anything is encoded: a failure leaves the target as it was.
        for index in 0..viewport.draws.len() {
            let draw = viewport.draws[index];
            self.pipeline(device, draw_key(&draw, sample_count), report)?;
        }
        if additive {
            self.resolve_pipeline(device, report)?;
        }

        // The uniforms that changed.
        if let Some(frame) = &frame {
            self.write_uniforms(device, queue, viewport, frame, report);
        }

        // What no render drew for a while goes; then the targets of the pool.
        let renders = self.renders;
        self.meshes.retain(|_, mesh| renders - mesh.last_used < LIFETIME);
        self.targets.retain(|targets| renders - targets.last_used < LIFETIME);
        self.textures.retain(|_, texture| texture.identity.is_alive() && renders - texture.last_used < LIFETIME);
        self.refused_textures.retain(|_, last| renders - *last < LIFETIME);
        let index = self.targets(device, target.texture_size, sample_count, additive, report);
        let targets = &self.targets[index];

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("FerroUI 3D scene") });
        {
            let resolved = targets.resolved.as_ref().map(|(view, _)| view);
            let (color, resolve_target) = match (&targets.multisampled, additive) {
                (Some(multisampled), false) => (multisampled, Some(target.view)),
                (Some(multisampled), true) => (multisampled, resolved),
                (None, false) => (target.view, None),
                (None, true) => (resolved.unwrap_or(target.view), None),
            };
            let store = if targets.multisampled.is_some() { wgpu::StoreOp::Discard } else { wgpu::StoreOp::Store };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FerroUI 3D scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color,
                    depth_slice: None,
                    resolve_target,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store },
                })],
                // Cleared once per render: the depth of a render is its own.
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            if let (Some((_, frame_group)), Some((_, draw_group))) = (&viewport.frame_buffer, &viewport.draw_buffer) {
                pass.set_viewport(0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
                pass.set_scissor_rect(0, 0, width, height);
                pass.set_bind_group(0, frame_group, &[]);
                let mut current: Option<DrawPipelineKey> = None;
                for (index, draw) in viewport.draws.iter().enumerate() {
                    let key = draw_key(draw, sample_count);
                    let (Some(pipeline), Some(mesh)) = (self.pipelines.get(&key), self.meshes.get(&draw.mesh)) else {
                        continue;
                    };
                    let Some(buffers) = &mesh.buffers else { continue };
                    let texture_group = if samples_texture(draw.brush) {
                        let texture = match &draw.texture {
                            Some(texture) => self.textures.get(texture),
                            None => self.empty_texture.as_ref(),
                        };
                        match texture {
                            Some(texture) => Some(&texture.group),
                            None => continue,
                        }
                    } else {
                        None
                    };
                    if current != Some(key) {
                        pass.set_pipeline(pipeline);
                        current = Some(key);
                    }
                    let offsets = [(index * self.draw_stride) as u32, (index * self.brush_stride) as u32];
                    pass.set_bind_group(1, draw_group, &offsets);
                    if let Some(group) = texture_group {
                        pass.set_bind_group(crate::shader::BRUSH_TEXTURE_GROUP, group, &[]);
                    }
                    pass.set_vertex_buffer(0, buffers.vertices.slice(..buffers.vertex_bytes));
                    pass.set_index_buffer(
                        buffers.indices.slice(..u64::from(draw.index_count) * 4),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.draw_indexed(0..draw.index_count, 0, 0..1);
                }
            }
        }
        if additive {
            if let (Some(pipeline), Some((_, group))) = (&self.resolve_pipeline, &targets.resolved) {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("FerroUI 3D scene resolve"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
                pass.set_viewport(0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
                pass.set_scissor_rect(0, 0, width, height);
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        queue.submit(Some(encoder.finish()));
        count(|counters| counters.passes += 1);
        Ok(())
    }

    /// The draws of the layers of one side of a model, in order, with their uniforms and the
    /// textures of their brushes (uploaded when their keys are new). The first diffuse layer
    /// writes depth (the first layer when the side has no diffuse layer).
    #[allow(clippy::too_many_arguments)]
    fn add_draws(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport: &mut Scene3DViewport,
        model: &Scene3DModel,
        layers: &[Scene3DMaterialLayer],
        face: Face,
        (index_count, texture_bounds): (u32, Rect),
        textures: &dyn Scene3DDeviceTextures,
        report: &mut Scene3DRenderReport,
    ) {
        let depth_layer =
            layers.iter().position(|layer| LayerKind::of(layer.kind) == LayerKind::Diffuse).unwrap_or(0);
        for (index, layer) in layers.iter().enumerate() {
            let (uniforms, _) = DrawUniforms::of(model, layer, face);
            // What this executor is given is already converted (its contract).
            let on_device = |bitmap: &SharedBitmapImpl| match textures.is_on_device(bitmap) {
                true => DeviceBitmap::Converted,
                false => DeviceBitmap::NotOnDevice,
            };
            let mut brush = MaterialBrush::of_with(layer.brush.as_ref(), texture_bounds, &on_device);
            let mut texture = None;
            if let Some(source) = brush.texture().cloned() {
                if self.brush_texture(device, queue, &source, textures, report) {
                    texture = Some(source.key());
                } else {
                    // Texels that cannot be read or uploaded: the layer is drawn with its colour.
                    brush = brush.to_unsampled();
                }
            } else if brush.shader() == BrushShader::Image {
                // An image brush without an image samples the empty texture (it paints nothing).
                self.empty_texture(device, queue, report);
            }
            if brush.is_unsampled() {
                crate::note_unsampled_brush();
            }
            let start = viewport.draw_scratch.len();
            uniforms.write_std140(&mut viewport.draw_scratch);
            viewport.draw_scratch.resize(start + self.draw_stride, 0);
            let start = viewport.brush_scratch.len();
            brush.uniforms().write_std140(&mut viewport.brush_scratch);
            viewport.brush_scratch.resize(start + self.brush_stride, 0);
            viewport.draws.push(DrawRecord {
                mesh: (model.mesh.id, model.mesh.version),
                index_count,
                blend: if uniforms.kind() == LayerKind::Diffuse { Scene3DBlend::SourceOver } else { Scene3DBlend::Additive },
                face,
                depth_write: index == depth_layer,
                brush: brush.shader(),
                texture,
            });
        }
    }

    /// The texture of `source` on the device, uploaded when its key is new: into the texture of
    /// an older version of the same bitmap that no draw of this render uses when it has the same
    /// size, otherwise into a new one. `false` when its texels cannot be read or are larger than
    /// the device takes (asked again only after [`LIFETIME`] renders).
    fn brush_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &BrushTextureSource,
        textures: &dyn Scene3DDeviceTextures,
        report: &mut Scene3DRenderReport,
    ) -> bool {
        let key = source.key();
        let renders = self.renders;
        if let Some(texture) = self.textures.get_mut(&key) {
            texture.last_used = renders;
            return true;
        }
        if let Some(bitmap) = source.bitmap().filter(|bitmap| textures.is_on_device(&***bitmap)) {
            let given = textures.device_texture(&**bitmap).filter(|given| {
                given.texture.format() == COLOR_FORMAT
                    && given.texture.sample_count() == 1
                    && given.texture.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING)
            });
            if let Some(given) = given {
                let size = (given.texture.width(), given.texture.height());
                let group = self.texture_group(device, &given.view, "FerroUI 3D device texture");
                self.textures.insert(
                    key,
                    BrushTexture {
                        identity: source.identity(),
                        texture: given.texture,
                        size,
                        group,
                        last_used: renders,
                        owner: Some(given.owner),
                    },
                );
                count_textures(|counters| counters.textures_on_device += 1);
                return true;
            }
        }
        if let Some(last) = self.refused_textures.get_mut(&key) {
            *last = renders;
            return false;
        }
        let max_side = device.limits().max_texture_dimension_2d;
        let texels = source.texels().filter(|texels| {
            texels.width > 0 && texels.height > 0 && texels.width <= max_side && texels.height <= max_side
        });
        let Some(texels) = texels else {
            self.refused_textures.insert(key, renders);
            count_textures(|counters| counters.textures_refused += 1);
            return false;
        };
        let size = (texels.width, texels.height);
        let replaced_key = match key {
            BrushTextureKey::Image { identity, .. } => self
                .textures
                .iter()
                .find(|(other, texture)| {
                    matches!(other, BrushTextureKey::Image { identity: other, .. } if *other == identity)
                        && texture.last_used != renders
                        && texture.size == size
                        && texture.owner.is_none()
                })
                .map(|(other, _)| *other),
            _ => None,
        };
        let replaced = replaced_key.and_then(|other| self.textures.remove(&other));
        let texture = match replaced {
            Some(texture) => BrushTexture { identity: source.identity(), last_used: renders, ..texture },
            None => {
                report.resources_created += 1;
                count(|counters| counters.textures_created += 1);
                self.create_brush_texture(device, source.identity(), size, "FerroUI 3D brush texture")
            }
        };
        write_texels(queue, &texture.texture, size, &texels.rgba);
        count_textures(|counters| {
            counters.textures_uploaded += 1;
            counters.texture_bytes += texels.rgba.len() as u64;
        });
        self.textures.insert(key, BrushTexture { last_used: renders, ..texture });
        true
    }

    /// The transparent texture of one texel an image brush without an image samples, made on
    /// first need.
    fn empty_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, report: &mut Scene3DRenderReport) {
        if self.empty_texture.is_some() {
            return;
        }
        let identity = BrushTextureIdentity::none();
        let texture = self.create_brush_texture(device, identity, (1, 1), "FerroUI 3D empty brush");
        write_texels(queue, &texture.texture, (1, 1), &[0; 4]);
        report.resources_created += 1;
        count(|counters| counters.textures_created += 1);
        self.empty_texture = Some(texture);
    }

    /// A texture of brush texels of `size` with its bind group.
    fn create_brush_texture(
        &self,
        device: &wgpu::Device,
        identity: BrushTextureIdentity,
        size: (u32, u32),
        label: &str,
    ) -> BrushTexture {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = self.texture_group(device, &view, label);
        BrushTexture { identity, texture, size, group, last_used: self.renders, owner: None }
    }

    /// The bind group of a brush texture: its view and the sampler.
    fn texture_group(&self, device: &wgpu::Device, view: &wgpu::TextureView, label: &str) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        })
    }

    /// The buffers of `mesh` on the device, uploaded when its `(id, version)` is new; returns
    /// the number of its indices (0 for a mesh without triangles) and the box of its texture
    /// coordinates (computed once per version).
    fn mesh(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mesh: &MeshData,
        report: &mut Scene3DRenderReport,
    ) -> (u32, Rect) {
        let key = (mesh.id, mesh.version);
        let renders = self.renders;
        if let Some(buffers) = self.meshes.get_mut(&key) {
            buffers.last_used = renders;
            return (buffers.index_count, buffers.texture_bounds);
        }
        // A new version replaces a version of the mesh that no draw of this render uses, and
        // reuses its buffers when they are large enough.
        let replaced = self
            .meshes
            .iter()
            .find(|(&(id, _), buffers)| id == mesh.id && buffers.last_used != renders)
            .map(|(&key, _)| key)
            .and_then(|key| self.meshes.remove(&key))
            .and_then(|old| old.buffers);
        self.vertex_scratch.clear();
        self.index_scratch.clear();
        let index_count = write_mesh(mesh, &mut self.vertex_scratch, &mut self.index_scratch) as u32;
        let buffers = if index_count == 0 || self.vertex_scratch.is_empty() {
            None
        } else {
            self.index_bytes.clear();
            for index in &self.index_scratch {
                self.index_bytes.extend_from_slice(&index.to_ne_bytes());
            }
            let vertex_bytes = self.vertex_scratch.len() as u64;
            let index_bytes = self.index_bytes.len() as u64;
            let buffers = match replaced {
                Some(old) if old.vertex_capacity >= vertex_bytes && old.index_capacity >= index_bytes => {
                    MeshGpuBuffers { vertex_bytes, ..old }
                }
                _ => {
                    let buffer = |size: u64, usage: wgpu::BufferUsages, label: &str| {
                        device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some(label),
                            size,
                            usage: usage | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        })
                    };
                    report.resources_created += 2;
                    count(|counters| counters.buffers_created += 2);
                    MeshGpuBuffers {
                        vertices: buffer(vertex_bytes, wgpu::BufferUsages::VERTEX, "FerroUI 3D mesh vertices"),
                        vertex_capacity: vertex_bytes,
                        vertex_bytes,
                        indices: buffer(index_bytes, wgpu::BufferUsages::INDEX, "FerroUI 3D mesh indices"),
                        index_capacity: index_bytes,
                    }
                }
            };
            queue.write_buffer(&buffers.vertices, 0, &self.vertex_scratch);
            queue.write_buffer(&buffers.indices, 0, &self.index_bytes);
            report.meshes_uploaded += 1;
            count(|counters| {
                counters.mesh_uploads += 1;
                counters.mesh_bytes += vertex_bytes + index_bytes;
            });
            Some(buffers)
        };
        let texture_bounds = texture_coordinate_bounds(mesh);
        self.meshes.insert(key, MeshBuffers { buffers, index_count, texture_bounds, last_used: renders });
        (index_count, texture_bounds)
    }

    /// The pipeline of `key`, made on first demand.
    fn pipeline(
        &mut self,
        device: &wgpu::Device,
        key: DrawPipelineKey,
        report: &mut Scene3DRenderReport,
    ) -> Result<(), Scene3DWgpuError> {
        if self.pipelines.contains_key(&key) {
            return Ok(());
        }
        let layout = if samples_texture(key.brush) { &self.textured_pipeline_layout } else { &self.pipeline_layout };
        let brush = key.brush;
        let full_key = key;
        let key = key.layer;
        let blend = match key.blend {
            Scene3DBlend::SourceOver => wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            Scene3DBlend::Additive => {
                let add = wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                };
                wgpu::BlendState { color: add, alpha: add }
            }
        };
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: VERTEX_STRIDE as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        };
        let (pipeline, error) = scoped(device, || {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("FerroUI 3D scene"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &self.module,
                    entry_point: Some(VERTEX_ENTRY),
                    buffers: &[Some(vertex_layout)],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    // Counterclockwise on the screen is a front face (WPF).
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(match key.face {
                        Face::Front => wgpu::Face::Back,
                        Face::Back => wgpu::Face::Front,
                    }),
                    ..wgpu::PrimitiveState::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: key.depth_format,
                    depth_write_enabled: Some(key.depth_write),
                    depth_compare: Some(if key.depth_write {
                        wgpu::CompareFunction::Less
                    } else {
                        wgpu::CompareFunction::LessEqual
                    }),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState { count: key.sample_count, ..wgpu::MultisampleState::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &self.module,
                    entry_point: Some(fragment_entry(brush)),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: key.color_format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                multiview_mask: None,
                cache: None,
            })
        });
        if let Some(error) = error {
            return Err(Scene3DWgpuError { reason: format!("the pipeline {full_key:?} cannot be made: {error}") });
        }
        self.pipelines.insert(full_key, pipeline);
        report.pipelines_created += 1;
        count(|counters| counters.pipelines_created += 1);
        Ok(())
    }

    /// The pipeline of the resolve of decision N, made on first demand.
    fn resolve_pipeline(&mut self, device: &wgpu::Device, report: &mut Scene3DRenderReport) -> Result<(), Scene3DWgpuError> {
        if self.resolve_pipeline.is_some() {
            return Ok(());
        }
        let (pipeline, error) = scoped(device, || {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("FerroUI 3D scene resolve"),
                layout: Some(&self.resolve_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.module,
                    entry_point: Some(RESOLVE_VERTEX_ENTRY),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.module,
                    entry_point: Some(RESOLVE_FRAGMENT_ENTRY),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                multiview_mask: None,
                cache: None,
            })
        });
        if let Some(error) = error {
            return Err(Scene3DWgpuError { reason: format!("the resolve pipeline cannot be made: {error}") });
        }
        self.resolve_pipeline = Some(pipeline);
        report.pipelines_created += 1;
        count(|counters| counters.pipelines_created += 1);
        Ok(())
    }

    /// Writes the uniforms of the render that differ from what the buffers of `viewport` hold,
    /// making (or growing) the buffers first when they are missing or too small.
    fn write_uniforms(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport: &mut Scene3DViewport,
        frame: &FrameUniforms,
        report: &mut Scene3DRenderReport,
    ) {
        let mut writes = 0u64;
        let mut bytes = 0u64;

        viewport.frame_scratch.clear();
        frame.write_std140(&mut viewport.frame_scratch);
        if viewport.frame_buffer.is_none() {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("FerroUI 3D scene frame"),
                size: viewport.frame_scratch.len() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("FerroUI 3D scene frame"),
                layout: &self.frame_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
            });
            viewport.frame_buffer = Some((buffer, group));
            viewport.frame_bytes.clear();
            report.resources_created += 1;
            count(|counters| counters.buffers_created += 1);
        }
        if viewport.frame_bytes != viewport.frame_scratch {
            if let Some((buffer, _)) = &viewport.frame_buffer {
                queue.write_buffer(buffer, 0, &viewport.frame_scratch);
            }
            viewport.frame_bytes.clone_from(&viewport.frame_scratch);
            writes += 1;
            bytes += viewport.frame_scratch.len() as u64;
        }

        let draws = viewport.draws.len().max(1);
        if viewport.draw_buffer.is_none() || viewport.brush_buffer.is_none() || viewport.draw_capacity < draws {
            let capacity = draws.next_power_of_two();
            let buffer = |stride: usize, label: &str| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: (capacity * stride) as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            };
            let draw_buffer = buffer(self.draw_stride, "FerroUI 3D scene draws");
            let brush_buffer = buffer(self.brush_stride, "FerroUI 3D scene brushes");
            fn binding(binding: u32, buffer: &wgpu::Buffer, size: usize) -> wgpu::BindGroupEntry<'_> {
                wgpu::BindGroupEntry {
                    binding,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer,
                        offset: 0,
                        size: NonZeroU64::new(size as u64),
                    }),
                }
            }
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("FerroUI 3D scene draws"),
                layout: &self.draw_layout,
                entries: &[
                    binding(0, &draw_buffer, std::mem::size_of::<DrawUniforms>()),
                    binding(BRUSH_BINDING, &brush_buffer, std::mem::size_of::<BrushUniforms>()),
                ],
            });
            viewport.draw_buffer = Some((draw_buffer, group));
            viewport.brush_buffer = Some(brush_buffer);
            viewport.draw_capacity = capacity;
            viewport.draw_bytes.clear();
            viewport.brush_bytes.clear();
            report.resources_created += 2;
            count(|counters| counters.buffers_created += 2);
        }
        // The ranges of the draws whose bytes changed (all of them in a new buffer).
        if let Some((buffer, _)) = &viewport.draw_buffer {
            if let Some(written) =
                write_changed(queue, buffer, &viewport.draw_bytes, &viewport.draw_scratch, self.draw_stride)
            {
                writes += 1;
                bytes += written;
            }
        }
        if let Some(buffer) = &viewport.brush_buffer {
            if let Some(written) =
                write_changed(queue, buffer, &viewport.brush_bytes, &viewport.brush_scratch, self.brush_stride)
            {
                writes += 1;
                bytes += written;
            }
        }
        viewport.draw_bytes.clone_from(&viewport.draw_scratch);
        viewport.brush_bytes.clone_from(&viewport.brush_scratch);

        report.uniform_bytes += bytes;
        count(|counters| {
            counters.uniform_writes += writes;
            counters.uniform_bytes += bytes;
        });
    }

    /// The index of the targets of the pool for `size` and `sample_count`, made on first need
    /// (the resolved colour when `resolved`).
    fn targets(
        &mut self,
        device: &wgpu::Device,
        size: (u32, u32),
        sample_count: u32,
        resolved: bool,
        report: &mut Scene3DRenderReport,
    ) -> usize {
        let renders = self.renders;
        let texture = |format: wgpu::TextureFormat, samples: u32, usage: wgpu::TextureUsages, label: &str| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width: size.0.max(1), height: size.1.max(1), depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let index = match self.targets.iter().position(|t| t.size == size && t.sample_count == sample_count) {
            Some(index) => index,
            None => {
                let multisampled = (sample_count > 1).then(|| {
                    texture(COLOR_FORMAT, sample_count, wgpu::TextureUsages::RENDER_ATTACHMENT, "FerroUI 3D scene colour")
                });
                let depth = texture(DEPTH_FORMAT, sample_count, wgpu::TextureUsages::RENDER_ATTACHMENT, "FerroUI 3D scene depth");
                let made = 1 + u32::from(multisampled.is_some());
                report.resources_created += made;
                count(|counters| counters.textures_created += u64::from(made));
                self.targets.push(Targets { size, sample_count, multisampled, depth, resolved: None, last_used: renders });
                self.targets.len() - 1
            }
        };
        if resolved && self.targets[index].resolved.is_none() {
            let view = texture(
                COLOR_FORMAT,
                1,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                "FerroUI 3D scene resolved colour",
            );
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("FerroUI 3D scene resolve"),
                layout: &self.resolve_layout,
                entries: &[wgpu::BindGroupEntry { binding: RESOLVED_BINDING, resource: wgpu::BindingResource::TextureView(&view) }],
            });
            self.targets[index].resolved = Some((view, group));
            report.resources_created += 1;
            count(|counters| counters.textures_created += 1);
        }
        self.targets[index].last_used = renders;
        index
    }
}

/// The pipeline key of a draw at `sample_count` samples.
fn draw_key(draw: &DrawRecord, sample_count: u32) -> DrawPipelineKey {
    DrawPipelineKey {
        layer: Scene3DPipelineKey {
            blend: draw.blend,
            face: draw.face,
            depth_write: draw.depth_write,
            sample_count,
            color_format: COLOR_FORMAT,
            depth_format: DEPTH_FORMAT,
        },
        brush: draw.brush,
    }
}

/// Writes to `buffer` the one range of the records of `stride` bytes of `new` from the first to
/// the last that differ from `old`; returns the bytes written, `None` when nothing changed.
fn write_changed(queue: &wgpu::Queue, buffer: &wgpu::Buffer, old: &[u8], new: &[u8], stride: usize) -> Option<u64> {
    let changed = |index: usize| {
        let range = index * stride..(index + 1) * stride;
        old.get(range.clone()) != new.get(range)
    };
    let count_of_records = new.len() / stride.max(1);
    let first = (0..count_of_records).find(|&index| changed(index))?;
    let last = (first..count_of_records).rev().find(|&index| changed(index)).unwrap_or(first);
    let range = first * stride..(last + 1) * stride;
    queue.write_buffer(buffer, range.start as u64, &new[range.clone()]);
    Some(range.len() as u64)
}

/// Writes the RGBA8 `rgba` (`size.0 * 4` bytes a row) into `texture`.
fn write_texels(queue: &wgpu::Queue, texture: &wgpu::Texture, size: (u32, u32), rgba: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * size.0), rows_per_image: Some(size.1) },
        wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
    );
}

#[cfg(test)]
#[path = "wgpu_executor_tests.rs"]
mod tests;
