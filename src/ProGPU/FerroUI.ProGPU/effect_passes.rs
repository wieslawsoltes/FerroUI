//! The render passes of the layers with an effect (docs/porting/progpu-backend.md, section 2.7).
//!
//! What the engine has no group effect for is a pass of this backend on the device of the frame:
//! the drawing context renders the content of the layer into a texture of the pool
//! (`drawing_context/effect_layers.rs`), a pass reads it and writes another texture of the pool,
//! and the scene the layer was pushed in samples the result. Nothing is read back or drawn on the
//! processor.
//!
//! A program is compiled once per device (and per type of a custom effect); the uniforms are
//! written for each pass. A texture of the pool is in use while something else holds it, so a
//! texture a scene samples comes back after that scene is rendered and steady frames allocate
//! nothing.

use crate::device::{thread_is_ending, ProGpuDevice};
use crate::effect_programs::*;
use crate::effect_uniforms as blocks;
use crate::gpu::{Gpu, GpuTexture};
use crate::notes;
use crate::scene::SceneBuilder;
use ferroui_base::media::effects::{
    EffectImage, EffectImageWeak, ImmutableCustomEffect, ShaderUniformType, MAX_SHADER_CHILDREN,
    SHADER_WGSL_CHILDREN_BINDING, SHADER_WGSL_CONTENT_BINDING, SHADER_WGSL_PASS_BINDING, SHADER_WGSL_UNIFORMS_BINDING,
};
use ferroui_base::platform::MultiplyLayerFactors;
use progpu_native_sys::webgpu::*;
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::ptr;
use std::rc::Rc;
use std::sync::{Arc, Weak};

/// The sides of the textures of the pool are multiples of this.
const POOL_STEP: u32 = 32;
/// The largest side the pool rounds up to.
const POOL_MAX_SIDE: u32 = 8192;
/// A free texture of the pool unused for this many frames goes.
const POOL_KEPT_FRAMES: u64 = 120;
/// The most scene builders kept for the content of layers.
const MAX_SCENES: usize = 8;
/// The most bind groups a program keeps.
const MAX_BIND_GROUPS: usize = 64;
/// The most textures of multiply factors kept.
const MAX_FACTOR_TEXTURES: usize = 8;

/// A program of a layer with an effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum LayerProgram {
    Lighting,
    ColorMatrix,
    LightMultiply,
    FactorMultiply,
    PixelOperation,
    Convolution,
}

impl LayerProgram {
    fn source(self) -> String {
        match self {
            LayerProgram::Lighting => format!("{LIGHT_COMMON_WGSL}{LIGHTING_FRAGMENT_WGSL}"),
            LayerProgram::ColorMatrix => format!("{LAYER_VERTEX_WGSL}{COLOR_MATRIX_WGSL}"),
            LayerProgram::LightMultiply => format!("{LIGHT_COMMON_WGSL}{LIGHT_MULTIPLY_FRAGMENT_WGSL}"),
            LayerProgram::FactorMultiply => format!("{LAYER_VERTEX_WGSL}{FACTOR_MULTIPLY_WGSL}"),
            LayerProgram::PixelOperation => format!("{LAYER_VERTEX_WGSL}{PIXEL_OPERATION_WGSL}"),
            LayerProgram::Convolution => format!("{LAYER_VERTEX_WGSL}{CONVOLUTION_WGSL}"),
        }
    }

    /// The textures the program reads.
    fn inputs(self) -> usize {
        match self {
            LayerProgram::Lighting => 0,
            LayerProgram::FactorMultiply | LayerProgram::PixelOperation => 2,
            _ => 1,
        }
    }

    /// The floats of the uniform block.
    fn uniform_floats(self) -> usize {
        match self {
            LayerProgram::Lighting | LayerProgram::LightMultiply => blocks::LIGHTING_FLOATS,
            LayerProgram::ColorMatrix => blocks::COLOR_MATRIX_FLOATS,
            LayerProgram::FactorMultiply => blocks::FACTOR_MULTIPLY_FLOATS,
            LayerProgram::PixelOperation => blocks::PIXEL_OPERATION_FLOATS,
            LayerProgram::Convolution => blocks::CONVOLUTION_FLOATS,
        }
    }
}

/// What the validation of a pipeline said; on a page it says so later, from the event loop.
#[derive(Default)]
struct PipelineCheck {
    done: Cell<bool>,
    error: RefCell<Option<String>>,
}

unsafe extern "C" fn on_pipeline_checked(
    status: WGPUPopErrorScopeStatus,
    kind: WGPUErrorType,
    message: WGPUStringView,
    check: *mut c_void,
    _: *mut c_void,
) {
    // SAFETY: the reference `create_pipeline` gave up, and the callback runs once.
    let check = unsafe { Rc::from_raw(check as *const PipelineCheck) };
    if status != WGPUPopErrorScopeStatus_Success || kind != WGPUErrorType_NoError {
        let text = if message.data.is_null() || message.length == 0 {
            "the device refused the program".to_owned()
        } else {
            // SAFETY: the message is valid for the duration of the callback.
            let bytes = unsafe { std::slice::from_raw_parts(message.data as *const u8, message.length) };
            String::from_utf8_lossy(bytes).into_owned()
        };
        *check.error.borrow_mut() = Some(text);
    }
    check.done.set(true);
}

fn text(value: &str) -> WGPUStringView {
    WGPUStringView { data: value.as_ptr() as *const c_char, length: value.len() }
}

/// One binding of a pass, in the order of the layout.
enum Slot {
    Uniform(u32),
    Texture(u32),
}

fn layout_entry(slot: &Slot) -> WGPUBindGroupLayoutEntry {
    // SAFETY: all-zero is a valid record: every binding kind is "not used", the chain is empty.
    let mut entry: WGPUBindGroupLayoutEntry = unsafe { std::mem::zeroed() };
    entry.visibility = WGPUShaderStage_Fragment;
    match *slot {
        Slot::Uniform(binding) => {
            entry.binding = binding;
            entry.buffer.type_ = WGPUBufferBindingType_Uniform;
        }
        Slot::Texture(binding) => {
            entry.binding = binding;
            entry.texture.sampleType = WGPUTextureSampleType_UnfilterableFloat;
            entry.texture.viewDimension = WGPUTextureViewDimension_2D;
        }
    }
    entry
}

/// The compiled pipeline of a program with its uniform buffers and the bind groups it was run with.
struct Program {
    pipeline: WGPURenderPipeline,
    layout: WGPUBindGroupLayout,
    buffers: Vec<(WGPUBuffer, u64)>,
    bind_groups: Vec<(BindKey, WGPUBindGroup)>,
    check: Rc<PipelineCheck>,
}

/// The identities of the textures of a bind group.
type BindKey = [u64; 1 + MAX_SHADER_CHILDREN];

impl Program {
    /// Compiles `source` for targets of `RGBA8Unorm`, under an error scope: a program the device
    /// refuses is an error here and not a failure of the device.
    fn new(device: &ProGpuDevice, source: &str, slots: &[Slot], buffers: &[u64]) -> Option<Self> {
        let raw = device.device();
        let entries: Vec<WGPUBindGroupLayoutEntry> = slots.iter().map(layout_entry).collect();
        let check = Rc::new(PipelineCheck::default());
        // SAFETY: the calls of the WebGPU C ABI with the device; every record outlives its call,
        // and the objects made here are owned by the program or released below.
        unsafe {
            wgpuDevicePushErrorScope(raw, WGPUErrorFilter_Validation);
            let mut wgsl = WGPUShaderSourceWGSL {
                chain: WGPUChainedStruct { next: ptr::null_mut(), sType: WGPUSType_ShaderSourceWGSL },
                code: text(source),
            };
            let module = wgpuDeviceCreateShaderModule(
                raw,
                &WGPUShaderModuleDescriptor { nextInChain: &mut wgsl.chain, label: text("FerroUI effect") },
            );
            let layout = wgpuDeviceCreateBindGroupLayout(
                raw,
                &WGPUBindGroupLayoutDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: text("FerroUI effect"),
                    entryCount: entries.len(),
                    entries: entries.as_ptr(),
                },
            );
            let pipeline_layout = wgpuDeviceCreatePipelineLayout(
                raw,
                &WGPUPipelineLayoutDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: text("FerroUI effect"),
                    bindGroupLayoutCount: 1,
                    bindGroupLayouts: &layout,
                    immediateSize: 0,
                },
            );
            let target = WGPUColorTargetState {
                nextInChain: ptr::null_mut(),
                format: WGPUTextureFormat_RGBA8Unorm,
                blend: ptr::null(),
                writeMask: WGPUColorWriteMask_All,
            };
            let fragment = WGPUFragmentState {
                nextInChain: ptr::null_mut(),
                module,
                entryPoint: text("fs_main"),
                constantCount: 0,
                constants: ptr::null(),
                targetCount: 1,
                targets: &target,
            };
            let pipeline = wgpuDeviceCreateRenderPipeline(
                raw,
                &WGPURenderPipelineDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: text("FerroUI effect"),
                    layout: pipeline_layout,
                    vertex: WGPUVertexState {
                        nextInChain: ptr::null_mut(),
                        module,
                        entryPoint: text("vs_main"),
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
                },
            );
            let reference = Rc::into_raw(check.clone());
            wgpuDevicePopErrorScope(
                raw,
                WGPUPopErrorScopeCallbackInfo {
                    nextInChain: ptr::null_mut(),
                    mode: WGPUCallbackMode_AllowSpontaneous,
                    callback: Some(on_pipeline_checked),
                    userdata1: reference as *mut c_void,
                    userdata2: ptr::null_mut(),
                },
            );
            #[cfg(not(target_os = "emscripten"))]
            if !check.done.get() {
                wgpuDevicePoll(raw, 1, ptr::null());
            }
            if !pipeline_layout.is_null() {
                wgpuPipelineLayoutRelease(pipeline_layout);
            }
            if !module.is_null() {
                wgpuShaderModuleRelease(module);
            }
            let refused = check.error.borrow().is_some() || pipeline.is_null();
            if refused {
                if !pipeline.is_null() {
                    wgpuRenderPipelineRelease(pipeline);
                }
                if !layout.is_null() {
                    wgpuBindGroupLayoutRelease(layout);
                }
                let reason = check.error.borrow().clone().unwrap_or_else(|| "no pipeline".to_owned());
                notes::failure(&format!("an effect program was refused: {reason}"));
                return None;
            }
            let buffers = buffers
                .iter()
                .map(|&size| {
                    let buffer = wgpuDeviceCreateBuffer(
                        raw,
                        &WGPUBufferDescriptor {
                            nextInChain: ptr::null_mut(),
                            label: text("FerroUI effect"),
                            usage: WGPUBufferUsage_Uniform | WGPUBufferUsage_CopyDst,
                            size,
                            mappedAtCreation: 0,
                        },
                    );
                    (buffer, size)
                })
                .collect();
            Some(Program { pipeline, layout, buffers, bind_groups: Vec::new(), check })
        }
    }

    /// Whether the device refused the program after it was made (on a page, where the answer
    /// comes from the event loop).
    fn refused(&self) -> bool {
        self.check.error.borrow().is_some()
    }

    fn clear_bind_groups(&mut self) {
        for (_, group) in self.bind_groups.drain(..) {
            if !thread_is_ending() {
                // SAFETY: the group is owned here.
                unsafe { wgpuBindGroupRelease(group) };
            }
        }
    }

    /// The bind group of the buffers and `textures`, in the order of `slots`, kept by the
    /// identities of the textures (the textures of the pool, the same in steady frames).
    fn bind_group(&mut self, device: &ProGpuDevice, slots: &[Slot], textures: &[&GpuTexture]) -> WGPUBindGroup {
        let mut key: BindKey = [0; 1 + MAX_SHADER_CHILDREN];
        for (slot, texture) in key.iter_mut().zip(textures) {
            *slot = texture.id();
        }
        if let Some((_, group)) = self.bind_groups.iter().find(|(bound, _)| *bound == key) {
            return *group;
        }
        if self.bind_groups.len() >= MAX_BIND_GROUPS {
            self.clear_bind_groups();
        }
        let (mut next_buffer, mut next_texture) = (0, 0);
        let entries: Vec<WGPUBindGroupEntry> = slots
            .iter()
            .map(|slot| {
                // SAFETY: all-zero is a valid record: no chain, no resource yet.
                let mut entry: WGPUBindGroupEntry = unsafe { std::mem::zeroed() };
                match *slot {
                    Slot::Uniform(binding) => {
                        let (buffer, size) = self.buffers[next_buffer];
                        next_buffer += 1;
                        entry.binding = binding;
                        entry.buffer = buffer;
                        entry.size = size;
                    }
                    Slot::Texture(binding) => {
                        entry.binding = binding;
                        entry.textureView = textures[next_texture].view();
                        next_texture += 1;
                    }
                }
                entry
            })
            .collect();
        // SAFETY: the layout, the buffers and the views are alive; the record is borrowed.
        let group = unsafe {
            wgpuDeviceCreateBindGroup(
                device.device(),
                &WGPUBindGroupDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: text("FerroUI effect"),
                    layout: self.layout,
                    entryCount: entries.len(),
                    entries: entries.as_ptr(),
                },
            )
        };
        self.bind_groups.push((key, group));
        group
    }

    /// Runs the program over `output`, cleared first, on its own submission: the uniforms just
    /// written reach the device before this pass and after the pass before it.
    fn draw(&self, device: &ProGpuDevice, group: WGPUBindGroup, output: &GpuTexture) {
        // SAFETY: the calls of the WebGPU C ABI with live objects; the records are borrowed.
        unsafe {
            let encoder = wgpuDeviceCreateCommandEncoder(device.device(), ptr::null());
            let attachment = WGPURenderPassColorAttachment {
                nextInChain: ptr::null_mut(),
                view: output.view(),
                depthSlice: WGPU_DEPTH_SLICE_UNDEFINED,
                resolveTarget: ptr::null_mut(),
                loadOp: WGPULoadOp_Clear,
                storeOp: WGPUStoreOp_Store,
                clearValue: WGPUColor { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
            };
            let pass = wgpuCommandEncoderBeginRenderPass(
                encoder,
                &WGPURenderPassDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: text("FerroUI effect"),
                    colorAttachmentCount: 1,
                    colorAttachments: &attachment,
                    depthStencilAttachment: ptr::null(),
                    occlusionQuerySet: ptr::null_mut(),
                    timestampWrites: ptr::null(),
                },
            );
            wgpuRenderPassEncoderSetPipeline(pass, self.pipeline);
            wgpuRenderPassEncoderSetBindGroup(pass, 0, group, 0, ptr::null());
            wgpuRenderPassEncoderDraw(pass, 3, 1, 0, 0);
            wgpuRenderPassEncoderEnd(pass);
            wgpuRenderPassEncoderRelease(pass);
            let commands = wgpuCommandEncoderFinish(encoder, ptr::null());
            wgpuQueueSubmit(device.queue(), 1, &commands);
            wgpuCommandBufferRelease(commands);
            wgpuCommandEncoderRelease(encoder);
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        // The objects of a thread that is ending are left alone (`thread_is_ending`).
        if thread_is_ending() {
            return;
        }
        self.clear_bind_groups();
        // SAFETY: the handles are owned here.
        unsafe {
            for (buffer, _) in self.buffers.drain(..) {
                wgpuBufferRelease(buffer);
            }
            wgpuRenderPipelineRelease(self.pipeline);
            wgpuBindGroupLayoutRelease(self.layout);
        }
    }
}

fn write_buffer(device: &ProGpuDevice, buffer: WGPUBuffer, bytes: &[u8]) {
    // SAFETY: the buffer is alive and at least this long; the data is copied by the call.
    unsafe { wgpuQueueWriteBuffer(device.queue(), buffer, 0, bytes.as_ptr() as *const c_void, bytes.len()) };
}

/// The floats of a uniform block as bytes.
fn float_bytes(floats: &[f32]) -> &[u8] {
    // SAFETY: any `f32` is four initialized bytes, and `u8` has no alignment to keep.
    unsafe { std::slice::from_raw_parts(floats.as_ptr() as *const u8, std::mem::size_of_val(floats)) }
}

fn layer_slots(program: LayerProgram) -> Vec<Slot> {
    let mut slots = vec![Slot::Uniform(0)];
    slots.extend((0..program.inputs() as u32).map(|input| Slot::Texture(1 + input)));
    slots
}

/// The program of a custom effect: its layout in the block of the convention.
struct ShaderProgram {
    program: Program,
    slots: Vec<Slot>,
    offsets: Vec<u32>,
    size: usize,
    children: usize,
}

/// Where a shader effect is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShaderArea {
    /// The device pixel of the area at the top left texel of the textures (whole pixels).
    pub(crate) origin: [f32; 2],
    /// The width and the height of the whole area (`layer_size`).
    pub(crate) size: [f32; 2],
    /// Device pixels per device independent pixel (`layer_scale`).
    pub(crate) scale: f32,
    /// The part of the textures that holds the area, in pixels.
    pub(crate) held: (u32, u32),
}

/// The byte offset of each member of `struct Uniforms` of a program and the size of the block,
/// when its members are those of the convention (`layout`: names, order and types): the rules of
/// WGSL for what the convention can hold (`f32`, `i32`, `vec2<f32>`, `vec4<f32>`).
pub(crate) fn uniform_block_layout(
    wgsl: &str,
    layout: &[(String, ShaderUniformType)],
) -> Result<(Vec<u32>, usize), String> {
    let source = without_comments(wgsl);
    let start = find_struct(&source).ok_or("the program has no `struct Uniforms`")?;
    let body = &source[start..];
    let open = body.find('{').ok_or("`struct Uniforms` has no body")?;
    let close = body[open..].find('}').ok_or("`struct Uniforms` is not closed")? + open;
    let members: Vec<&str> =
        body[open + 1..close].split(',').map(str::trim).filter(|member| !member.is_empty()).collect();
    if members.len() != layout.len() {
        return Err(format!("Uniforms has {} members, the convention has {}", members.len(), layout.len()));
    }
    let (mut offsets, mut at, mut largest) = (Vec::with_capacity(layout.len()), 0u32, 4u32);
    for (member, (name, kind)) in members.iter().zip(layout) {
        let Some((declared, ty)) = member.split_once(':') else {
            return Err(format!("the member `{member}` of Uniforms has no type"));
        };
        let (declared, ty) = (declared.trim(), ty.split_whitespace().collect::<String>());
        if declared != name.as_str() {
            return Err(format!("the member `{declared}` of Uniforms should be `{name}`"));
        }
        let (size, same) = match kind {
            ShaderUniformType::Float => (4, ty == "f32"),
            ShaderUniformType::Int => (4, ty == "i32"),
            ShaderUniformType::Float2 => (8, ty == "vec2<f32>" || ty == "vec2f"),
            ShaderUniformType::Float4 => (16, ty == "vec4<f32>" || ty == "vec4f"),
            #[allow(unreachable_patterns)]
            _ => return Err(format!("the convention has a type this backend has no layout for: {name}")),
        };
        if !same {
            return Err(format!("the member `{name}` of Uniforms is not a {}", kind.wgsl()));
        }
        at = at.div_ceil(size) * size;
        offsets.push(at);
        at += size;
        largest = largest.max(size);
    }
    let span = at.div_ceil(largest) * largest;
    Ok((offsets, (span as usize).max(16).div_ceil(16) * 16))
}

/// `source` with its comments replaced by a space.
fn without_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let (mut out, mut at, mut last) = (String::with_capacity(source.len()), 0, 0);
    while at + 1 < bytes.len() {
        if bytes[at] == b'/' && bytes[at + 1] == b'/' {
            out.push_str(&source[last..at]);
            out.push(' ');
            at = source[at..].find('\n').map_or(bytes.len(), |line| at + line);
            last = at;
        } else if bytes[at] == b'/' && bytes[at + 1] == b'*' {
            out.push_str(&source[last..at]);
            out.push(' ');
            // Block comments of WGSL nest.
            let (mut depth, mut inside) = (1, at + 2);
            while depth > 0 && inside + 1 < bytes.len() {
                match (bytes[inside], bytes[inside + 1]) {
                    (b'/', b'*') => (depth, inside) = (depth + 1, inside + 2),
                    (b'*', b'/') => (depth, inside) = (depth - 1, inside + 2),
                    _ => inside += 1,
                }
            }
            at = if depth > 0 { bytes.len() } else { inside };
            last = at;
        } else {
            at += 1;
        }
    }
    out.push_str(&source[last.min(source.len())..]);
    out
}

/// Where the declaration of `struct Uniforms` starts.
fn find_struct(source: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = source[from..].find("struct") {
        let at = from + found;
        let before = source[..at].chars().next_back();
        let rest = &source[at + "struct".len()..];
        let name = rest.trim_start();
        let separated = rest.len() != name.len();
        let ends = name.strip_prefix("Uniforms").is_some_and(|tail| {
            !tail.chars().next().is_some_and(|next| next.is_alphanumeric() || next == '_')
        });
        if separated && ends && !before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            return Some(at);
        }
        from = at + "struct".len();
    }
    None
}

/// A free or used texture of the pool.
struct Pooled {
    texture: Rc<GpuTexture>,
    last_frame: u64,
}

/// The texture of an image of a shader effect, kept while a handle of the image lives.
struct ImageTexture {
    handle: EffectImageWeak,
    image: u64,
    version: i32,
    texture: Rc<GpuTexture>,
}

/// The passes of a device.
#[derive(Default)]
pub(crate) struct EffectPasses {
    programs: RefCell<HashMap<LayerProgram, Option<Program>>>,
    shaders: RefCell<HashMap<TypeId, Option<ShaderProgram>>>,
    pool: RefCell<Vec<Pooled>>,
    scenes: RefCell<Vec<SceneBuilder>>,
    factors: RefCell<Vec<(Weak<Vec<u8>>, Rc<GpuTexture>)>>,
    images: RefCell<Vec<ImageTexture>>,
    empty_image: RefCell<Option<Rc<GpuTexture>>>,
    image_releases: Cell<u64>,
    frame: Cell<u64>,
}

impl EffectPasses {
    /// The start of a frame: what the pool has not used for a while goes.
    pub(crate) fn begin_frame(&self) {
        let frame = self.frame.get() + 1;
        self.frame.set(frame);
        self.pool.borrow_mut().retain(|entry| {
            Rc::strong_count(&entry.texture) > 1 || frame - entry.last_frame <= POOL_KEPT_FRAMES
        });
    }

    /// A free texture of at least `width` by `height` pixels (rounded up to the step of the pool).
    pub(crate) fn texture(&self, gpu: &Gpu, width: u32, height: u32) -> Rc<GpuTexture> {
        let round = |side: u32| side.max(1).div_ceil(POOL_STEP).saturating_mul(POOL_STEP).min(POOL_MAX_SIDE).max(side);
        let (width, height) = (round(width), round(height));
        let frame = self.frame.get();
        let mut pool = self.pool.borrow_mut();
        if let Some(entry) = pool.iter_mut().find(|entry| {
            Rc::strong_count(&entry.texture) == 1 && entry.texture.width() == width && entry.texture.height() == height
        }) {
            entry.last_frame = frame;
            return entry.texture.clone();
        }
        let texture = GpuTexture::new(gpu.device(), width, height);
        pool.push(Pooled { texture: texture.clone(), last_frame: frame });
        texture
    }

    /// A builder for the content of a layer: its scene stays on the engine between frames.
    pub(crate) fn take_scene(&self) -> SceneBuilder {
        self.scenes.borrow_mut().pop().unwrap_or_default()
    }

    /// Gives a builder back; past the most kept, what the engine holds of its scene goes with it.
    pub(crate) fn give_scene(&self, gpu: &Gpu, scene: SceneBuilder) {
        let mut scenes = self.scenes.borrow_mut();
        if scenes.len() < MAX_SCENES {
            scenes.push(scene);
        } else if let Some(engine) = gpu.offscreen() {
            engine.release_scene(scene.scene_id());
        }
    }

    /// `program` over `inputs` into a texture of the pool of the size of the first input (or of
    /// `size`, for a program without inputs). `None` when the device refused the program.
    pub(crate) fn run(
        &self,
        gpu: &Gpu,
        program: LayerProgram,
        uniforms: &[f32],
        inputs: &[&Rc<GpuTexture>],
        size: (u32, u32),
    ) -> Option<Rc<GpuTexture>> {
        if inputs.len() != program.inputs() || uniforms.len() != program.uniform_floats() {
            return None;
        }
        let device = gpu.device();
        let slots = layer_slots(program);
        let mut programs = self.programs.borrow_mut();
        let compiled = programs.entry(program).or_insert_with(|| {
            let bytes = (program.uniform_floats() * 4) as u64;
            Program::new(device, &program.source(), &slots, &[bytes])
        });
        let compiled = compiled.as_mut().filter(|compiled| !compiled.refused())?;
        let (width, height) = inputs.first().map_or(size, |first| (first.width(), first.height()));
        let output = self.texture(gpu, width, height);
        let textures: Vec<&GpuTexture> = inputs.iter().map(|input| input.as_ref()).collect();
        let group = compiled.bind_group(device, &slots, &textures);
        write_buffer(device, compiled.buffers[0].0, float_bytes(uniforms));
        compiled.draw(device, group, &output);
        Some(output)
    }

    /// The texture of the factors of a multiply layer, uploaded once while the factors live.
    pub(crate) fn factor_texture(&self, gpu: &Gpu, factors: &MultiplyLayerFactors) -> Option<Rc<GpuTexture>> {
        let (width, height) = (factors.size.width, factors.size.height);
        if width <= 0 || height <= 0 || factors.rgba.len() < width as usize * height as usize * 4 {
            return None;
        }
        let key = Arc::downgrade(&factors.rgba);
        let mut kept = self.factors.borrow_mut();
        kept.retain(|(owner, _)| owner.strong_count() > 0);
        if let Some((_, texture)) = kept.iter().find(|(owner, _)| owner.ptr_eq(&key)) {
            return Some(texture.clone());
        }
        if kept.len() >= MAX_FACTOR_TEXTURES {
            kept.remove(0);
        }
        let texture = GpuTexture::new_image(gpu.device(), width as u32, height as u32);
        texture.write(&factors.rgba[..width as usize * height as usize * 4]);
        kept.push((key, texture.clone()));
        Some(texture)
    }

    /// Whether the program of `effect` is ready on this device: compiled once per type. A program
    /// that does not compile, or whose uniforms are not those of the convention, is said once and
    /// the effect is drawn as an effect without a factory.
    pub(crate) fn prepare_shader(&self, gpu: &Gpu, effect: &ImmutableCustomEffect) -> bool {
        let mut shaders = self.shaders.borrow_mut();
        let entry = shaders.entry(effect.value_type_id()).or_insert_with(|| {
            let result = Self::compile_shader(gpu.device(), effect);
            if let Err(reason) = &result {
                notes::note_once(
                    &format!("shader-effect-{}", effect.name()),
                    &format!("the effect {} is drawn without its program: {reason}", effect.name()),
                );
            }
            result.ok()
        });
        entry.as_ref().is_some_and(|shader| !shader.program.refused())
    }

    fn compile_shader(device: &ProGpuDevice, effect: &ImmutableCustomEffect) -> Result<ShaderProgram, String> {
        let description = effect.shader().ok_or("the effect is not a shader effect")?;
        effect.check_shader_children()?;
        let (offsets, size) = uniform_block_layout(&description.wgsl, &effect.shader_uniform_layout())?;
        let children = description.children.len();
        let mut slots = vec![
            Slot::Uniform(SHADER_WGSL_UNIFORMS_BINDING),
            Slot::Uniform(SHADER_WGSL_PASS_BINDING),
            Slot::Texture(SHADER_WGSL_CONTENT_BINDING),
        ];
        slots.extend((0..children as u32).map(|child| Slot::Texture(SHADER_WGSL_CHILDREN_BINDING + child)));
        let program = Program::new(device, &description.wgsl_module(), &slots, &[size as u64, 16])
            .ok_or("the device refused its WGSL")?;
        Ok(ShaderProgram { program, slots, offsets, size, children })
    }

    /// The shader of `effect` over `content` into a texture of the pool of the same size. `None`
    /// when its program failed or its uniforms or images do not match it (said once).
    pub(crate) fn run_shader(
        &self,
        gpu: &Gpu,
        content: &Rc<GpuTexture>,
        effect: &ImmutableCustomEffect,
        area: ShaderArea,
    ) -> Option<Rc<GpuTexture>> {
        if !self.prepare_shader(gpu, effect) {
            return None;
        }
        self.release_dropped_images();
        let mut children: Vec<Rc<GpuTexture>> = Vec::with_capacity(MAX_SHADER_CHILDREN);
        effect.for_each_shader_child(&mut |_, image| children.push(self.image_texture(gpu, image)));
        let device = gpu.device();
        let mut shaders = self.shaders.borrow_mut();
        let shader = shaders.get_mut(&effect.value_type_id())?.as_mut()?;
        let key = format!("shader-effect-{}", effect.name());
        if children.len() != shader.children {
            notes::note_once(&key, &format!("the images of the effect {} do not match it", effect.name()));
            return None;
        }
        let mut block = vec![0u8; shader.size];
        let complete =
            blocks::fill_shader_uniforms(&mut block, &shader.offsets, effect, area.size, area.scale);
        if !complete {
            notes::note_once(&key, &format!("the uniforms of the effect {} do not match it", effect.name()));
            return None;
        }
        let pass = [area.origin[0], area.origin[1], area.held.0 as f32, area.held.1 as f32];
        let output = self.texture(gpu, content.width(), content.height());
        let mut textures: Vec<&GpuTexture> = Vec::with_capacity(1 + children.len());
        textures.push(content);
        textures.extend(children.iter().map(|child| child.as_ref()));
        let group = shader.program.bind_group(device, &shader.slots, &textures);
        write_buffer(device, shader.program.buffers[0].0, &block);
        write_buffer(device, shader.program.buffers[1].0, float_bytes(&pass));
        shader.program.draw(device, group, &output);
        Some(output)
    }

    /// The textures of images that were dropped go, with the bind groups that hold them.
    fn release_dropped_images(&self) {
        let generation = EffectImage::release_generation();
        if self.image_releases.replace(generation) == generation {
            return;
        }
        let mut images = self.images.borrow_mut();
        let before = images.len();
        images.retain(|image| image.handle.is_alive());
        if images.len() != before {
            for shader in self.shaders.borrow_mut().values_mut().flatten() {
                shader.program.clear_bind_groups();
            }
        }
    }

    /// The texture of `image`: its pixels uploaded the first time it is drawn (or at a new
    /// version); a transparent pixel for an empty image or one that cannot be read.
    fn image_texture(&self, gpu: &Gpu, image: &EffectImage) -> Rc<GpuTexture> {
        if !image.is_empty() {
            let (id, version) = (image.id(), image.version());
            let mut images = self.images.borrow_mut();
            if let Some(found) = images.iter().find(|kept| kept.image == id && kept.version == version) {
                return found.texture.clone();
            }
            images.retain(|kept| kept.image != id);
            let uploaded = image.pixels().filter(|pixels| {
                let largest = POOL_MAX_SIDE as usize;
                pixels.width() > 0 && pixels.height() > 0 && pixels.width() <= largest && pixels.height() <= largest
            });
            if let Some(pixels) = uploaded {
                let texture = GpuTexture::new_image(gpu.device(), pixels.width() as u32, pixels.height() as u32);
                texture.write(pixels.rgba());
                images.push(ImageTexture { handle: image.downgrade(), image: id, version, texture: texture.clone() });
                return texture;
            }
        }
        self.empty_image
            .borrow_mut()
            .get_or_insert_with(|| {
                let texture = GpuTexture::new_image(gpu.device(), 1, 1);
                texture.write(&[0; 4]);
                texture
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(members: &[(&str, ShaderUniformType)]) -> Vec<(String, ShaderUniformType)> {
        members.iter().map(|(name, kind)| ((*name).to_owned(), *kind)).collect()
    }

    #[test]
    fn the_offsets_of_a_block_follow_the_alignment_of_wgsl() {
        let wgsl = "// struct Uniforms { nope: f32 }\n\
                    struct Uniforms {\n  a: f32, /* one */\n  b: vec2<f32>,\n  c: i32,\n  d: vec4f,\n}\n\
                    fn effect() {}";
        let expected = layout(&[
            ("a", ShaderUniformType::Float),
            ("b", ShaderUniformType::Float2),
            ("c", ShaderUniformType::Int),
            ("d", ShaderUniformType::Float4),
        ]);
        let (offsets, size) = uniform_block_layout(wgsl, &expected).expect("the layout");
        assert_eq!(offsets, [0, 8, 16, 32]);
        assert_eq!(size, 48);
    }

    #[test]
    fn a_block_that_is_not_the_convention_is_refused() {
        let expected = layout(&[("a", ShaderUniformType::Float)]);
        assert!(uniform_block_layout("struct Other { a: f32 }", &expected).is_err());
        assert!(uniform_block_layout("struct Uniforms { b: f32 }", &expected).is_err());
        assert!(uniform_block_layout("struct Uniforms { a: i32 }", &expected).is_err());
        assert!(uniform_block_layout("struct Uniforms { a: f32, b: f32 }", &expected).is_err());
        assert!(uniform_block_layout("struct MyUniforms { a: f32 }", &expected).is_err());
    }
}
