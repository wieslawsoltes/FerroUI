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
//! - **Uniforms.** A [`Scene3DViewport`] (one per kept output of the caller) holds the uniform
//!   buffers of a viewport: the [`FrameUniforms`] in one, the [`DrawUniforms`] of its draws in
//!   another at a stride of the device's `min_uniform_buffer_offset_alignment`, bound with dynamic
//!   offsets. A render writes only the bytes that changed: a camera that moves writes the frame
//!   uniforms alone.
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

use crate::shader::{
    write_mesh, FRAGMENT_ENTRY, RESOLVED_BINDING, RESOLVE_FRAGMENT_ENTRY, RESOLVE_VERTEX_ENTRY, SCENE3D_WGSL,
    VERTEX_ATTRIBUTE_OFFSETS, VERTEX_ENTRY, VERTEX_STRIDE,
};
use crate::uniforms::{BrushSampling, DrawUniforms, Face, FrameUniforms, LayerKind, Std140};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::rendering::scene3d::{MeshData, Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot};
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

/// What a pipeline of a draw is made for: a `Copy` key, one pipeline per key and device.
///
/// The brush kind and the frame-sampling variant of section 3.5 join the key with the brushes of
/// step D6 and the video materials of step D7; until then every layer is drawn with the colour
/// of its uniforms, by the one vertex layout of [`crate::shader`].
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
}

/// The uniform buffers of a viewport, with the bytes they hold, and the draws of its last render.
/// The caller keeps one with each output it keeps (`docs/porting/media3d.md`, section 3.15).
#[derive(Default)]
pub struct Scene3DViewport {
    frame_buffer: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    draw_buffer: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    /// The draws `draw_buffer` holds room for.
    draw_capacity: usize,
    /// The bytes in the buffers, as last written.
    frame_bytes: Vec<u8>,
    draw_bytes: Vec<u8>,
    /// The bytes of the render being prepared.
    frame_scratch: Vec<u8>,
    draw_scratch: Vec<u8>,
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
    last_used: u64,
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
    resolve_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    resolve_pipeline_layout: wgpu::PipelineLayout,
    pipelines: HashMap<Scene3DPipelineKey, wgpu::RenderPipeline>,
    resolve_pipeline: Option<wgpu::RenderPipeline>,
    /// Whether 4x MSAA is drawn: the formats support it, and no pipeline failed with it.
    multisample: bool,
    /// The bytes between the draw uniforms of two draws.
    draw_stride: usize,
    meshes: HashMap<(u64, u64), MeshBuffers>,
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
        let uniform_entry = |dynamic: bool, size: usize| wgpu::BindGroupLayoutEntry {
            binding: 0,
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
            entries: &[uniform_entry(false, std::mem::size_of::<FrameUniforms>())],
        });
        let draw_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("FerroUI 3D scene draw"),
            entries: &[uniform_entry(true, std::mem::size_of::<DrawUniforms>())],
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
            resolve_layout,
            pipeline_layout,
            resolve_pipeline_layout,
            pipelines: HashMap::new(),
            resolve_pipeline: None,
            multisample,
            draw_stride: draw_uniforms_stride(device.limits().min_uniform_buffer_offset_alignment),
            meshes: HashMap::new(),
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
        self.renders += 1;
        let mut report = Scene3DRenderReport::default();
        let sample_count = if target.antialiased && self.multisample { MULTISAMPLE_COUNT } else { 1 };
        let (result, error) = scoped(device, || self.render_with(device, queue, viewport, scene, target, sample_count, &mut report));
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
        let (result, error) = scoped(device, || self.render_with(device, queue, viewport, scene, target, 1, &mut report));
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
        if frame.is_some() {
            for model in &scene.models {
                if model.front.is_empty() && model.back.is_empty() {
                    continue;
                }
                let index_count = self.mesh(device, queue, &model.mesh, report);
                if index_count == 0 {
                    continue;
                }
                for (face, layers) in [(Face::Front, &model.front), (Face::Back, &model.back)] {
                    self.add_draws(viewport, model, layers, face, index_count);
                }
            }
        }
        report.draws = viewport.draws.len() as u32;
        let additive = viewport.draws.iter().any(|draw| draw.blend == Scene3DBlend::Additive);

        // The pipelines, before anything is encoded: a failure leaves the target as it was.
        for index in 0..viewport.draws.len() {
            let draw = viewport.draws[index];
            let key = Scene3DPipelineKey {
                blend: draw.blend,
                face: draw.face,
                depth_write: draw.depth_write,
                sample_count,
                color_format: COLOR_FORMAT,
                depth_format: DEPTH_FORMAT,
            };
            self.pipeline(device, key, report)?;
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
                let mut current: Option<Scene3DPipelineKey> = None;
                for (index, draw) in viewport.draws.iter().enumerate() {
                    let key = Scene3DPipelineKey {
                        blend: draw.blend,
                        face: draw.face,
                        depth_write: draw.depth_write,
                        sample_count,
                        color_format: COLOR_FORMAT,
                        depth_format: DEPTH_FORMAT,
                    };
                    let (Some(pipeline), Some(mesh)) = (self.pipelines.get(&key), self.meshes.get(&draw.mesh)) else {
                        continue;
                    };
                    let Some(buffers) = &mesh.buffers else { continue };
                    if current != Some(key) {
                        pass.set_pipeline(pipeline);
                        current = Some(key);
                    }
                    pass.set_bind_group(1, draw_group, &[(index * self.draw_stride) as u32]);
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

    /// The draws of the layers of one side of a model, in order, with their uniforms. The first
    /// diffuse layer writes depth (the first layer when the side has no diffuse layer).
    fn add_draws(
        &self,
        viewport: &mut Scene3DViewport,
        model: &Scene3DModel,
        layers: &[Scene3DMaterialLayer],
        face: Face,
        index_count: u32,
    ) {
        let depth_layer =
            layers.iter().position(|layer| LayerKind::of(layer.kind) == LayerKind::Diffuse).unwrap_or(0);
        for (index, layer) in layers.iter().enumerate() {
            let (uniforms, sampling) = DrawUniforms::of(model, layer, face);
            if sampling == BrushSampling::Unsampled {
                crate::note_unsampled_brush();
            }
            let start = viewport.draw_scratch.len();
            uniforms.write_std140(&mut viewport.draw_scratch);
            viewport.draw_scratch.resize(start + self.draw_stride, 0);
            viewport.draws.push(DrawRecord {
                mesh: (model.mesh.id, model.mesh.version),
                index_count,
                blend: if uniforms.kind() == LayerKind::Diffuse { Scene3DBlend::SourceOver } else { Scene3DBlend::Additive },
                face,
                depth_write: index == depth_layer,
            });
        }
    }

    /// The buffers of `mesh` on the device, uploaded when its `(id, version)` is new; returns
    /// the number of its indices (0 for a mesh without triangles).
    fn mesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, mesh: &MeshData, report: &mut Scene3DRenderReport) -> u32 {
        let key = (mesh.id, mesh.version);
        let renders = self.renders;
        if let Some(buffers) = self.meshes.get_mut(&key) {
            buffers.last_used = renders;
            return buffers.index_count;
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
        self.meshes.insert(key, MeshBuffers { buffers, index_count, last_used: renders });
        index_count
    }

    /// The pipeline of `key`, made on first demand.
    fn pipeline(&mut self, device: &wgpu::Device, key: Scene3DPipelineKey, report: &mut Scene3DRenderReport) -> Result<(), Scene3DWgpuError> {
        if self.pipelines.contains_key(&key) {
            return Ok(());
        }
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
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.module,
                    entry_point: Some(VERTEX_ENTRY),
                    buffers: &[vertex_layout],
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
                    entry_point: Some(FRAGMENT_ENTRY),
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
            return Err(Scene3DWgpuError { reason: format!("the pipeline {key:?} cannot be made: {error}") });
        }
        self.pipelines.insert(key, pipeline);
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
        if viewport.draw_buffer.is_none() || viewport.draw_capacity < draws {
            let capacity = draws.next_power_of_two();
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("FerroUI 3D scene draws"),
                size: (capacity * self.draw_stride) as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("FerroUI 3D scene draws"),
                layout: &self.draw_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: NonZeroU64::new(std::mem::size_of::<DrawUniforms>() as u64),
                    }),
                }],
            });
            viewport.draw_buffer = Some((buffer, group));
            viewport.draw_capacity = capacity;
            viewport.draw_bytes.clear();
            report.resources_created += 1;
            count(|counters| counters.buffers_created += 1);
        }
        // The range of the draws whose bytes changed (all of them in a new buffer).
        let stride = self.draw_stride;
        let new = &viewport.draw_scratch;
        let old = &viewport.draw_bytes;
        let changed = |index: usize| {
            let range = index * stride..(index + 1) * stride;
            old.get(range.clone()) != new.get(range)
        };
        let count_of_draws = new.len() / stride;
        let first = (0..count_of_draws).find(|&index| changed(index));
        if let Some(first) = first {
            let last = (first..count_of_draws).rev().find(|&index| changed(index)).unwrap_or(first);
            let range = first * stride..(last + 1) * stride;
            if let Some((buffer, _)) = &viewport.draw_buffer {
                queue.write_buffer(buffer, range.start as u64, &new[range.clone()]);
            }
            writes += 1;
            bytes += range.len() as u64;
        }
        viewport.draw_bytes.clone_from(&viewport.draw_scratch);

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

#[cfg(test)]
#[path = "wgpu_executor_tests.rs"]
mod tests;
