//! The Metal executor of 3D scenes (#608, `docs/porting/media3d.md`, section 3.19, step D5): the
//! renderer of the 3D scenes of the Skia backend on Graphite, which draws on the Metal device of
//! the platform. It draws a [`Scene3DSnapshot`] with the one shader of the executors,
//! `scene3d.wgsl` translated by naga to the Metal Shading Language at build time
//! (`ferroui_render3d::shader::msl`), into a texture of the device ([`Scene3DMetalTexture`]) that
//! the backend wraps as an image; stage for stage as the CPU reference and the wgpu executor of
//! `ferroui-render3d` draw it:
//!
//! - **Draws.** One draw per material layer of each model, the front layers then the back layers
//!   of a model, in tree order. A draw of the front layers culls the back faces and the other way
//!   round (counterclockwise on the screen is a front face); the first diffuse layer of a side
//!   writes depth with `Less`, the others test `LessEqual` without writing; a diffuse layer is
//!   blended premultiplied source-over, a specular or emissive layer is added.
//! - **Compiled once.** The library is compiled from the source once per executor (one executor
//!   per device), with the invariance of the clip position preserved. The two depth states are
//!   made with it. Pipelines are made on first demand by a `Copy` key (the blend, the sample
//!   count and the variant of the brush shader; culling and depth are state of the encoder in
//!   Metal), kept, and counted ([`Scene3DMetalCounters::pipelines_created`]).
//! - **Meshes** are uploaded once per `(id, version)` as an interleaved vertex buffer and a `u32`
//!   index buffer, kept while they are drawn. A new version is a new pair of buffers: Metal keeps
//!   the buffers of the version a submitted render still reads until that render ran.
//! - **Brushes.** The texture of a brush is uploaded once per key and device into a private
//!   texture, by a copy encoded in the command buffer of the render that first draws it (so a
//!   new version of a bitmap written into the texture of the version it replaces waits for the
//!   renders that read it); one inlined sampler (linear, the edges clamped).
//! - **Uniforms.** A [`Scene3DMetalViewport`] holds the uniform buffers of a viewport in
//!   [`UNIFORM_SLOTS`] slots used in turn, so that the CPU never writes a buffer a submitted render
//!   still reads; a slot whose last render has not run waits for it. The buffers of all the slots
//!   are made together, by the first render that needs them. A render writes into its slot only
//!   the bytes that differ from what the slot holds: a camera that moves writes the frame uniforms
//!   alone once every slot holds the draws.
//! - **Targets.** The multisampled colour, the depth (`Depth32Float`) and the resolved colour of
//!   decision N are private textures pooled by the size of the output and the sample count; depth
//!   is cleared once per render and not stored. 4x MSAA where the device takes it, 1x for aliased
//!   edges, resolved on the GPU. A scene with specular or emissive layers is resolved into the
//!   pooled colour and the resolve function writes the output from it, with the alpha of each
//!   pixel raised to its largest colour channel.
//! - **Order.** A render is one command buffer of the queue given to the executor, committed when
//!   the render returns. On the queue Skia's Graphite context submits to, it runs before the frame
//!   that draws its output, which is submitted later.
//!
//! Errors: a library or a pipeline that cannot be made with one sample, and a render whose command
//! buffer failed on the GPU (seen by the next render), are a [`Scene3DMetalError`] the caller
//! reports once before it draws the CPU reference; a pipeline that cannot be made with four
//! samples is reported once, and the executor draws with one from then on.
//!
//! `unsafe`, as in `offscreen.rs`: the handles of the caller, the copies of bytes into buffers and
//! the calls of the encoders that take offsets and indices.

use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::SharedBitmapImpl;
use ferroui_base::rendering::scene3d::{MeshData, Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot};
use ferroui_base::Rect;
use ferroui_render3d::brush::DeviceBitmap;
use ferroui_render3d::brush::{
    texture_coordinate_bounds, BrushShader, BrushTextureIdentity, BrushTextureKey, BrushTextureSource,
    MaterialBrush, BRUSH_UNIFORMS_STRIDE,
};
use ferroui_render3d::shader::{msl, samples_texture, write_mesh, VERTEX_ATTRIBUTE_OFFSETS, VERTEX_STRIDE};
use ferroui_render3d::uniforms::{DrawUniforms, Face, FrameUniforms, LayerKind, Std140, DRAW_UNIFORMS_STRIDE};
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2::sel;
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBlendFactor, MTLBlendOperation, MTLBlitCommandEncoder, MTLBuffer, MTLClearColor, MTLColorWriteMask,
    MTLCommandBuffer, MTLCommandBufferStatus, MTLCommandEncoder, MTLCommandQueue, MTLCompareFunction,
    MTLCompileOptions, MTLCullMode, MTLDepthStencilDescriptor, MTLDepthStencilState, MTLDevice, MTLFunction,
    MTLIndexType, MTLLibrary, MTLLoadAction, MTLOrigin, MTLPixelFormat, MTLPrimitiveType, MTLRenderCommandEncoder,
    MTLRenderPassDescriptor, MTLRenderPipelineDescriptor, MTLRenderPipelineState, MTLResourceOptions,
    MTLScissorRect, MTLSize, MTLStorageMode, MTLStoreAction, MTLTexture, MTLTextureDescriptor, MTLTextureType,
    MTLTextureUsage, MTLVertexDescriptor, MTLVertexFormat, MTLVertexStepFunction, MTLViewport, MTLWinding,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};

type Device = ProtocolObject<dyn MTLDevice>;
type CommandQueueObject = ProtocolObject<dyn MTLCommandQueue>;
type CommandBuffer = ProtocolObject<dyn MTLCommandBuffer>;
type Buffer = ProtocolObject<dyn MTLBuffer>;
type Texture = ProtocolObject<dyn MTLTexture>;
type Function = ProtocolObject<dyn MTLFunction>;
type PipelineState = ProtocolObject<dyn MTLRenderPipelineState>;
type DepthState = ProtocolObject<dyn MTLDepthStencilState>;

/// The samples of an antialiased render (decision F).
pub const MULTISAMPLE_COUNT: u32 = 4;

/// The slots of uniform buffers of a viewport, used in turn: a render writes the slot that the
/// render before the one before it used, which has run in a frame or two.
pub const UNIFORM_SLOTS: usize = 3;

/// The renders a mesh, a texture or a target of the pool is kept for without being drawn.
const LIFETIME: u64 = 240;

/// The largest side of a texture of a brush the executor uploads: what every Metal device takes.
const MAX_TEXTURE_SIDE: u32 = 8192;

/// The bytes between the draw uniforms of two draws: the alignment of a constant buffer offset on
/// every Metal device.
const DRAW_STRIDE: usize = DRAW_UNIFORMS_STRIDE;
/// The bytes between the brush uniforms of two draws.
const BRUSH_STRIDE: usize = BRUSH_UNIFORMS_STRIDE;

/// What the Metal executors of this thread did since it started: what the steady-state tests
/// count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scene3DMetalCounters {
    /// Render pipelines made (the draws' and the resolve's).
    pub pipelines_created: u64,
    /// Libraries compiled from the source (one per executor).
    pub libraries_compiled: u64,
    /// Meshes uploaded (a new `(id, version)`).
    pub mesh_uploads: u64,
    /// The bytes of the vertices and indices uploaded.
    pub mesh_bytes: u64,
    /// Writes of uniform buffers.
    pub uniform_writes: u64,
    /// The bytes of the uniforms written.
    pub uniform_bytes: u64,
    /// Renders of a scene (command buffers with a 3D pass) committed.
    pub passes: u64,
    /// Renders that waited for the GPU to finish an earlier render of the same uniform slot.
    pub uniform_waits: u64,
    /// Textures made (multisampled colour, depth, resolved colour, the textures of brushes and
    /// the outputs).
    pub textures_created: u64,
    /// Buffers made (vertex, index and uniform buffers; not the staging of a texture upload).
    pub buffers_created: u64,
    /// Textures of brushes written: a new key (a gradient's stops, a bitmap's version).
    pub textures_uploaded: u64,
    /// The bytes of those texels.
    pub texture_bytes: u64,
    /// Textures of brushes that could not be uploaded (no readable pixels, larger than the device
    /// takes): their layers are drawn with their colours and counted
    /// (`ferroui_render3d::layers_with_unsampled_brushes`).
    pub textures_refused: u64,
    /// Textures of brushes the renderer already had on the device (a frame of a video, step D7):
    /// sampled as they are, with nothing written.
    pub textures_on_device: u64,
}

impl Scene3DMetalCounters {
    const ZERO: Scene3DMetalCounters = Scene3DMetalCounters {
        pipelines_created: 0,
        libraries_compiled: 0,
        mesh_uploads: 0,
        mesh_bytes: 0,
        uniform_writes: 0,
        uniform_bytes: 0,
        passes: 0,
        uniform_waits: 0,
        textures_created: 0,
        buffers_created: 0,
        textures_uploaded: 0,
        texture_bytes: 0,
        textures_refused: 0,
        textures_on_device: 0,
    };
}

ferroui_base::ferro_thread_local! {
    /// What the Metal executors of this thread did.
    static COUNTERS: Cell<Scene3DMetalCounters> = const { Cell::new(Scene3DMetalCounters::ZERO) };
}

/// What the Metal executors of this thread did since it started.
pub fn counters() -> Scene3DMetalCounters {
    COUNTERS.with(Cell::get)
}

fn count(change: impl FnOnce(&mut Scene3DMetalCounters)) {
    COUNTERS.with(|counters| {
        let mut value = counters.get();
        change(&mut value);
        counters.set(value);
    });
}

/// Why the executor cannot draw a scene: the caller draws the CPU reference instead, and reports
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene3DMetalError {
    /// What failed, in the words of Metal.
    pub reason: String,
}

impl Scene3DMetalError {
    fn new(reason: impl Into<String>) -> Scene3DMetalError {
        Scene3DMetalError { reason: reason.into() }
    }
}

impl std::fmt::Display for Scene3DMetalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The 3D scene cannot be drawn with Metal: {}", self.reason)
    }
}

impl std::error::Error for Scene3DMetalError {}

/// A texture of the device a scene is drawn into: RGBA8 (`MTLPixelFormat::RGBA8Unorm`),
/// premultiplied, one sample, private, drawn to and sampled.
pub struct Scene3DMetalTexture {
    texture: Retained<Texture>,
    width: u32,
    height: u32,
}

impl Scene3DMetalTexture {
    /// The `id<MTLTexture>`, valid while this texture lives: what the backend wraps for its
    /// drawing (a wrapper takes a reference of its own).
    pub fn as_ptr(&self) -> *mut c_void {
        Retained::as_ptr(&self.texture) as *mut c_void
    }

    /// The width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }
}

/// The bitmaps a renderer has as textures of the device of the executor, whose pixels cannot be
/// read: the frames of a video or a camera (step D7).
///
/// # Safety
/// [`device_texture`](Self::device_texture) returns a valid `id<MTLTexture>`, which stays valid
/// until the executor has taken its own reference when the call returns. The renderer keeps the
/// storage of the frame from the producer until the GPU has done the renders that read it.
pub unsafe trait Scene3DMetalDeviceTextures {
    /// How `bitmap` is on this device. Asked for every layer that has such a bitmap.
    fn sampling(&self, bitmap: &SharedBitmapImpl) -> DeviceBitmap;

    /// The `id<MTLTexture>` of `bitmap`, asked once for each bitmap the executor does not hold.
    fn device_texture(&self, bitmap: &SharedBitmapImpl) -> Option<*mut c_void>;
}

/// A renderer without such bitmaps.
struct NoDeviceTextures;

// SAFETY: it gives no texture.
unsafe impl Scene3DMetalDeviceTextures for NoDeviceTextures {
    fn sampling(&self, _: &SharedBitmapImpl) -> DeviceBitmap {
        DeviceBitmap::NotOnDevice
    }

    fn device_texture(&self, _: &SharedBitmapImpl) -> Option<*mut c_void> {
        None
    }
}

/// The texture a render draws into.
pub struct Scene3DMetalTarget<'a> {
    /// The output, a texture made by [`Scene3DMetalExecutor::create_output_texture`]. The scene is
    /// drawn at its top left; the rest of it is cleared to transparent.
    pub texture: &'a Scene3DMetalTexture,
    /// The size of the image of the scene, in pixels: at most the size of the texture.
    pub size: (u32, u32),
    /// Whether edges are antialiased (4x MSAA); `false` takes one sample per pixel
    /// (`RenderOptions.EdgeMode` `Aliased`).
    pub antialiased: bool,
}

/// What one render did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scene3DMetalRenderReport {
    /// The samples per pixel it drew with.
    pub sample_count: u32,
    /// The draws it issued.
    pub draws: u32,
    /// The meshes it uploaded.
    pub meshes_uploaded: u32,
    /// The textures of brushes it uploaded.
    pub textures_uploaded: u32,
    /// The pipelines it made.
    pub pipelines_created: u32,
    /// The textures and buffers it made.
    pub resources_created: u32,
    /// The bytes of uniforms it wrote.
    pub uniform_bytes: u64,
}

impl Scene3DMetalRenderReport {
    /// Whether the render wrote uniforms and nothing else: no mesh or texture uploaded, no
    /// pipeline, texture or buffer made (the steady state of a moving camera).
    pub fn is_uniform_only(&self) -> bool {
        self.meshes_uploaded == 0
            && self.textures_uploaded == 0
            && self.pipelines_created == 0
            && self.resources_created == 0
    }
}

/// How a draw is blended into the colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Blend {
    /// A diffuse layer: premultiplied source-over.
    SourceOver,
    /// A specular or emissive layer: added, with alpha 0.
    Additive,
}

/// What a pipeline of a draw is made for. Culling and the depth test are state of the encoder in
/// Metal, not of the pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PipelineKey {
    blend: Blend,
    sample_count: u32,
    brush: BrushShader,
}

/// A draw of a render: one layer of one model on one side of its faces.
#[derive(Clone, Copy, Debug)]
struct DrawRecord {
    mesh: (u64, u64),
    index_count: u32,
    blend: Blend,
    face: Face,
    depth_write: bool,
    brush: BrushShader,
    /// The texture of the brush (`None` for a brush that samples none, or an image brush without
    /// an image, which binds the empty texture).
    texture: Option<BrushTextureKey>,
}

/// One slot of the uniform buffers of a viewport, with the bytes it holds and the last render that
/// reads it.
#[derive(Default)]
struct UniformSlot {
    frame: Option<Retained<Buffer>>,
    draws: Option<Retained<Buffer>>,
    brushes: Option<Retained<Buffer>>,
    /// The draws `draws` and `brushes` hold room for.
    capacity: usize,
    frame_bytes: Vec<u8>,
    draw_bytes: Vec<u8>,
    brush_bytes: Vec<u8>,
    in_flight: Option<Retained<CommandBuffer>>,
}

/// The uniform buffers of a viewport, with the bytes they hold, and the draws of its last render.
/// The caller keeps one with each output it keeps.
#[derive(Default)]
pub struct Scene3DMetalViewport {
    slots: Vec<UniformSlot>,
    next: usize,
    frame_scratch: Vec<u8>,
    draw_scratch: Vec<u8>,
    brush_scratch: Vec<u8>,
    draws: Vec<DrawRecord>,
}

impl Scene3DMetalViewport {
    /// A viewport without buffers yet: they are made by its first render.
    pub fn new() -> Scene3DMetalViewport {
        Scene3DMetalViewport::default()
    }
}

/// A mesh on the device.
struct MeshBuffers {
    /// The vertices and the indices; `None` for a mesh without triangles.
    buffers: Option<(Retained<Buffer>, Retained<Buffer>)>,
    index_count: u32,
    /// The box of its texture coordinates, which relative brushes are mapped onto.
    texture_bounds: Rect,
    last_used: u64,
}

/// The texture of a brush on the device.
struct BrushTexture {
    /// Keeps the address of a bitmap from naming another bitmap while the texture is kept.
    identity: BrushTextureIdentity,
    texture: Retained<Texture>,
    size: (u32, u32),
    last_used: u64,
    /// A texture of the renderer (step D7): sampled, never written.
    on_device: bool,
}

/// Texels to copy into a texture before the render pass of a render.
struct Upload {
    staging: Retained<Buffer>,
    texture: Retained<Texture>,
    size: (u32, u32),
}

/// The textures of the pool for one size and sample count.
struct Targets {
    size: (u32, u32),
    sample_count: u32,
    multisampled: Option<Retained<Texture>>,
    depth: Retained<Texture>,
    resolved: Option<Retained<Texture>>,
    last_used: u64,
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
            "3D scenes (Viewport3D) are drawn without 4x MSAA on this Metal device: {Reason} \
             (docs/porting/media3d.md, section 3.19).",
            &[&reason],
        );
    }
}

/// The Metal executor of one device and queue: its library, depth states, pipelines, meshes,
/// textures and targets.
pub struct Scene3DMetalExecutor {
    device: Retained<Device>,
    queue: Retained<CommandQueueObject>,
    library: Retained<ProtocolObject<dyn MTLLibrary>>,
    vertex_descriptor: Retained<MTLVertexDescriptor>,
    /// `Less` with depth written, and `LessEqual` without.
    depth_write: Retained<DepthState>,
    depth_test: Retained<DepthState>,
    functions: HashMap<&'static str, Retained<Function>>,
    pipelines: HashMap<PipelineKey, Retained<PipelineState>>,
    resolve_pipeline: Option<Retained<PipelineState>>,
    /// Whether 4x MSAA is drawn: the device takes it, and no pipeline failed with it.
    multisample: bool,
    meshes: HashMap<(u64, u64), MeshBuffers>,
    textures: HashMap<BrushTextureKey, BrushTexture>,
    /// The keys whose texels could not be read or uploaded, with the render that last asked: not
    /// read again every frame.
    refused_textures: HashMap<BrushTextureKey, u64>,
    /// A transparent texture of one texel: what an image brush without an image samples.
    empty_texture: Option<Retained<Texture>>,
    targets: Vec<Targets>,
    uploads: Vec<Upload>,
    /// The command buffer of the last render: a failure on the GPU is seen by the next render.
    last_render: Option<Retained<CommandBuffer>>,
    /// The renders so far: the clock of the pools.
    renders: u64,
    vertex_scratch: Vec<u8>,
    index_scratch: Vec<u32>,
    index_bytes: Vec<u8>,
}

impl Scene3DMetalExecutor {
    /// The executor of `device`, drawing with command buffers of `queue`: the library compiled from
    /// the shader, the vertex descriptor and the depth states, made once. The pipelines, meshes and
    /// targets are made on first demand.
    ///
    /// # Safety
    /// `device` is a valid `id<MTLDevice>` and `queue` a valid `id<MTLCommandQueue>` of it; the
    /// executor takes references of its own.
    pub unsafe fn new(device: *mut c_void, queue: *mut c_void) -> Result<Scene3DMetalExecutor, Scene3DMetalError> {
        // SAFETY: the caller hands out valid handles; `retain` takes a reference of its own.
        let device = unsafe { Retained::retain(device.cast::<Device>()) };
        // SAFETY: as above.
        let queue = unsafe { Retained::retain(queue.cast::<CommandQueueObject>()) };
        match (device, queue) {
            (Some(device), Some(queue)) => Self::with_device(device, queue),
            _ => Err(Scene3DMetalError::new("no Metal device or command queue")),
        }
    }

    fn with_device(
        device: Retained<Device>,
        queue: Retained<CommandQueueObject>,
    ) -> Result<Scene3DMetalExecutor, Scene3DMetalError> {
        objc2::rc::autoreleasepool(|_| {
            let options = MTLCompileOptions::new();
            // `[[invariant]]` positions keep their meaning only with the invariance preserved
            // (macOS 11, iOS 14): the layers of a material test `LessEqual` against the depth of
            // the first.
            if options.respondsToSelector(sel!(setPreserveInvariance:)) {
                options.setPreserveInvariance(true);
            }
            let library = device
                .newLibraryWithSource_options_error(&NSString::from_str(msl::SOURCE), Some(&options))
                .map_err(|error| {
                    Scene3DMetalError::new(format!("the shader does not compile: {}", error.localizedDescription()))
                })?;
            count(|counters| counters.libraries_compiled += 1);

            let vertex_descriptor = MTLVertexDescriptor::vertexDescriptor();
            let formats = [MTLVertexFormat::Float3, MTLVertexFormat::Float3, MTLVertexFormat::Float2];
            for (location, (format, offset)) in formats.into_iter().zip(VERTEX_ATTRIBUTE_OFFSETS).enumerate() {
                // SAFETY: indices below the 31 attributes of a vertex descriptor.
                let attribute = unsafe { vertex_descriptor.attributes().objectAtIndexedSubscript(location) };
                attribute.setFormat(format);
                // SAFETY: the offsets are within the stride of the vertices.
                unsafe {
                    attribute.setOffset(offset);
                    attribute.setBufferIndex(msl::VERTEX_BUFFER_INDEX);
                }
            }
            // SAFETY: the buffer index of the vertices is below the 31 buffers of a stage.
            let layout = unsafe { vertex_descriptor.layouts().objectAtIndexedSubscript(msl::VERTEX_BUFFER_INDEX) };
            // SAFETY: the stride of the interleaved vertices, a multiple of four.
            unsafe { layout.setStride(VERTEX_STRIDE) };
            layout.setStepFunction(MTLVertexStepFunction::PerVertex);

            let depth_state = |compare: MTLCompareFunction, write: bool| {
                let descriptor = MTLDepthStencilDescriptor::new();
                descriptor.setDepthCompareFunction(compare);
                descriptor.setDepthWriteEnabled(write);
                device.newDepthStencilStateWithDescriptor(&descriptor)
            };
            let (Some(depth_write), Some(depth_test)) =
                (depth_state(MTLCompareFunction::Less, true), depth_state(MTLCompareFunction::LessEqual, false))
            else {
                return Err(Scene3DMetalError::new("the depth states cannot be made"));
            };

            let multisample = device.supportsTextureSampleCount(MULTISAMPLE_COUNT as usize);
            if !multisample {
                report_single_sample("the device takes no four samples");
            }
            Ok(Scene3DMetalExecutor {
                device,
                queue,
                library,
                vertex_descriptor,
                depth_write,
                depth_test,
                functions: HashMap::new(),
                pipelines: HashMap::new(),
                resolve_pipeline: None,
                multisample,
                meshes: HashMap::new(),
                textures: HashMap::new(),
                refused_textures: HashMap::new(),
                empty_texture: None,
                targets: Vec::new(),
                uploads: Vec::new(),
                last_render: None,
                renders: 0,
                vertex_scratch: Vec::new(),
                index_scratch: Vec::new(),
                index_bytes: Vec::new(),
            })
        })
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

    /// A texture of the device of `width` by `height` pixels to draw scenes into, or `None` when
    /// the device cannot make it.
    pub fn create_output_texture(&self, width: u32, height: u32) -> Option<Scene3DMetalTexture> {
        let (width, height) = (width.max(1), height.max(1));
        let texture = self.texture(
            MTLPixelFormat::RGBA8Unorm,
            (width, height),
            1,
            MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead,
        )?;
        Some(Scene3DMetalTexture { texture, width, height })
    }

    /// Draws `scene` into `target` with the uniform buffers of `viewport`, and commits the command
    /// buffer to the queue of the executor. A failure with four samples is reported once and the
    /// render is drawn with one; a failure with one is the error, and nothing is drawn.
    pub fn render(
        &mut self,
        viewport: &mut Scene3DMetalViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DMetalTarget<'_>,
    ) -> Result<Scene3DMetalRenderReport, Scene3DMetalError> {
        self.render_with_textures(viewport, scene, target, &NoDeviceTextures)
    }

    /// [`render`](Self::render) for a renderer that has some bitmaps of the scene as textures of
    /// the device (`textures`, step D7): they are sampled without a copy.
    pub fn render_with_textures(
        &mut self,
        viewport: &mut Scene3DMetalViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DMetalTarget<'_>,
        textures: &dyn Scene3DMetalDeviceTextures,
    ) -> Result<Scene3DMetalRenderReport, Scene3DMetalError> {
        if let Some(last) = &self.last_render {
            if last.status() == MTLCommandBufferStatus::Error {
                let reason =
                    last.error().map_or_else(|| "unknown".to_owned(), |error| error.localizedDescription().to_string());
                return Err(Scene3DMetalError::new(format!("a render failed on the GPU: {reason}")));
            }
        }
        self.renders += 1;
        objc2::rc::autoreleasepool(|_| {
            let sample_count = if target.antialiased && self.multisample { MULTISAMPLE_COUNT } else { 1 };
            let mut report = Scene3DMetalRenderReport::default();
            match self.render_with(viewport, scene, target, textures, sample_count, &mut report) {
                Ok(()) => Ok(report),
                Err(error) if sample_count == 1 => Err(error),
                Err(error) => {
                    // Four samples failed: one from now on.
                    self.multisample = false;
                    report_single_sample(&error.reason);
                    let mut report = Scene3DMetalRenderReport::default();
                    self.render_with(viewport, scene, target, textures, 1, &mut report).map(|()| report)
                }
            }
        })
    }

    fn render_with(
        &mut self,
        viewport: &mut Scene3DMetalViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DMetalTarget<'_>,
        textures: &dyn Scene3DMetalDeviceTextures,
        sample_count: u32,
        report: &mut Scene3DMetalRenderReport,
    ) -> Result<(), Scene3DMetalError> {
        report.sample_count = sample_count;
        let texture_size = (target.texture.width, target.texture.height);
        let width = target.size.0.clamp(1, texture_size.0);
        let height = target.size.1.clamp(1, texture_size.1);
        self.uploads.clear();

        // The draws and their uniforms, and the meshes and textures they need.
        let frame = FrameUniforms::of(scene);
        viewport.draws.clear();
        viewport.draw_scratch.clear();
        viewport.brush_scratch.clear();
        if frame.is_some() {
            for model in &scene.models {
                if model.front.is_empty() && model.back.is_empty() {
                    continue;
                }
                let (index_count, texture_bounds) = self.mesh(&model.mesh, report)?;
                if index_count == 0 {
                    continue;
                }
                for (face, layers) in [(Face::Front, &model.front), (Face::Back, &model.back)] {
                    self.add_draws(viewport, model, layers, face, (index_count, texture_bounds), textures, report)?;
                }
            }
        }
        report.draws = viewport.draws.len() as u32;
        let additive = viewport.draws.iter().any(|draw| draw.blend == Blend::Additive);

        // The pipelines, before anything is encoded: a failure leaves the target as it was.
        for index in 0..viewport.draws.len() {
            let draw = viewport.draws[index];
            self.pipeline(draw_key(&draw, sample_count), report)?;
        }
        if additive {
            self.resolve_pipeline(report)?;
        }

        // What no render drew for a while goes; then the targets of the pool.
        let renders = self.renders;
        self.meshes.retain(|_, mesh| renders - mesh.last_used < LIFETIME);
        self.targets.retain(|targets| renders - targets.last_used < LIFETIME);
        self.textures.retain(|_, texture| {
            texture.identity.is_alive() && renders - texture.last_used < LIFETIME
        });
        self.refused_textures.retain(|_, last| renders - *last < LIFETIME);
        let targets = self.targets(texture_size, sample_count, additive, report)?;

        // The uniforms that changed, into the slot of this render.
        let slot = match &frame {
            Some(frame) => Some(self.write_uniforms(viewport, frame, report)?),
            None => None,
        };
        let targets = &self.targets[targets];

        let command_buffer = self.queue.commandBuffer().ok_or_else(|| Scene3DMetalError::new("no command buffer"))?;
        command_buffer.setLabel(Some(&NSString::from_str("FerroUI 3D scene")));
        if !self.uploads.is_empty() {
            let blit = command_buffer
                .blitCommandEncoder()
                .ok_or_else(|| Scene3DMetalError::new("no blit command encoder"))?;
            for upload in &self.uploads {
                let (upload_width, upload_height) = (upload.size.0 as usize, upload.size.1 as usize);
                // SAFETY: the staging buffer holds `width * 4` bytes a row for `height` rows, and
                // the texture is at least that large.
                unsafe {
                    blit.copyFromBuffer_sourceOffset_sourceBytesPerRow_sourceBytesPerImage_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                        &upload.staging,
                        0,
                        upload_width * 4,
                        upload_width * upload_height * 4,
                        MTLSize { width: upload_width, height: upload_height, depth: 1 },
                        &upload.texture,
                        0,
                        0,
                        MTLOrigin { x: 0, y: 0, z: 0 },
                    );
                }
            }
            blit.endEncoding();
        }

        let resolved = targets.resolved.as_deref();
        let output = &*target.texture.texture;
        let (color, resolve) = match (targets.multisampled.as_deref(), additive) {
            (Some(multisampled), false) => (multisampled, Some(output)),
            (Some(multisampled), true) => (multisampled, resolved),
            (None, false) => (output, None),
            (None, true) => (resolved.unwrap_or(output), None),
        };
        let pass = MTLRenderPassDescriptor::renderPassDescriptor();
        // SAFETY: attachment 0 exists in every render pass descriptor.
        let attachment = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
        attachment.setTexture(Some(color));
        attachment.setResolveTexture(resolve);
        attachment.setLoadAction(MTLLoadAction::Clear);
        attachment.setClearColor(MTLClearColor { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.0 });
        let store = if resolve.is_some() { MTLStoreAction::MultisampleResolve } else { MTLStoreAction::Store };
        attachment.setStoreAction(store);
        // Cleared once per render: the depth of a render is its own.
        let depth = pass.depthAttachment();
        depth.setTexture(Some(&targets.depth));
        depth.setLoadAction(MTLLoadAction::Clear);
        depth.setClearDepth(1.0);
        depth.setStoreAction(MTLStoreAction::DontCare);
        let encoder = command_buffer
            .renderCommandEncoderWithDescriptor(&pass)
            .ok_or_else(|| Scene3DMetalError::new("no render command encoder"))?;
        if let Some(slot) = slot.map(|index| &viewport.slots[index]) {
            if let (Some(frame), Some(draws), Some(brushes)) = (&slot.frame, &slot.draws, &slot.brushes) {
                encoder.setViewport(MTLViewport {
                    originX: 0.0,
                    originY: 0.0,
                    width: f64::from(width),
                    height: f64::from(height),
                    znear: 0.0,
                    zfar: 1.0,
                });
                encoder.setScissorRect(MTLScissorRect { x: 0, y: 0, width: width as usize, height: height as usize });
                // Counterclockwise on the screen is a front face (WPF).
                encoder.setFrontFacingWinding(MTLWinding::CounterClockwise);
                // SAFETY: the slots of the argument tables of the functions (`shader::msl`), below
                // 31; the buffers hold the frame uniforms and the uniforms of every draw.
                unsafe {
                    encoder.setVertexBuffer_offset_atIndex(Some(frame), 0, msl::FRAME_BUFFER_INDEX);
                    encoder.setFragmentBuffer_offset_atIndex(Some(frame), 0, msl::FRAME_BUFFER_INDEX);
                    encoder.setVertexBuffer_offset_atIndex(Some(draws), 0, msl::DRAW_BUFFER_INDEX);
                    encoder.setFragmentBuffer_offset_atIndex(Some(draws), 0, msl::DRAW_BUFFER_INDEX);
                    encoder.setFragmentBuffer_offset_atIndex(Some(brushes), 0, msl::BRUSH_BUFFER_INDEX);
                }
                let mut current: Option<PipelineKey> = None;
                let mut current_depth: Option<bool> = None;
                let mut current_face: Option<Face> = None;
                for (index, draw) in viewport.draws.iter().enumerate() {
                    let key = draw_key(draw, sample_count);
                    let (Some(pipeline), Some(mesh)) = (self.pipelines.get(&key), self.meshes.get(&draw.mesh)) else {
                        continue;
                    };
                    let Some((vertices, indices)) = &mesh.buffers else { continue };
                    let texture = if samples_texture(draw.brush) {
                        let texture = match &draw.texture {
                            Some(texture) => self.textures.get(texture).map(|texture| &*texture.texture),
                            None => self.empty_texture.as_deref(),
                        };
                        match texture {
                            Some(texture) => Some(texture),
                            None => continue,
                        }
                    } else {
                        None
                    };
                    if current != Some(key) {
                        encoder.setRenderPipelineState(pipeline);
                        current = Some(key);
                    }
                    if current_depth != Some(draw.depth_write) {
                        let state = if draw.depth_write { &self.depth_write } else { &self.depth_test };
                        encoder.setDepthStencilState(Some(state));
                        current_depth = Some(draw.depth_write);
                    }
                    if current_face != Some(draw.face) {
                        encoder.setCullMode(match draw.face {
                            Face::Front => MTLCullMode::Back,
                            Face::Back => MTLCullMode::Front,
                        });
                        current_face = Some(draw.face);
                    }
                    // SAFETY: the offsets are within the buffers (a record per draw at the strides
                    // the buffers were made with), the slots below 31, and the index buffer holds
                    // `index_count` `u32` indices of vertices of the vertex buffer.
                    unsafe {
                        encoder.setVertexBufferOffset_atIndex(index * DRAW_STRIDE, msl::DRAW_BUFFER_INDEX);
                        encoder.setFragmentBufferOffset_atIndex(index * DRAW_STRIDE, msl::DRAW_BUFFER_INDEX);
                        encoder.setFragmentBufferOffset_atIndex(index * BRUSH_STRIDE, msl::BRUSH_BUFFER_INDEX);
                        if let Some(texture) = texture {
                            encoder.setFragmentTexture_atIndex(Some(texture), msl::BRUSH_TEXTURE_INDEX);
                        }
                        encoder.setVertexBuffer_offset_atIndex(Some(vertices), 0, msl::VERTEX_BUFFER_INDEX);
                        encoder.drawIndexedPrimitives_indexCount_indexType_indexBuffer_indexBufferOffset(
                            MTLPrimitiveType::Triangle,
                            draw.index_count as usize,
                            MTLIndexType::UInt32,
                            indices,
                            0,
                        );
                    }
                }
            }
        }
        encoder.endEncoding();

        if additive {
            if let (Some(pipeline), Some(resolved)) = (&self.resolve_pipeline, &targets.resolved) {
                let pass = MTLRenderPassDescriptor::renderPassDescriptor();
                // SAFETY: attachment 0 exists in every render pass descriptor.
                let attachment = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
                attachment.setTexture(Some(output));
                attachment.setLoadAction(MTLLoadAction::Clear);
                attachment.setClearColor(MTLClearColor { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.0 });
                attachment.setStoreAction(MTLStoreAction::Store);
                let encoder = command_buffer
                    .renderCommandEncoderWithDescriptor(&pass)
                    .ok_or_else(|| Scene3DMetalError::new("no render command encoder"))?;
                encoder.setViewport(MTLViewport {
                    originX: 0.0,
                    originY: 0.0,
                    width: f64::from(width),
                    height: f64::from(height),
                    znear: 0.0,
                    zfar: 1.0,
                });
                encoder.setScissorRect(MTLScissorRect { x: 0, y: 0, width: width as usize, height: height as usize });
                encoder.setRenderPipelineState(pipeline);
                // SAFETY: the slot of the resolved colour, below 31; a triangle over the target.
                unsafe {
                    encoder.setFragmentTexture_atIndex(Some(resolved), msl::RESOLVED_TEXTURE_INDEX);
                    encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3);
                }
                encoder.endEncoding();
            }
        }
        command_buffer.commit();
        if let Some(index) = slot {
            viewport.slots[index].in_flight = Some(command_buffer.clone());
        }
        self.last_render = Some(command_buffer);
        self.uploads.clear();
        count(|counters| counters.passes += 1);
        Ok(())
    }

    /// The draws of the layers of one side of a model, in order, with their uniforms and the
    /// textures of their brushes (uploaded when their keys are new). The first diffuse layer
    /// writes depth (the first layer when the side has no diffuse layer).
    #[allow(clippy::too_many_arguments)]
    fn add_draws(
        &mut self,
        viewport: &mut Scene3DMetalViewport,
        model: &Scene3DModel,
        layers: &[Scene3DMaterialLayer],
        face: Face,
        (index_count, texture_bounds): (u32, Rect),
        textures: &dyn Scene3DMetalDeviceTextures,
        report: &mut Scene3DMetalRenderReport,
    ) -> Result<(), Scene3DMetalError> {
        let depth_layer =
            layers.iter().position(|layer| LayerKind::of(layer.kind) == LayerKind::Diffuse).unwrap_or(0);
        for (index, layer) in layers.iter().enumerate() {
            let (uniforms, _) = DrawUniforms::of(model, layer, face);
            let sampling = |bitmap: &SharedBitmapImpl| textures.sampling(bitmap);
            let mut brush = MaterialBrush::of_with(layer.brush.as_ref(), texture_bounds, &sampling);
            let mut texture = None;
            if let Some(source) = brush.texture().cloned() {
                if self.brush_texture(&source, textures, report) {
                    texture = Some(source.key());
                } else {
                    // Texels that cannot be read or uploaded: the layer is drawn with its colour.
                    brush = brush.to_unsampled();
                }
            } else if brush.shader() == BrushShader::Image {
                // An image brush without an image samples the empty texture (it paints nothing).
                self.empty_texture(report)?;
            }
            if brush.is_unsampled() {
                ferroui_render3d::report_layer_with_unsampled_brush();
            }
            let start = viewport.draw_scratch.len();
            uniforms.write_std140(&mut viewport.draw_scratch);
            viewport.draw_scratch.resize(start + DRAW_STRIDE, 0);
            let start = viewport.brush_scratch.len();
            brush.uniforms().write_std140(&mut viewport.brush_scratch);
            viewport.brush_scratch.resize(start + BRUSH_STRIDE, 0);
            viewport.draws.push(DrawRecord {
                mesh: (model.mesh.id, model.mesh.version),
                index_count,
                blend: if uniforms.kind() == LayerKind::Diffuse { Blend::SourceOver } else { Blend::Additive },
                face,
                depth_write: index == depth_layer,
                brush: brush.shader(),
                texture,
            });
        }
        Ok(())
    }

    /// A texture of the device: private, one mip level, `sample_count` samples.
    fn texture(
        &self,
        format: MTLPixelFormat,
        size: (u32, u32),
        sample_count: u32,
        usage: MTLTextureUsage,
    ) -> Option<Retained<Texture>> {
        // SAFETY: a plain description: a two-dimensional texture of a size of at least one pixel.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                format,
                size.0.max(1) as usize,
                size.1.max(1) as usize,
                false,
            )
        };
        if sample_count > 1 {
            descriptor.setTextureType(MTLTextureType::Type2DMultisample);
            // SAFETY: a sample count the device takes (`supportsTextureSampleCount`).
            unsafe { descriptor.setSampleCount(sample_count as usize) };
        }
        descriptor.setUsage(usage);
        descriptor.setStorageMode(MTLStorageMode::Private);
        let texture = self.device.newTextureWithDescriptor(&descriptor)?;
        count(|counters| counters.textures_created += 1);
        Some(texture)
    }

    /// A buffer of the device holding `bytes` (shared with the CPU), or of `length` zero bytes.
    fn buffer(&self, bytes: &[u8], length: usize) -> Option<Retained<Buffer>> {
        let length = length.max(bytes.len()).max(4);
        let buffer = self.device.newBufferWithLength_options(length, MTLResourceOptions::StorageModeShared)?;
        // SAFETY: the buffer is new (no command reads it) and holds at least `bytes.len()` bytes.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.contents().as_ptr().cast::<u8>(), bytes.len()) };
        Some(buffer)
    }

    /// The texture of `source` on the device, uploaded when its key is new: into the texture of
    /// an older version of the same bitmap that no draw of this render uses when it has the same
    /// size, otherwise into a new one. `false` when its texels cannot be read or are larger than
    /// the device takes (asked again only after [`LIFETIME`] renders).
    fn brush_texture(
        &mut self,
        source: &BrushTextureSource,
        textures: &dyn Scene3DMetalDeviceTextures,
        report: &mut Scene3DMetalRenderReport,
    ) -> bool {
        let key = source.key();
        let renders = self.renders;
        if let Some(texture) = self.textures.get_mut(&key) {
            texture.last_used = renders;
            return true;
        }
        if let Some(bitmap) = source.bitmap() {
            if textures.sampling(&**bitmap) != DeviceBitmap::NotOnDevice {
                if let Some(texture) = textures.device_texture(&**bitmap).and_then(|raw| self.given_texture(raw)) {
                    let size = (texture.width() as u32, texture.height() as u32);
                    let identity = source.identity();
                    let kept = BrushTexture { identity, texture, size, last_used: renders, on_device: true };
                    self.textures.insert(key, kept);
                    count(|counters| counters.textures_on_device += 1);
                    return true;
                }
            }
        }
        if let Some(last) = self.refused_textures.get_mut(&key) {
            *last = renders;
            return false;
        }
        let texels = source.texels().filter(|texels| {
            let side = 1..=MAX_TEXTURE_SIDE;
            side.contains(&texels.width) && side.contains(&texels.height)
        });
        let refuse = |executor: &mut Self| {
            executor.refused_textures.insert(key, renders);
            count(|counters| counters.textures_refused += 1);
            false
        };
        let Some(texels) = texels else { return refuse(self) };
        let size = (texels.width, texels.height);
        let replaced_key = match key {
            BrushTextureKey::Image { identity, .. } => self
                .textures
                .iter()
                .find(|(other, texture)| {
                    matches!(other, BrushTextureKey::Image { identity: other, .. } if *other == identity)
                        && texture.last_used != renders
                        && texture.size == size
                        && !texture.on_device
                })
                .map(|(other, _)| *other),
            _ => None,
        };
        let texture = match replaced_key.and_then(|other| self.textures.remove(&other)) {
            Some(texture) => texture.texture,
            None => match self.texture(MTLPixelFormat::RGBA8Unorm, size, 1, MTLTextureUsage::ShaderRead) {
                Some(texture) => {
                    report.resources_created += 1;
                    texture
                }
                None => return refuse(self),
            },
        };
        let Some(staging) = self.buffer(&texels.rgba, 0) else { return refuse(self) };
        self.uploads.push(Upload { staging, texture: texture.clone(), size });
        report.textures_uploaded += 1;
        count(|counters| {
            counters.textures_uploaded += 1;
            counters.texture_bytes += texels.rgba.len() as u64;
        });
        let identity = source.identity();
        self.textures.insert(key, BrushTexture { identity, texture, size, last_used: renders, on_device: false });
        true
    }

    /// A texture a renderer gave: a reference of the executor, when it can be sampled here (a
    /// texture of this device of 8-bit RGBA or BGRA, one sample, readable by shaders).
    fn given_texture(&self, raw: *mut c_void) -> Option<Retained<Texture>> {
        use objc2_metal::MTLResource;
        // SAFETY: a valid `id<MTLTexture>` (the contract of `Scene3DMetalDeviceTextures`).
        let texture = unsafe { Retained::retain(raw.cast::<Texture>()) }?;
        let format = texture.pixelFormat();
        let same_device = std::ptr::eq(
            Retained::as_ptr(&texture.device()).cast::<()>(),
            Retained::as_ptr(&self.device).cast::<()>(),
        );
        (same_device
            && (format == MTLPixelFormat::BGRA8Unorm || format == MTLPixelFormat::RGBA8Unorm)
            && texture.sampleCount() == 1
            && texture.usage().contains(MTLTextureUsage::ShaderRead))
        .then_some(texture)
    }

    /// The transparent texture of one texel an image brush without an image samples, made on
    /// first need.
    fn empty_texture(&mut self, report: &mut Scene3DMetalRenderReport) -> Result<(), Scene3DMetalError> {
        if self.empty_texture.is_some() {
            return Ok(());
        }
        let texture = self
            .texture(MTLPixelFormat::RGBA8Unorm, (1, 1), 1, MTLTextureUsage::ShaderRead)
            .ok_or_else(|| Scene3DMetalError::new("the empty texture cannot be made"))?;
        let staging = self.buffer(&[0; 4], 0).ok_or_else(|| Scene3DMetalError::new("no staging buffer"))?;
        self.uploads.push(Upload { staging, texture: texture.clone(), size: (1, 1) });
        report.resources_created += 1;
        self.empty_texture = Some(texture);
        Ok(())
    }

    /// The buffers of `mesh` on the device, uploaded when its `(id, version)` is new; returns
    /// the number of its indices (0 for a mesh without triangles) and the box of its texture
    /// coordinates (computed once per version). A new version replaces the buffers of the version
    /// before it.
    fn mesh(
        &mut self,
        mesh: &MeshData,
        report: &mut Scene3DMetalRenderReport,
    ) -> Result<(u32, Rect), Scene3DMetalError> {
        let key = (mesh.id, mesh.version);
        let renders = self.renders;
        if let Some(buffers) = self.meshes.get_mut(&key) {
            buffers.last_used = renders;
            return Ok((buffers.index_count, buffers.texture_bounds));
        }
        // The version this one replaces, unless a draw of this render uses it.
        if let Some(old) =
            self.meshes.iter().find(|(&(id, _), kept)| id == mesh.id && kept.last_used != renders).map(|(&key, _)| key)
        {
            self.meshes.remove(&old);
        }
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
            let vertices = self.buffer(&self.vertex_scratch, 0);
            let indices = self.buffer(&self.index_bytes, 0);
            let (Some(vertices), Some(indices)) = (vertices, indices) else {
                return Err(Scene3DMetalError::new("the buffers of a mesh cannot be made"));
            };
            report.resources_created += 2;
            report.meshes_uploaded += 1;
            let bytes = (self.vertex_scratch.len() + self.index_bytes.len()) as u64;
            count(|counters| {
                counters.buffers_created += 2;
                counters.mesh_uploads += 1;
                counters.mesh_bytes += bytes;
            });
            Some((vertices, indices))
        };
        let texture_bounds = texture_coordinate_bounds(mesh);
        self.meshes.insert(key, MeshBuffers { buffers, index_count, texture_bounds, last_used: renders });
        Ok((index_count, texture_bounds))
    }

    /// The function `name` of the library, looked up once.
    fn function(&mut self, name: &'static str) -> Result<Retained<Function>, Scene3DMetalError> {
        if let Some(function) = self.functions.get(name) {
            return Ok(function.clone());
        }
        let function = self
            .library
            .newFunctionWithName(&NSString::from_str(name))
            .ok_or_else(|| Scene3DMetalError::new(format!("the library has no function {name}")))?;
        self.functions.insert(name, function.clone());
        Ok(function)
    }

    /// The pipeline of `key`, made on first demand.
    fn pipeline(&mut self, key: PipelineKey, report: &mut Scene3DMetalRenderReport) -> Result<(), Scene3DMetalError> {
        if self.pipelines.contains_key(&key) {
            return Ok(());
        }
        let vertex = self.function(msl::VERTEX_FUNCTION)?;
        let fragment = self.function(msl::fragment_function(key.brush))?;
        let descriptor = MTLRenderPipelineDescriptor::new();
        descriptor.setLabel(Some(&NSString::from_str("FerroUI 3D scene")));
        descriptor.setVertexFunction(Some(&vertex));
        descriptor.setFragmentFunction(Some(&fragment));
        descriptor.setVertexDescriptor(Some(&self.vertex_descriptor));
        descriptor.setRasterSampleCount(key.sample_count as usize);
        descriptor.setDepthAttachmentPixelFormat(MTLPixelFormat::Depth32Float);
        // SAFETY: attachment 0 exists in every pipeline descriptor.
        let attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
        attachment.setPixelFormat(MTLPixelFormat::RGBA8Unorm);
        attachment.setBlendingEnabled(true);
        let destination = match key.blend {
            Blend::SourceOver => MTLBlendFactor::OneMinusSourceAlpha,
            Blend::Additive => MTLBlendFactor::One,
        };
        attachment.setSourceRGBBlendFactor(MTLBlendFactor::One);
        attachment.setSourceAlphaBlendFactor(MTLBlendFactor::One);
        attachment.setDestinationRGBBlendFactor(destination);
        attachment.setDestinationAlphaBlendFactor(destination);
        attachment.setRgbBlendOperation(MTLBlendOperation::Add);
        attachment.setAlphaBlendOperation(MTLBlendOperation::Add);
        attachment.setWriteMask(MTLColorWriteMask::All);
        let pipeline = self.device.newRenderPipelineStateWithDescriptor_error(&descriptor).map_err(|error| {
            Scene3DMetalError::new(format!("the pipeline {key:?} cannot be made: {}", error.localizedDescription()))
        })?;
        self.pipelines.insert(key, pipeline);
        report.pipelines_created += 1;
        count(|counters| counters.pipelines_created += 1);
        Ok(())
    }

    /// The pipeline of the resolve of decision N, made on first demand.
    fn resolve_pipeline(&mut self, report: &mut Scene3DMetalRenderReport) -> Result<(), Scene3DMetalError> {
        if self.resolve_pipeline.is_some() {
            return Ok(());
        }
        let vertex = self.function(msl::RESOLVE_VERTEX_FUNCTION)?;
        let fragment = self.function(msl::RESOLVE_FRAGMENT_FUNCTION)?;
        let descriptor = MTLRenderPipelineDescriptor::new();
        descriptor.setLabel(Some(&NSString::from_str("FerroUI 3D scene resolve")));
        descriptor.setVertexFunction(Some(&vertex));
        descriptor.setFragmentFunction(Some(&fragment));
        // SAFETY: attachment 0 exists in every pipeline descriptor.
        let attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
        attachment.setPixelFormat(MTLPixelFormat::RGBA8Unorm);
        let pipeline = self.device.newRenderPipelineStateWithDescriptor_error(&descriptor).map_err(|error| {
            Scene3DMetalError::new(format!("the resolve pipeline cannot be made: {}", error.localizedDescription()))
        })?;
        self.resolve_pipeline = Some(pipeline);
        report.pipelines_created += 1;
        count(|counters| counters.pipelines_created += 1);
        Ok(())
    }

    /// Writes into the next uniform slot of `viewport` the uniforms of the render that differ from
    /// what the slot holds, after the render that last read the slot has run; the buffers of all
    /// the slots are made (or grown) first when they are missing or too small. Returns the index of
    /// the slot.
    fn write_uniforms(
        &self,
        viewport: &mut Scene3DMetalViewport,
        frame: &FrameUniforms,
        report: &mut Scene3DMetalRenderReport,
    ) -> Result<usize, Scene3DMetalError> {
        if viewport.slots.is_empty() {
            viewport.slots.resize_with(UNIFORM_SLOTS, UniformSlot::default);
        }
        let index = viewport.next % viewport.slots.len();
        viewport.next = (index + 1) % viewport.slots.len();
        viewport.frame_scratch.clear();
        frame.write_std140(&mut viewport.frame_scratch);
        let draws = viewport.draws.len().max(1);

        // The buffers of every slot are made (or grown) together, by the render that first needs
        // them: the renders after it make none while the draws fit. A render still running keeps
        // the buffers it reads (a command buffer retains them).
        let frame_length = viewport.frame_scratch.len();
        let mut made = 0u32;
        for slot in &mut viewport.slots {
            if slot.frame.is_none() {
                slot.frame = Some(self.buffer(&[], frame_length).ok_or_else(no_buffer)?);
                slot.frame_bytes.clear();
                made += 1;
            }
            if slot.draws.is_none() || slot.brushes.is_none() || slot.capacity < draws {
                let capacity = draws.next_power_of_two();
                slot.draws = Some(self.buffer(&[], capacity * DRAW_STRIDE).ok_or_else(no_buffer)?);
                slot.brushes = Some(self.buffer(&[], capacity * BRUSH_STRIDE).ok_or_else(no_buffer)?);
                slot.capacity = capacity;
                slot.draw_bytes.clear();
                slot.brush_bytes.clear();
                made += 2;
            }
        }
        report.resources_created += made;
        count(|counters| counters.buffers_created += u64::from(made));
        let slot = &mut viewport.slots[index];

        // The GPU may still read the slot: its render is waited for (a frame or two behind).
        if let Some(in_flight) = slot.in_flight.take() {
            let status = in_flight.status();
            if status != MTLCommandBufferStatus::Completed && status != MTLCommandBufferStatus::Error {
                in_flight.waitUntilCompleted();
                count(|counters| counters.uniform_waits += 1);
            }
        }

        let mut writes = 0u64;
        let mut bytes = 0u64;
        if let Some(buffer) = &slot.frame {
            if slot.frame_bytes != viewport.frame_scratch {
                write_range(buffer, 0, &viewport.frame_scratch);
                slot.frame_bytes.clone_from(&viewport.frame_scratch);
                writes += 1;
                bytes += viewport.frame_scratch.len() as u64;
            }
        }
        // The ranges of the draws whose bytes changed (all of them in a new buffer).
        for (buffer, old, new, stride) in [
            (&slot.draws, &mut slot.draw_bytes, &viewport.draw_scratch, DRAW_STRIDE),
            (&slot.brushes, &mut slot.brush_bytes, &viewport.brush_scratch, BRUSH_STRIDE),
        ] {
            if let Some(buffer) = buffer {
                if let Some(written) = write_changed(buffer, old, new, stride) {
                    writes += 1;
                    bytes += written;
                }
            }
            old.clone_from(new);
        }

        report.uniform_bytes += bytes;
        count(|counters| {
            counters.uniform_writes += writes;
            counters.uniform_bytes += bytes;
        });
        Ok(index)
    }

    /// The index of the targets of the pool for `size` and `sample_count`, made on first need (the
    /// resolved colour when `resolved`).
    fn targets(
        &mut self,
        size: (u32, u32),
        sample_count: u32,
        resolved: bool,
        report: &mut Scene3DMetalRenderReport,
    ) -> Result<usize, Scene3DMetalError> {
        let renders = self.renders;
        let made = |texture: Option<Retained<Texture>>| {
            texture.ok_or_else(|| Scene3DMetalError::new("a target cannot be made"))
        };
        let index = match self.targets.iter().position(|t| t.size == size && t.sample_count == sample_count) {
            Some(index) => index,
            None => {
                let attachment =
                    |format| self.texture(format, size, sample_count, MTLTextureUsage::RenderTarget);
                let multisampled =
                    if sample_count > 1 { Some(made(attachment(MTLPixelFormat::RGBA8Unorm))?) } else { None };
                let depth = made(attachment(MTLPixelFormat::Depth32Float))?;
                report.resources_created += 1 + u32::from(multisampled.is_some());
                let targets = Targets { size, sample_count, multisampled, depth, resolved: None, last_used: renders };
                self.targets.push(targets);
                self.targets.len() - 1
            }
        };
        if resolved && self.targets[index].resolved.is_none() {
            let usage = MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead;
            let texture = made(self.texture(MTLPixelFormat::RGBA8Unorm, size, 1, usage))?;
            self.targets[index].resolved = Some(texture);
            report.resources_created += 1;
        }
        self.targets[index].last_used = renders;
        Ok(index)
    }
}

fn no_buffer() -> Scene3DMetalError {
    Scene3DMetalError::new("a uniform buffer cannot be made")
}

/// The pipeline key of a draw at `sample_count` samples.
fn draw_key(draw: &DrawRecord, sample_count: u32) -> PipelineKey {
    PipelineKey { blend: draw.blend, sample_count, brush: draw.brush }
}

/// Copies `bytes` into `buffer` at `offset`.
fn write_range(buffer: &Buffer, offset: usize, bytes: &[u8]) {
    assert!(offset + bytes.len() <= buffer.length(), "a write within the buffer");
    let Some(destination) = NonNull::new(buffer.contents().as_ptr().cast::<u8>()) else { return };
    // SAFETY: the range is within the buffer (asserted), which is shared with the CPU, and no
    // command that reads it is running (the slots of `write_uniforms`).
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination.as_ptr().add(offset), bytes.len()) };
}

/// Writes to `buffer` the one range of the records of `stride` bytes of `new` from the first to
/// the last that differ from `old`; returns the bytes written, `None` when nothing changed.
fn write_changed(buffer: &Buffer, old: &[u8], new: &[u8], stride: usize) -> Option<u64> {
    let changed = |index: usize| {
        let range = index * stride..(index + 1) * stride;
        old.get(range.clone()) != new.get(range)
    };
    let records = new.len() / stride.max(1);
    let first = (0..records).find(|&index| changed(index))?;
    let last = (first..records).rev().find(|&index| changed(index)).unwrap_or(first);
    let range = first * stride..(last + 1) * stride;
    write_range(buffer, range.start, &new[range.clone()]);
    Some(range.len() as u64)
}

#[cfg(test)]
#[path = "scene3d_tests.rs"]
mod tests;
