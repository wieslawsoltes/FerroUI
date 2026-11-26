//! The GL executor of 3D scenes (#608, step D8 of `docs/porting/media3d.md`, section 3.20): the
//! one shader of `ferroui-render3d`, translated to GLSL ES 3.00 at build time, drawn on a GL or GLES
//! context of version 3 (ANGLE, EGL, Android GLES, CGL, WebGL2) into a texture of that context.
//!
//! What it keeps, as the wgpu and Metal executors do:
//!
//! - **Programs** are linked once per context and brush shader, on first demand.
//! - **Meshes** are uploaded once per `(id, version)`, **brush textures** once per key.
//! - **Uniforms** are written only where their bytes changed: a moving camera writes the frame
//!   uniforms alone.
//! - **Targets** (the multisampled colour, the depth and the resolved colour) are pooled by size
//!   and sample count.
//!
//! The clip space of the translated shader is GL's with y flipped: the first row of an output is
//! the top of the scene (a renderer wraps it with a top left origin) and a front face is clockwise
//! in window coordinates.
//!
//! **The state of the context.** A render changes the bindings and the fixed-function state of
//! the context and does not restore them: the caller tells its renderer afterwards (Skia's
//! `DirectContext::reset`). A render needs the context current.
//!
//! **Names.** GL names belong to the context, so nothing is deleted on drop, where the context may
//! not be current. [`Scene3DGlExecutor::dispose`] deletes them with the context current; a context
//! that is gone took them with it.
//!
//! Errors: a program that does not compile or link, an incomplete framebuffer or a GL error of a
//! render are a [`Scene3DGlError`] the caller reports once before it draws the CPU reference. A
//! failure with four samples is reported once, and the executor draws with one from then on.

use crate::gl_consts::*;
use crate::{GlInterface, GlProfileType, GlVersion};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::SharedBitmapImpl;
use ferroui_base::rendering::scene3d::{MeshData, Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot};
use ferroui_base::Rect;
use ferroui_render3d::brush::DeviceBitmap;
use ferroui_render3d::brush::{
    texture_coordinate_bounds, BrushShader, BrushTextureIdentity, BrushTextureKey, BrushTextureSource,
    MaterialBrush, BRUSH_UNIFORMS_STRIDE,
};
use ferroui_render3d::shader::glsl::{self, GlslStage};
use ferroui_render3d::shader::{samples_texture, write_mesh, VERTEX_ATTRIBUTE_OFFSETS, VERTEX_STRIDE};
use ferroui_render3d::uniforms::{DrawUniforms, Face, FrameUniforms, LayerKind, Std140, DRAW_UNIFORMS_STRIDE};
use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

// What the generated table of constants does not hold.
const GL_DEPTH_COMPONENT24: i32 = 0x81A6;
const GL_UNIFORM_BUFFER: i32 = 0x8A11;
const GL_UNIFORM_BUFFER_OFFSET_ALIGNMENT: i32 = 0x8A34;
const GL_PIXEL_UNPACK_BUFFER: i32 = 0x88EC;
const GL_DYNAMIC_DRAW: i32 = 0x88E8;
const GL_UNSIGNED_INT: i32 = 0x1405;
const GL_BLEND: i32 = 0x0BE2;
const GL_DITHER: i32 = 0x0BD0;
const GL_STENCIL_TEST: i32 = 0x0B90;
const GL_FRAMEBUFFER_SRGB: i32 = 0x8DB9;
const GL_LEQUAL: i32 = 0x0203;
const GL_ONE: i32 = 1;
const GL_ONE_MINUS_SRC_ALPHA: i32 = 0x0303;
const GL_FUNC_ADD: i32 = 0x8006;
const GL_FRONT: i32 = 0x0404;
const GL_CW: i32 = 0x0900;
const GL_MAX_SAMPLES: i32 = 0x8D57;
const GL_MAX_TEXTURE_SIZE: i32 = 0x0D33;
const GL_UNPACK_ROW_LENGTH: i32 = 0x0CF2;
const GL_UNPACK_SKIP_ROWS: i32 = 0x0CF3;
const GL_UNPACK_SKIP_PIXELS: i32 = 0x0CF4;
const GL_INVALID_INDEX: u32 = u32::MAX;

/// The samples of an antialiased render (decision F).
pub const MULTISAMPLE_COUNT: u32 = 4;

/// The renders a mesh, a texture or a target of the pool is kept for without being drawn.
const LIFETIME: u64 = 240;

/// The bytes between the draw uniforms of two draws: at least the alignment of a uniform buffer
/// offset on every context.
const DRAW_STRIDE: usize = DRAW_UNIFORMS_STRIDE;
/// The bytes between the brush uniforms of two draws.
const BRUSH_STRIDE: usize = BRUSH_UNIFORMS_STRIDE;

/// What the GL executors of this thread did since it started: what the steady-state tests count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scene3DGlCounters {
    /// Programs linked.
    pub programs_linked: u64,
    /// Meshes uploaded (a new `(id, version)`).
    pub mesh_uploads: u64,
    /// The bytes of the vertices and indices uploaded.
    pub mesh_bytes: u64,
    /// Writes of uniform buffers.
    pub uniform_writes: u64,
    /// The bytes of the uniforms written.
    pub uniform_bytes: u64,
    /// Renders of a scene.
    pub passes: u64,
    /// Textures and renderbuffers made.
    pub textures_created: u64,
    /// Buffers made.
    pub buffers_created: u64,
    /// Textures of brushes written.
    pub textures_uploaded: u64,
    /// The bytes of those texels.
    pub texture_bytes: u64,
    /// Textures of brushes that could not be uploaded.
    pub textures_refused: u64,
    /// Textures of brushes the renderer already had on the context (a frame of a video).
    pub textures_on_device: u64,
}

impl Scene3DGlCounters {
    const ZERO: Scene3DGlCounters = Scene3DGlCounters {
        programs_linked: 0,
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
    static COUNTERS: Cell<Scene3DGlCounters> = const { Cell::new(Scene3DGlCounters::ZERO) };
}

/// What the GL executors of this thread did since it started.
pub fn counters() -> Scene3DGlCounters {
    COUNTERS.with(Cell::get)
}

fn count(change: impl FnOnce(&mut Scene3DGlCounters)) {
    COUNTERS.with(|counters| {
        let mut value = counters.get();
        change(&mut value);
        counters.set(value);
    });
}

/// Why the executor cannot draw a scene.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene3DGlError {
    /// What failed.
    pub reason: String,
}

impl Scene3DGlError {
    fn new(reason: impl Into<String>) -> Scene3DGlError {
        Scene3DGlError { reason: reason.into() }
    }
}

impl std::fmt::Display for Scene3DGlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}

impl std::error::Error for Scene3DGlError {}

/// A texture of the context a scene is drawn into: RGBA8, premultiplied, its first row the top of
/// the scene.
pub struct Scene3DGlTexture {
    id: i32,
    width: u32,
    height: u32,
}

impl Scene3DGlTexture {
    /// The name of the texture.
    pub fn id(&self) -> i32 {
        self.id
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

/// The texture a render draws into.
pub struct Scene3DGlTarget<'a> {
    /// The output, made by [`Scene3DGlExecutor::create_output_texture`]. The scene is drawn into
    /// its first rows; the rest of it is cleared to transparent.
    pub texture: &'a Scene3DGlTexture,
    /// The size of the image of the scene, in pixels: at most the size of the texture.
    pub size: (u32, u32),
    /// Whether edges are antialiased (4x MSAA).
    pub antialiased: bool,
}

/// What one render did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scene3DGlRenderReport {
    /// The samples per pixel it drew with.
    pub sample_count: u32,
    /// The draws it issued.
    pub draws: u32,
    /// The meshes it uploaded.
    pub meshes_uploaded: u32,
    /// The textures of brushes it uploaded.
    pub textures_uploaded: u32,
    /// The programs it linked.
    pub programs_linked: u32,
    /// The textures, renderbuffers and buffers it made.
    pub resources_created: u32,
    /// The bytes of uniforms it wrote.
    pub uniform_bytes: u64,
}

impl Scene3DGlRenderReport {
    /// Whether the render wrote uniforms and nothing else: the steady state of a moving camera.
    pub fn is_uniform_only(&self) -> bool {
        self.meshes_uploaded == 0
            && self.textures_uploaded == 0
            && self.programs_linked == 0
            && self.resources_created == 0
    }
}

/// The bitmaps a renderer has as textures of the context, whose pixels cannot be read: the frames
/// of a video or a camera (step D7).
pub trait Scene3DGlDeviceTextures {
    /// How `bitmap` is on this context. Asked for every layer that has such a bitmap.
    fn sampling(&self, bitmap: &SharedBitmapImpl) -> DeviceBitmap;

    /// The name of the `GL_TEXTURE_2D` of `bitmap`, asked once for each bitmap the executor does
    /// not hold. The renderer keeps the texture and its storage until the GPU has done the render.
    fn device_texture(&self, bitmap: &SharedBitmapImpl) -> Option<(i32, (u32, u32))>;
}

/// A renderer without such bitmaps.
struct NoDeviceTextures;

impl Scene3DGlDeviceTextures for NoDeviceTextures {
    fn sampling(&self, _: &SharedBitmapImpl) -> DeviceBitmap {
        DeviceBitmap::NotOnDevice
    }

    fn device_texture(&self, _: &SharedBitmapImpl) -> Option<(i32, (u32, u32))> {
        None
    }
}

/// How a draw is blended into the colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Blend {
    /// A diffuse layer: premultiplied source-over.
    SourceOver,
    /// A specular or emissive layer: added.
    Additive,
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

/// The uniform buffers of a viewport, with the bytes they hold, and the draws of its last render.
/// The caller keeps one with each output it keeps.
#[derive(Default)]
pub struct Scene3DGlViewport {
    frame: i32,
    draw_buffer: i32,
    brush_buffer: i32,
    /// The draws `draw_buffer` and `brush_buffer` hold room for.
    capacity: usize,
    frame_bytes: Vec<u8>,
    draw_bytes: Vec<u8>,
    brush_bytes: Vec<u8>,
    frame_scratch: Vec<u8>,
    draw_scratch: Vec<u8>,
    brush_scratch: Vec<u8>,
    draws: Vec<DrawRecord>,
}

impl Scene3DGlViewport {
    /// A viewport without buffers yet: they are made by its first render.
    pub fn new() -> Scene3DGlViewport {
        Scene3DGlViewport::default()
    }

    /// Deletes the buffers, with the context of `executor` current.
    pub fn dispose(&mut self, executor: &Scene3DGlExecutor) {
        for buffer in [&mut self.frame, &mut self.draw_buffer, &mut self.brush_buffer] {
            if *buffer != 0 {
                executor.gl.delete_buffer(*buffer);
                *buffer = 0;
            }
        }
        self.capacity = 0;
        self.frame_bytes.clear();
        self.draw_bytes.clear();
        self.brush_bytes.clear();
    }
}

/// A mesh on the context.
struct MeshBuffers {
    /// The vertex and index buffers; `None` for a mesh without triangles.
    buffers: Option<(i32, i32)>,
    index_count: u32,
    /// The box of its texture coordinates, which relative brushes are mapped onto.
    texture_bounds: Rect,
    last_used: u64,
}

/// The texture of a brush on the context.
struct BrushTexture {
    /// Keeps the address of a bitmap from naming another bitmap while the texture is kept.
    identity: BrushTextureIdentity,
    texture: i32,
    size: (u32, u32),
    last_used: u64,
    /// A texture of the renderer: sampled, never written or deleted.
    on_device: bool,
}

/// The attachments of the pool for one size and sample count.
struct Targets {
    size: (u32, u32),
    sample_count: u32,
    /// The multisampled colour (four samples only).
    multisampled: Option<i32>,
    depth: i32,
    /// The resolved colour `fs_resolve` reads, made on first need.
    resolved: Option<i32>,
    last_used: u64,
}

static FAILURE_OF_MULTISAMPLING_REPORTED: AtomicBool = AtomicBool::new(false);

fn report_single_sample(reason: &str) {
    if FAILURE_OF_MULTISAMPLING_REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
        logger.log_with_values(
            None,
            "3D scenes (Viewport3D) are drawn without 4x MSAA on this GL context: {Reason} \
             (docs/porting/media3d.md, section 3.20).",
            &[&reason],
        );
    }
}

/// The GL executor of one context: see the module.
pub struct Scene3DGlExecutor {
    gl: Rc<GlInterface>,
    /// A desktop GL context: the sources are compiled as GLSL 3.30.
    desktop: bool,
    vertex_array: i32,
    draw_framebuffer: i32,
    resolve_framebuffer: i32,
    vertex_shader: Option<i32>,
    programs: HashMap<BrushShader, i32>,
    resolve_program: Option<i32>,
    multisample: bool,
    max_texture_side: u32,
    meshes: HashMap<(u64, u64), MeshBuffers>,
    textures: HashMap<BrushTextureKey, BrushTexture>,
    refused_textures: HashMap<BrushTextureKey, u64>,
    empty_texture: Option<i32>,
    targets: Vec<Targets>,
    renders: u64,
    vertex_scratch: Vec<u8>,
    index_scratch: Vec<u32>,
    index_bytes: Vec<u8>,
}

/// Whether a context of `version` runs the translated shader.
fn version_is_supported(version: GlVersion) -> bool {
    match version.type_() {
        GlProfileType::OpenGLES => version.major() >= 3,
        GlProfileType::OpenGL => (version.major(), version.minor()) >= (3, 3),
    }
}

impl Scene3DGlExecutor {
    /// The executor of the current context `gl` of `version`. An error for a context below
    /// OpenGL 3.3 or OpenGL ES 3.0, or without what the executor calls.
    pub fn new(gl: Rc<GlInterface>, version: GlVersion) -> Result<Scene3DGlExecutor, Scene3DGlError> {
        if !version_is_supported(version) {
            return Err(Scene3DGlError::new(format!(
                "OpenGL {}.{} has no uniform buffers for the 3D shader",
                version.major(),
                version.minor()
            )));
        }
        let available = gl.is_get_uniform_block_index_available()
            && gl.is_uniform_block_binding_available()
            && gl.is_bind_buffer_range_available()
            && gl.is_buffer_sub_data_available()
            && gl.is_blend_func_separate_available()
            && gl.is_blend_equation_available()
            && gl.is_cull_face_available()
            && gl.is_front_face_available()
            && gl.is_color_mask_available()
            && gl.is_scissor_available()
            && gl.is_disable_vertex_attrib_array_available()
            && gl.is_renderbuffer_storage_multisample_available()
            && gl.is_blit_framebuffer_available()
            && gl.is_gen_vertex_arrays_available()
            && gl.is_bind_vertex_array_available()
            && gl.is_delete_vertex_arrays_available()
            && gl.is_pixel_storei_available()
            && gl.is_tex_sub_image_2d_available();
        if !available {
            return Err(Scene3DGlError::new("the context lacks an entry point of the 3D executor"));
        }
        // The records of the draws are bound at multiples of their stride.
        if gl.get_integerv(GL_UNIFORM_BUFFER_OFFSET_ALIGNMENT) > DRAW_STRIDE.min(BRUSH_STRIDE) as i32 {
            return Err(Scene3DGlError::new("the alignment of uniform buffers is above the stride of the draws"));
        }
        let multisample = gl.get_integerv(GL_MAX_SAMPLES) >= MULTISAMPLE_COUNT as i32;
        let max_texture_side = gl.get_integerv(GL_MAX_TEXTURE_SIZE).max(1) as u32;
        let vertex_array = gl.gen_vertex_array();
        let draw_framebuffer = gl.gen_framebuffer();
        let resolve_framebuffer = gl.gen_framebuffer();
        Ok(Scene3DGlExecutor {
            desktop: version.type_() == GlProfileType::OpenGL,
            gl,
            vertex_array,
            draw_framebuffer,
            resolve_framebuffer,
            vertex_shader: None,
            programs: HashMap::new(),
            resolve_program: None,
            multisample,
            max_texture_side,
            meshes: HashMap::new(),
            textures: HashMap::new(),
            refused_textures: HashMap::new(),
            empty_texture: None,
            targets: Vec::new(),
            renders: 0,
            vertex_scratch: Vec::new(),
            index_scratch: Vec::new(),
            index_bytes: Vec::new(),
        })
    }

    /// The samples of an antialiased render on this context.
    pub fn multisamples(&self) -> bool {
        self.multisample
    }

    /// The textures of brushes the executor keeps.
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Deletes every name the executor made, with its context current.
    pub fn dispose(&mut self) {
        let gl = self.gl.clone();
        for (_, mesh) in self.meshes.drain() {
            if let Some((vertices, indices)) = mesh.buffers {
                gl.delete_buffer(vertices);
                gl.delete_buffer(indices);
            }
        }
        for (_, texture) in self.textures.drain() {
            if !texture.on_device {
                gl.delete_texture(texture.texture);
            }
        }
        if let Some(texture) = self.empty_texture.take() {
            gl.delete_texture(texture);
        }
        for targets in self.targets.drain(..) {
            delete_targets(&gl, &targets);
        }
        for (_, program) in self.programs.drain() {
            gl.delete_program(program);
        }
        if let Some(program) = self.resolve_program.take() {
            gl.delete_program(program);
        }
        if let Some(shader) = self.vertex_shader.take() {
            gl.delete_shader(shader);
        }
        for framebuffer in [&mut self.draw_framebuffer, &mut self.resolve_framebuffer] {
            if *framebuffer != 0 {
                gl.delete_framebuffer(*framebuffer);
                *framebuffer = 0;
            }
        }
        if self.vertex_array != 0 {
            gl.delete_vertex_array(self.vertex_array);
            self.vertex_array = 0;
        }
    }

    /// An output texture of `width` by `height` pixels. `None` when the context cannot make it.
    pub fn create_output_texture(&self, width: u32, height: u32) -> Option<Scene3DGlTexture> {
        if width == 0 || height == 0 || width > self.max_texture_side || height > self.max_texture_side {
            return None;
        }
        let id = self.new_texture((width, height), std::ptr::null());
        (id != 0).then_some(Scene3DGlTexture { id, width, height })
    }

    /// Deletes a texture of [`create_output_texture`](Self::create_output_texture).
    pub fn delete_output_texture(&self, texture: Scene3DGlTexture) {
        self.gl.delete_texture(texture.id);
    }

    /// A texture of RGBA8, linear, its edges clamped, with `pixels` (or undefined ones).
    fn new_texture(&self, size: (u32, u32), pixels: *const c_void) -> i32 {
        let gl = &self.gl;
        let id = gl.gen_texture();
        gl.active_texture(GL_TEXTURE0);
        gl.bind_texture(GL_TEXTURE_2D, id);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        self.reset_unpack();
        // SAFETY: `pixels` is null or holds `width * height * 4` bytes (the callers).
        unsafe {
            gl.tex_image_2d(
                GL_TEXTURE_2D,
                0,
                GL_RGBA8,
                size.0 as i32,
                size.1 as i32,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                pixels,
            );
        }
        count(|counters| counters.textures_created += 1);
        id
    }

    /// The state of pixel uploads a renderer may have left.
    fn reset_unpack(&self) {
        let gl = &self.gl;
        gl.bind_buffer(GL_PIXEL_UNPACK_BUFFER, 0);
        gl.pixel_storei(GL_UNPACK_ALIGNMENT, 4);
        gl.pixel_storei(GL_UNPACK_ROW_LENGTH, 0);
        gl.pixel_storei(GL_UNPACK_SKIP_ROWS, 0);
        gl.pixel_storei(GL_UNPACK_SKIP_PIXELS, 0);
    }

    /// Draws `scene` into `target` with the uniform buffers of `viewport`. A failure with four
    /// samples is reported once and the render is drawn with one; a failure with one is the
    /// error. The state of the context is left changed: see the module.
    pub fn render(
        &mut self,
        viewport: &mut Scene3DGlViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DGlTarget<'_>,
    ) -> Result<Scene3DGlRenderReport, Scene3DGlError> {
        self.render_with_textures(viewport, scene, target, &NoDeviceTextures)
    }

    /// [`render`](Self::render) for a renderer that has some bitmaps of the scene as textures of
    /// the context (`textures`, step D7): they are sampled without a copy.
    pub fn render_with_textures(
        &mut self,
        viewport: &mut Scene3DGlViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DGlTarget<'_>,
        textures: &dyn Scene3DGlDeviceTextures,
    ) -> Result<Scene3DGlRenderReport, Scene3DGlError> {
        self.renders += 1;
        // Errors of the renderer before this render are not this render's.
        while self.gl.get_error() != GL_NO_ERROR {}
        let sample_count = if target.antialiased && self.multisample { MULTISAMPLE_COUNT } else { 1 };
        let mut report = Scene3DGlRenderReport::default();
        match self.render_with(viewport, scene, target, textures, sample_count, &mut report) {
            Ok(()) => Ok(report),
            Err(error) if sample_count == 1 => Err(error),
            Err(error) => {
                // Four samples failed: one from now on.
                self.multisample = false;
                report_single_sample(&error.reason);
                while self.gl.get_error() != GL_NO_ERROR {}
                let mut report = Scene3DGlRenderReport::default();
                self.render_with(viewport, scene, target, textures, 1, &mut report).map(|()| report)
            }
        }
    }

    fn render_with(
        &mut self,
        viewport: &mut Scene3DGlViewport,
        scene: &Scene3DSnapshot,
        target: &Scene3DGlTarget<'_>,
        textures: &dyn Scene3DGlDeviceTextures,
        sample_count: u32,
        report: &mut Scene3DGlRenderReport,
    ) -> Result<(), Scene3DGlError> {
        report.sample_count = sample_count;
        let texture_size = (target.texture.width, target.texture.height);
        let width = target.size.0.clamp(1, texture_size.0) as i32;
        let height = target.size.1.clamp(1, texture_size.1) as i32;

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
                let (index_count, texture_bounds) = self.mesh(&model.mesh, report);
                if index_count == 0 {
                    continue;
                }
                for (face, layers) in [(Face::Front, &model.front), (Face::Back, &model.back)] {
                    let mesh = (index_count, texture_bounds);
                    self.add_draws(viewport, model, layers, face, mesh, textures, report);
                }
            }
        }
        report.draws = viewport.draws.len() as u32;
        let additive = viewport.draws.iter().any(|draw| draw.blend == Blend::Additive);

        // The programs, before anything is drawn: a failure leaves the target as it was.
        for index in 0..viewport.draws.len() {
            let brush = viewport.draws[index].brush;
            self.program(brush, report)?;
        }
        if additive {
            self.resolve_program(report)?;
        }

        // What no render drew for a while goes; then the targets of the pool.
        self.drop_unused();
        let targets = self.targets(texture_size, sample_count, additive, report)?;
        if let Some(frame) = &frame {
            self.write_uniforms(viewport, frame, report);
        }
        let gl = self.gl.clone();
        let (multisampled, depth, resolved) = {
            let targets = &self.targets[targets];
            (targets.multisampled, targets.depth, targets.resolved)
        };

        // What the draws go into: the multisampled colour, or the texture they end up in.
        let single = if additive { resolved.unwrap_or(target.texture.id) } else { target.texture.id };
        gl.bind_framebuffer(GL_FRAMEBUFFER, self.draw_framebuffer);
        match multisampled {
            Some(color) => {
                gl.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, 0, 0);
                gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_RENDERBUFFER, color);
            }
            None => {
                gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_RENDERBUFFER, 0);
                gl.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, single, 0);
            }
        }
        gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, depth);
        if gl.check_framebuffer_status(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE {
            return Err(Scene3DGlError::new("the framebuffer of a 3D render is not complete"));
        }

        self.set_pass_state(width, height);
        gl.depth_mask(1);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear_depth(1.0);
        // The whole output is cleared, so what lies outside the image is transparent.
        gl.disable(GL_SCISSOR_TEST);
        gl.clear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
        gl.enable(GL_SCISSOR_TEST);
        if frame.is_some() {
            self.draw(viewport);
        }

        // The multisampled colour into the texture it ends up in.
        if multisampled.is_some() {
            gl.bind_framebuffer(GL_DRAW_FRAMEBUFFER, self.resolve_framebuffer);
            gl.framebuffer_texture_2d(GL_DRAW_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, single, 0);
            if gl.check_framebuffer_status(GL_DRAW_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE {
                return Err(Scene3DGlError::new("the resolve framebuffer of a 3D render is not complete"));
            }
            gl.bind_framebuffer(GL_READ_FRAMEBUFFER, self.draw_framebuffer);
            gl.disable(GL_SCISSOR_TEST);
            // The whole of the attachments: what lies outside the image is transparent there too.
            let (all_x, all_y) = (texture_size.0 as i32, texture_size.1 as i32);
            gl.blit_framebuffer(0, 0, all_x, all_y, 0, 0, all_x, all_y, GL_COLOR_BUFFER_BIT, GL_NEAREST);
            gl.enable(GL_SCISSOR_TEST);
        }

        // Decision N: the additive layers leave colours above their alpha; the resolve raises the
        // alpha of each pixel to its largest channel.
        if additive {
            if let (Some(program), Some(resolved)) = (self.resolve_program, resolved) {
                gl.bind_framebuffer(GL_FRAMEBUFFER, self.resolve_framebuffer);
                gl.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, target.texture.id, 0);
                if gl.check_framebuffer_status(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE {
                    return Err(Scene3DGlError::new("the output of a 3D render is not complete"));
                }
                gl.disable(GL_SCISSOR_TEST);
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                gl.clear(GL_COLOR_BUFFER_BIT);
                gl.enable(GL_SCISSOR_TEST);
                gl.disable(GL_DEPTH_TEST);
                gl.disable(GL_CULL_FACE);
                gl.disable(GL_BLEND);
                gl.use_program(program);
                gl.active_texture(GL_TEXTURE0 + glsl::RESOLVED_TEXTURE_UNIT as i32);
                gl.bind_texture(GL_TEXTURE_2D, resolved);
                gl.bind_vertex_array(self.vertex_array);
                for location in 0..VERTEX_ATTRIBUTE_OFFSETS.len() {
                    gl.disable_vertex_attrib_array(location as i32);
                }
                // SAFETY: a triangle made by the vertex stage from its index: no attribute is read.
                unsafe { gl.draw_arrays(GL_TRIANGLES, 0, 3) };
                gl.active_texture(GL_TEXTURE0);
            }
        }

        gl.bind_framebuffer(GL_FRAMEBUFFER, 0);
        gl.bind_vertex_array(0);
        gl.use_program(0);
        let error = gl.get_error();
        if error != GL_NO_ERROR {
            return Err(Scene3DGlError::new(format!("a 3D render failed with the GL error {error:#x}")));
        }
        // The commands reach the GPU before the renderer samples the output.
        gl.flush();
        count(|counters| counters.passes += 1);
        Ok(())
    }

    /// The fixed-function state of the draws of a render, whatever the renderer left.
    fn set_pass_state(&self, width: i32, height: i32) {
        let gl = &self.gl;
        gl.viewport(0, 0, width, height);
        gl.scissor(0, 0, width, height);
        gl.enable(GL_SCISSOR_TEST);
        gl.disable(GL_STENCIL_TEST);
        gl.disable(GL_DITHER);
        if self.desktop {
            gl.disable(GL_FRAMEBUFFER_SRGB);
        }
        gl.color_mask(1, 1, 1, 1);
        gl.enable(GL_DEPTH_TEST);
        gl.depth_func(GL_LEQUAL);
        gl.enable(GL_CULL_FACE);
        // The clip space is flipped in y: a face counterclockwise on the screen is clockwise here.
        gl.front_face(GL_CW);
        gl.enable(GL_BLEND);
        gl.blend_equation(GL_FUNC_ADD);
    }

    /// Issues the draws of `viewport`, whose uniforms are written.
    fn draw(&self, viewport: &Scene3DGlViewport) {
        let gl = &self.gl;
        gl.bind_vertex_array(self.vertex_array);
        for location in 0..VERTEX_ATTRIBUTE_OFFSETS.len() {
            gl.enable_vertex_attrib_array(location as i32);
        }
        gl.bind_buffer_range(
            GL_UNIFORM_BUFFER,
            glsl::FRAME_BLOCK_BINDING,
            viewport.frame,
            0,
            viewport.frame_bytes.len() as isize,
        );
        let mut current_program = None;
        let mut current_blend = None;
        let mut current_depth = None;
        let mut current_face = None;
        let mut current_mesh = None;
        let mut current_texture = None;
        gl.active_texture(GL_TEXTURE0 + glsl::BRUSH_TEXTURE_UNIT as i32);
        for (index, draw) in viewport.draws.iter().enumerate() {
            let (Some(&program), Some(mesh)) = (self.programs.get(&draw.brush), self.meshes.get(&draw.mesh)) else {
                continue;
            };
            let Some((vertices, indices)) = mesh.buffers else { continue };
            let texture = if samples_texture(draw.brush) {
                let texture = match &draw.texture {
                    Some(key) => self.textures.get(key).map(|texture| texture.texture),
                    None => self.empty_texture,
                };
                match texture {
                    Some(texture) => Some(texture),
                    None => continue,
                }
            } else {
                None
            };
            if current_program != Some(program) {
                gl.use_program(program);
                current_program = Some(program);
            }
            if current_blend != Some(draw.blend) {
                let destination = match draw.blend {
                    Blend::SourceOver => GL_ONE_MINUS_SRC_ALPHA,
                    Blend::Additive => GL_ONE,
                };
                gl.blend_func_separate(GL_ONE, destination, GL_ONE, destination);
                current_blend = Some(draw.blend);
            }
            if current_depth != Some(draw.depth_write) {
                gl.depth_mask(i32::from(draw.depth_write));
                current_depth = Some(draw.depth_write);
            }
            if current_face != Some(draw.face) {
                gl.cull_face(match draw.face {
                    Face::Front => GL_BACK,
                    Face::Back => GL_FRONT,
                });
                current_face = Some(draw.face);
            }
            if current_mesh != Some(draw.mesh) {
                gl.bind_buffer(GL_ARRAY_BUFFER, vertices);
                gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, indices);
                for (location, (offset, size)) in VERTEX_ATTRIBUTE_OFFSETS.iter().zip([3, 3, 2]).enumerate() {
                    // SAFETY: the offsets are within a vertex of `VERTEX_STRIDE` bytes of the
                    // bound buffer; a pointer here is an offset into it.
                    unsafe {
                        gl.vertex_attrib_pointer(
                            location as i32,
                            size,
                            GL_FLOAT,
                            0,
                            VERTEX_STRIDE as i32,
                            *offset as *const c_void,
                        );
                    }
                }
                current_mesh = Some(draw.mesh);
            }
            if let Some(texture) = texture {
                if current_texture != Some(texture) {
                    gl.bind_texture(GL_TEXTURE_2D, texture);
                    current_texture = Some(texture);
                }
            }
            gl.bind_buffer_range(
                GL_UNIFORM_BUFFER,
                glsl::DRAW_BLOCK_BINDING,
                viewport.draw_buffer,
                (index * DRAW_STRIDE) as isize,
                DRAW_STRIDE as isize,
            );
            gl.bind_buffer_range(
                GL_UNIFORM_BUFFER,
                glsl::BRUSH_BLOCK_BINDING,
                viewport.brush_buffer,
                (index * BRUSH_STRIDE) as isize,
                BRUSH_STRIDE as isize,
            );
            // SAFETY: the bound index buffer holds `index_count` `u32` indices of vertices of the
            // bound vertex buffer; the pointer is the offset 0 into it.
            unsafe {
                gl.draw_elements(GL_TRIANGLES, draw.index_count as i32, GL_UNSIGNED_INT, std::ptr::null());
            }
        }
    }

    /// Deletes what no render drew for [`LIFETIME`] renders, and textures of bitmaps that are gone.
    fn drop_unused(&mut self) {
        let renders = self.renders;
        let gl = self.gl.clone();
        self.meshes.retain(|_, mesh| {
            let keep = renders - mesh.last_used < LIFETIME;
            if let (false, Some((vertices, indices))) = (keep, mesh.buffers) {
                gl.delete_buffer(vertices);
                gl.delete_buffer(indices);
            }
            keep
        });
        self.targets.retain(|targets| {
            let keep = renders - targets.last_used < LIFETIME;
            if !keep {
                delete_targets(&gl, targets);
            }
            keep
        });
        self.textures.retain(|_, texture| {
            let keep = texture.identity.is_alive() && renders - texture.last_used < LIFETIME;
            if !keep && !texture.on_device {
                gl.delete_texture(texture.texture);
            }
            keep
        });
        self.refused_textures.retain(|_, last| renders - *last < LIFETIME);
    }

    /// The draws of the layers of one side of a model, in order, with their uniforms and the
    /// textures of their brushes. The first diffuse layer writes depth (the first layer when the
    /// side has no diffuse layer).
    #[allow(clippy::too_many_arguments)]
    fn add_draws(
        &mut self,
        viewport: &mut Scene3DGlViewport,
        model: &Scene3DModel,
        layers: &[Scene3DMaterialLayer],
        face: Face,
        (index_count, texture_bounds): (u32, Rect),
        textures: &dyn Scene3DGlDeviceTextures,
        report: &mut Scene3DGlRenderReport,
    ) {
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
                self.empty_texture(report);
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
    }

    /// The texture of `source` on the context, uploaded when its key is new. `false` when its
    /// texels cannot be read or are larger than the context takes.
    fn brush_texture(
        &mut self,
        source: &BrushTextureSource,
        textures: &dyn Scene3DGlDeviceTextures,
        report: &mut Scene3DGlRenderReport,
    ) -> bool {
        let key = source.key();
        let renders = self.renders;
        if let Some(texture) = self.textures.get_mut(&key) {
            texture.last_used = renders;
            return true;
        }
        if let Some(bitmap) = source.bitmap() {
            if textures.sampling(&**bitmap) != DeviceBitmap::NotOnDevice {
                if let Some((texture, size)) = textures.device_texture(&**bitmap) {
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
        let max_side = self.max_texture_side;
        let texels = source.texels().filter(|texels| {
            (1..=max_side).contains(&texels.width) && (1..=max_side).contains(&texels.height)
        });
        let Some(texels) = texels else {
            self.refused_textures.insert(key, renders);
            count(|counters| counters.textures_refused += 1);
            return false;
        };
        let size = (texels.width, texels.height);
        // A new version of a bitmap goes into the texture of the old one, when no draw of this
        // render uses it and it has the same size.
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
            Some(replaced) => {
                let gl = &self.gl;
                gl.active_texture(GL_TEXTURE0);
                gl.bind_texture(GL_TEXTURE_2D, replaced.texture);
                self.reset_unpack();
                // SAFETY: the texels hold `width * height * 4` bytes, the size of the texture.
                unsafe {
                    gl.tex_sub_image_2d(
                        GL_TEXTURE_2D,
                        0,
                        0,
                        0,
                        size.0 as i32,
                        size.1 as i32,
                        GL_RGBA,
                        GL_UNSIGNED_BYTE,
                        texels.rgba.as_ptr().cast(),
                    );
                }
                replaced.texture
            }
            None => {
                report.resources_created += 1;
                self.new_texture(size, texels.rgba.as_ptr().cast())
            }
        };
        report.textures_uploaded += 1;
        count(|counters| {
            counters.textures_uploaded += 1;
            counters.texture_bytes += texels.rgba.len() as u64;
        });
        let identity = source.identity();
        self.textures.insert(key, BrushTexture { identity, texture, size, last_used: renders, on_device: false });
        true
    }

    /// The transparent texture of one texel an image brush without an image samples.
    fn empty_texture(&mut self, report: &mut Scene3DGlRenderReport) {
        if self.empty_texture.is_some() {
            return;
        }
        let texel = [0u8; 4];
        self.empty_texture = Some(self.new_texture((1, 1), texel.as_ptr().cast()));
        report.resources_created += 1;
    }

    /// The buffers of `mesh` on the context, uploaded when its `(id, version)` is new; returns the
    /// number of its indices and the box of its texture coordinates.
    fn mesh(&mut self, mesh: &MeshData, report: &mut Scene3DGlRenderReport) -> (u32, Rect) {
        let key = (mesh.id, mesh.version);
        let renders = self.renders;
        if let Some(buffers) = self.meshes.get_mut(&key) {
            buffers.last_used = renders;
            return (buffers.index_count, buffers.texture_bounds);
        }
        // The version this one replaces, unless a draw of this render uses it.
        let old = self.meshes.iter().find(|(&(id, _), kept)| id == mesh.id && kept.last_used != renders);
        if let Some(old) = old.map(|(key, _)| *key) {
            if let Some(MeshBuffers { buffers: Some((vertices, indices)), .. }) = self.meshes.remove(&old) {
                self.gl.delete_buffer(vertices);
                self.gl.delete_buffer(indices);
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
            let gl = &self.gl;
            // The element binding belongs to the vertex array: no array of the renderer changes.
            gl.bind_vertex_array(self.vertex_array);
            let vertices = gl.gen_buffer();
            let indices = gl.gen_buffer();
            gl.bind_buffer(GL_ARRAY_BUFFER, vertices);
            gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, indices);
            // SAFETY: the sizes are those of the slices the pointers come from.
            unsafe {
                gl.buffer_data(
                    GL_ARRAY_BUFFER,
                    self.vertex_scratch.len() as isize,
                    self.vertex_scratch.as_ptr().cast(),
                    GL_STATIC_DRAW,
                );
                gl.buffer_data(
                    GL_ELEMENT_ARRAY_BUFFER,
                    self.index_bytes.len() as isize,
                    self.index_bytes.as_ptr().cast(),
                    GL_STATIC_DRAW,
                );
            }
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
        (index_count, texture_bounds)
    }

    /// The source of a stage for this context: GLSL ES 3.00, or GLSL 3.30 on desktop GL, which
    /// takes the same language for what the translated shader uses.
    fn compile(&self, kind: i32, stage: &GlslStage) -> Result<i32, Scene3DGlError> {
        let gl = &self.gl;
        let shader = gl.create_shader(kind);
        let replaced;
        let source = if self.desktop {
            replaced = stage.source.replacen("#version 300 es", "#version 330 core", 1);
            replaced.as_str()
        } else {
            stage.source
        };
        if let Some(log) = gl.compile_shader_and_get_error(shader, source) {
            gl.delete_shader(shader);
            return Err(Scene3DGlError::new(format!("a 3D shader does not compile: {log}")));
        }
        Ok(shader)
    }

    /// A program of `vertex` and a new fragment shader of `fragment`, with its uniform blocks and
    /// samplers bound.
    fn link(
        &self,
        vertex: i32,
        stages: [&GlslStage; 2],
        report: &mut Scene3DGlRenderReport,
    ) -> Result<i32, Scene3DGlError> {
        let gl = &self.gl;
        let fragment = self.compile(GL_FRAGMENT_SHADER, stages[1])?;
        let program = gl.create_program();
        gl.attach_shader(program, vertex);
        gl.attach_shader(program, fragment);
        let failure = gl.link_program_and_get_error(program);
        // The program keeps what it was linked from.
        gl.delete_shader(fragment);
        if let Some(log) = failure {
            gl.delete_program(program);
            return Err(Scene3DGlError::new(format!("a 3D program does not link: {log}")));
        }
        gl.use_program(program);
        for stage in stages {
            for (name, binding) in stage.uniform_blocks {
                let index = gl.get_uniform_block_index_string(program, name);
                if index != GL_INVALID_INDEX {
                    gl.uniform_block_binding(program, index, *binding);
                }
            }
            for (name, unit) in stage.samplers {
                let location = gl.get_uniform_location_string(program, name);
                if location >= 0 {
                    gl.uniform1i(location, *unit as i32);
                }
            }
        }
        report.programs_linked += 1;
        count(|counters| counters.programs_linked += 1);
        Ok(program)
    }

    /// The program of the draws of `brush`, linked on first demand.
    fn program(&mut self, brush: BrushShader, report: &mut Scene3DGlRenderReport) -> Result<i32, Scene3DGlError> {
        if let Some(&program) = self.programs.get(&brush) {
            return Ok(program);
        }
        let vertex = match self.vertex_shader {
            Some(vertex) => vertex,
            None => {
                let vertex = self.compile(GL_VERTEX_SHADER, &glsl::VERTEX)?;
                self.vertex_shader = Some(vertex);
                vertex
            }
        };
        let program = self.link(vertex, [&glsl::VERTEX, glsl::fragment_stage(brush)], report)?;
        self.programs.insert(brush, program);
        Ok(program)
    }

    /// The program of the resolve, linked on first demand.
    fn resolve_program(&mut self, report: &mut Scene3DGlRenderReport) -> Result<i32, Scene3DGlError> {
        if let Some(program) = self.resolve_program {
            return Ok(program);
        }
        let vertex = self.compile(GL_VERTEX_SHADER, &glsl::RESOLVE_VERTEX)?;
        let linked = self.link(vertex, [&glsl::RESOLVE_VERTEX, &glsl::RESOLVE_FRAGMENT], report);
        self.gl.delete_shader(vertex);
        let program = linked?;
        self.resolve_program = Some(program);
        Ok(program)
    }

    /// The index of the targets of `size` and `sample_count`, made on first demand.
    fn targets(
        &mut self,
        size: (u32, u32),
        sample_count: u32,
        additive: bool,
        report: &mut Scene3DGlRenderReport,
    ) -> Result<usize, Scene3DGlError> {
        let renders = self.renders;
        let index = match self.targets.iter().position(|t| t.size == size && t.sample_count == sample_count) {
            Some(index) => index,
            None => {
                let gl = &self.gl;
                let (width, height) = (size.0 as i32, size.1 as i32);
                let samples = sample_count as i32;
                let storage = |internal_format: i32| {
                    let renderbuffer = gl.gen_renderbuffer();
                    gl.bind_renderbuffer(GL_RENDERBUFFER, renderbuffer);
                    if samples > 1 {
                        gl.renderbuffer_storage_multisample(GL_RENDERBUFFER, samples, internal_format, width, height);
                    } else {
                        gl.renderbuffer_storage(GL_RENDERBUFFER, internal_format, width, height);
                    }
                    renderbuffer
                };
                let multisampled = (samples > 1).then(|| storage(GL_RGBA8));
                let depth = storage(GL_DEPTH_COMPONENT24);
                gl.bind_renderbuffer(GL_RENDERBUFFER, 0);
                let made = 1 + u32::from(multisampled.is_some());
                report.resources_created += made;
                count(|counters| counters.textures_created += u64::from(made));
                self.targets.push(Targets {
                    size,
                    sample_count,
                    multisampled,
                    depth,
                    resolved: None,
                    last_used: renders,
                });
                self.targets.len() - 1
            }
        };
        self.targets[index].last_used = renders;
        if additive && self.targets[index].resolved.is_none() {
            let resolved = self.new_texture(size, std::ptr::null());
            if resolved == 0 {
                return Err(Scene3DGlError::new("the resolved colour of a 3D render cannot be made"));
            }
            self.targets[index].resolved = Some(resolved);
            report.resources_created += 1;
        }
        Ok(index)
    }

    /// Writes the uniforms of the draws of `viewport` where their bytes changed.
    fn write_uniforms(
        &self,
        viewport: &mut Scene3DGlViewport,
        frame: &FrameUniforms,
        report: &mut Scene3DGlRenderReport,
    ) {
        let gl = &self.gl;
        viewport.frame_scratch.clear();
        frame.write_std140(&mut viewport.frame_scratch);
        let draws = viewport.draws.len().max(1);
        let mut made = 0u32;
        let new_buffer = |length: usize| {
            let buffer = gl.gen_buffer();
            gl.bind_buffer(GL_UNIFORM_BUFFER, buffer);
            // SAFETY: a null pointer asks for uninitialized storage of `length` bytes.
            unsafe { gl.buffer_data(GL_UNIFORM_BUFFER, length as isize, std::ptr::null(), GL_DYNAMIC_DRAW) };
            buffer
        };
        if viewport.frame == 0 {
            viewport.frame = new_buffer(viewport.frame_scratch.len());
            viewport.frame_bytes.clear();
            made += 1;
        }
        if viewport.draw_buffer == 0 || viewport.brush_buffer == 0 || viewport.capacity < draws {
            for buffer in [viewport.draw_buffer, viewport.brush_buffer] {
                if buffer != 0 {
                    gl.delete_buffer(buffer);
                }
            }
            let capacity = draws.next_power_of_two();
            viewport.draw_buffer = new_buffer(capacity * DRAW_STRIDE);
            viewport.brush_buffer = new_buffer(capacity * BRUSH_STRIDE);
            viewport.capacity = capacity;
            viewport.draw_bytes.clear();
            viewport.brush_bytes.clear();
            made += 2;
        }
        report.resources_created += made;
        count(|counters| counters.buffers_created += u64::from(made));

        let mut writes = 0u64;
        let mut bytes = 0u64;
        if viewport.frame_bytes != viewport.frame_scratch {
            gl.bind_buffer(GL_UNIFORM_BUFFER, viewport.frame);
            write_range(gl, 0, &viewport.frame_scratch);
            viewport.frame_bytes.clone_from(&viewport.frame_scratch);
            writes += 1;
            bytes += viewport.frame_scratch.len() as u64;
        }
        for (buffer, old, new, stride) in [
            (viewport.draw_buffer, &mut viewport.draw_bytes, &viewport.draw_scratch, DRAW_STRIDE),
            (viewport.brush_buffer, &mut viewport.brush_bytes, &viewport.brush_scratch, BRUSH_STRIDE),
        ] {
            gl.bind_buffer(GL_UNIFORM_BUFFER, buffer);
            if let Some(written) = write_changed(gl, old, new, stride) {
                writes += 1;
                bytes += written;
            }
            old.clone_from(new);
        }
        gl.bind_buffer(GL_UNIFORM_BUFFER, 0);
        report.uniform_bytes += bytes;
        count(|counters| {
            counters.uniform_writes += writes;
            counters.uniform_bytes += bytes;
        });
    }
}

/// Deletes the attachments of `targets`.
fn delete_targets(gl: &GlInterface, targets: &Targets) {
    if let Some(multisampled) = targets.multisampled {
        gl.delete_renderbuffer(multisampled);
    }
    gl.delete_renderbuffer(targets.depth);
    if let Some(resolved) = targets.resolved {
        gl.delete_texture(resolved);
    }
}

/// Copies `bytes` into the bound uniform buffer at `offset`.
fn write_range(gl: &GlInterface, offset: usize, bytes: &[u8]) {
    // SAFETY: the callers write within the buffer, which they made at least this large.
    unsafe { gl.buffer_sub_data(GL_UNIFORM_BUFFER, offset as isize, bytes.len() as isize, bytes.as_ptr().cast()) };
}

/// Writes the range of whole records of `stride` bytes where `new` differs from `old` (all of
/// `new` when their lengths differ) into the bound uniform buffer: the bytes written, or `None`.
fn write_changed(gl: &GlInterface, old: &[u8], new: &[u8], stride: usize) -> Option<u64> {
    if new.is_empty() {
        return None;
    }
    if old.len() != new.len() {
        write_range(gl, 0, new);
        return Some(new.len() as u64);
    }
    let first = old.chunks_exact(stride).zip(new.chunks_exact(stride)).position(|(a, b)| a != b)?;
    let last = old.chunks_exact(stride).zip(new.chunks_exact(stride)).rposition(|(a, b)| a != b)?;
    let range = first * stride..(last + 1) * stride;
    write_range(gl, range.start, &new[range.clone()]);
    Some(range.len() as u64)
}

#[cfg(test)]
#[path = "scene3d_tests.rs"]
mod tests;
