//! The one shader of the GPU executors (`docs/porting/media3d.md`, sections 3.4 and 3.15):
//! `shaders/scene3d.wgsl`, a port of ProGPU's mesh shader restricted to what WebGL2 and OpenGL ES
//! 3.0 run, with the uniform layout of [`crate::uniforms`] and the lighting of
//! [`crate::lighting`].
//!
//! The wgpu executor (feature `wgpu`) hands the source to wgpu, which translates it for its
//! backend; the GL, Metal and Vulkan executors of later steps translate it with naga at build
//! time. The tests of this module validate it with naga and translate it to GLSL ES 3.00 for
//! WebGL2.

/// The WGSL of the 3D executors.
pub const SCENE3D_WGSL: &str = include_str!("shaders/scene3d.wgsl");

/// The vertex stage of a draw: the world position and normal of a vertex and its clip position.
pub const VERTEX_ENTRY: &str = "vs_main";
/// The fragment stage of a draw: the lighting and colour of a material layer.
pub const FRAGMENT_ENTRY: &str = "fs_main";
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
/// The binding (in group 0 of the resolve's own layout) of the resolved colour `fs_resolve` reads.
pub const RESOLVED_BINDING: u32 = 1;

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

#[cfg(test)]
#[path = "shader_tests.rs"]
mod tests;
