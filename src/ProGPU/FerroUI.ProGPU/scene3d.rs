//! 3D scenes on the device of the backend (#608, docs/porting/media3d.md, section 3.15): the
//! executor of the wgpu route of `ferroui-render3d`, stage for stage, on the WebGPU C ABI, so it
//! also runs on the browser's WebGPU. The shader, the uniforms, the vertex layout and the brushes
//! are those of `ferroui-render3d`: one source for every backend.
//!
//! - One draw per material layer, the front layers then the back layers of a model, in tree
//!   order. The first diffuse layer of a side writes depth with `Less`, the others test
//!   `LessEqual`; a diffuse layer is blended source-over, a specular or emissive layer is added.
//! - Pipelines are made on first demand, once per key; meshes are uploaded once per
//!   `(id, version)`; a brush texture once per key; a render writes only the uniforms that changed.
//! - 4x MSAA, one sample for aliased edges or after a failure with four. A scene with added layers
//!   is resolved through `fs_resolve`, which raises the alpha to the largest colour channel.
//! - A kept output is drawn again while its snapshot, size, scale, edges and brush inputs are
//!   the same: no 3D pass.
//!
//! A scene that cannot be drawn on the device is not drawn, and counted by the caller: there is no
//! CPU path here. A frame of a video has no texture of this device yet, so a layer with one is
//! drawn with its colour, which is said once in the log.

use crate::device::{thread_is_ending, ProGpuDevice};
use crate::effect_passes::{text, with_error_scope};
use crate::gpu::{Gpu, GpuTexture};
use crate::notes;
use ferroui_base::rendering::scene3d::{MeshData, Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot};
use ferroui_base::{PixelSize, Rect};
use ferroui_render3d::brush::{
    scene_texture_inputs, texture_coordinate_bounds, BrushShader, BrushTextureIdentity, BrushTextureKey,
    BrushTextureSource, BrushUniforms, MaterialBrush,
};
use ferroui_render3d::shader::{
    fragment_entry, samples_texture, write_mesh, BRUSH_BINDING, BRUSH_TEXTURE_GROUP, RESOLVED_BINDING,
    RESOLVE_FRAGMENT_ENTRY, RESOLVE_VERTEX_ENTRY, SCENE3D_WGSL, VERTEX_ATTRIBUTE_OFFSETS, VERTEX_ENTRY, VERTEX_STRIDE,
};
use ferroui_render3d::uniforms::{DrawUniforms, Face, FrameUniforms, LayerKind, Std140};
use progpu_native_sys::webgpu::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::rc::Rc;
use std::sync::{Arc, Weak};

const COLOR_FORMAT: WGPUTextureFormat = WGPUTextureFormat_RGBA8Unorm;
const DEPTH_FORMAT: WGPUTextureFormat = WGPUTextureFormat_Depth32Float;
/// The samples of an antialiased render. WebGPU guarantees them for both formats.
const MULTISAMPLE_COUNT: u32 = 4;
/// The renders a mesh or a target of the pool is kept for without being drawn.
const LIFETIME: u64 = 240;
/// The outputs a device keeps: the viewports of a window or two.
const CAPACITY: usize = 8;
/// The alignment of dynamic uniform offsets: the largest WebGPU allows a device to ask for.
const UNIFORM_ALIGNMENT: usize = 256;
/// The largest side of a brush texture: the limit every WebGPU device has.
const MAX_TEXTURE_SIDE: u32 = 8192;

macro_rules! owned {
    ($name:ident, $raw:ty, $release:ident) => {
        struct $name($raw);

        impl Drop for $name {
            fn drop(&mut self) {
                if !self.0.is_null() && !thread_is_ending() {
                    // SAFETY: the handle is owned here.
                    unsafe { $release(self.0) };
                }
            }
        }
    };
}

owned!(Buffer, WGPUBuffer, wgpuBufferRelease);
owned!(BindGroup, WGPUBindGroup, wgpuBindGroupRelease);
owned!(BindLayout, WGPUBindGroupLayout, wgpuBindGroupLayoutRelease);
owned!(PipelineLayout, WGPUPipelineLayout, wgpuPipelineLayoutRelease);
owned!(Pipeline, WGPURenderPipeline, wgpuRenderPipelineRelease);
owned!(Sampler, WGPUSampler, wgpuSamplerRelease);
owned!(Module, WGPUShaderModule, wgpuShaderModuleRelease);
owned!(RawTexture, WGPUTexture, wgpuTextureRelease);
owned!(View, WGPUTextureView, wgpuTextureViewRelease);

/// A texture with its view (the view goes first).
struct OwnedTexture {
    view: View,
    texture: RawTexture,
}

fn create_texture(
    device: WGPUDevice,
    size: (u32, u32),
    format: WGPUTextureFormat,
    samples: u32,
    usage: WGPUTextureUsage,
) -> OwnedTexture {
    // SAFETY: the calls of the WebGPU C ABI with the device; the descriptor is borrowed.
    unsafe {
        let texture = wgpuDeviceCreateTexture(
            device,
            &WGPUTextureDescriptor {
                nextInChain: ptr::null_mut(),
                label: text("FerroUI 3D scene"),
                usage,
                dimension: WGPUTextureDimension_2D,
                size: WGPUExtent3D { width: size.0.max(1), height: size.1.max(1), depthOrArrayLayers: 1 },
                format,
                mipLevelCount: 1,
                sampleCount: samples,
                viewFormatCount: 0,
                viewFormats: ptr::null(),
            },
        );
        OwnedTexture { view: View(wgpuTextureCreateView(texture, ptr::null())), texture: RawTexture(texture) }
    }
}

fn create_buffer(device: WGPUDevice, size: u64, usage: WGPUBufferUsage) -> Buffer {
    // SAFETY: the call of the WebGPU C ABI with the device; the descriptor is borrowed.
    Buffer(unsafe {
        wgpuDeviceCreateBuffer(
            device,
            &WGPUBufferDescriptor {
                nextInChain: ptr::null_mut(),
                label: text("FerroUI 3D scene"),
                usage: usage | WGPUBufferUsage_CopyDst,
                size,
                mappedAtCreation: 0,
            },
        )
    })
}

fn write_buffer(queue: WGPUQueue, buffer: WGPUBuffer, offset: u64, bytes: &[u8]) {
    // SAFETY: the buffer is alive and holds the range; the data is copied by the call.
    unsafe { wgpuQueueWriteBuffer(queue, buffer, offset, bytes.as_ptr() as *const c_void, bytes.len()) };
}

fn write_texels(queue: WGPUQueue, texture: WGPUTexture, size: (u32, u32), rgba: &[u8]) {
    // SAFETY: the texture is alive and of that size; the records are borrowed, the data copied.
    unsafe {
        wgpuQueueWriteTexture(
            queue,
            &WGPUTexelCopyTextureInfo {
                texture,
                mipLevel: 0,
                origin: WGPUOrigin3D { x: 0, y: 0, z: 0 },
                aspect: WGPUTextureAspect_All,
            },
            rgba.as_ptr() as *const c_void,
            rgba.len(),
            &WGPUTexelCopyBufferLayout { offset: 0, bytesPerRow: size.0 * 4, rowsPerImage: size.1 },
            &WGPUExtent3D { width: size.0, height: size.1, depthOrArrayLayers: 1 },
        );
    }
}

fn uniform_entry(binding: u32, dynamic: bool, size: usize) -> WGPUBindGroupLayoutEntry {
    // SAFETY: all-zero is a valid record: every binding kind is "not used", the chain is empty.
    let mut entry: WGPUBindGroupLayoutEntry = unsafe { std::mem::zeroed() };
    entry.binding = binding;
    entry.visibility = WGPUShaderStage_Vertex | WGPUShaderStage_Fragment;
    entry.buffer.type_ = WGPUBufferBindingType_Uniform;
    entry.buffer.hasDynamicOffset = u32::from(dynamic);
    entry.buffer.minBindingSize = size as u64;
    entry
}

fn texture_entry(binding: u32, sample_type: WGPUTextureSampleType) -> WGPUBindGroupLayoutEntry {
    // SAFETY: as in `uniform_entry`.
    let mut entry: WGPUBindGroupLayoutEntry = unsafe { std::mem::zeroed() };
    entry.binding = binding;
    entry.visibility = WGPUShaderStage_Fragment;
    entry.texture.sampleType = sample_type;
    entry.texture.viewDimension = WGPUTextureViewDimension_2D;
    entry
}

fn create_layout(device: WGPUDevice, entries: &[WGPUBindGroupLayoutEntry]) -> BindLayout {
    // SAFETY: the call of the WebGPU C ABI with the device; the records are borrowed.
    BindLayout(unsafe {
        wgpuDeviceCreateBindGroupLayout(
            device,
            &WGPUBindGroupLayoutDescriptor {
                nextInChain: ptr::null_mut(),
                label: text("FerroUI 3D scene"),
                entryCount: entries.len(),
                entries: entries.as_ptr(),
            },
        )
    })
}

fn create_pipeline_layout(device: WGPUDevice, layouts: &[WGPUBindGroupLayout]) -> PipelineLayout {
    // SAFETY: the call of the WebGPU C ABI with the device; the records are borrowed.
    PipelineLayout(unsafe {
        wgpuDeviceCreatePipelineLayout(
            device,
            &WGPUPipelineLayoutDescriptor {
                nextInChain: ptr::null_mut(),
                label: text("FerroUI 3D scene"),
                bindGroupLayoutCount: layouts.len(),
                bindGroupLayouts: layouts.as_ptr(),
                immediateSize: 0,
            },
        )
    })
}

fn buffer_entry(binding: u32, buffer: WGPUBuffer, size: u64) -> WGPUBindGroupEntry {
    // SAFETY: all-zero is a valid record: no chain, no resource yet.
    let mut entry: WGPUBindGroupEntry = unsafe { std::mem::zeroed() };
    entry.binding = binding;
    entry.buffer = buffer;
    entry.size = size;
    entry
}

fn create_group(device: WGPUDevice, layout: WGPUBindGroupLayout, entries: &[WGPUBindGroupEntry]) -> BindGroup {
    // SAFETY: the layout and the resources are alive; the records are borrowed.
    BindGroup(unsafe {
        wgpuDeviceCreateBindGroup(
            device,
            &WGPUBindGroupDescriptor {
                nextInChain: ptr::null_mut(),
                label: text("FerroUI 3D scene"),
                layout,
                entryCount: entries.len(),
                entries: entries.as_ptr(),
            },
        )
    })
}

/// The stride of records of `size` bytes in a buffer bound with dynamic offsets.
fn stride_of(size: usize) -> usize {
    size.div_ceil(UNIFORM_ALIGNMENT) * UNIFORM_ALIGNMENT
}

/// How a draw is blended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Blend {
    SourceOver,
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
#[derive(Clone, Copy)]
struct DrawRecord {
    mesh: (u64, u64),
    index_count: u32,
    blend: Blend,
    face: Face,
    depth_write: bool,
    brush: BrushShader,
    texture: Option<BrushTextureKey>,
}

impl DrawRecord {
    fn key(&self, sample_count: u32) -> PipelineKey {
        PipelineKey {
            blend: self.blend,
            face: self.face,
            depth_write: self.depth_write,
            sample_count,
            brush: self.brush,
        }
    }
}

/// The uniform buffers of a viewport with the bytes they hold, and the draws of its last render.
#[derive(Default)]
struct Viewport {
    frame_buffer: Option<(Buffer, BindGroup)>,
    draw_buffer: Option<(Buffer, BindGroup)>,
    brush_buffer: Option<Buffer>,
    draw_capacity: usize,
    frame_bytes: Vec<u8>,
    draw_bytes: Vec<u8>,
    brush_bytes: Vec<u8>,
    frame_scratch: Vec<u8>,
    draw_scratch: Vec<u8>,
    brush_scratch: Vec<u8>,
    draws: Vec<DrawRecord>,
}

struct MeshGpuBuffers {
    vertices: Buffer,
    vertex_capacity: u64,
    vertex_bytes: u64,
    indices: Buffer,
    index_capacity: u64,
}

struct MeshBuffers {
    /// `None` for a mesh without triangles.
    buffers: Option<MeshGpuBuffers>,
    index_count: u32,
    texture_bounds: Rect,
    last_used: u64,
}

struct BrushTexture {
    /// Keeps the address of a bitmap from naming another bitmap while the texture is kept; `None`
    /// for the empty texture.
    identity: Option<BrushTextureIdentity>,
    group: BindGroup,
    texture: OwnedTexture,
    size: (u32, u32),
    last_used: u64,
}

/// The textures of the pool for one size and sample count.
struct Targets {
    size: (u32, u32),
    sample_count: u32,
    multisampled: Option<OwnedTexture>,
    depth: OwnedTexture,
    resolved: Option<(BindGroup, OwnedTexture)>,
    last_used: u64,
}

/// What a render draws into.
struct RenderTarget {
    view: WGPUTextureView,
    texture_size: (u32, u32),
    size: (u32, u32),
    antialiased: bool,
}

/// The executor of one device.
struct Executor {
    module: Module,
    frame_layout: BindLayout,
    draw_layout: BindLayout,
    texture_layout: BindLayout,
    resolve_layout: BindLayout,
    pipeline_layout: PipelineLayout,
    textured_pipeline_layout: PipelineLayout,
    resolve_pipeline_layout: PipelineLayout,
    sampler: Sampler,
    pipelines: HashMap<PipelineKey, Pipeline>,
    resolve_pipeline: Option<Pipeline>,
    multisample: bool,
    draw_stride: usize,
    brush_stride: usize,
    meshes: HashMap<(u64, u64), MeshBuffers>,
    textures: HashMap<BrushTextureKey, BrushTexture>,
    refused_textures: HashMap<BrushTextureKey, u64>,
    empty_texture: Option<BrushTexture>,
    targets: Vec<Targets>,
    renders: u64,
    vertex_scratch: Vec<u8>,
    index_scratch: Vec<u32>,
    index_bytes: Vec<u8>,
}

impl Executor {
    fn new(device: &ProGpuDevice) -> Result<Self, String> {
        let raw = device.device();
        let (executor, error) = with_error_scope(device, || Self::create(raw));
        match error {
            Some(error) => Err(error),
            None => Ok(executor),
        }
    }

    fn create(raw: WGPUDevice) -> Self {
        let mut wgsl = WGPUShaderSourceWGSL {
            chain: WGPUChainedStruct { next: ptr::null_mut(), sType: WGPUSType_ShaderSourceWGSL },
            code: text(SCENE3D_WGSL),
        };
        // SAFETY: the chain outlives the call; the descriptor is borrowed.
        let module = Module(unsafe {
            wgpuDeviceCreateShaderModule(
                raw,
                &WGPUShaderModuleDescriptor { nextInChain: &mut wgsl.chain, label: text("FerroUI 3D scene") },
            )
        });
        let frame_layout = create_layout(raw, &[uniform_entry(0, false, std::mem::size_of::<FrameUniforms>())]);
        let draw_layout = create_layout(
            raw,
            &[
                uniform_entry(0, true, std::mem::size_of::<DrawUniforms>()),
                uniform_entry(BRUSH_BINDING, true, std::mem::size_of::<BrushUniforms>()),
            ],
        );
        // SAFETY: as in `uniform_entry`.
        let mut sampler_entry: WGPUBindGroupLayoutEntry = unsafe { std::mem::zeroed() };
        sampler_entry.binding = 1;
        sampler_entry.visibility = WGPUShaderStage_Fragment;
        sampler_entry.sampler.type_ = WGPUSamplerBindingType_Filtering;
        let texture_layout = create_layout(raw, &[texture_entry(0, WGPUTextureSampleType_Float), sampler_entry]);
        let resolve_layout =
            create_layout(raw, &[texture_entry(RESOLVED_BINDING, WGPUTextureSampleType_UnfilterableFloat)]);
        let pipeline_layout = create_pipeline_layout(raw, &[frame_layout.0, draw_layout.0]);
        let textured_pipeline_layout = create_pipeline_layout(raw, &[frame_layout.0, draw_layout.0, texture_layout.0]);
        let resolve_pipeline_layout = create_pipeline_layout(raw, &[resolve_layout.0]);
        // SAFETY: the call of the WebGPU C ABI with the device; the descriptor is borrowed.
        let sampler = Sampler(unsafe {
            wgpuDeviceCreateSampler(
                raw,
                &WGPUSamplerDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: text("FerroUI 3D scene brush"),
                    addressModeU: WGPUAddressMode_ClampToEdge,
                    addressModeV: WGPUAddressMode_ClampToEdge,
                    addressModeW: WGPUAddressMode_ClampToEdge,
                    magFilter: WGPUFilterMode_Linear,
                    minFilter: WGPUFilterMode_Linear,
                    mipmapFilter: WGPUMipmapFilterMode_Nearest,
                    lodMinClamp: 0.0,
                    lodMaxClamp: 32.0,
                    compare: 0,
                    maxAnisotropy: 1,
                },
            )
        });
        Executor {
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
            multisample: true,
            draw_stride: stride_of(std::mem::size_of::<DrawUniforms>()),
            brush_stride: stride_of(std::mem::size_of::<BrushUniforms>()),
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

    /// Draws `scene` into `target`. A failure with four samples is said once and the render is
    /// drawn with one; a failure with one is the error, and nothing is drawn.
    fn render(
        &mut self,
        device: &ProGpuDevice,
        viewport: &mut Viewport,
        scene: &Scene3DSnapshot,
        target: &RenderTarget,
    ) -> Result<(), String> {
        self.renders += 1;
        let samples = if target.antialiased && self.multisample { MULTISAMPLE_COUNT } else { 1 };
        let (result, error) = with_error_scope(device, || self.render_with(device, viewport, scene, target, samples));
        let failure = match (result, error) {
            (Ok(()), None) => return Ok(()),
            (Err(error), _) | (Ok(()), Some(error)) => error,
        };
        if samples == 1 {
            return Err(failure);
        }
        self.multisample = false;
        notes::note_once("scene3d-msaa", &format!("3D scenes are drawn without 4x MSAA on this device: {failure}"));
        let (result, error) = with_error_scope(device, || self.render_with(device, viewport, scene, target, 1));
        match (result, error) {
            (Ok(()), None) => Ok(()),
            (Err(error), _) | (Ok(()), Some(error)) => Err(error),
        }
    }

    fn render_with(
        &mut self,
        device: &ProGpuDevice,
        viewport: &mut Viewport,
        scene: &Scene3DSnapshot,
        target: &RenderTarget,
        sample_count: u32,
    ) -> Result<(), String> {
        let (raw, queue) = (device.device(), device.queue());
        let width = target.size.0.clamp(1, target.texture_size.0.max(1));
        let height = target.size.1.clamp(1, target.texture_size.1.max(1));

        let frame = FrameUniforms::of(scene);
        viewport.draws.clear();
        viewport.draw_scratch.clear();
        viewport.brush_scratch.clear();
        if frame.is_some() {
            for model in &scene.models {
                if model.front.is_empty() && model.back.is_empty() {
                    continue;
                }
                let (index_count, bounds) = self.mesh(raw, queue, &model.mesh);
                if index_count == 0 {
                    continue;
                }
                for (face, layers) in [(Face::Front, &model.front), (Face::Back, &model.back)] {
                    self.add_draws(raw, queue, viewport, model, layers, face, (index_count, bounds));
                }
            }
        }
        let additive = viewport.draws.iter().any(|draw| draw.blend == Blend::Additive);

        // The pipelines, before anything is encoded: a failure leaves the target as it was.
        for index in 0..viewport.draws.len() {
            let key = viewport.draws[index].key(sample_count);
            self.pipeline(device, key)?;
        }
        if additive {
            self.create_resolve_pipeline(device)?;
        }
        if let Some(frame) = &frame {
            self.write_uniforms(raw, queue, viewport, frame);
        }

        let renders = self.renders;
        self.meshes.retain(|_, mesh| renders - mesh.last_used < LIFETIME);
        self.targets.retain(|targets| renders - targets.last_used < LIFETIME);
        self.textures.retain(|_, texture| {
            let alive = texture.identity.as_ref().is_some_and(BrushTextureIdentity::is_alive);
            alive && renders - texture.last_used < LIFETIME
        });
        self.refused_textures.retain(|_, last| renders - *last < LIFETIME);
        let index = self.targets_of(raw, target.texture_size, sample_count, additive);
        let targets = &self.targets[index];

        let resolved = targets.resolved.as_ref().map(|(_, texture)| texture.view.0);
        let (color, resolve_target) = match (targets.multisampled.as_ref().map(|texture| texture.view.0), additive) {
            (Some(multisampled), false) => (multisampled, target.view),
            (Some(multisampled), true) => (multisampled, resolved.unwrap_or(ptr::null_mut())),
            (None, false) => (target.view, ptr::null_mut()),
            (None, true) => (resolved.unwrap_or(target.view), ptr::null_mut()),
        };
        let store = if targets.multisampled.is_some() { WGPUStoreOp_Discard } else { WGPUStoreOp_Store };
        let clear = WGPUColor { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
        // SAFETY: the calls of the WebGPU C ABI with live objects, which the executor and the
        // viewport keep until the commands are submitted; the records are borrowed.
        unsafe {
            let encoder = wgpuDeviceCreateCommandEncoder(raw, ptr::null());
            let attachment = WGPURenderPassColorAttachment {
                nextInChain: ptr::null_mut(),
                view: color,
                depthSlice: WGPU_DEPTH_SLICE_UNDEFINED,
                resolveTarget: resolve_target,
                loadOp: WGPULoadOp_Clear,
                storeOp: store,
                clearValue: clear,
            };
            // SAFETY: all-zero stencil operations are "undefined", as a format without stencil
            // requires.
            let mut depth: WGPURenderPassDepthStencilAttachment = std::mem::zeroed();
            depth.view = targets.depth.view.0;
            depth.depthLoadOp = WGPULoadOp_Clear;
            depth.depthStoreOp = WGPUStoreOp_Discard;
            depth.depthClearValue = 1.0;
            let pass = wgpuCommandEncoderBeginRenderPass(
                encoder,
                &WGPURenderPassDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: text("FerroUI 3D scene"),
                    colorAttachmentCount: 1,
                    colorAttachments: &attachment,
                    depthStencilAttachment: &depth,
                    occlusionQuerySet: ptr::null_mut(),
                    timestampWrites: ptr::null(),
                },
            );
            if let (Some((_, frame_group)), Some((_, draw_group))) = (&viewport.frame_buffer, &viewport.draw_buffer) {
                wgpuRenderPassEncoderSetViewport(pass, 0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
                wgpuRenderPassEncoderSetScissorRect(pass, 0, 0, width, height);
                wgpuRenderPassEncoderSetBindGroup(pass, 0, frame_group.0, 0, ptr::null());
                let mut current = None;
                for (index, draw) in viewport.draws.iter().enumerate() {
                    let key = draw.key(sample_count);
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
                            Some(texture) => Some(texture.group.0),
                            None => continue,
                        }
                    } else {
                        None
                    };
                    if current != Some(key) {
                        wgpuRenderPassEncoderSetPipeline(pass, pipeline.0);
                        current = Some(key);
                    }
                    let offsets = [(index * self.draw_stride) as u32, (index * self.brush_stride) as u32];
                    wgpuRenderPassEncoderSetBindGroup(pass, 1, draw_group.0, 2, offsets.as_ptr());
                    if let Some(group) = texture_group {
                        wgpuRenderPassEncoderSetBindGroup(pass, BRUSH_TEXTURE_GROUP, group, 0, ptr::null());
                    }
                    wgpuRenderPassEncoderSetVertexBuffer(pass, 0, buffers.vertices.0, 0, buffers.vertex_bytes);
                    wgpuRenderPassEncoderSetIndexBuffer(
                        pass,
                        buffers.indices.0,
                        WGPUIndexFormat_Uint32,
                        0,
                        u64::from(draw.index_count) * 4,
                    );
                    wgpuRenderPassEncoderDrawIndexed(pass, draw.index_count, 1, 0, 0, 0);
                }
            }
            wgpuRenderPassEncoderEnd(pass);
            wgpuRenderPassEncoderRelease(pass);
            if additive {
                if let (Some(pipeline), Some((group, _))) = (&self.resolve_pipeline, &targets.resolved) {
                    let attachment = WGPURenderPassColorAttachment {
                        nextInChain: ptr::null_mut(),
                        view: target.view,
                        depthSlice: WGPU_DEPTH_SLICE_UNDEFINED,
                        resolveTarget: ptr::null_mut(),
                        loadOp: WGPULoadOp_Clear,
                        storeOp: WGPUStoreOp_Store,
                        clearValue: clear,
                    };
                    let pass = wgpuCommandEncoderBeginRenderPass(
                        encoder,
                        &WGPURenderPassDescriptor {
                            nextInChain: ptr::null_mut(),
                            label: text("FerroUI 3D scene resolve"),
                            colorAttachmentCount: 1,
                            colorAttachments: &attachment,
                            depthStencilAttachment: ptr::null(),
                            occlusionQuerySet: ptr::null_mut(),
                            timestampWrites: ptr::null(),
                        },
                    );
                    wgpuRenderPassEncoderSetViewport(pass, 0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
                    wgpuRenderPassEncoderSetScissorRect(pass, 0, 0, width, height);
                    wgpuRenderPassEncoderSetPipeline(pass, pipeline.0);
                    wgpuRenderPassEncoderSetBindGroup(pass, 0, group.0, 0, ptr::null());
                    wgpuRenderPassEncoderDraw(pass, 3, 1, 0, 0);
                    wgpuRenderPassEncoderEnd(pass);
                    wgpuRenderPassEncoderRelease(pass);
                }
            }
            let commands = wgpuCommandEncoderFinish(encoder, ptr::null());
            wgpuQueueSubmit(queue, 1, &commands);
            wgpuCommandBufferRelease(commands);
            wgpuCommandEncoderRelease(encoder);
        }
        Ok(())
    }

    /// The draws of the layers of one side of a model. The first diffuse layer writes depth (the
    /// first layer when the side has no diffuse layer).
    #[allow(clippy::too_many_arguments)]
    fn add_draws(
        &mut self,
        raw: WGPUDevice,
        queue: WGPUQueue,
        viewport: &mut Viewport,
        model: &Scene3DModel,
        layers: &[Scene3DMaterialLayer],
        face: Face,
        (index_count, texture_bounds): (u32, Rect),
    ) {
        let depth_layer = layers.iter().position(|layer| LayerKind::of(layer.kind) == LayerKind::Diffuse).unwrap_or(0);
        for (index, layer) in layers.iter().enumerate() {
            let (uniforms, _) = DrawUniforms::of(model, layer, face);
            let mut brush = MaterialBrush::of(layer.brush.as_ref(), texture_bounds);
            let mut texture = None;
            if let Some(source) = brush.texture().cloned() {
                if self.brush_texture(raw, queue, &source) {
                    texture = Some(source.key());
                } else {
                    brush = brush.to_unsampled();
                }
            } else if brush.shader() == BrushShader::Image {
                self.create_empty_texture(raw, queue);
            }
            if brush.is_unsampled() {
                notes::note_once("scene3d-brush", "a 3D layer whose brush cannot be sampled is drawn with its colour");
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
                blend: if uniforms.kind() == LayerKind::Diffuse { Blend::SourceOver } else { Blend::Additive },
                face,
                depth_write: index == depth_layer,
                brush: brush.shader(),
                texture,
            });
        }
    }

    /// The texture of `source`, uploaded when its key is new: into the texture of an older
    /// version of the same bitmap no draw of this render uses when it has the same size.
    fn brush_texture(&mut self, raw: WGPUDevice, queue: WGPUQueue, source: &BrushTextureSource) -> bool {
        let key = source.key();
        let renders = self.renders;
        if let Some(texture) = self.textures.get_mut(&key) {
            texture.last_used = renders;
            return true;
        }
        if let Some(last) = self.refused_textures.get_mut(&key) {
            *last = renders;
            return false;
        }
        let texels = source.texels().filter(|texels| {
            let fits = texels.width <= MAX_TEXTURE_SIDE && texels.height <= MAX_TEXTURE_SIDE;
            texels.width > 0 && texels.height > 0 && fits
        });
        let Some(texels) = texels else {
            self.refused_textures.insert(key, renders);
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
                })
                .map(|(other, _)| *other),
            _ => None,
        };
        let replaced = replaced_key.and_then(|other| self.textures.remove(&other));
        let texture = match replaced {
            Some(texture) => BrushTexture { identity: Some(source.identity()), last_used: renders, ..texture },
            None => self.create_brush_texture(raw, Some(source.identity()), size),
        };
        write_texels(queue, texture.texture.texture.0, size, &texels.rgba);
        self.textures.insert(key, texture);
        true
    }

    fn create_empty_texture(&mut self, raw: WGPUDevice, queue: WGPUQueue) {
        if self.empty_texture.is_none() {
            let texture = self.create_brush_texture(raw, None, (1, 1));
            write_texels(queue, texture.texture.texture.0, (1, 1), &[0; 4]);
            self.empty_texture = Some(texture);
        }
    }

    fn create_brush_texture(
        &self,
        raw: WGPUDevice,
        identity: Option<BrushTextureIdentity>,
        size: (u32, u32),
    ) -> BrushTexture {
        let texture =
            create_texture(raw, size, COLOR_FORMAT, 1, WGPUTextureUsage_TextureBinding | WGPUTextureUsage_CopyDst);
        // SAFETY: all-zero is a valid record: no chain, no resource yet.
        let (mut view, mut sampler): (WGPUBindGroupEntry, WGPUBindGroupEntry) =
            unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
        view.binding = 0;
        view.textureView = texture.view.0;
        sampler.binding = 1;
        sampler.sampler = self.sampler.0;
        let group = create_group(raw, self.texture_layout.0, &[view, sampler]);
        BrushTexture { identity, group, texture, size, last_used: self.renders }
    }

    /// The buffers of `mesh`, uploaded when its `(id, version)` is new; the number of its indices
    /// and the box of its texture coordinates.
    fn mesh(&mut self, raw: WGPUDevice, queue: WGPUQueue, mesh: &MeshData) -> (u32, Rect) {
        let key = (mesh.id, mesh.version);
        let renders = self.renders;
        if let Some(buffers) = self.meshes.get_mut(&key) {
            buffers.last_used = renders;
            return (buffers.index_count, buffers.texture_bounds);
        }
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
                _ => MeshGpuBuffers {
                    vertices: create_buffer(raw, vertex_bytes, WGPUBufferUsage_Vertex),
                    vertex_capacity: vertex_bytes,
                    vertex_bytes,
                    indices: create_buffer(raw, index_bytes, WGPUBufferUsage_Index),
                    index_capacity: index_bytes,
                },
            };
            write_buffer(queue, buffers.vertices.0, 0, &self.vertex_scratch);
            write_buffer(queue, buffers.indices.0, 0, &self.index_bytes);
            Some(buffers)
        };
        let texture_bounds = texture_coordinate_bounds(mesh);
        self.meshes.insert(key, MeshBuffers { buffers, index_count, texture_bounds, last_used: renders });
        (index_count, texture_bounds)
    }

    fn pipeline(&mut self, device: &ProGpuDevice, key: PipelineKey) -> Result<(), String> {
        if self.pipelines.contains_key(&key) {
            return Ok(());
        }
        let layout = if samples_texture(key.brush) { &self.textured_pipeline_layout } else { &self.pipeline_layout };
        let add = WGPUBlendComponent {
            operation: WGPUBlendOperation_Add,
            srcFactor: WGPUBlendFactor_One,
            dstFactor: WGPUBlendFactor_One,
        };
        let over = WGPUBlendComponent { dstFactor: WGPUBlendFactor_OneMinusSrcAlpha, ..add };
        let blend = match key.blend {
            Blend::SourceOver => WGPUBlendState { color: over, alpha: over },
            Blend::Additive => WGPUBlendState { color: add, alpha: add },
        };
        let attribute = |format: WGPUVertexFormat, location: usize| WGPUVertexAttribute {
            nextInChain: ptr::null_mut(),
            format,
            offset: VERTEX_ATTRIBUTE_OFFSETS[location] as u64,
            shaderLocation: location as u32,
        };
        let attributes = [
            attribute(WGPUVertexFormat_Float32x3, 0),
            attribute(WGPUVertexFormat_Float32x3, 1),
            attribute(WGPUVertexFormat_Float32x2, 2),
        ];
        let vertex_layout = WGPUVertexBufferLayout {
            nextInChain: ptr::null_mut(),
            stepMode: WGPUVertexStepMode_Vertex,
            arrayStride: VERTEX_STRIDE as u64,
            attributeCount: attributes.len(),
            attributes: attributes.as_ptr(),
        };
        let stencil = WGPUStencilFaceState {
            compare: WGPUCompareFunction_Always,
            failOp: WGPUStencilOperation_Keep,
            depthFailOp: WGPUStencilOperation_Keep,
            passOp: WGPUStencilOperation_Keep,
        };
        let depth = WGPUDepthStencilState {
            nextInChain: ptr::null_mut(),
            format: DEPTH_FORMAT,
            depthWriteEnabled: if key.depth_write { WGPUOptionalBool_True } else { WGPUOptionalBool_False },
            depthCompare: if key.depth_write { WGPUCompareFunction_Less } else { WGPUCompareFunction_LessEqual },
            stencilFront: stencil,
            stencilBack: stencil,
            stencilReadMask: 0,
            stencilWriteMask: 0,
            depthBias: 0,
            depthBiasSlopeScale: 0.0,
            depthBiasClamp: 0.0,
        };
        let color = WGPUColorTargetState {
            nextInChain: ptr::null_mut(),
            format: COLOR_FORMAT,
            blend: &blend,
            writeMask: WGPUColorWriteMask_All,
        };
        let fragment = WGPUFragmentState {
            nextInChain: ptr::null_mut(),
            module: self.module.0,
            entryPoint: text(fragment_entry(key.brush)),
            constantCount: 0,
            constants: ptr::null(),
            targetCount: 1,
            targets: &color,
        };
        let descriptor = WGPURenderPipelineDescriptor {
            nextInChain: ptr::null_mut(),
            label: text("FerroUI 3D scene"),
            layout: layout.0,
            vertex: WGPUVertexState {
                nextInChain: ptr::null_mut(),
                module: self.module.0,
                entryPoint: text(VERTEX_ENTRY),
                constantCount: 0,
                constants: ptr::null(),
                bufferCount: 1,
                buffers: &vertex_layout,
            },
            primitive: WGPUPrimitiveState {
                nextInChain: ptr::null_mut(),
                topology: WGPUPrimitiveTopology_TriangleList,
                stripIndexFormat: 0,
                // Counterclockwise on the screen is a front face (WPF).
                frontFace: WGPUFrontFace_CCW,
                cullMode: match key.face {
                    Face::Front => WGPUCullMode_Back,
                    Face::Back => WGPUCullMode_Front,
                },
                unclippedDepth: 0,
            },
            depthStencil: &depth,
            multisample: WGPUMultisampleState {
                nextInChain: ptr::null_mut(),
                count: key.sample_count,
                mask: u32::MAX,
                alphaToCoverageEnabled: 0,
            },
            fragment: &fragment,
        };
        // SAFETY: every record the descriptor points to lives until the call returns.
        let create = || Pipeline(unsafe { wgpuDeviceCreateRenderPipeline(device.device(), &descriptor) });
        let (pipeline, error) = with_error_scope(device, create);
        if let Some(error) = error {
            return Err(format!("the pipeline {key:?} cannot be made: {error}"));
        }
        self.pipelines.insert(key, pipeline);
        Ok(())
    }

    fn create_resolve_pipeline(&mut self, device: &ProGpuDevice) -> Result<(), String> {
        if self.resolve_pipeline.is_some() {
            return Ok(());
        }
        let color = WGPUColorTargetState {
            nextInChain: ptr::null_mut(),
            format: COLOR_FORMAT,
            blend: ptr::null(),
            writeMask: WGPUColorWriteMask_All,
        };
        let fragment = WGPUFragmentState {
            nextInChain: ptr::null_mut(),
            module: self.module.0,
            entryPoint: text(RESOLVE_FRAGMENT_ENTRY),
            constantCount: 0,
            constants: ptr::null(),
            targetCount: 1,
            targets: &color,
        };
        let descriptor = WGPURenderPipelineDescriptor {
            nextInChain: ptr::null_mut(),
            label: text("FerroUI 3D scene resolve"),
            layout: self.resolve_pipeline_layout.0,
            vertex: WGPUVertexState {
                nextInChain: ptr::null_mut(),
                module: self.module.0,
                entryPoint: text(RESOLVE_VERTEX_ENTRY),
                constantCount: 0,
                constants: ptr::null(),
                bufferCount: 0,
                buffers: ptr::null(),
            },
            primitive: WGPUPrimitiveState {
                nextInChain: ptr::null_mut(),
                topology: WGPUPrimitiveTopology_TriangleList,
                stripIndexFormat: 0,
                frontFace: WGPUFrontFace_CCW,
                cullMode: WGPUCullMode_None,
                unclippedDepth: 0,
            },
            depthStencil: ptr::null(),
            multisample: WGPUMultisampleState {
                nextInChain: ptr::null_mut(),
                count: 1,
                mask: u32::MAX,
                alphaToCoverageEnabled: 0,
            },
            fragment: &fragment,
        };
        // SAFETY: every record the descriptor points to lives until the call returns.
        let create = || Pipeline(unsafe { wgpuDeviceCreateRenderPipeline(device.device(), &descriptor) });
        let (pipeline, error) = with_error_scope(device, create);
        if let Some(error) = error {
            return Err(format!("the resolve pipeline cannot be made: {error}"));
        }
        self.resolve_pipeline = Some(pipeline);
        Ok(())
    }

    /// Writes the uniforms that differ from what the buffers of `viewport` hold, making or growing
    /// the buffers first.
    fn write_uniforms(&self, raw: WGPUDevice, queue: WGPUQueue, viewport: &mut Viewport, frame: &FrameUniforms) {
        viewport.frame_scratch.clear();
        frame.write_std140(&mut viewport.frame_scratch);
        if viewport.frame_buffer.is_none() {
            let size = viewport.frame_scratch.len() as u64;
            let buffer = create_buffer(raw, size, WGPUBufferUsage_Uniform);
            let group = create_group(raw, self.frame_layout.0, &[buffer_entry(0, buffer.0, size)]);
            viewport.frame_buffer = Some((buffer, group));
            viewport.frame_bytes.clear();
        }
        if viewport.frame_bytes != viewport.frame_scratch {
            if let Some((buffer, _)) = &viewport.frame_buffer {
                write_buffer(queue, buffer.0, 0, &viewport.frame_scratch);
            }
            viewport.frame_bytes.clone_from(&viewport.frame_scratch);
        }

        let draws = viewport.draws.len().max(1);
        if viewport.draw_buffer.is_none() || viewport.brush_buffer.is_none() || viewport.draw_capacity < draws {
            let capacity = draws.next_power_of_two();
            let draw_buffer = create_buffer(raw, (capacity * self.draw_stride) as u64, WGPUBufferUsage_Uniform);
            let brush_buffer = create_buffer(raw, (capacity * self.brush_stride) as u64, WGPUBufferUsage_Uniform);
            let group = create_group(
                raw,
                self.draw_layout.0,
                &[
                    buffer_entry(0, draw_buffer.0, std::mem::size_of::<DrawUniforms>() as u64),
                    buffer_entry(BRUSH_BINDING, brush_buffer.0, std::mem::size_of::<BrushUniforms>() as u64),
                ],
            );
            viewport.draw_buffer = Some((draw_buffer, group));
            viewport.brush_buffer = Some(brush_buffer);
            viewport.draw_capacity = capacity;
            viewport.draw_bytes.clear();
            viewport.brush_bytes.clear();
        }
        if let Some((buffer, _)) = &viewport.draw_buffer {
            write_changed(queue, buffer.0, &viewport.draw_bytes, &viewport.draw_scratch, self.draw_stride);
        }
        if let Some(buffer) = &viewport.brush_buffer {
            write_changed(queue, buffer.0, &viewport.brush_bytes, &viewport.brush_scratch, self.brush_stride);
        }
        viewport.draw_bytes.clone_from(&viewport.draw_scratch);
        viewport.brush_bytes.clone_from(&viewport.brush_scratch);
    }

    /// The index of the targets of the pool for `size` and `sample_count`.
    fn targets_of(&mut self, raw: WGPUDevice, size: (u32, u32), sample_count: u32, resolved: bool) -> usize {
        let renders = self.renders;
        let index = match self.targets.iter().position(|t| t.size == size && t.sample_count == sample_count) {
            Some(index) => index,
            None => {
                let usage = WGPUTextureUsage_RenderAttachment;
                let multisampled =
                    (sample_count > 1).then(|| create_texture(raw, size, COLOR_FORMAT, sample_count, usage));
                let depth = create_texture(raw, size, DEPTH_FORMAT, sample_count, usage);
                let last_used = renders;
                self.targets.push(Targets { size, sample_count, multisampled, depth, resolved: None, last_used });
                self.targets.len() - 1
            }
        };
        if resolved && self.targets[index].resolved.is_none() {
            let usage = WGPUTextureUsage_RenderAttachment | WGPUTextureUsage_TextureBinding;
            let texture = create_texture(raw, size, COLOR_FORMAT, 1, usage);
            // SAFETY: all-zero is a valid record: no chain, no resource yet.
            let mut entry: WGPUBindGroupEntry = unsafe { std::mem::zeroed() };
            entry.binding = RESOLVED_BINDING;
            entry.textureView = texture.view.0;
            let group = create_group(raw, self.resolve_layout.0, &[entry]);
            self.targets[index].resolved = Some((group, texture));
        }
        self.targets[index].last_used = renders;
        index
    }
}

/// Writes the one range of records of `stride` bytes from the first to the last that differ.
fn write_changed(queue: WGPUQueue, buffer: WGPUBuffer, old: &[u8], new: &[u8], stride: usize) {
    let changed = |index: usize| {
        let range = index * stride..(index + 1) * stride;
        old.get(range.clone()) != new.get(range)
    };
    let records = new.len() / stride.max(1);
    let Some(first) = (0..records).find(|&index| changed(index)) else { return };
    let last = (first..records).rev().find(|&index| changed(index)).unwrap_or(first);
    let range = first * stride..(last + 1) * stride;
    write_buffer(queue, buffer, range.start as u64, &new[range]);
}

/// What a finished output shows: the same stamp of the same scene is the same pixels.
#[derive(Clone, Copy, PartialEq)]
struct OutputStamp {
    generations: (u64, u64, u64),
    size: (u32, u32),
    scale_bits: u64,
    antialiased: bool,
    inputs: u64,
}

/// A place in a window and the pixels of it.
type OutputPlace = ((i32, i32), (u32, u32));

/// How an output is reused for another render, the lowest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Reuse {
    /// The same place, for the same scene or for a scene that is gone.
    SamePlaceFree,
    /// The same place, now for another scene.
    SamePlace,
    /// Elsewhere, for a scene that is gone.
    Free,
    /// In use elsewhere: taken only when no output can be added.
    InUse,
}

/// The texture of a scene on the device, with the uniform buffers of its viewport.
struct KeptOutput {
    source: Weak<Scene3DSnapshot>,
    /// `None` while it holds no finished image.
    stamp: Option<OutputStamp>,
    place: OutputPlace,
    texture: Rc<GpuTexture>,
    viewport: Viewport,
    last_used: u64,
}

impl KeptOutput {
    fn is_of(&self, scene: &Arc<Scene3DSnapshot>) -> bool {
        ptr::eq(self.source.as_ptr(), Arc::as_ptr(scene))
    }

    fn reuse(&self, scene: &Arc<Scene3DSnapshot>, place: OutputPlace) -> Reuse {
        let free = self.source.strong_count() == 0;
        match (self.place == place, free || self.is_of(scene)) {
            (true, true) => Reuse::SamePlaceFree,
            (true, false) => Reuse::SamePlace,
            (false, _) if free => Reuse::Free,
            (false, _) => Reuse::InUse,
        }
    }
}

/// The executor of a device and the outputs it keeps.
#[derive(Default)]
pub(crate) struct Scene3DPasses {
    /// Made at the first scene; an executor that cannot be made is said once.
    executor: RefCell<Option<Option<Executor>>>,
    outputs: RefCell<Vec<KeptOutput>>,
    clock: std::cell::Cell<u64>,
}

/// A scene drawn on the device: the texture it is in, and the pixels of the image at its top left.
pub(crate) struct DrawnScene {
    pub(crate) texture: Rc<GpuTexture>,
    pub(crate) size: (u32, u32),
}

impl Scene3DPasses {
    /// The texture that holds `scene` drawn at `size` pixels: a kept output when nothing changed,
    /// otherwise a new render. `None` when the device cannot draw it (said once).
    pub(crate) fn scene_texture(
        &self,
        gpu: &Gpu,
        scene: &Arc<Scene3DSnapshot>,
        size: PixelSize,
        scale: f64,
        antialiased: bool,
        origin: (i32, i32),
    ) -> Option<DrawnScene> {
        let size = (size.width.max(1) as u32, size.height.max(1) as u32);
        let stamp = OutputStamp {
            generations: scene.generations(),
            size,
            scale_bits: scale.to_bits(),
            antialiased,
            inputs: scene_texture_inputs(scene),
        };
        let mut executor = self.executor.borrow_mut();
        let executor = executor
            .get_or_insert_with(|| {
                Executor::new(gpu.device())
                    .map_err(|error| {
                        notes::note_once("scene3d-executor", &format!("3D scenes are not drawn on this device: {error}"))
                    })
                    .ok()
            })
            .as_mut()?;
        let now = self.clock.get() + 1;
        self.clock.set(now);
        let mut outputs = self.outputs.borrow_mut();
        if let Some(kept) = outputs.iter_mut().find(|kept| kept.stamp == Some(stamp) && kept.is_of(scene)) {
            kept.last_used = now;
            return Some(DrawnScene { texture: kept.texture.clone(), size });
        }

        let place = (origin, size);
        let best = (0..outputs.len()).min_by_key(|&index| (outputs[index].reuse(scene, place), outputs[index].last_used));
        let index = match best {
            Some(index) if outputs.len() >= CAPACITY || outputs[index].reuse(scene, place) != Reuse::InUse => index,
            _ => {
                outputs.push(KeptOutput {
                    source: Weak::new(),
                    stamp: None,
                    place,
                    texture: gpu.passes().texture(gpu, size.0, size.1),
                    viewport: Viewport::default(),
                    last_used: now,
                });
                outputs.len() - 1
            }
        };
        let output = &mut outputs[index];
        // A texture too small, or one a scene that is not rendered yet still samples (held by more
        // than the pool and this output): another of the pool.
        let held_elsewhere = Rc::strong_count(&output.texture) > 2;
        if held_elsewhere || output.texture.width() < size.0 || output.texture.height() < size.1 {
            output.texture = gpu.passes().texture(gpu, size.0, size.1);
        }
        // Until the render ends the output holds no whole frame.
        output.stamp = None;
        output.source = Weak::new();
        let target = RenderTarget {
            view: output.texture.view(),
            texture_size: (output.texture.width(), output.texture.height()),
            size,
            antialiased,
        };
        if let Err(error) = executor.render(gpu.device(), &mut output.viewport, scene, &target) {
            notes::note_once("scene3d-render", &format!("a 3D scene was not drawn: {error}"));
            return None;
        }
        output.texture.changed();
        output.stamp = Some(stamp);
        output.source = Arc::downgrade(scene);
        output.place = place;
        output.last_used = now;
        Some(DrawnScene { texture: output.texture.clone(), size })
    }
}
