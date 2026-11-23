//! What the GL executor decides without a context. The draws are held to the CPU reference on real
//! contexts by the comparison scenes of the Skia backend.

use super::*;

#[test]
fn the_executor_takes_opengl_3_3_and_opengl_es_3() {
    let supported = |profile, major, minor| version_is_supported(GlVersion::new(profile, major, minor));
    assert!(supported(GlProfileType::OpenGLES, 3, 0));
    assert!(supported(GlProfileType::OpenGLES, 3, 2));
    assert!(!supported(GlProfileType::OpenGLES, 2, 0));
    assert!(supported(GlProfileType::OpenGL, 3, 3));
    assert!(supported(GlProfileType::OpenGL, 4, 1));
    assert!(!supported(GlProfileType::OpenGL, 3, 2));
    assert!(!supported(GlProfileType::OpenGL, 2, 1));
}

#[test]
fn every_stage_names_the_bindings_of_the_shader() {
    for stage in [&glsl::VERTEX, &glsl::RESOLVE_VERTEX, &glsl::RESOLVE_FRAGMENT, &glsl::FRAGMENT] {
        assert!(stage.source.starts_with("#version 300 es"));
    }
    let bindings = |stage: &GlslStage| stage.uniform_blocks.iter().map(|(_, binding)| *binding).collect::<Vec<_>>();
    // The frame and the draw for the vertices; the brush too for a brush that reads it.
    let mut vertex = bindings(&glsl::VERTEX);
    vertex.sort_unstable();
    assert_eq!(vec![glsl::FRAME_BLOCK_BINDING, glsl::DRAW_BLOCK_BINDING], vertex);
    let mut image = bindings(&glsl::IMAGE_FRAGMENT);
    image.sort_unstable();
    assert_eq!(vec![glsl::FRAME_BLOCK_BINDING, glsl::DRAW_BLOCK_BINDING, glsl::BRUSH_BLOCK_BINDING], image);
    assert_eq!(1, glsl::IMAGE_FRAGMENT.samplers.len());
    assert_eq!(glsl::BRUSH_TEXTURE_UNIT, glsl::IMAGE_FRAGMENT.samplers[0].1);
    assert_eq!(1, glsl::RESOLVE_FRAGMENT.samplers.len());
    assert_eq!(glsl::RESOLVED_TEXTURE_UNIT, glsl::RESOLVE_FRAGMENT.samplers[0].1);
    assert!(glsl::RESOLVE_FRAGMENT.uniform_blocks.is_empty());
}

#[test]
fn the_clip_space_of_the_stages_is_glsl() {
    // Depth from 0 to 1 and y up become GL's: the vertex stage is written with the adjustment.
    assert!(glsl::VERTEX.source.contains("gl_Position.yz"), "{}", glsl::VERTEX.source);
}

#[test]
fn a_report_of_uniforms_alone_is_the_steady_state() {
    let report = Scene3DGlRenderReport { sample_count: 4, draws: 3, uniform_bytes: 64, ..Default::default() };
    assert!(report.is_uniform_only());
    assert!(!Scene3DGlRenderReport { programs_linked: 1, ..report }.is_uniform_only());
    assert!(!Scene3DGlRenderReport { meshes_uploaded: 1, ..report }.is_uniform_only());
    assert!(!Scene3DGlRenderReport { textures_uploaded: 1, ..report }.is_uniform_only());
    assert!(!Scene3DGlRenderReport { resources_created: 1, ..report }.is_uniform_only());
}
