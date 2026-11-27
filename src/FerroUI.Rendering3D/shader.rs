//! The one shader of the GPU executors (`docs/porting/media3d.md`, sections 3.4 and 3.15):
//! `shaders/scene3d.wgsl`, a port of ProGPU's mesh shader restricted to what WebGL2 and OpenGL ES
//! 3.0 run, with the uniform layout of [`crate::uniforms`] and the lighting of
//! [`crate::lighting`].
//!
//! The wgpu executor (feature `wgpu`) hands the source to wgpu, which translates it for its
//! backend; the GL, Metal and Vulkan executors of later steps translate it with naga at build
//! time. The tests of this module validate it with naga and translate it to GLSL ES 3.00 for
//! WebGL2.
//!
//! The draws of a material layer have one fragment entry point per variant of the brush shader
//! (step D6, [`fragment_entry`]): a bit of the pipeline key of the executors.

use crate::brush::BrushShader;

/// The WGSL of the 3D executors.
pub const SCENE3D_WGSL: &str = include_str!("shaders/scene3d.wgsl");

/// The vertex stage of a draw: the world position and normal of a vertex and its clip position.
pub const VERTEX_ENTRY: &str = "vs_main";
/// The fragment stage of a draw: the lighting and colour of a material layer of a solid colour
/// ([`BrushShader::Solid`]).
pub const FRAGMENT_ENTRY: &str = "fs_main";
/// The fragment stage of a draw whose brush is a gradient with its stops inline
/// ([`BrushShader::Gradient`]).
pub const GRADIENT_FRAGMENT_ENTRY: &str = "fs_gradient";
/// The fragment stage of a draw whose brush is a gradient read from its lookup texture
/// ([`BrushShader::GradientTexture`]).
pub const GRADIENT_TEXTURE_FRAGMENT_ENTRY: &str = "fs_gradient_texture";
/// The fragment stage of a draw whose brush is an image ([`BrushShader::Image`]).
pub const IMAGE_FRAGMENT_ENTRY: &str = "fs_image";
/// The vertex stage of the resolve: a triangle over the target.
pub const RESOLVE_VERTEX_ENTRY: &str = "vs_resolve";
/// The fragment stage of the resolve: the resolved colour with its alpha raised to its largest
/// colour channel (decision N).
pub const RESOLVE_FRAGMENT_ENTRY: &str = "fs_resolve";

/// The bind group of the [`FrameUniforms`](crate::uniforms::FrameUniforms) of a render (binding 0).
pub const FRAME_GROUP: u32 = 0;
/// The bind group of the [`DrawUniforms`](crate::uniforms::DrawUniforms) of a draw (binding 0), bound at a
/// dynamic offset per draw.
pub const DRAW_GROUP: u32 = 1;
/// The binding (in group 1) of the [`BrushUniforms`](crate::brush::BrushUniforms) of a draw, bound at
/// a dynamic offset per draw.
pub const BRUSH_BINDING: u32 = 1;
/// The bind group of the texture of a brush (binding 0) and its sampler (binding 1), in the
/// pipelines of [`BrushShader::GradientTexture`] and [`BrushShader::Image`].
pub const BRUSH_TEXTURE_GROUP: u32 = 2;
/// The binding (in group 0 of the resolve's own layout) of the resolved colour `fs_resolve` reads.
pub const RESOLVED_BINDING: u32 = 1;

/// The fragment entry point of the draws of a brush shader.
pub fn fragment_entry(shader: BrushShader) -> &'static str {
    match shader {
        BrushShader::Solid => FRAGMENT_ENTRY,
        BrushShader::Gradient => GRADIENT_FRAGMENT_ENTRY,
        BrushShader::GradientTexture => GRADIENT_TEXTURE_FRAGMENT_ENTRY,
        BrushShader::Image => IMAGE_FRAGMENT_ENTRY,
    }
}

/// Whether the draws of a brush shader sample a texture (and bind [`BRUSH_TEXTURE_GROUP`]).
pub fn samples_texture(shader: BrushShader) -> bool {
    matches!(shader, BrushShader::GradientTexture | BrushShader::Image)
}

/// The bytes of one interleaved vertex: the position (`vec3<f32>`, location 0), the normal
/// (`vec3<f32>`, location 1) and the texture coordinate (`vec2<f32>`, location 2).
pub const VERTEX_STRIDE: usize = 32;
/// The byte offsets of the attributes of a vertex, by location.
pub const VERTEX_ATTRIBUTE_OFFSETS: [usize; 3] = [0, 12, 24];

/// Appends the interleaved vertices of `mesh` to `out` ([`VERTEX_STRIDE`] bytes each, in the byte
/// order of the machine), with the normal and texture coordinate the CPU reference takes for a
/// vertex that has none (zero), and its triangles to `indices`: those whose three indices name a
/// vertex, as the CPU reference draws them. Returns the number of indices appended.
pub fn write_mesh(mesh: &ferroui_base::rendering::scene3d::MeshData, out: &mut Vec<u8>, indices: &mut Vec<u32>) -> usize {
    out.reserve(mesh.positions.len() * VERTEX_STRIDE);
    for (index, position) in mesh.positions.iter().enumerate() {
        let normal = mesh.normals.get(index).map_or([0.0; 3], |n| [n.x, n.y, n.z]);
        let uv = mesh.texture_coordinates.get(index).map_or([0.0; 2], |uv| [uv.x, uv.y]);
        for value in [position.x, position.y, position.z, normal[0], normal[1], normal[2], uv[0], uv[1]] {
            out.extend_from_slice(&value.to_ne_bytes());
        }
    }
    let count = mesh.positions.len();
    let start = indices.len();
    for triangle in mesh.indices.chunks_exact(3) {
        if triangle.iter().all(|&index| (index as usize) < count) {
            indices.extend_from_slice(triangle);
        }
    }
    indices.len() - start
}

/// The shader in the Metal Shading Language (feature `msl`, step D5,
/// `docs/porting/media3d.md`, section 3.19): [`SCENE3D_WGSL`] translated by naga at build time
/// (`build.rs`), for the Metal executor of `ferroui-metal` that the Skia backend draws with on
/// Graphite. The device compiles [`msl::SOURCE`] once (`newLibraryWithSource`); nothing of naga
/// is linked.
///
/// Every function binds its resources at the same slots of the argument tables: the uniform
/// buffers at [`msl::FRAME_BUFFER_INDEX`], [`msl::DRAW_BUFFER_INDEX`] and
/// [`msl::BRUSH_BUFFER_INDEX`] (the dynamic offsets of the wgpu executor are buffer offsets
/// there), the vertices at [`msl::VERTEX_BUFFER_INDEX`], the texture of a brush at
/// [`msl::BRUSH_TEXTURE_INDEX`] and the resolved colour of the resolve at
/// [`msl::RESOLVED_TEXTURE_INDEX`]. The sampler of the brush textures (linear, the edges clamped,
/// no mipmaps) is inlined into the functions, so no sampler is bound.
#[cfg(feature = "msl")]
pub mod msl {
    use crate::brush::BrushShader;

    include!(concat!(env!("OUT_DIR"), "/scene3d_msl.rs"));

    /// The fragment function of the draws of a brush shader.
    pub fn fragment_function(shader: BrushShader) -> &'static str {
        match shader {
            BrushShader::Solid => FRAGMENT_FUNCTION,
            BrushShader::Gradient => GRADIENT_FRAGMENT_FUNCTION,
            BrushShader::GradientTexture => GRADIENT_TEXTURE_FRAGMENT_FUNCTION,
            BrushShader::Image => IMAGE_FRAGMENT_FUNCTION,
        }
    }
}

/// The shader in GLSL ES 3.00 (feature `glsl`, step D8, `docs/porting/media3d.md`, section 3.20):
/// [`SCENE3D_WGSL`] translated by naga at build time (`build.rs`), one source per entry point, with
/// the names of its uniform blocks and samplers, for the GL executor of `ferroui-opengl`. The clip
/// space is GL's with y flipped, so the first row of an output is the top of the scene and a front
/// face is clockwise in window coordinates. Nothing of naga is linked.
#[cfg(feature = "glsl")]
pub mod glsl {
    use crate::brush::BrushShader;

    /// The binding point of the frame uniforms.
    pub const FRAME_BLOCK_BINDING: u32 = 0;
    /// The binding point of the draw uniforms.
    pub const DRAW_BLOCK_BINDING: u32 = 1;
    /// The binding point of the brush uniforms.
    pub const BRUSH_BLOCK_BINDING: u32 = 2;
    /// The texture unit of the texture of a brush.
    pub const BRUSH_TEXTURE_UNIT: u32 = 0;
    /// The texture unit of the resolved colour.
    pub const RESOLVED_TEXTURE_UNIT: u32 = 1;

    /// One entry point in GLSL ES 3.00.
    pub struct GlslStage {
        /// The source.
        pub source: &'static str,
        /// The names of its uniform blocks, with their binding points.
        pub uniform_blocks: &'static [(&'static str, u32)],
        /// The names of its samplers, with their texture units.
        pub samplers: &'static [(&'static str, u32)],
    }

    include!(concat!(env!("OUT_DIR"), "/scene3d_glsl.rs"));

    /// The fragment stage of the draws of a brush shader.
    pub fn fragment_stage(shader: BrushShader) -> &'static GlslStage {
        match shader {
            BrushShader::Solid => &FRAGMENT,
            BrushShader::Gradient => &GRADIENT_FRAGMENT,
            BrushShader::GradientTexture => &GRADIENT_TEXTURE_FRAGMENT,
            BrushShader::Image => &IMAGE_FRAGMENT,
        }
    }
}

/// The shader in SPIR-V 1.0 (feature `spirv`, step D9, `docs/porting/media3d.md`, section 3.21):
/// [`SCENE3D_WGSL`] translated by naga at build time (`build.rs`), one module per entry point, each
/// with the `@group` and `@binding` of the source as its descriptor sets and bindings. The clip
/// space is the source's: the executor draws with a viewport of negative height. Nothing of naga
/// is linked.
#[cfg(feature = "spirv")]
pub mod spirv {
    use crate::brush::BrushShader;

    include!(concat!(env!("OUT_DIR"), "/scene3d_spirv.rs"));

    /// The fragment module of the draws of a brush shader, and its entry point.
    pub fn fragment_module(shader: BrushShader) -> (&'static [u8], &'static std::ffi::CStr) {
        match shader {
            BrushShader::Solid => (FRAGMENT, c"fs_main"),
            BrushShader::Gradient => (GRADIENT_FRAGMENT, c"fs_gradient"),
            BrushShader::GradientTexture => (GRADIENT_TEXTURE_FRAGMENT, c"fs_gradient_texture"),
            BrushShader::Image => (IMAGE_FRAGMENT, c"fs_image"),
        }
    }
}

#[cfg(test)]
#[path = "shader_tests.rs"]
mod tests;
