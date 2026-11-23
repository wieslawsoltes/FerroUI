//! The Vulkan executor of 3D scenes (#608, step D9 of `docs/porting/media3d.md`, section 3.21): the
//! one shader of `ferroui-render3d`, translated to SPIR-V at build time, drawn on the device and
//! queue of a Vulkan context into an image of that device.
//!
//! What it keeps, as the other executors do:
//!
//! - **Pipelines** are made once per device and key, on first demand.
//! - **Meshes** are uploaded once per `(id, version)`, **brush textures** once per key.
//! - **Uniforms** live in three slots per viewport used in turn, each written only where its bytes
//!   differ, so the CPU never writes a buffer a submitted render reads.
//! - **Targets** (the multisampled colour, the depth and the resolved colour) are pooled by size
//!   and sample count.
//!
//! The clip space is the shader's: a render draws with a viewport of negative height (Vulkan 1.1),
//! so the first row of an output is the top of the scene.
//!
//! **Order.** A render is one command buffer submitted to the main queue of the device, with the
//! device locked. Its render passes end with the output in `SHADER_READ_ONLY_OPTIMAL` and a
//! dependency on everything submitted after, so a renderer that samples the output in a later
//! submission reads what was drawn.
//!
//! Errors: a failure of Vulkan is a [`Scene3DVkError`] the caller reports once before it draws the
//! CPU reference. A failure with four samples is reported once, and the executor draws with one
//! from then on.

use crate::IVulkanDevice;
use ash::vk::{self, Handle};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::SharedBitmapImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::scene3d::{MeshData, Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot};
use ferroui_base::Rect;
use ferroui_render3d::brush::DeviceBitmap;
use ferroui_render3d::brush::{
    texture_coordinate_bounds, BrushShader, BrushTextureIdentity, BrushTextureKey, BrushTextureSource,
    MaterialBrush, BRUSH_UNIFORMS_STRIDE,
};
use ferroui_render3d::shader::{samples_texture, spirv, write_mesh, VERTEX_ATTRIBUTE_OFFSETS, VERTEX_STRIDE};
use ferroui_render3d::uniforms::{DrawUniforms, Face, FrameUniforms, LayerKind, Std140, DRAW_UNIFORMS_STRIDE};
use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::{c_void, CStr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The samples of an antialiased render (decision F).
pub const MULTISAMPLE_COUNT: u32 = 4;

/// The slots of uniform buffers of a viewport, used in turn.
pub const UNIFORM_SLOTS: usize = 3;

/// The command buffers a device has in flight.
const FRAMES_IN_FLIGHT: usize = 4;

/// The renders a mesh, a texture or a target of the pool is kept for without being drawn.
const LIFETIME: u64 = 240;

/// The format of the colour: premultiplied RGBA8.
const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;

/// The descriptor sets of the pool of a device.
const MAX_DESCRIPTOR_SETS: u32 = 1024;

/// The bytes between the draw uniforms of two draws: at least the largest alignment of a uniform
/// buffer offset Vulkan allows.
const DRAW_STRIDE: usize = DRAW_UNIFORMS_STRIDE;
/// The bytes between the brush uniforms of two draws.
const BRUSH_STRIDE: usize = BRUSH_UNIFORMS_STRIDE;

/// What the Vulkan executors of this thread did since it started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scene3DVkCounters {
    /// Pipelines made.
    pub pipelines_created: u64,
    /// Meshes uploaded (a new `(id, version)`).
    pub mesh_uploads: u64,
    /// The bytes of the vertices and indices uploaded.
    pub mesh_bytes: u64,
    /// Writes of uniform buffers.
    pub uniform_writes: u64,
    /// The bytes of the uniforms written.
    pub uniform_bytes: u64,
    /// Renders of a scene submitted.
    pub passes: u64,
    /// Images made.
    pub textures_created: u64,
    /// Buffers made.
    pub buffers_created: u64,
    /// Textures of brushes written.
    pub textures_uploaded: u64,
    /// The bytes of those texels.
    pub texture_bytes: u64,
    /// Textures of brushes that could not be uploaded.
    pub textures_refused: u64,
    /// Textures of brushes the renderer already had on the device (a frame of a video).
    pub textures_on_device: u64,
}

impl Scene3DVkCounters {
    const ZERO: Scene3DVkCounters = Scene3DVkCounters {
        pipelines_created: 0,
        mesh_uploads: 0,
        mesh_bytes: 0,
        uniform_writes: 0,
        uniform_bytes: 0,
        passes: 0,
        textures_created: 0,
        buffers_created: 0,
        textures_uploaded: 0,
        texture_bytes: 0,
        textures_refused: 0,
        textures_on_device: 0,
    };
}

ferroui_base::ferro_thread_local! {
    static COUNTERS: Cell<Scene3DVkCounters> = const { Cell::new(Scene3DVkCounters::ZERO) };
}

/// What the Vulkan executors of this thread did since it started.
pub fn counters() -> Scene3DVkCounters {
    COUNTERS.with(Cell::get)
}

fn count(change: impl FnOnce(&mut Scene3DVkCounters)) {
    COUNTERS.with(|counters| {
        let mut value = counters.get();
        change(&mut value);
        counters.set(value);
    });
}

/// Why the executor cannot draw a scene.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene3DVkError {
    /// What failed.
    pub reason: String,
}

impl Scene3DVkError {
    fn new(reason: impl Into<String>) -> Scene3DVkError {
        Scene3DVkError { reason: reason.into() }
    }
}

impl std::fmt::Display for Scene3DVkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}

impl std::error::Error for Scene3DVkError {}

/// The error of a Vulkan call named `what`.
fn failed(what: &'static str) -> impl Fn(vk::Result) -> Scene3DVkError {
    move |result| Scene3DVkError::new(format!("{what} failed: {result:?}"))
}

/// An image with its memory and view.
struct ImageBundle {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
}

/// An output a scene is drawn into: RGBA8, premultiplied, its first row the top of the scene,
/// left in `SHADER_READ_ONLY_OPTIMAL` by a render.
pub struct Scene3DVkTexture {
    bundle: ImageBundle,
    width: u32,
    height: u32,
    allocation_size: u64,
}

impl Scene3DVkTexture {
    /// The `VkImage`.
    pub fn image(&self) -> vk::Image {
        self.bundle.image
    }

    /// The memory of the image.
    pub fn memory(&self) -> vk::DeviceMemory {
        self.bundle.memory
    }

    /// The size of the memory of the image, in bytes.
    pub fn allocation_size(&self) -> u64 {
        self.allocation_size
    }

    /// The format of the image.
    pub fn format(&self) -> vk::Format {
        COLOR_FORMAT
    }

    /// The usage the image was made with.
    pub fn usage(&self) -> vk::ImageUsageFlags {
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED
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

/// The image a render draws into.
pub struct Scene3DVkTarget<'a> {
    /// The output, made by [`Scene3DVkExecutor::create_output_texture`].
    pub texture: &'a Scene3DVkTexture,
    /// The size of the image of the scene, in pixels: at most the size of the texture.
    pub size: (u32, u32),
    /// Whether edges are antialiased (4x MSAA).
    pub antialiased: bool,
}

/// What one render did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scene3DVkRenderReport {
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
    /// The images and buffers it made.
    pub resources_created: u32,
    /// The bytes of uniforms it wrote.
    pub uniform_bytes: u64,
}

impl Scene3DVkRenderReport {
    /// Whether the render wrote uniforms and nothing else: the steady state of a moving camera.
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
    /// A specular or emissive layer: added.
    Additive,
}

/// What a pipeline of a draw is made for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PipelineKey {
    blend: Blend,
    face: Face,
    depth_write: bool,
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

/// A buffer of memory the CPU writes, mapped for its life.
#[derive(Clone, Copy)]
struct MappedBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    mapped: *mut u8,
    length: usize,
}

/// One slot of the uniform buffers of a viewport.
#[derive(Default)]
struct UniformSlot {
    frame: Option<MappedBuffer>,
    draws: Option<MappedBuffer>,
    brushes: Option<MappedBuffer>,
    frame_set: vk::DescriptorSet,
    draw_set: vk::DescriptorSet,
    /// The draws `draws` and `brushes` hold room for.
    capacity: usize,
    frame_bytes: Vec<u8>,
    draw_bytes: Vec<u8>,
    brush_bytes: Vec<u8>,
    /// The submission that last read the slot.
    submitted: u64,
}

/// The uniform buffers of a viewport and the draws of its last render. The caller keeps one with
/// each output it keeps.
#[derive(Default)]
pub struct Scene3DVkViewport {
    slots: Vec<UniformSlot>,
    next: usize,
    frame_scratch: Vec<u8>,
    draw_scratch: Vec<u8>,
    brush_scratch: Vec<u8>,
    draws: Vec<DrawRecord>,
}

impl Scene3DVkViewport {
    /// A viewport without buffers yet: they are made by its first render.
    pub fn new() -> Scene3DVkViewport {
        Scene3DVkViewport::default()
    }

    /// Destroys the buffers, after the GPU has done every render of `executor`.
    pub fn dispose(&mut self, executor: &mut Scene3DVkExecutor) {
        executor.wait_idle();
        for slot in self.slots.drain(..) {
            for buffer in [slot.frame, slot.draws, slot.brushes].into_iter().flatten() {
                executor.destroy_buffer(buffer);
            }
            let sets: Vec<vk::DescriptorSet> =
                [slot.frame_set, slot.draw_set].into_iter().filter(|set| *set != vk::DescriptorSet::null()).collect();
            if !sets.is_empty() {
                // SAFETY: the sets come from this pool, and the GPU has finished with them.
                let _ = unsafe { executor.api.free_descriptor_sets(executor.descriptor_pool, &sets) };
            }
        }
    }
}

/// A mesh on the device.
struct MeshBuffers {
    /// The vertex and index buffers; `None` for a mesh without triangles.
    buffers: Option<(MappedBuffer, MappedBuffer)>,
    index_count: u32,
    /// The box of its texture coordinates, which relative brushes are mapped onto.
    texture_bounds: Rect,
    last_used: u64,
}

/// The texture of a brush on the device, with its descriptor set.
struct BrushTexture {
    /// Keeps the address of a bitmap from naming another bitmap while the texture is kept.
    identity: BrushTextureIdentity,
    /// The image and its memory; `None` for a view of the renderer, which it owns.
    owned: Option<ImageBundle>,
    /// A view of the renderer's image, which this texture destroys.
    view: vk::ImageView,
    set: vk::DescriptorSet,
    size: (u32, u32),
    last_used: u64,
}

/// The attachments of the pool for one size and sample count.
struct Targets {
    size: (u32, u32),
    sample_count: u32,
    /// The multisampled colour (four samples only).
    multisampled: Option<ImageBundle>,
    depth: ImageBundle,
    /// The resolved colour `fs_resolve` reads, with its descriptor set; made on first need.
    resolved: Option<(ImageBundle, vk::DescriptorSet)>,
    last_used: u64,
}

/// A command buffer in flight.
struct InFlight {
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
    /// The submission of this command buffer; 0 for none.
    serial: u64,
    /// Whether the GPU may still be running it.
    pending: bool,
    /// The staging buffers its uploads read.
    staging: Vec<MappedBuffer>,
}

/// What no later render reads, destroyed once the GPU has done the submissions that may.
enum Retired {
    Buffer(MappedBuffer),
    Image(ImageBundle),
    Texture(BrushTexture),
}

static SINGLE_SAMPLE_REPORTED: AtomicBool = AtomicBool::new(false);

fn report_single_sample(reason: &str) {
    if SINGLE_SAMPLE_REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
        logger.log_with_values(
            None,
            "3D scenes (Viewport3D) are drawn without 4x MSAA on this Vulkan device: {Reason} \
             (docs/porting/media3d.md, section 3.21).",
            &[&reason],
        );
    }
}

/// The bitmaps a renderer has as images of the device, whose pixels cannot be read: the frames of
/// a video or a camera (step D7).
///
/// # Safety
/// [`device_image`](Self::device_image) gives an image of this device with a sampled usage, in
/// `SHADER_READ_ONLY_OPTIMAL`, which the renderer keeps until the GPU has done the render.
pub unsafe trait Scene3DVkDeviceTextures {
    /// How `bitmap` is on this device. Asked for every layer that has such a bitmap.
    fn sampling(&self, bitmap: &SharedBitmapImpl) -> DeviceBitmap;

    /// The image of `bitmap`, its format and size, asked once for each bitmap the executor does
    /// not hold.
    fn device_image(&self, bitmap: &SharedBitmapImpl) -> Option<(vk::Image, vk::Format, (u32, u32))>;
}

/// A renderer without such bitmaps.
struct NoDeviceTextures;

// SAFETY: it gives no image.
unsafe impl Scene3DVkDeviceTextures for NoDeviceTextures {
    fn sampling(&self, _: &SharedBitmapImpl) -> DeviceBitmap {
        DeviceBitmap::NotOnDevice
    }

    fn device_image(&self, _: &SharedBitmapImpl) -> Option<(vk::Image, vk::Format, (u32, u32))> {
        None
    }
}

/// The Vulkan executor of one device: see the module.
pub struct Scene3DVkExecutor {
    device: Arc<dyn IVulkanDevice>,
    api: ash::Device,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    depth_format: vk::Format,
    multisample: bool,
    max_side: u32,
    command_pool: vk::CommandPool,
    in_flight: Vec<InFlight>,
    next_in_flight: usize,
    serial: u64,
    descriptor_pool: vk::DescriptorPool,
    frame_layout: vk::DescriptorSetLayout,
    draw_layout: vk::DescriptorSetLayout,
    texture_layout: vk::DescriptorSetLayout,
    resolve_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    textured_pipeline_layout: vk::PipelineLayout,
    resolve_pipeline_layout: vk::PipelineLayout,
    sampler: vk::Sampler,
    vertex_module: vk::ShaderModule,
    fragment_modules: HashMap<BrushShader, vk::ShaderModule>,
    resolve_modules: Option<(vk::ShaderModule, vk::ShaderModule)>,
    /// The render passes of the draws, by sample count (1 and 4).
    single_pass: vk::RenderPass,
    multisampled_pass: vk::RenderPass,
    resolve_pass: vk::RenderPass,
    pipelines: HashMap<PipelineKey, vk::Pipeline>,
    resolve_pipeline: Option<vk::Pipeline>,
    framebuffers: HashMap<(u64, u64, u64), vk::Framebuffer>,
    meshes: HashMap<(u64, u64), MeshBuffers>,
    textures: HashMap<BrushTextureKey, BrushTexture>,
    refused_textures: HashMap<BrushTextureKey, u64>,
    empty_texture: Option<(ImageBundle, vk::DescriptorSet)>,
    targets: Vec<Targets>,
    /// What goes once the submission of its serial is done.
    retired: Vec<(u64, Retired)>,
    renders: u64,
    vertex_scratch: Vec<u8>,
    index_scratch: Vec<u32>,
    index_bytes: Vec<u8>,
}

/// The words of a SPIR-V module of `bytes`.
fn spirv_words(bytes: &[u8]) -> Vec<u32> {
    bytes.chunks_exact(4).map(|word| u32::from_ne_bytes([word[0], word[1], word[2], word[3]])).collect()
}

/// A render pass of the draws: into a colour and a depth of `samples`, resolved into a further
/// attachment with four samples, leaving what is read in `SHADER_READ_ONLY_OPTIMAL`.
fn create_draw_pass(
    api: &ash::Device,
    depth_format: vk::Format,
    samples: vk::SampleCountFlags,
) -> Result<vk::RenderPass, Scene3DVkError> {
    let multisampled = samples != vk::SampleCountFlags::TYPE_1;
    let color = vk::AttachmentDescription::default()
        .format(COLOR_FORMAT)
        .samples(samples)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(if multisampled { vk::AttachmentStoreOp::DONT_CARE } else { vk::AttachmentStoreOp::STORE })
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(if multisampled {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        } else {
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        });
    let depth = vk::AttachmentDescription::default()
        .format(depth_format)
        .samples(samples)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    let resolve = vk::AttachmentDescription::default()
        .format(COLOR_FORMAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::DONT_CARE)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    let color_reference =
        [vk::AttachmentReference::default().attachment(0).layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
    let depth_reference =
        vk::AttachmentReference::default().attachment(1).layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    let resolve_reference =
        [vk::AttachmentReference::default().attachment(2).layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
    let mut subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_reference)
        .depth_stencil_attachment(&depth_reference);
    if multisampled {
        subpass = subpass.resolve_attachments(&resolve_reference);
    }
    let dependencies = pass_dependencies();
    let single = [color, depth];
    let triple = [color, depth, resolve];
    let attachments: &[vk::AttachmentDescription] = if multisampled { &triple } else { &single };
    let subpasses = [subpass];
    let info = vk::RenderPassCreateInfo::default()
        .attachments(attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);
    // SAFETY: the description and what it points to live for the call.
    unsafe { api.create_render_pass(&info, None) }.map_err(failed("vkCreateRenderPass"))
}

/// The order of a pass against what was submitted before it and what is submitted after it.
fn pass_dependencies() -> [vk::SubpassDependency; 2] {
    [
        vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::ALL_COMMANDS)
            .dst_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                    | vk::PipelineStageFlags::FRAGMENT_SHADER,
            )
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
                    | vk::AccessFlags::SHADER_READ,
            ),
        vk::SubpassDependency::default()
            .src_subpass(0)
            .dst_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::ALL_COMMANDS)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::MEMORY_READ),
    ]
}

/// The pass of the resolve of decision N: one colour, left to be sampled.
fn create_resolve_pass(api: &ash::Device) -> Result<vk::RenderPass, Scene3DVkError> {
    let color = [vk::AttachmentDescription::default()
        .format(COLOR_FORMAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let reference =
        [vk::AttachmentReference::default().attachment(0).layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
    let subpasses = [vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&reference)];
    let dependencies = pass_dependencies();
    let info =
        vk::RenderPassCreateInfo::default().attachments(&color).subpasses(&subpasses).dependencies(&dependencies);
    // SAFETY: the description and what it points to live for the call.
    unsafe { api.create_render_pass(&info, None) }.map_err(failed("vkCreateRenderPass"))
}

impl Scene3DVkExecutor {
    /// The executor of `device`. An error for a device below Vulkan 1.1 or without a depth format.
    pub fn new(device: Arc<dyn IVulkanDevice>) -> Result<Scene3DVkExecutor, Scene3DVkError> {
        let instance = device.instance();
        let instance_handle = instance.handle();
        let device_handle = device.handle();
        let physical_device = vk::PhysicalDevice::from_raw(device.physical_device_handle() as u64);
        let instance_proc = |name: &CStr| -> *const c_void {
            name.to_str().map_or(std::ptr::null(), |name| {
                instance.get_instance_proc_address(instance_handle, name) as *const c_void
            })
        };
        // SAFETY: the addresses of the entry points of those names, with their signatures.
        let (get_properties, get_memory_properties, get_format_properties) = unsafe {
            let properties = instance_proc(c"vkGetPhysicalDeviceProperties");
            let memory = instance_proc(c"vkGetPhysicalDeviceMemoryProperties");
            let format = instance_proc(c"vkGetPhysicalDeviceFormatProperties");
            if properties.is_null() || memory.is_null() || format.is_null() {
                return Err(Scene3DVkError::new("the instance lacks an entry point of the 3D executor"));
            }
            (
                std::mem::transmute::<*const c_void, vk::PFN_vkGetPhysicalDeviceProperties>(properties),
                std::mem::transmute::<*const c_void, vk::PFN_vkGetPhysicalDeviceMemoryProperties>(memory),
                std::mem::transmute::<*const c_void, vk::PFN_vkGetPhysicalDeviceFormatProperties>(format),
            )
        };
        let mut properties = vk::PhysicalDeviceProperties::default();
        let mut memory_properties = vk::PhysicalDeviceMemoryProperties::default();
        // SAFETY: valid handles, and structures the calls write.
        unsafe {
            get_properties(physical_device, &mut properties);
            get_memory_properties(physical_device, &mut memory_properties);
        }
        if vk::api_version_major(properties.api_version) == 1 && vk::api_version_minor(properties.api_version) < 1 {
            return Err(Scene3DVkError::new("the 3D executor needs Vulkan 1.1 (a viewport of negative height)"));
        }
        let limits = properties.limits;
        if limits.min_uniform_buffer_offset_alignment > DRAW_STRIDE.min(BRUSH_STRIDE) as u64 {
            return Err(Scene3DVkError::new("the alignment of uniform buffers is above the stride of the draws"));
        }
        let depth_format = [vk::Format::D32_SFLOAT, vk::Format::X8_D24_UNORM_PACK32, vk::Format::D16_UNORM]
            .into_iter()
            .find(|format| {
                let mut value = vk::FormatProperties::default();
                // SAFETY: a valid handle, and a structure the call writes.
                unsafe { get_format_properties(physical_device, *format, &mut value) };
                value.optimal_tiling_features.contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
            })
            .ok_or_else(|| Scene3DVkError::new("the device has no depth format"))?;
        let four = vk::SampleCountFlags::TYPE_4;
        let multisample = limits.framebuffer_color_sample_counts.contains(four)
            && limits.framebuffer_depth_sample_counts.contains(four);

        // SAFETY: the entry points of the device, resolved by name from its instance.
        let api = unsafe {
            ash::Device::load_with(
                |name| {
                    name.to_str().map_or(std::ptr::null(), |name| {
                        instance.get_device_proc_address(device_handle, name) as *const c_void
                    })
                },
                vk::Device::from_raw(device_handle as u64),
            )
        };

        let lock = device.lock();
        let built = Self::build(device.clone(), api, memory_properties, depth_format, multisample, limits);
        lock.dispose();
        built
    }

    fn build(
        device: Arc<dyn IVulkanDevice>,
        api: ash::Device,
        memory_properties: vk::PhysicalDeviceMemoryProperties,
        depth_format: vk::Format,
        multisample: bool,
        limits: vk::PhysicalDeviceLimits,
    ) -> Result<Scene3DVkExecutor, Scene3DVkError> {
        // SAFETY: every description and what it points to lives for its call, on a valid device.
        unsafe {
            let pool_info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(device.graphics_queue_family_index());
            let command_pool = api.create_command_pool(&pool_info, None).map_err(failed("vkCreateCommandPool"))?;
            let allocate = vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(FRAMES_IN_FLIGHT as u32);
            let command_buffers =
                api.allocate_command_buffers(&allocate).map_err(failed("vkAllocateCommandBuffers"))?;
            let mut in_flight = Vec::with_capacity(FRAMES_IN_FLIGHT);
            for command_buffer in command_buffers {
                let info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
                let fence = api.create_fence(&info, None).map_err(failed("vkCreateFence"))?;
                in_flight.push(InFlight { command_buffer, fence, serial: 0, pending: false, staging: Vec::new() });
            }

            let sizes = [
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(MAX_DESCRIPTOR_SETS),
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                    .descriptor_count(MAX_DESCRIPTOR_SETS),
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(MAX_DESCRIPTOR_SETS),
                vk::DescriptorPoolSize::default().ty(vk::DescriptorType::SAMPLER).descriptor_count(MAX_DESCRIPTOR_SETS),
            ];
            let info = vk::DescriptorPoolCreateInfo::default()
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .max_sets(MAX_DESCRIPTOR_SETS)
                .pool_sizes(&sizes);
            let descriptor_pool = api.create_descriptor_pool(&info, None).map_err(failed("vkCreateDescriptorPool"))?;

            let stages = vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT;
            let set_layout = |bindings: &[vk::DescriptorSetLayoutBinding<'_>]| {
                let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings);
                api.create_descriptor_set_layout(&info, None).map_err(failed("vkCreateDescriptorSetLayout"))
            };
            let binding = |index: u32, ty: vk::DescriptorType, stages: vk::ShaderStageFlags| {
                vk::DescriptorSetLayoutBinding::default()
                    .binding(index)
                    .descriptor_type(ty)
                    .descriptor_count(1)
                    .stage_flags(stages)
            };
            let frame_layout = set_layout(&[binding(0, vk::DescriptorType::UNIFORM_BUFFER, stages)])?;
            let draw_layout = set_layout(&[
                binding(0, vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, stages),
                binding(1, vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, stages),
            ])?;
            let texture_layout = set_layout(&[
                binding(0, vk::DescriptorType::SAMPLED_IMAGE, vk::ShaderStageFlags::FRAGMENT),
                binding(1, vk::DescriptorType::SAMPLER, vk::ShaderStageFlags::FRAGMENT),
            ])?;
            // The resolved colour is `@group(0) @binding(1)`.
            let resolve_set_layout =
                set_layout(&[binding(1, vk::DescriptorType::SAMPLED_IMAGE, vk::ShaderStageFlags::FRAGMENT)])?;
            let pipeline_layout_of = |layouts: &[vk::DescriptorSetLayout]| {
                let info = vk::PipelineLayoutCreateInfo::default().set_layouts(layouts);
                api.create_pipeline_layout(&info, None).map_err(failed("vkCreatePipelineLayout"))
            };
            let pipeline_layout = pipeline_layout_of(&[frame_layout, draw_layout])?;
            let textured_pipeline_layout = pipeline_layout_of(&[frame_layout, draw_layout, texture_layout])?;
            let resolve_pipeline_layout = pipeline_layout_of(&[resolve_set_layout])?;

            let info = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .max_lod(0.0);
            let sampler = api.create_sampler(&info, None).map_err(failed("vkCreateSampler"))?;
            let words = spirv_words(spirv::VERTEX);
            let info = vk::ShaderModuleCreateInfo::default().code(&words);
            let vertex_module = api.create_shader_module(&info, None).map_err(failed("vkCreateShaderModule"))?;
            let single_pass = create_draw_pass(&api, depth_format, vk::SampleCountFlags::TYPE_1)?;
            let multisampled_pass = create_draw_pass(&api, depth_format, vk::SampleCountFlags::TYPE_4)?;
            let resolve_pass = create_resolve_pass(&api)?;

            Ok(Scene3DVkExecutor {
                device,
                api,
                memory_properties,
                depth_format,
                multisample,
                max_side: limits.max_image_dimension2_d,
                command_pool,
                in_flight,
                next_in_flight: 0,
                serial: 0,
                descriptor_pool,
                frame_layout,
                draw_layout,
                texture_layout,
                resolve_set_layout,
                pipeline_layout,
                textured_pipeline_layout,
                resolve_pipeline_layout,
                sampler,
                vertex_module,
                fragment_modules: HashMap::new(),
                resolve_modules: None,
                single_pass,
                multisampled_pass,
                resolve_pass,
                pipelines: HashMap::new(),
                resolve_pipeline: None,
                framebuffers: HashMap::new(),
                meshes: HashMap::new(),
                textures: HashMap::new(),
                refused_textures: HashMap::new(),
                empty_texture: None,
                targets: Vec::new(),
                retired: Vec::new(),
                renders: 0,
                vertex_scratch: Vec::new(),
                index_scratch: Vec::new(),
                index_bytes: Vec::new(),
            })
        }
    }

    /// The textures of brushes the executor keeps.
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Whether antialiased renders take four samples on this device.
    pub fn multisamples(&self) -> bool {
        self.multisample
    }

    /// Waits for the GPU to finish every render of this executor.
    pub fn wait_idle(&mut self) {
        let serial = self.serial;
        self.wait_for(serial);
    }

    /// Waits for the submissions up to `serial`, and gives back their staging buffers.
    fn wait_for(&mut self, serial: u64) {
        for index in 0..self.in_flight.len() {
            if !self.in_flight[index].pending || self.in_flight[index].serial > serial {
                continue;
            }
            let fence = self.in_flight[index].fence;
            // SAFETY: a fence of this device, submitted with its command buffer.
            let _ = unsafe { self.api.wait_for_fences(&[fence], true, u64::MAX) };
            self.complete(index);
        }
    }

    /// The GPU has done the command buffer `index`: its staging buffers go.
    fn complete(&mut self, index: usize) {
        self.in_flight[index].pending = false;
        let staging = std::mem::take(&mut self.in_flight[index].staging);
        for buffer in staging {
            self.destroy_buffer(buffer);
        }
    }

    /// Waits for everything on the queue, the draws of Skia that sample an output included.
    fn wait_queue_idle(&mut self) {
        let lock = self.device.lock();
        let queue = vk::Queue::from_raw(self.device.main_queue_handle() as u64);
        // SAFETY: the queue of this device, which is locked.
        let _ = unsafe { self.api.queue_wait_idle(queue) };
        lock.dispose();
        self.wait_idle();
    }

    /// Keeps `resource` until the GPU has done the submission being recorded and those before it.
    fn retire(&mut self, resource: Retired) {
        self.retired.push((self.serial + 1, resource));
    }

    /// Destroys what was retired and no submission in flight may read. Called between renders.
    fn reclaim(&mut self) {
        for index in 0..self.in_flight.len() {
            if !self.in_flight[index].pending {
                continue;
            }
            let fence = self.in_flight[index].fence;
            // SAFETY: a fence of this device.
            if !matches!(unsafe { self.api.get_fence_status(fence) }, Ok(false)) {
                self.complete(index);
            }
        }
        let oldest =
            self.in_flight.iter().filter(|frame| frame.pending).map(|frame| frame.serial).min().unwrap_or(u64::MAX);
        for (serial, resource) in std::mem::take(&mut self.retired) {
            if serial >= oldest {
                self.retired.push((serial, resource));
                continue;
            }
            match resource {
                Retired::Buffer(buffer) => self.destroy_buffer(buffer),
                Retired::Image(bundle) => self.destroy_image(bundle),
                Retired::Texture(texture) => self.destroy_brush_texture(texture),
            }
        }
    }

    /// Destroys everything the executor made, after the GPU has finished with it. The outputs and
    /// viewports of the caller are destroyed first.
    pub fn dispose(&mut self) {
        let lock = self.device.lock();
        self.wait_idle();
        // What a failed submission left, then everything retired.
        for index in 0..self.in_flight.len() {
            self.complete(index);
        }
        self.reclaim();
        let meshes: Vec<MeshBuffers> = self.meshes.drain().map(|(_, mesh)| mesh).collect();
        for (vertices, indices) in meshes.into_iter().filter_map(|mesh| mesh.buffers) {
            self.destroy_buffer(vertices);
            self.destroy_buffer(indices);
        }
        let textures: Vec<BrushTexture> = self.textures.drain().map(|(_, texture)| texture).collect();
        for texture in textures {
            self.destroy_brush_texture(texture);
        }
        if let Some((bundle, _)) = self.empty_texture.take() {
            self.destroy_image(bundle);
        }
        let targets: Vec<Targets> = self.targets.drain(..).collect();
        for targets in targets {
            self.destroy_targets(targets);
        }
        // SAFETY: the GPU has finished, and nothing below is used again.
        unsafe {
            for (_, framebuffer) in self.framebuffers.drain() {
                self.api.destroy_framebuffer(framebuffer, None);
            }
            for (_, pipeline) in self.pipelines.drain() {
                self.api.destroy_pipeline(pipeline, None);
            }
            if let Some(pipeline) = self.resolve_pipeline.take() {
                self.api.destroy_pipeline(pipeline, None);
            }
            for (_, module) in self.fragment_modules.drain() {
                self.api.destroy_shader_module(module, None);
            }
            if let Some((vertex, fragment)) = self.resolve_modules.take() {
                self.api.destroy_shader_module(vertex, None);
                self.api.destroy_shader_module(fragment, None);
            }
            self.api.destroy_shader_module(self.vertex_module, None);
            self.api.destroy_render_pass(self.single_pass, None);
            self.api.destroy_render_pass(self.multisampled_pass, None);
            self.api.destroy_render_pass(self.resolve_pass, None);
            self.api.destroy_sampler(self.sampler, None);
            self.api.destroy_pipeline_layout(self.pipeline_layout, None);
            self.api.destroy_pipeline_layout(self.textured_pipeline_layout, None);
            self.api.destroy_pipeline_layout(self.resolve_pipeline_layout, None);
            self.api.destroy_descriptor_set_layout(self.frame_layout, None);
            self.api.destroy_descriptor_set_layout(self.draw_layout, None);
            self.api.destroy_descriptor_set_layout(self.texture_layout, None);
            self.api.destroy_descriptor_set_layout(self.resolve_set_layout, None);
            self.api.destroy_descriptor_pool(self.descriptor_pool, None);
            for frame in self.in_flight.drain(..) {
                self.api.destroy_fence(frame.fence, None);
            }
            self.api.destroy_command_pool(self.command_pool, None);
        }
        lock.dispose();
    }

    fn memory_type(&self, bits: u32, wanted: vk::MemoryPropertyFlags) -> Option<u32> {
        (0..self.memory_properties.memory_type_count).find(|&index| {
            bits & (1 << index) != 0
                && self.memory_properties.memory_types[index as usize].property_flags.contains(wanted)
        })
    }

    /// A buffer of `length` bytes the CPU writes, mapped.
    fn create_buffer(&self, length: usize, usage: vk::BufferUsageFlags) -> Result<MappedBuffer, Scene3DVkError> {
        let length = length.max(4);
        let info = vk::BufferCreateInfo::default()
            .size(length as u64)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        // SAFETY: the descriptions live for their calls; the memory is bound once and mapped whole.
        unsafe {
            let buffer = self.api.create_buffer(&info, None).map_err(failed("vkCreateBuffer"))?;
            let requirements = self.api.get_buffer_memory_requirements(buffer);
            let wanted = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
            let Some(memory_type) = self.memory_type(requirements.memory_type_bits, wanted) else {
                self.api.destroy_buffer(buffer, None);
                return Err(Scene3DVkError::new("the device has no memory the CPU can write"));
            };
            let allocate =
                vk::MemoryAllocateInfo::default().allocation_size(requirements.size).memory_type_index(memory_type);
            let memory = match self.api.allocate_memory(&allocate, None) {
                Ok(memory) => memory,
                Err(error) => {
                    self.api.destroy_buffer(buffer, None);
                    return Err(failed("vkAllocateMemory")(error));
                }
            };
            let mapped = self
                .api
                .bind_buffer_memory(buffer, memory, 0)
                .and_then(|()| self.api.map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()));
            match mapped {
                Ok(mapped) => {
                    count(|counters| counters.buffers_created += 1);
                    Ok(MappedBuffer { buffer, memory, mapped: mapped.cast(), length })
                }
                Err(error) => {
                    self.api.destroy_buffer(buffer, None);
                    self.api.free_memory(memory, None);
                    Err(failed("vkMapMemory")(error))
                }
            }
        }
    }

    fn destroy_buffer(&self, buffer: MappedBuffer) {
        // SAFETY: the callers wait for the GPU, or the buffer has not been drawn for `LIFETIME`
        // renders, more than the submissions in flight.
        unsafe {
            self.api.destroy_buffer(buffer.buffer, None);
            self.api.free_memory(buffer.memory, None);
        }
    }

    /// An image of the device with its memory and a view.
    fn create_image(
        &self,
        size: (u32, u32),
        format: vk::Format,
        samples: vk::SampleCountFlags,
        usage: vk::ImageUsageFlags,
        aspect: vk::ImageAspectFlags,
    ) -> Result<(ImageBundle, u64), Scene3DVkError> {
        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D { width: size.0, height: size.1, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(samples)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        // SAFETY: the descriptions live for their calls; the memory is bound once.
        unsafe {
            let image = self.api.create_image(&info, None).map_err(failed("vkCreateImage"))?;
            let requirements = self.api.get_image_memory_requirements(image);
            let memory_type = self
                .memory_type(requirements.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)
                .or_else(|| self.memory_type(requirements.memory_type_bits, vk::MemoryPropertyFlags::empty()));
            let Some(memory_type) = memory_type else {
                self.api.destroy_image(image, None);
                return Err(Scene3DVkError::new("the device has no memory for an image"));
            };
            let allocate =
                vk::MemoryAllocateInfo::default().allocation_size(requirements.size).memory_type_index(memory_type);
            let memory = match self.api.allocate_memory(&allocate, None) {
                Ok(memory) => memory,
                Err(error) => {
                    self.api.destroy_image(image, None);
                    return Err(failed("vkAllocateMemory")(error));
                }
            };
            if let Err(error) = self.api.bind_image_memory(image, memory, 0) {
                self.api.destroy_image(image, None);
                self.api.free_memory(memory, None);
                return Err(failed("vkBindImageMemory")(error));
            }
            let view = match self.create_view(image, format, aspect) {
                Ok(view) => view,
                Err(error) => {
                    self.api.destroy_image(image, None);
                    self.api.free_memory(memory, None);
                    return Err(error);
                }
            };
            count(|counters| counters.textures_created += 1);
            Ok((ImageBundle { image, memory, view }, requirements.size))
        }
    }

    fn create_view(
        &self,
        image: vk::Image,
        format: vk::Format,
        aspect: vk::ImageAspectFlags,
    ) -> Result<vk::ImageView, Scene3DVkError> {
        let info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::default().aspect_mask(aspect).level_count(1).layer_count(1),
            );
        // SAFETY: the description lives for the call, on an image of this device.
        unsafe { self.api.create_image_view(&info, None) }.map_err(failed("vkCreateImageView"))
    }

    fn destroy_image(&self, bundle: ImageBundle) {
        // SAFETY: as for `destroy_buffer`.
        unsafe {
            self.api.destroy_image_view(bundle.view, None);
            self.api.destroy_image(bundle.image, None);
            self.api.free_memory(bundle.memory, None);
        }
    }

    fn destroy_brush_texture(&self, texture: BrushTexture) {
        // SAFETY: as for `destroy_buffer`; the set comes from this pool.
        unsafe {
            let _ = self.api.free_descriptor_sets(self.descriptor_pool, &[texture.set]);
            match texture.owned {
                Some(bundle) => self.destroy_image(bundle),
                None => self.api.destroy_image_view(texture.view, None),
            }
        }
    }

    fn destroy_targets(&mut self, targets: Targets) {
        let mut views = vec![targets.depth.view.as_raw()];
        if let Some(bundle) = &targets.multisampled {
            views.push(bundle.view.as_raw());
        }
        if let Some((bundle, _)) = &targets.resolved {
            views.push(bundle.view.as_raw());
        }
        self.drop_framebuffers(&views);
        if let Some(bundle) = targets.multisampled {
            self.destroy_image(bundle);
        }
        self.destroy_image(targets.depth);
        if let Some((bundle, set)) = targets.resolved {
            // SAFETY: the set comes from this pool, and no submission in flight reads it.
            let _ = unsafe { self.api.free_descriptor_sets(self.descriptor_pool, &[set]) };
            self.destroy_image(bundle);
        }
    }

    /// Destroys the framebuffers that hold one of `views`.
    fn drop_framebuffers(&mut self, views: &[u64]) {
        let api = &self.api;
        self.framebuffers.retain(|key, framebuffer| {
            let keep = ![key.0, key.1, key.2].iter().any(|view| views.contains(view));
            if !keep {
                // SAFETY: the callers wait for the GPU, or nothing drew it for `LIFETIME` renders.
                unsafe { api.destroy_framebuffer(*framebuffer, None) };
            }
            keep
        });
    }

    /// An output image of `width` by `height` pixels. `None` when the device cannot make it.
    pub fn create_output_texture(&self, width: u32, height: u32) -> Option<Scene3DVkTexture> {
        if width == 0 || height == 0 || width > self.max_side || height > self.max_side {
            return None;
        }
        let usage = vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED;
        let (bundle, allocation_size) = self
            .create_image(
                (width, height),
                COLOR_FORMAT,
                vk::SampleCountFlags::TYPE_1,
                usage,
                vk::ImageAspectFlags::COLOR,
            )
            .ok()?;
        Some(Scene3DVkTexture { bundle, width, height, allocation_size })
    }

    /// Destroys an output, after the queue has finished with it: what Skia submitted may sample it.
    pub fn delete_output_texture(&mut self, texture: Scene3DVkTexture) {
        self.wait_queue_idle();
        self.drop_framebuffers(&[texture.bundle.view.as_raw()]);
        self.destroy_image(texture.bundle);
    }

    /// Draws `scene` into `target` and submits it to the main queue, with the device locked by the
    /// caller or by this call. A failure with four samples is reported once and the render is
    /// drawn with one; a failure with one is the error.
    pub fn render(
        &mut self,
        viewport: &mut Scene3DVkViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DVkTarget<'_>,
    ) -> Result<Scene3DVkRenderReport, Scene3DVkError> {
        self.render_with_textures(viewport, scene, target, &NoDeviceTextures)
    }

    /// [`render`](Self::render) for a renderer that has some bitmaps of the scene as images of the
    /// device (`textures`, step D7): they are sampled without a copy.
    pub fn render_with_textures(
        &mut self,
        viewport: &mut Scene3DVkViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DVkTarget<'_>,
        textures: &dyn Scene3DVkDeviceTextures,
    ) -> Result<Scene3DVkRenderReport, Scene3DVkError> {
        let device = self.device.clone();
        let lock = device.lock();
        self.renders += 1;
        let sample_count = if target.antialiased && self.multisample { MULTISAMPLE_COUNT } else { 1 };
        let mut report = Scene3DVkRenderReport::default();
        let result = match self.render_with(viewport, scene, target, textures, sample_count, &mut report) {
            Ok(()) => Ok(report),
            Err(error) if sample_count == 1 => Err(error),
            Err(error) => {
                // Four samples failed: one from now on.
                self.multisample = false;
                report_single_sample(&error.reason);
                let mut report = Scene3DVkRenderReport::default();
                self.render_with(viewport, scene, target, textures, 1, &mut report).map(|()| report)
            }
        };
        lock.dispose();
        result
    }

    fn render_with(
        &mut self,
        viewport: &mut Scene3DVkViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DVkTarget<'_>,
        textures: &dyn Scene3DVkDeviceTextures,
        sample_count: u32,
        report: &mut Scene3DVkRenderReport,
    ) -> Result<(), Scene3DVkError> {
        report.sample_count = sample_count;
        let texture_size = (target.texture.width, target.texture.height);
        let width = target.size.0.clamp(1, texture_size.0);
        let height = target.size.1.clamp(1, texture_size.1);

        // The next command buffer, once the GPU has finished with it.
        let index = self.next_in_flight;
        self.next_in_flight = (index + 1) % self.in_flight.len();
        let serial = self.in_flight[index].serial;
        self.wait_for(serial);
        self.reclaim();
        let command_buffer = self.in_flight[index].command_buffer;
        let begin = vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        // SAFETY: the command buffer is not in flight (waited for above).
        unsafe {
            self.api
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
                .map_err(failed("vkResetCommandBuffer"))?;
            self.api.begin_command_buffer(command_buffer, &begin).map_err(failed("vkBeginCommandBuffer"))?;
        }

        // A failure leaves the command buffer outside a render pass. It is submitted all the same:
        // the textures this render registered are written by the uploads it recorded.
        let recorded = self.record(viewport, scene, target, textures, sample_count, (width, height), index, report);
        // SAFETY: the command buffer is recording.
        unsafe { self.api.end_command_buffer(command_buffer) }.map_err(failed("vkEndCommandBuffer"))?;

        let fence = self.in_flight[index].fence;
        let buffers = [command_buffer];
        let submit = vk::SubmitInfo::default().command_buffers(&buffers);
        let queue = vk::Queue::from_raw(self.device.main_queue_handle() as u64);
        // SAFETY: the fence is signaled and unused (waited for above), and the device is locked.
        unsafe {
            self.api.reset_fences(&[fence]).map_err(failed("vkResetFences"))?;
            if let Err(error) = self.api.queue_submit(queue, &[submit], fence) {
                // The fence stays unsignaled with nothing to signal it: a new one replaces it.
                let info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
                if let Ok(replacement) = self.api.create_fence(&info, None) {
                    self.api.destroy_fence(fence, None);
                    self.in_flight[index].fence = replacement;
                }
                return Err(failed("vkQueueSubmit")(error));
            }
        }
        self.serial += 1;
        self.in_flight[index].serial = self.serial;
        self.in_flight[index].pending = true;
        if let Some(slot) = recorded? {
            viewport.slots[slot].submitted = self.serial;
        }
        count(|counters| counters.passes += 1);
        Ok(())
    }

    /// Records the render into the command buffer `index`; returns the uniform slot it reads.
    #[allow(clippy::too_many_arguments)]
    fn record(
        &mut self,
        viewport: &mut Scene3DVkViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DVkTarget<'_>,
        textures: &dyn Scene3DVkDeviceTextures,
        sample_count: u32,
        (width, height): (u32, u32),
        index: usize,
        report: &mut Scene3DVkRenderReport,
    ) -> Result<Option<usize>, Scene3DVkError> {
        let command_buffer = self.in_flight[index].command_buffer;
        let texture_size = (target.texture.width, target.texture.height);

        // The draws and their uniforms, and the meshes and textures they need (their uploads are
        // recorded before the render pass).
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
                    let mesh = (index_count, texture_bounds);
                    self.add_draws(viewport, model, layers, face, mesh, textures, index, report)?;
                }
            }
        }
        report.draws = viewport.draws.len() as u32;
        let additive = viewport.draws.iter().any(|draw| draw.blend == Blend::Additive);

        // The pipelines, before anything is drawn.
        for draw in 0..viewport.draws.len() {
            let key = draw_key(&viewport.draws[draw], sample_count);
            self.pipeline(key, report)?;
        }
        if additive {
            self.create_resolve_pipeline(report)?;
        }

        // What no render drew for a while goes; then the targets of the pool.
        self.drop_unused();
        let targets = self.targets(texture_size, sample_count, additive, report)?;
        let slot = match &frame {
            Some(frame) => Some(self.write_uniforms(viewport, frame, report)?),
            None => None,
        };

        let (color_view, depth_view, resolved) = {
            let targets = &self.targets[targets];
            (
                targets.multisampled.as_ref().map(|bundle| bundle.view),
                targets.depth.view,
                targets.resolved.as_ref().map(|(bundle, set)| (bundle.view, *set)),
            )
        };
        let output_view = target.texture.bundle.view;
        // What the draws end up in.
        let single = if additive { resolved.map_or(output_view, |(view, _)| view) } else { output_view };
        let (pass, attachments): (vk::RenderPass, Vec<vk::ImageView>) = match color_view {
            Some(color) => (self.multisampled_pass, vec![color, depth_view, single]),
            None => (self.single_pass, vec![single, depth_view]),
        };
        let framebuffer = self.framebuffer(pass, &attachments, texture_size)?;
        let clear = [
            vk::ClearValue { color: vk::ClearColorValue { float32: [0.0; 4] } },
            vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } },
            vk::ClearValue { color: vk::ClearColorValue { float32: [0.0; 4] } },
        ];
        let area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width: texture_size.0, height: texture_size.1 },
        };
        let begin = vk::RenderPassBeginInfo::default()
            .render_pass(pass)
            .framebuffer(framebuffer)
            .render_area(area)
            .clear_values(&clear[..attachments.len().min(2)]);
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width, height },
        };
        // The viewport of negative height: the first row is the top of the scene.
        let view = vk::Viewport {
            x: 0.0,
            y: height as f32,
            width: width as f32,
            height: -(height as f32),
            min_depth: 0.0,
            max_depth: 1.0,
        };
        // SAFETY: the command buffer is recording; the pass, framebuffer and state are of this
        // device and match each other.
        unsafe {
            self.api.cmd_begin_render_pass(command_buffer, &begin, vk::SubpassContents::INLINE);
            self.api.cmd_set_viewport(command_buffer, 0, &[view]);
            self.api.cmd_set_scissor(command_buffer, 0, &[scissor]);
        }
        if let Some(slot) = slot {
            self.draw(command_buffer, viewport, slot, sample_count);
        }
        // SAFETY: the render pass begun above.
        unsafe { self.api.cmd_end_render_pass(command_buffer) };

        // Decision N: the alpha of each pixel raised to its largest channel.
        if let (true, Some((_, set)), Some(pipeline)) = (additive, resolved, self.resolve_pipeline) {
            let framebuffer = self.framebuffer(self.resolve_pass, &[output_view], texture_size)?;
            let clear = [vk::ClearValue { color: vk::ClearColorValue { float32: [0.0; 4] } }];
            let begin = vk::RenderPassBeginInfo::default()
                .render_pass(self.resolve_pass)
                .framebuffer(framebuffer)
                .render_area(area)
                .clear_values(&clear);
            // SAFETY: as above; the set holds the resolved colour, which the pass before left to be
            // sampled.
            unsafe {
                self.api.cmd_begin_render_pass(command_buffer, &begin, vk::SubpassContents::INLINE);
                self.api.cmd_set_viewport(command_buffer, 0, &[view]);
                self.api.cmd_set_scissor(command_buffer, 0, &[scissor]);
                self.api.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
                self.api.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.resolve_pipeline_layout,
                    0,
                    &[set],
                    &[],
                );
                self.api.cmd_draw(command_buffer, 3, 1, 0, 0);
                self.api.cmd_end_render_pass(command_buffer);
            }
        }
        Ok(slot)
    }

    /// Issues the draws of `viewport`, whose uniforms are in `slot`.
    fn draw(&self, command_buffer: vk::CommandBuffer, viewport: &Scene3DVkViewport, slot: usize, sample_count: u32) {
        let slot = &viewport.slots[slot];
        let mut current_pipeline = None;
        let mut current_mesh = None;
        let mut bound_frame = false;
        for (index, draw) in viewport.draws.iter().enumerate() {
            let key = draw_key(draw, sample_count);
            let (Some(&pipeline), Some(mesh)) = (self.pipelines.get(&key), self.meshes.get(&draw.mesh)) else {
                continue;
            };
            let Some((vertices, indices)) = &mesh.buffers else { continue };
            let samples = samples_texture(draw.brush);
            let texture_set = if samples {
                let set = match &draw.texture {
                    Some(texture) => self.textures.get(texture).map(|texture| texture.set),
                    None => self.empty_texture.as_ref().map(|(_, set)| *set),
                };
                match set {
                    Some(set) => Some(set),
                    None => continue,
                }
            } else {
                None
            };
            let layout = if samples { self.textured_pipeline_layout } else { self.pipeline_layout };
            // SAFETY: the command buffer is in a render pass; the pipeline, buffers and sets are of
            // this device; the dynamic offsets are records within the buffers; the index buffer
            // holds `index_count` indices of vertices of the vertex buffer.
            unsafe {
                if current_pipeline != Some(pipeline) {
                    self.api.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
                    current_pipeline = Some(pipeline);
                }
                if !bound_frame {
                    self.api.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        layout,
                        0,
                        &[slot.frame_set],
                        &[],
                    );
                    bound_frame = true;
                }
                let offsets = [(index * DRAW_STRIDE) as u32, (index * BRUSH_STRIDE) as u32];
                self.api.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    layout,
                    1,
                    &[slot.draw_set],
                    &offsets,
                );
                if let Some(set) = texture_set {
                    self.api.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        layout,
                        2,
                        &[set],
                        &[],
                    );
                }
                if current_mesh != Some(draw.mesh) {
                    self.api.cmd_bind_vertex_buffers(command_buffer, 0, &[vertices.buffer], &[0]);
                    self.api.cmd_bind_index_buffer(command_buffer, indices.buffer, 0, vk::IndexType::UINT32);
                    current_mesh = Some(draw.mesh);
                }
                self.api.cmd_draw_indexed(command_buffer, draw.index_count, 1, 0, 0, 0);
            }
        }
    }

    /// The framebuffer of `attachments`, kept while its views live.
    fn framebuffer(
        &mut self,
        pass: vk::RenderPass,
        attachments: &[vk::ImageView],
        size: (u32, u32),
    ) -> Result<vk::Framebuffer, Scene3DVkError> {
        let raw = |index: usize| attachments.get(index).map_or(0, |view| view.as_raw());
        let key = (raw(0), raw(1), raw(2));
        if let Some(&framebuffer) = self.framebuffers.get(&key) {
            return Ok(framebuffer);
        }
        let info = vk::FramebufferCreateInfo::default()
            .render_pass(pass)
            .attachments(attachments)
            .width(size.0)
            .height(size.1)
            .layers(1);
        // SAFETY: the description lives for the call, and the views match the pass.
        let framebuffer = unsafe { self.api.create_framebuffer(&info, None) }.map_err(failed("vkCreateFramebuffer"))?;
        self.framebuffers.insert(key, framebuffer);
        Ok(framebuffer)
    }

    /// Destroys what no render drew for [`LIFETIME`] renders, and textures of bitmaps that are gone.
    fn drop_unused(&mut self) {
        let renders = self.renders;
        let old_meshes: Vec<(u64, u64)> =
            self.meshes.iter().filter(|(_, mesh)| renders - mesh.last_used >= LIFETIME).map(|(key, _)| *key).collect();
        for key in old_meshes {
            if let Some(MeshBuffers { buffers: Some((vertices, indices)), .. }) = self.meshes.remove(&key) {
                self.destroy_buffer(vertices);
                self.destroy_buffer(indices);
            }
        }
        let mut index = 0;
        while index < self.targets.len() {
            if renders - self.targets[index].last_used >= LIFETIME {
                let targets = self.targets.swap_remove(index);
                self.destroy_targets(targets);
            } else {
                index += 1;
            }
        }
        let gone: Vec<BrushTextureKey> = self
            .textures
            .iter()
            .filter(|(_, texture)| !texture.identity.is_alive() || renders - texture.last_used >= LIFETIME)
            .map(|(key, _)| *key)
            .collect();
        for key in gone {
            // A render in flight may read it.
            if let Some(texture) = self.textures.remove(&key) {
                self.retire(Retired::Texture(texture));
            }
        }
        self.refused_textures.retain(|_, last| renders - *last < LIFETIME);
    }

    /// The draws of the layers of one side of a model; see the other executors.
    #[allow(clippy::too_many_arguments)]
    fn add_draws(
        &mut self,
        viewport: &mut Scene3DVkViewport,
        model: &Scene3DModel,
        layers: &[Scene3DMaterialLayer],
        face: Face,
        (index_count, texture_bounds): (u32, Rect),
        textures: &dyn Scene3DVkDeviceTextures,
        in_flight: usize,
        report: &mut Scene3DVkRenderReport,
    ) -> Result<(), Scene3DVkError> {
        let depth_layer =
            layers.iter().position(|layer| LayerKind::of(layer.kind) == LayerKind::Diffuse).unwrap_or(0);
        for (index, layer) in layers.iter().enumerate() {
            let (uniforms, _) = DrawUniforms::of(model, layer, face);
            let sampling = |bitmap: &SharedBitmapImpl| textures.sampling(bitmap);
            let mut brush = MaterialBrush::of_with(layer.brush.as_ref(), texture_bounds, &sampling);
            let mut texture = None;
            if let Some(source) = brush.texture().cloned() {
                if self.brush_texture(&source, textures, in_flight, report) {
                    texture = Some(source.key());
                } else {
                    // Texels that cannot be read or uploaded: the layer is drawn with its colour.
                    brush = brush.to_unsampled();
                }
            } else if brush.shader() == BrushShader::Image {
                // An image brush without an image samples the empty texture (it paints nothing).
                self.empty_texture(in_flight, report)?;
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

    /// A descriptor set of `view` and the sampler.
    fn texture_set(&self, view: vk::ImageView) -> Result<vk::DescriptorSet, Scene3DVkError> {
        let layouts = [self.texture_layout];
        let allocate =
            vk::DescriptorSetAllocateInfo::default().descriptor_pool(self.descriptor_pool).set_layouts(&layouts);
        // SAFETY: the descriptions live for their calls, and the set is new.
        unsafe {
            let set = self.api.allocate_descriptor_sets(&allocate).map_err(failed("vkAllocateDescriptorSets"))?[0];
            let image = [vk::DescriptorImageInfo::default()
                .image_view(view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
            let sampler = [vk::DescriptorImageInfo::default().sampler(self.sampler)];
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image),
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&sampler),
            ];
            self.api.update_descriptor_sets(&writes, &[]);
            Ok(set)
        }
    }

    /// Copies `rgba` into a new image through a staging buffer the command buffer `in_flight` keeps.
    fn upload_image(
        &mut self,
        size: (u32, u32),
        rgba: &[u8],
        in_flight: usize,
    ) -> Result<ImageBundle, Scene3DVkError> {
        let staging = self.create_buffer(rgba.len(), vk::BufferUsageFlags::TRANSFER_SRC)?;
        let usage = vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED;
        let flags = vk::SampleCountFlags::TYPE_1;
        let bundle = match self.create_image(size, COLOR_FORMAT, flags, usage, vk::ImageAspectFlags::COLOR) {
            Ok((bundle, _)) => bundle,
            Err(error) => {
                self.destroy_buffer(staging);
                return Err(error);
            }
        };
        // SAFETY: the mapping holds at least `rgba.len()` bytes.
        unsafe { std::ptr::copy_nonoverlapping(rgba.as_ptr(), staging.mapped, rgba.len()) };
        let command_buffer = self.in_flight[in_flight].command_buffer;
        let range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);
        let to_transfer = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(bundle.image)
            .subresource_range(range);
        let to_shader = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(bundle.image)
            .subresource_range(range);
        let copy = vk::BufferImageCopy::default()
            .image_subresource(
                vk::ImageSubresourceLayers::default().aspect_mask(vk::ImageAspectFlags::COLOR).layer_count(1),
            )
            .image_extent(vk::Extent3D { width: size.0, height: size.1, depth: 1 });
        // SAFETY: the command buffer is recording, outside a render pass; the staging buffer holds
        // the texels and lives until the GPU has done this command buffer.
        unsafe {
            self.api.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[to_transfer],
            );
            self.api.cmd_copy_buffer_to_image(
                command_buffer,
                staging.buffer,
                bundle.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy],
            );
            self.api.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[to_shader],
            );
        }
        self.in_flight[in_flight].staging.push(staging);
        Ok(bundle)
    }

    /// The texture of `source` on the device, uploaded when its key is new. `false` when its texels
    /// cannot be read or uploaded.
    fn brush_texture(
        &mut self,
        source: &BrushTextureSource,
        textures: &dyn Scene3DVkDeviceTextures,
        in_flight: usize,
        report: &mut Scene3DVkRenderReport,
    ) -> bool {
        let key = source.key();
        let renders = self.renders;
        if let Some(texture) = self.textures.get_mut(&key) {
            texture.last_used = renders;
            return true;
        }
        if let Some(texture) = self.device_texture(source, textures) {
            self.textures.insert(key, texture);
            count(|counters| counters.textures_on_device += 1);
            return true;
        }
        if let Some(last) = self.refused_textures.get_mut(&key) {
            *last = renders;
            return false;
        }
        let max_side = self.max_side;
        let texels = source
            .texels()
            .filter(|texels| (1..=max_side).contains(&texels.width) && (1..=max_side).contains(&texels.height));
        let refuse = |executor: &mut Self| {
            executor.refused_textures.insert(key, renders);
            count(|counters| counters.textures_refused += 1);
            false
        };
        let Some(texels) = texels else { return refuse(self) };
        let size = (texels.width, texels.height);
        // The older versions of a bitmap that no draw of this render uses.
        if let BrushTextureKey::Image { identity, .. } = key {
            let old: Vec<BrushTextureKey> = self
                .textures
                .iter()
                .filter(|(other, texture)| {
                    matches!(other, BrushTextureKey::Image { identity: other, .. } if *other == identity)
                        && texture.last_used != renders
                })
                .map(|(other, _)| *other)
                .collect();
            for other in old {
                if let Some(texture) = self.textures.remove(&other) {
                    self.retire(Retired::Texture(texture));
                }
            }
        }
        let Ok(bundle) = self.upload_image(size, &texels.rgba, in_flight) else { return refuse(self) };
        let set = match self.texture_set(bundle.view) {
            Ok(set) => set,
            Err(_) => {
                // The upload recorded for it still runs: the image goes after it.
                self.retire(Retired::Image(bundle));
                return refuse(self);
            }
        };
        report.resources_created += 1;
        report.textures_uploaded += 1;
        count(|counters| {
            counters.textures_uploaded += 1;
            counters.texture_bytes += texels.rgba.len() as u64;
        });
        let view = bundle.view;
        let identity = source.identity();
        self.textures.insert(key, BrushTexture { identity, owned: Some(bundle), view, set, size, last_used: renders });
        true
    }

    /// The texture of a bitmap the renderer has as an image of this device.
    fn device_texture(
        &self,
        source: &BrushTextureSource,
        textures: &dyn Scene3DVkDeviceTextures,
    ) -> Option<BrushTexture> {
        let bitmap = source.bitmap()?;
        if textures.sampling(&**bitmap) == DeviceBitmap::NotOnDevice {
            return None;
        }
        let (image, format, size) = textures.device_image(&**bitmap)?;
        let view = self.create_view(image, format, vk::ImageAspectFlags::COLOR).ok()?;
        match self.texture_set(view) {
            Ok(set) => Some(BrushTexture {
                identity: source.identity(),
                owned: None,
                view,
                set,
                size,
                last_used: self.renders,
            }),
            Err(_) => {
                // SAFETY: a view no command has used.
                unsafe { self.api.destroy_image_view(view, None) };
                None
            }
        }
    }

    /// The transparent texture of one texel an image brush without an image samples.
    fn empty_texture(&mut self, in_flight: usize, report: &mut Scene3DVkRenderReport) -> Result<(), Scene3DVkError> {
        if self.empty_texture.is_some() {
            return Ok(());
        }
        let bundle = self.upload_image((1, 1), &[0; 4], in_flight)?;
        let set = match self.texture_set(bundle.view) {
            Ok(set) => set,
            Err(error) => {
                // The upload recorded for it still runs: the image goes after it.
                self.retire(Retired::Image(bundle));
                return Err(error);
            }
        };
        self.empty_texture = Some((bundle, set));
        report.resources_created += 1;
        Ok(())
    }

    /// The buffers of `mesh` on the device, uploaded when its `(id, version)` is new.
    fn mesh(&mut self, mesh: &MeshData, report: &mut Scene3DVkRenderReport) -> Result<(u32, Rect), Scene3DVkError> {
        let key = (mesh.id, mesh.version);
        let renders = self.renders;
        if let Some(buffers) = self.meshes.get_mut(&key) {
            buffers.last_used = renders;
            return Ok((buffers.index_count, buffers.texture_bounds));
        }
        // The version this one replaces, unless a draw of this render uses it.
        let old = self.meshes.iter().find(|(key, kept)| key.0 == mesh.id && kept.last_used != renders);
        if let Some(old) = old.map(|(key, _)| *key) {
            // A render in flight may read it.
            if let Some(MeshBuffers { buffers: Some((vertices, indices)), .. }) = self.meshes.remove(&old) {
                self.retire(Retired::Buffer(vertices));
                self.retire(Retired::Buffer(indices));
            }
        }
        self.vertex_scratch.clear();
        self.index_scratch.clear();
        let index_count = write_mesh(mesh, &mut self.vertex_scratch, &mut self.index_scratch) as u32;
        let buffers = if index_count == 0 {
            None
        } else {
            self.index_bytes.clear();
            for index in &self.index_scratch {
                self.index_bytes.extend_from_slice(&index.to_ne_bytes());
            }
            let vertices = self.create_buffer(self.vertex_scratch.len(), vk::BufferUsageFlags::VERTEX_BUFFER)?;
            let indices = self.create_buffer(self.index_bytes.len(), vk::BufferUsageFlags::INDEX_BUFFER)?;
            // SAFETY: the mappings hold at least these bytes, and no command reads the new buffers.
            unsafe {
                std::ptr::copy_nonoverlapping(self.vertex_scratch.as_ptr(), vertices.mapped, self.vertex_scratch.len());
                std::ptr::copy_nonoverlapping(self.index_bytes.as_ptr(), indices.mapped, self.index_bytes.len());
            }
            report.resources_created += 2;
            report.meshes_uploaded += 1;
            let bytes = (self.vertex_scratch.len() + self.index_bytes.len()) as u64;
            count(|counters| {
                counters.mesh_uploads += 1;
                counters.mesh_bytes += bytes;
            });
            Some((vertices, indices))
        };
        let texture_bounds = texture_coordinate_bounds(mesh);
        self.meshes.insert(key, MeshBuffers { buffers, index_count, texture_bounds, last_used: renders });
        Ok((index_count, texture_bounds))
    }

    fn shader_module(&self, bytes: &[u8]) -> Result<vk::ShaderModule, Scene3DVkError> {
        let words = spirv_words(bytes);
        let info = vk::ShaderModuleCreateInfo::default().code(&words);
        // SAFETY: the description lives for the call, and the words are a SPIR-V module.
        unsafe { self.api.create_shader_module(&info, None) }.map_err(failed("vkCreateShaderModule"))
    }

    /// The pipeline of `key`, made on first demand.
    fn pipeline(&mut self, key: PipelineKey, report: &mut Scene3DVkRenderReport) -> Result<(), Scene3DVkError> {
        if self.pipelines.contains_key(&key) {
            return Ok(());
        }
        let (bytes, entry) = spirv::fragment_module(key.brush);
        let fragment = match self.fragment_modules.get(&key.brush) {
            Some(module) => *module,
            None => {
                let module = self.shader_module(bytes)?;
                self.fragment_modules.insert(key.brush, module);
                module
            }
        };
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(self.vertex_module)
                .name(c"vs_main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment)
                .name(entry),
        ];
        let bindings = [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(VERTEX_STRIDE as u32)
            .input_rate(vk::VertexInputRate::VERTEX)];
        let formats = [vk::Format::R32G32B32_SFLOAT, vk::Format::R32G32B32_SFLOAT, vk::Format::R32G32_SFLOAT];
        let attributes: Vec<vk::VertexInputAttributeDescription> = VERTEX_ATTRIBUTE_OFFSETS
            .iter()
            .zip(formats)
            .enumerate()
            .map(|(location, (offset, format))| {
                vk::VertexInputAttributeDescription::default()
                    .location(location as u32)
                    .binding(0)
                    .format(format)
                    .offset(*offset as u32)
            })
            .collect();
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attributes);
        let assembly =
            vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(match key.face {
                Face::Front => vk::CullModeFlags::BACK,
                Face::Back => vk::CullModeFlags::FRONT,
            })
            // With the viewport of negative height, counterclockwise on the screen (WPF).
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);
        let samples = sample_flags(key.sample_count);
        let multisample = vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(samples);
        let depth = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(key.depth_write)
            .depth_compare_op(if key.depth_write { vk::CompareOp::LESS } else { vk::CompareOp::LESS_OR_EQUAL });
        let destination = match key.blend {
            Blend::SourceOver => vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            Blend::Additive => vk::BlendFactor::ONE,
        };
        let attachments = [vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(destination)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(destination)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(vk::ColorComponentFlags::RGBA)];
        let blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&attachments);
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
        let layout =
            if samples_texture(key.brush) { self.textured_pipeline_layout } else { self.pipeline_layout };
        let pass = if key.sample_count > 1 { self.multisampled_pass } else { self.single_pass };
        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&assembly)
            .viewport_state(&viewport)
            .rasterization_state(&rasterization)
            .multisample_state(&multisample)
            .depth_stencil_state(&depth)
            .color_blend_state(&blend)
            .dynamic_state(&dynamic)
            .layout(layout)
            .render_pass(pass);
        // SAFETY: the description and what it points to live for the call.
        let pipelines = unsafe { self.api.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None) };
        let pipeline = pipelines.map_err(|(_, error)| failed("vkCreateGraphicsPipelines")(error))?[0];
        self.pipelines.insert(key, pipeline);
        report.pipelines_created += 1;
        count(|counters| counters.pipelines_created += 1);
        Ok(())
    }

    /// The pipeline of the resolve, made on first demand.
    fn create_resolve_pipeline(&mut self, report: &mut Scene3DVkRenderReport) -> Result<(), Scene3DVkError> {
        if self.resolve_pipeline.is_some() {
            return Ok(());
        }
        let (vertex, fragment) = match self.resolve_modules {
            Some(modules) => modules,
            None => {
                let vertex = self.shader_module(spirv::RESOLVE_VERTEX)?;
                let fragment = self.shader_module(spirv::RESOLVE_FRAGMENT)?;
                self.resolve_modules = Some((vertex, fragment));
                (vertex, fragment)
            }
        };
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex)
                .name(c"vs_resolve"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment)
                .name(c"fs_resolve"),
        ];
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
        let assembly =
            vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .line_width(1.0);
        let multisample =
            vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let attachments = [vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)];
        let blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&attachments);
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&assembly)
            .viewport_state(&viewport)
            .rasterization_state(&rasterization)
            .multisample_state(&multisample)
            .color_blend_state(&blend)
            .dynamic_state(&dynamic)
            .layout(self.resolve_pipeline_layout)
            .render_pass(self.resolve_pass);
        // SAFETY: the description and what it points to live for the call.
        let pipelines = unsafe { self.api.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None) };
        let pipeline = pipelines.map_err(|(_, error)| failed("vkCreateGraphicsPipelines")(error))?[0];
        self.resolve_pipeline = Some(pipeline);
        report.pipelines_created += 1;
        count(|counters| counters.pipelines_created += 1);
        Ok(())
    }

    /// The index of the targets of `size` and `sample_count`, made on first demand.
    fn targets(
        &mut self,
        size: (u32, u32),
        sample_count: u32,
        additive: bool,
        report: &mut Scene3DVkRenderReport,
    ) -> Result<usize, Scene3DVkError> {
        let renders = self.renders;
        let index = match self.targets.iter().position(|t| t.size == size && t.sample_count == sample_count) {
            Some(index) => index,
            None => {
                let samples = sample_flags(sample_count);
                let multisampled = if sample_count > 1 {
                    let usage = vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT;
                    Some(self.create_image(size, COLOR_FORMAT, samples, usage, vk::ImageAspectFlags::COLOR)?.0)
                } else {
                    None
                };
                let depth = self
                    .create_image(
                        size,
                        self.depth_format,
                        samples,
                        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                        vk::ImageAspectFlags::DEPTH,
                    )?
                    .0;
                report.resources_created += 1 + u32::from(multisampled.is_some());
                let resolved = None;
                self.targets.push(Targets { size, sample_count, multisampled, depth, resolved, last_used: renders });
                self.targets.len() - 1
            }
        };
        self.targets[index].last_used = renders;
        if additive && self.targets[index].resolved.is_none() {
            let usage = vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED;
            let flags = vk::SampleCountFlags::TYPE_1;
            let bundle = self.create_image(size, COLOR_FORMAT, flags, usage, vk::ImageAspectFlags::COLOR)?.0;
            let layouts = [self.resolve_set_layout];
            let allocate =
                vk::DescriptorSetAllocateInfo::default().descriptor_pool(self.descriptor_pool).set_layouts(&layouts);
            // SAFETY: the descriptions live for their calls, and the set is new.
            let set = unsafe {
                let sets = self.api.allocate_descriptor_sets(&allocate).map_err(failed("vkAllocateDescriptorSets"))?;
                let set = sets[0];
                let image = [vk::DescriptorImageInfo::default()
                    .image_view(bundle.view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
                let writes = [vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image)];
                self.api.update_descriptor_sets(&writes, &[]);
                set
            };
            self.targets[index].resolved = Some((bundle, set));
            report.resources_created += 1;
        }
        Ok(index)
    }

    /// Writes the uniforms of the draws of `viewport` into its next slot, where their bytes
    /// changed; returns the slot.
    fn write_uniforms(
        &mut self,
        viewport: &mut Scene3DVkViewport,
        frame: &FrameUniforms,
        report: &mut Scene3DVkRenderReport,
    ) -> Result<usize, Scene3DVkError> {
        if viewport.slots.is_empty() {
            viewport.slots.resize_with(UNIFORM_SLOTS, UniformSlot::default);
        }
        let index = viewport.next % viewport.slots.len();
        viewport.next = (index + 1) % viewport.slots.len();
        // The GPU may still read the slot: its render is waited for (a frame or two behind).
        let submitted = viewport.slots[index].submitted;
        self.wait_for(submitted);
        viewport.frame_scratch.clear();
        frame.write_std140(&mut viewport.frame_scratch);
        let draws = viewport.draws.len().max(1);
        let frame_length = viewport.frame_scratch.len();
        let mut made = 0u32;
        let slot = &mut viewport.slots[index];
        if slot.frame.is_none() {
            let buffer = self.create_buffer(frame_length, vk::BufferUsageFlags::UNIFORM_BUFFER)?;
            let layouts = [self.frame_layout];
            let allocate =
                vk::DescriptorSetAllocateInfo::default().descriptor_pool(self.descriptor_pool).set_layouts(&layouts);
            // SAFETY: the descriptions live for their calls, and the set is new.
            unsafe {
                let sets = self.api.allocate_descriptor_sets(&allocate).map_err(failed("vkAllocateDescriptorSets"))?;
                let set = sets[0];
                let info = [vk::DescriptorBufferInfo::default()
                    .buffer(buffer.buffer)
                    .offset(0)
                    .range(frame_length as u64)];
                let writes = [vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&info)];
                self.api.update_descriptor_sets(&writes, &[]);
                slot.frame_set = set;
            }
            slot.frame = Some(buffer);
            slot.frame_bytes.clear();
            made += 1;
        }
        if slot.draws.is_none() || slot.brushes.is_none() || slot.capacity < draws {
            for buffer in [slot.draws.take(), slot.brushes.take()].into_iter().flatten() {
                self.destroy_buffer(buffer);
            }
            let capacity = draws.next_power_of_two();
            let draw_buffer = self.create_buffer(capacity * DRAW_STRIDE, vk::BufferUsageFlags::UNIFORM_BUFFER)?;
            let brush_buffer = self.create_buffer(capacity * BRUSH_STRIDE, vk::BufferUsageFlags::UNIFORM_BUFFER)?;
            // SAFETY: the descriptions live for their calls; the slot is not in flight.
            unsafe {
                if slot.draw_set == vk::DescriptorSet::null() {
                    let layouts = [self.draw_layout];
                    let allocate = vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(self.descriptor_pool)
                        .set_layouts(&layouts);
                    slot.draw_set =
                        self.api.allocate_descriptor_sets(&allocate).map_err(failed("vkAllocateDescriptorSets"))?[0];
                }
                let draw_info = [vk::DescriptorBufferInfo::default()
                    .buffer(draw_buffer.buffer)
                    .offset(0)
                    .range(DRAW_STRIDE as u64)];
                let brush_info = [vk::DescriptorBufferInfo::default()
                    .buffer(brush_buffer.buffer)
                    .offset(0)
                    .range(BRUSH_STRIDE as u64)];
                let writes = [
                    vk::WriteDescriptorSet::default()
                        .dst_set(slot.draw_set)
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                        .buffer_info(&draw_info),
                    vk::WriteDescriptorSet::default()
                        .dst_set(slot.draw_set)
                        .dst_binding(1)
                        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                        .buffer_info(&brush_info),
                ];
                self.api.update_descriptor_sets(&writes, &[]);
            }
            slot.draws = Some(draw_buffer);
            slot.brushes = Some(brush_buffer);
            slot.capacity = capacity;
            slot.draw_bytes.clear();
            slot.brush_bytes.clear();
            made += 2;
        }
        report.resources_created += made;

        let mut writes = 0u64;
        let mut bytes = 0u64;
        if let Some(buffer) = &slot.frame {
            if slot.frame_bytes != viewport.frame_scratch {
                write_mapped(buffer, 0, &viewport.frame_scratch);
                slot.frame_bytes.clone_from(&viewport.frame_scratch);
                writes += 1;
                bytes += viewport.frame_scratch.len() as u64;
            }
        }
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
}

/// The sample count flag of `count`.
fn sample_flags(count: u32) -> vk::SampleCountFlags {
    if count > 1 {
        vk::SampleCountFlags::TYPE_4
    } else {
        vk::SampleCountFlags::TYPE_1
    }
}

fn draw_key(draw: &DrawRecord, sample_count: u32) -> PipelineKey {
    PipelineKey { blend: draw.blend, face: draw.face, depth_write: draw.depth_write, sample_count, brush: draw.brush }
}

/// Copies `bytes` into the mapping of `buffer` at `offset`.
fn write_mapped(buffer: &MappedBuffer, offset: usize, bytes: &[u8]) {
    assert!(offset + bytes.len() <= buffer.length, "a write within the buffer");
    // SAFETY: the range is within the mapping (asserted), coherent memory no command in flight
    // reads (the slots of `write_uniforms`).
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.mapped.add(offset), bytes.len()) };
}

/// Writes the range of whole records of `stride` bytes where `new` differs from `old` (all of
/// `new` when their lengths differ): the bytes written, or `None`.
fn write_changed(buffer: &MappedBuffer, old: &[u8], new: &[u8], stride: usize) -> Option<u64> {
    if new.is_empty() {
        return None;
    }
    if old.len() != new.len() {
        write_mapped(buffer, 0, new);
        return Some(new.len() as u64);
    }
    let first = old.chunks_exact(stride).zip(new.chunks_exact(stride)).position(|(a, b)| a != b)?;
    let last = old.chunks_exact(stride).zip(new.chunks_exact(stride)).rposition(|(a, b)| a != b)?;
    let range = first * stride..(last + 1) * stride;
    write_mapped(buffer, range.start, &new[range.clone()]);
    Some(range.len() as u64)
}

#[cfg(test)]
#[path = "scene3d_tests.rs"]
mod tests;
