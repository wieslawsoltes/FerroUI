use super::*;
use crate::uniforms::{DrawUniforms, FrameUniforms, LightUniforms, MAX_LIGHTS, WGSL_DECLARATIONS};
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::rendering::scene3d::MeshData;
use std::mem::{offset_of, size_of};

/// The shader parsed and validated by naga, as wgpu validates it before it creates a module.
fn module() -> (naga::Module, naga::valid::ModuleInfo) {
    let module = naga::front::wgsl::parse_str(SCENE3D_WGSL)
        .unwrap_or_else(|error| panic!("scene3d.wgsl does not parse:\n{}", error.emit_to_string(SCENE3D_WGSL)));
    let mut validator =
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty());
    let info = validator
        .validate(&module)
        .unwrap_or_else(|error| panic!("scene3d.wgsl is not valid:\n{}", error.emit_to_string(SCENE3D_WGSL)));
    (module, info)
}

/// One entry point in GLSL ES 3.00 for WebGL2, with naga's reflection.
fn glsl(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
    stage: naga::ShaderStage,
    entry_point: &str,
) -> (String, naga::back::glsl::ReflectionInfo) {
    let options = naga::back::glsl::Options {
        version: naga::back::glsl::Version::Embedded { version: 300, is_webgl: true },
        ..naga::back::glsl::Options::default()
    };
    let pipeline_options =
        naga::back::glsl::PipelineOptions { shader_stage: stage, entry_point: entry_point.to_owned(), multiview: None };
    let mut output = String::new();
    let reflection = {
        let mut writer = naga::back::glsl::Writer::new(
            &mut output,
            module,
            info,
            &options,
            &pipeline_options,
            naga::proc::BoundsCheckPolicies::default(),
        )
        .unwrap_or_else(|error| panic!("{entry_point}: {error}"));
        writer.write().unwrap_or_else(|error| panic!("{entry_point}: {error}"))
    };
    (output, reflection)
}

/// What every stage in GLSL ES 3.00 is: one `main`, balanced, with a float precision in a
/// fragment stage.
fn assert_glsl_es(source: &str, fragment: bool, what: &str) {
    assert!(source.starts_with("#version 300 es"), "{what}: {source}");
    assert_eq!(1, source.matches("void main(").count(), "{what}: {source}");
    if fragment {
        assert!(source.contains("precision highp float;"), "{what}: {source}");
    }
    assert_eq!(source.matches('{').count(), source.matches('}').count(), "{what}: {source}");
    assert_eq!(source.matches('(').count(), source.matches(')').count(), "{what}: {source}");
}

#[test]
fn scene3d_wgsl_is_valid_and_has_its_entry_points() {
    let (module, _) = module();
    let stages: Vec<(&str, naga::ShaderStage)> =
        module.entry_points.iter().map(|entry| (entry.name.as_str(), entry.stage)).collect();
    for (name, stage) in [
        (VERTEX_ENTRY, naga::ShaderStage::Vertex),
        (FRAGMENT_ENTRY, naga::ShaderStage::Fragment),
        (RESOLVE_VERTEX_ENTRY, naga::ShaderStage::Vertex),
        (RESOLVE_FRAGMENT_ENTRY, naga::ShaderStage::Fragment),
    ] {
        assert!(stages.contains(&(name, stage)), "{name} is a {stage:?} entry point: {stages:?}");
    }
    assert_eq!(4, stages.len(), "{stages:?}");
}

#[test]
fn scene3d_wgsl_translates_to_glsl_es_3_for_webgl2() {
    let (module, info) = module();
    let (vertex, vertex_reflection) = glsl(&module, &info, naga::ShaderStage::Vertex, VERTEX_ENTRY);
    assert_glsl_es(&vertex, false, VERTEX_ENTRY);
    assert!(vertex.contains("gl_Position"), "{vertex}");
    // The depth of the layers of a material is the same in every draw.
    assert!(vertex.contains("invariant gl_Position;"), "{vertex}");
    // The frame and the draw: two uniform blocks, no texture.
    assert_eq!(2, vertex_reflection.uniforms.len(), "{vertex}");
    assert!(vertex_reflection.texture_mapping.is_empty(), "{vertex}");

    let (fragment, fragment_reflection) = glsl(&module, &info, naga::ShaderStage::Fragment, FRAGMENT_ENTRY);
    assert_glsl_es(&fragment, true, FRAGMENT_ENTRY);
    assert!(fragment.contains("gl_FrontFacing"), "{fragment}");
    assert_eq!(2, fragment_reflection.uniforms.len(), "{fragment}");
    assert!(fragment_reflection.texture_mapping.is_empty(), "{fragment}");
    // The uniform blocks are std140, the layout of the Rust structures.
    assert!(fragment.contains("std140"), "{fragment}");

    let (resolve_vertex, _) = glsl(&module, &info, naga::ShaderStage::Vertex, RESOLVE_VERTEX_ENTRY);
    assert_glsl_es(&resolve_vertex, false, RESOLVE_VERTEX_ENTRY);
    assert!(resolve_vertex.contains("gl_VertexID"), "{resolve_vertex}");
    let (resolve_fragment, resolve_reflection) =
        glsl(&module, &info, naga::ShaderStage::Fragment, RESOLVE_FRAGMENT_ENTRY);
    assert_glsl_es(&resolve_fragment, true, RESOLVE_FRAGMENT_ENTRY);
    // The resolved colour is fetched by texel, without a sampler of its own.
    assert!(resolve_fragment.contains("texelFetch("), "{resolve_fragment}");
    assert_eq!(1, resolve_reflection.texture_mapping.len(), "{resolve_fragment}");
    assert!(resolve_reflection.uniforms.is_empty(), "{resolve_fragment}");
}

#[test]
fn the_shader_holds_the_uniform_declarations_of_the_cpu_reference() {
    assert!(
        SCENE3D_WGSL.contains(WGSL_DECLARATIONS),
        "scene3d.wgsl declares the structures of uniforms.rs (WGSL_DECLARATIONS) unchanged"
    );
    assert!(SCENE3D_WGSL.contains(&format!("const MAX_LIGHTS: u32 = {MAX_LIGHTS}u;")));
}

#[test]
fn naga_lays_the_uniforms_out_as_the_rust_structures() {
    let (module, _) = module();
    let members = |name: &str| -> (u32, Vec<(String, u32)>) {
        let (_, ty) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("the shader declares {name}"));
        match &ty.inner {
            naga::TypeInner::Struct { members, span } => (
                *span,
                members.iter().map(|member| (member.name.clone().unwrap_or_default(), member.offset)).collect(),
            ),
            other => panic!("{name} is a structure: {other:?}"),
        }
    };
    let owned = |members: &[(&str, usize)]| -> Vec<(String, u32)> {
        members.iter().map(|(name, offset)| ((*name).to_owned(), *offset as u32)).collect()
    };

    let (span, light) = members("LightUniforms");
    assert_eq!(size_of::<LightUniforms>() as u32, span);
    assert_eq!(
        owned(&[
            ("metadata", offset_of!(LightUniforms, metadata)),
            ("color", offset_of!(LightUniforms, color)),
            ("position_range", offset_of!(LightUniforms, position_range)),
            ("direction_inner_cos", offset_of!(LightUniforms, direction_inner_cos)),
            ("attenuation_outer_cos", offset_of!(LightUniforms, attenuation_outer_cos)),
        ]),
        light
    );

    let (span, frame) = members("FrameUniforms");
    assert_eq!(size_of::<FrameUniforms>() as u32, span);
    assert_eq!(
        owned(&[
            ("view_projection", offset_of!(FrameUniforms, view_projection)),
            ("camera_position", offset_of!(FrameUniforms, camera_position)),
            ("light_count", offset_of!(FrameUniforms, light_count)),
            ("lights", offset_of!(FrameUniforms, lights)),
        ]),
        frame
    );

    let (span, draw) = members("DrawUniforms");
    assert_eq!(size_of::<DrawUniforms>() as u32, span);
    assert_eq!(
        owned(&[
            ("world", offset_of!(DrawUniforms, world)),
            ("normal_matrix", offset_of!(DrawUniforms, normal_matrix)),
            ("color", offset_of!(DrawUniforms, color)),
            ("ambient_color", offset_of!(DrawUniforms, ambient_color)),
            ("material", offset_of!(DrawUniforms, material)),
        ]),
        draw
    );
}

/// What WebGL2 and OpenGL ES 3.0 run: no storage buffers, uniform blocks under 16 KiB, and the
/// three vertex attributes of [`VERTEX_ATTRIBUTE_OFFSETS`].
#[test]
fn the_shader_uses_only_what_webgl2_runs() {
    let (module, _) = module();
    for (_, variable) in module.global_variables.iter() {
        match variable.space {
            naga::AddressSpace::Uniform => {
                let size = module.types[variable.ty].inner.size(module.to_ctx());
                assert!(size <= 16 * 1024, "{:?}: a uniform block of {size} bytes", variable.name);
            }
            naga::AddressSpace::Handle => {}
            other => panic!("{:?} is in the address space {other:?}: only uniforms and textures", variable.name),
        }
    }
    let vertex = module.entry_points.iter().find(|entry| entry.name == VERTEX_ENTRY).expect("vs_main");
    let argument = &vertex.function.arguments[0];
    match &module.types[argument.ty].inner {
        naga::TypeInner::Struct { members, .. } => {
            let locations: Vec<u32> = members
                .iter()
                .filter_map(|member| match member.binding {
                    Some(naga::Binding::Location { location, .. }) => Some(location),
                    _ => None,
                })
                .collect();
            assert_eq!(vec![0, 1, 2], locations);
            let sizes: Vec<u32> = members.iter().map(|member| module.types[member.ty].inner.size(module.to_ctx())).collect();
            assert_eq!(VERTEX_STRIDE as u32, sizes.iter().sum::<u32>());
            let mut offset = 0;
            for (size, expected) in sizes.iter().zip(VERTEX_ATTRIBUTE_OFFSETS) {
                assert_eq!(expected as u32, offset);
                offset += size;
            }
        }
        other => panic!("the vertex input is a structure: {other:?}"),
    }
}

#[test]
fn a_mesh_is_written_interleaved_with_the_triangles_the_cpu_reference_draws() {
    let mesh = MeshData {
        id: 1,
        version: 1,
        positions: vec![Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0)],
        // One normal short, no texture coordinates: zero, as the CPU reference takes them.
        normals: vec![Vector3::new(0.0, 0.0, 1.0); 2],
        normals_generated: false,
        texture_coordinates: vec![Vector2::new(0.5, 0.25)],
        // A whole triangle, one naming a vertex that does not exist, and a partial one.
        indices: vec![0, 1, 2, 0, 1, 7, 2, 1],
        bounds: None,
    };
    let mut bytes = Vec::new();
    let mut indices = Vec::new();
    assert_eq!(3, write_mesh(&mesh, &mut bytes, &mut indices));
    assert_eq!(vec![0, 1, 2], indices);
    assert_eq!(3 * VERTEX_STRIDE, bytes.len());
    let floats: Vec<f32> =
        bytes.chunks_exact(4).map(|chunk| f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])).collect();
    assert_eq!(&[0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5, 0.25], &floats[0..8]);
    assert_eq!(&[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0], &floats[8..16]);
    assert_eq!(&[0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], &floats[16..24]);
}
