//! The render passes of the layers with an effect (docs/porting/progpu-backend.md, section 2.7).
//!
//! The passes are the engine's (`progpu_native_layer_effects.h`): the drawing context renders the
//! content of a layer into a texture of the pool (`effect_layers.rs`), a pass of the engine reads
//! it and writes another texture of the pool, and the scene the layer was pushed in samples the
//! result. Nothing is read back or drawn on the processor.
//!
//! What is here is the host's side: the pool, the scenes of the layers, the textures of factors
//! and images, and the layout of the uniforms of a custom effect. A texture of the pool is in use
//! while something else holds it, so a texture a scene samples comes back after that scene is
//! rendered and steady frames allocate nothing.

use crate::device::{thread_is_ending, ProGpuDevice, ProGpuEngine};
use crate::gpu::{Gpu, GpuTexture};
use crate::notes;
use crate::scene::SceneBuilder;
use ferroui_base::media::effects::{
    EffectImage, EffectImageWeak, ImmutableCustomEffect, ShaderUniformType, ShaderUniformValue, MAX_SHADER_CHILDREN,
};
use ferroui_base::platform::MultiplyLayerFactors;
use progpu_native_sys::layer_effects::*;
use progpu_native_sys::progpu::PROGPU_NATIVE_STATUS_SUCCESS;
use progpu_native_sys::webgpu::*;
use std::any::TypeId;
use std::cell::{Cell, OnceCell, RefCell};
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
/// The most textures of multiply factors kept.
const MAX_FACTOR_TEXTURES: usize = 8;
/// The side of the tile of the noise of materials.
pub(crate) const NOISE_TILE_SIDE: u32 = 256;
/// How much of the noise a material shows.
const NOISE_OPACITY: f32 = 0.0225;

/// A pass of the engine with its parameters.
pub(crate) enum LayerPass {
    ColorMatrix(progpu_native_layer_color_matrix),
    Convolution(progpu_native_layer_convolution),
    Composite(progpu_native_layer_composite),
    Lighting(Box<progpu_native_layer_lighting>),
    FactorMultiply(progpu_native_layer_factor_multiply),
}

/// The layer effects of the engine that renders offscreen.
struct EngineEffects {
    raw: *mut progpu_native_layer_effects,
    engine: Rc<ProGpuEngine>,
}

impl EngineEffects {
    fn new(engine: &Rc<ProGpuEngine>) -> Option<Self> {
        let mut raw = ptr::null_mut();
        // SAFETY: the engine is alive and entered on this thread; the slot is written.
        let status = unsafe { progpu_native_layer_effects_create(engine.enter(), &mut raw) };
        if status != PROGPU_NATIVE_STATUS_SUCCESS || raw.is_null() {
            notes::failure("the engine has no layer effects on this device");
            return None;
        }
        Some(EngineEffects { raw, engine: engine.clone() })
    }

    /// The effects, with the engine entered on this thread.
    fn enter(&self) -> *mut progpu_native_layer_effects {
        self.engine.enter();
        self.raw
    }
}

impl Drop for EngineEffects {
    fn drop(&mut self) {
        // The objects of a thread that is ending are left alone (`thread_is_ending`).
        if thread_is_ending() {
            return;
        }
        // SAFETY: made in `new`; the engine this holds outlives it.
        unsafe { progpu_native_layer_effects_destroy(self.enter()) };
    }
}

/// What the validation of a program said; on a page it says so later, from the event loop.
#[derive(Default)]
struct ProgramCheck {
    done: Cell<bool>,
    error: RefCell<Option<String>>,
}

unsafe extern "C" fn on_program_checked(
    status: WGPUPopErrorScopeStatus,
    kind: WGPUErrorType,
    message: WGPUStringView,
    check: *mut c_void,
    _: *mut c_void,
) {
    // SAFETY: the reference `HostProgram::new` gave up, and the callback runs once.
    let check = unsafe { Rc::from_raw(check as *const ProgramCheck) };
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

/// The compiled program of a custom effect, on the engine.
struct HostProgram {
    raw: *mut progpu_native_layer_effect_program,
    check: Rc<ProgramCheck>,
    engine: Rc<ProGpuEngine>,
}

impl HostProgram {
    /// Compiles `source` under an error scope: a program the device refuses is an error here and
    /// not a failure of the device.
    fn new(
        effects: &EngineEffects,
        device: &ProGpuDevice,
        source: &str,
        uniform_size: u32,
        images: u32,
    ) -> Option<Self> {
        let raw_device = device.device();
        let check = Rc::new(ProgramCheck::default());
        let mut raw = ptr::null_mut();
        // SAFETY: the calls of the C ABIs with live objects; the source and the slot outlive the
        // call, and the callback takes the reference given up here.
        let status = unsafe {
            wgpuDevicePushErrorScope(raw_device, WGPUErrorFilter_Validation);
            let status = progpu_native_layer_effects_create_program(
                effects.enter(),
                source.as_ptr() as *const c_char,
                source.len(),
                uniform_size,
                images,
                &mut raw,
            );
            wgpuDevicePopErrorScope(
                raw_device,
                WGPUPopErrorScopeCallbackInfo {
                    nextInChain: ptr::null_mut(),
                    mode: WGPUCallbackMode_AllowSpontaneous,
                    callback: Some(on_program_checked),
                    userdata1: Rc::into_raw(check.clone()) as *mut c_void,
                    userdata2: ptr::null_mut(),
                },
            );
            #[cfg(not(target_os = "emscripten"))]
            if !check.done.get() {
                wgpuDevicePoll(raw_device, 1, ptr::null());
            }
            status
        };
        let program = (!raw.is_null()).then(|| HostProgram { raw, check, engine: effects.engine.clone() });
        match program {
            Some(program) if status == PROGPU_NATIVE_STATUS_SUCCESS && !program.refused() => Some(program),
            other => {
                let reason = other.as_ref().and_then(|program| program.check.error.borrow().clone());
                let reason = reason.unwrap_or_else(|| "the engine made no program".to_owned());
                notes::failure(&format!("an effect program was refused: {reason}"));
                None
            }
        }
    }

    /// Whether the device refused the program after it was made (on a page, where the answer
    /// comes from the event loop).
    fn refused(&self) -> bool {
        self.check.error.borrow().is_some()
    }
}

impl Drop for HostProgram {
    fn drop(&mut self) {
        if thread_is_ending() {
            return;
        }
        self.engine.enter();
        // SAFETY: made in `new`; the engine this holds outlives it.
        unsafe { progpu_native_layer_effect_program_destroy(self.raw) };
    }
}

/// The program of a custom effect: its layout in the block of the convention.
struct ShaderProgram {
    program: HostProgram,
    offsets: Vec<u32>,
    size: usize,
    children: usize,
}

/// The uniforms of `effect` over `area` as the block of its program; `None` when they are not the
/// members `offsets` was made from.
fn shader_block(effect: &ImmutableCustomEffect, offsets: &[u32], size: usize, area: ShaderArea) -> Option<Vec<u8>> {
    let mut block = vec![0u8; size];
    let mut members = offsets.iter();
    let mut matches = true;
    effect.for_each_shader_uniform(area.size, area.scale, &mut |_, value| {
        let Some(&offset) = members.next() else {
            matches = false;
            return;
        };
        let mut at = offset as usize;
        let mut put = |bytes: [u8; 4]| {
            match block.get_mut(at..at + bytes.len()) {
                Some(slot) => slot.copy_from_slice(&bytes),
                None => matches = false,
            }
            at += bytes.len();
        };
        if let ShaderUniformValue::Int(number) = value {
            put(number.to_ne_bytes());
        } else {
            value.floats().iter().for_each(|number| put(number.to_ne_bytes()));
        }
    });
    (matches && members.next().is_none()).then_some(block)
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
    shaders: RefCell<HashMap<TypeId, Option<ShaderProgram>>>,
    effects: OnceCell<Option<EngineEffects>>,
    pool: RefCell<Vec<Pooled>>,
    scenes: RefCell<Vec<SceneBuilder>>,
    factors: RefCell<Vec<(Weak<Vec<u8>>, Rc<GpuTexture>)>>,
    images: RefCell<Vec<ImageTexture>>,
    empty_image: RefCell<Option<Rc<GpuTexture>>>,
    noise_tile: RefCell<Option<Rc<GpuTexture>>>,
    image_releases: Cell<u64>,
    frame: Cell<u64>,
}

fn target_of(output: &GpuTexture, inputs: [usize; 2], size: (u32, u32)) -> progpu_native_layer_effect_target {
    progpu_native_layer_effect_target {
        struct_size: std::mem::size_of::<progpu_native_layer_effect_target>() as u32,
        width: size.0,
        height: size.1,
        reserved: 0,
        output_view: output.view() as usize,
        first_view: inputs[0],
        second_view: inputs[1],
    }
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

    fn effects(&self, gpu: &Gpu) -> Option<&EngineEffects> {
        self.effects.get_or_init(|| gpu.offscreen().and_then(EngineEffects::new)).as_ref()
    }

    /// `pass` over `inputs` into a texture of the pool of the size of the first input (or of
    /// `size`, for a pass without inputs); the result is in the `size` pixels at its top left.
    /// `None` when the engine refused the pass.
    pub(crate) fn run(
        &self,
        gpu: &Gpu,
        pass: &LayerPass,
        inputs: &[&Rc<GpuTexture>],
        size: (u32, u32),
    ) -> Option<Rc<GpuTexture>> {
        let effects = self.effects(gpu)?;
        let (width, height) = inputs.first().map_or(size, |first| (first.width(), first.height()));
        let output = self.texture(gpu, width, height);
        let input = |index: usize| inputs.get(index).map_or(0, |texture| texture.view() as usize);
        let target = target_of(&output, [input(0), input(1)], size);
        let raw = effects.enter();
        // SAFETY: the effects are alive and entered, and the views and the records outlive the call.
        let status = unsafe {
            match pass {
                LayerPass::ColorMatrix(record) => progpu_native_layer_effects_color_matrix(raw, &target, record),
                LayerPass::Convolution(record) => progpu_native_layer_effects_convolution(raw, &target, record),
                LayerPass::Composite(record) => progpu_native_layer_effects_composite(raw, &target, record),
                LayerPass::Lighting(record) => progpu_native_layer_effects_lighting(raw, &target, &**record),
                LayerPass::FactorMultiply(record) => {
                    progpu_native_layer_effects_factor_multiply(raw, &target, record)
                }
            }
        };
        if status != PROGPU_NATIVE_STATUS_SUCCESS {
            notes::failure(&format!("the engine refused an effect pass (status {status})"));
            return None;
        }
        output.changed();
        Some(output)
    }

    /// The tile of the noise of materials, rendered once by the engine.
    pub(crate) fn noise_tile(&self, gpu: &Gpu) -> Option<Rc<GpuTexture>> {
        if let Some(tile) = self.noise_tile.borrow().as_ref() {
            return Some(tile.clone());
        }
        let effects = self.effects(gpu)?;
        let tile = GpuTexture::new(gpu.device(), NOISE_TILE_SIDE, NOISE_TILE_SIDE);
        let target = target_of(&tile, [0, 0], (NOISE_TILE_SIDE, NOISE_TILE_SIDE));
        let noise = progpu_native_layer_material_noise {
            struct_size: std::mem::size_of::<progpu_native_layer_material_noise>() as u32,
            opacity: NOISE_OPACITY,
            color: [1.0; 3],
            origin_x: 0.0,
            origin_y: 0.0,
        };
        // SAFETY: the effects are alive and entered, and the view and the record outlive the call.
        let status = unsafe { progpu_native_layer_effects_material_noise(effects.enter(), &target, &noise) };
        if status != PROGPU_NATIVE_STATUS_SUCCESS {
            notes::failure(&format!("the engine refused the noise of a material (status {status})"));
            return None;
        }
        tile.changed();
        *self.noise_tile.borrow_mut() = Some(tile.clone());
        Some(tile)
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
            let result = self.compile_shader(gpu, effect);
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

    fn compile_shader(&self, gpu: &Gpu, effect: &ImmutableCustomEffect) -> Result<ShaderProgram, String> {
        let description = effect.shader().ok_or("the effect is not a shader effect")?;
        effect.check_shader_children()?;
        let (offsets, size) = uniform_block_layout(&description.wgsl, &effect.shader_uniform_layout())?;
        let children = description.children.len();
        let effects = self.effects(gpu).ok_or("the engine has no layer effects")?;
        let module = description.wgsl_module();
        let program = HostProgram::new(effects, gpu.device(), &module, size as u32, children as u32)
            .ok_or("the device refused its WGSL")?;
        Ok(ShaderProgram { program, offsets, size, children })
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
        let effects = self.effects(gpu)?;
        let shaders = self.shaders.borrow();
        let shader = shaders.get(&effect.value_type_id())?.as_ref()?;
        let key = format!("shader-effect-{}", effect.name());
        if children.len() != shader.children {
            notes::note_once(&key, &format!("the images of the effect {} do not match it", effect.name()));
            return None;
        }
        let Some(block) = shader_block(effect, &shader.offsets, shader.size, area) else {
            notes::note_once(&key, &format!("the uniforms of the effect {} do not match it", effect.name()));
            return None;
        };
        let pass = [area.origin[0], area.origin[1], area.held.0 as f32, area.held.1 as f32];
        let output = self.texture(gpu, content.width(), content.height());
        let target = target_of(&output, [content.view() as usize, 0], area.held);
        let views: Vec<usize> = children.iter().map(|child| child.view() as usize).collect();
        // SAFETY: the effects and the program are alive and entered; the block, the views and the
        // records outlive the call.
        let status = unsafe {
            progpu_native_layer_effects_run_program(
                effects.enter(),
                shader.program.raw,
                &target,
                block.as_ptr() as *const c_void,
                block.len(),
                pass.as_ptr(),
                views.as_ptr(),
                views.len() as u32,
            )
        };
        if status != PROGPU_NATIVE_STATUS_SUCCESS {
            notes::note_once(&key, &format!("the engine refused the effect {} (status {status})", effect.name()));
            return None;
        }
        output.changed();
        Some(output)
    }

    /// The textures of images that were dropped go.
    fn release_dropped_images(&self) {
        let generation = EffectImage::release_generation();
        if self.image_releases.replace(generation) == generation {
            return;
        }
        self.images.borrow_mut().retain(|image| image.handle.is_alive());
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
