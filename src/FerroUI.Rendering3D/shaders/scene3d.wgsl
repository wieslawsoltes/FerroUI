// The shader of FerroUI's 3D scenes (#608, docs/porting/media3d.md, sections 3.4 and 3.15): one
// WGSL source for every GPU executor. The wgpu executor (Vello hybrid and GPU) hands it to wgpu as
// it is; the GL, Metal and Vulkan executors (steps D5, D8, D9) translate it with naga at build time.
//
// Ported from ProGPU's `Mesh3DSolid.wgsl` (MIT License): the vertex stage and `ComputeWpfLighting`,
// rewritten to the subset that WebGL2 and OpenGL ES 3.0 run:
//
// - uniform buffers only (no storage buffers, no vertex pulling): the camera and the lights of a
//   render in one block (group 0), the matrices and colours of a draw in another, bound at a
//   dynamic offset per draw (group 1);
// - interleaved vertex attributes (position, normal, texture coordinate) drawn with u32 indices;
// - `@builtin(front_facing)` for the side of a face;
// - a fixed loop bound of 16 lights, f32 throughout, and uniform blocks under 16 KiB.
//
// The functions are those of the CPU reference (`lighting.rs`, `cpu/mod.rs`) in the same order of
// operations, so the comparison tests check math, not translations:
//
// - the lights past the 16th are left out (the uniforms hold 16; decision L);
// - a diffuse layer is blended premultiplied source-over, a specular or emissive layer is added
//   with alpha 0 (the blend states of the pipelines);
// - the first diffuse layer of a side writes depth with `Less`, the others test `LessEqual`
//   without writing (the depth states of the pipelines);
// - `fs_resolve` raises the alpha of a pixel to its largest colour channel after the
//   multisampled colour is resolved, so added light over a transparent pixel is a valid
//   premultiplied colour (decision N).
//
// The structures below are `uniforms::WGSL_DECLARATIONS` of the crate, member for member: the
// tests of `shader.rs` check that this file holds them unchanged and that naga lays them out at
// the offsets of the Rust structures.

const MAX_LIGHTS: u32 = 16u;

struct LightUniforms {
    metadata: vec4<f32>,
    color: vec4<f32>,
    position_range: vec4<f32>,
    direction_inner_cos: vec4<f32>,
    attenuation_outer_cos: vec4<f32>,
};

struct FrameUniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_count: vec4<u32>,
    lights: array<LightUniforms, 16>,
};

struct DrawUniforms {
    world: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
    color: vec4<f32>,
    ambient_color: vec4<f32>,
    material: vec4<f32>,
};

// The camera and the lights of the render.
@group(0) @binding(0) var<uniform> frame: FrameUniforms;
// The draw: one material layer of one model on one side of its faces, at a dynamic offset.
@group(1) @binding(0) var<uniform> draw: DrawUniforms;
// The resolved colour of the render, which `fs_resolve` reads (its own pipeline layout).
@group(0) @binding(1) var resolved: texture_2d<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture_coordinate: vec2<f32>,
};

struct VertexOutput {
    // Invariant: the layers of a material are drawn over the same triangles with `LessEqual`, so
    // every draw computes the same depth for them.
    @builtin(position) @invariant clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    // Read by the brushes of step D6.
    @location(2) texture_coordinate: vec2<f32>,
};

// The vector at unit length; the zero vector stays zero (WGSL's `normalize` of zero is undefined:
// the CPU reference keeps it zero, which lights nothing).
fn safe_normalize(a: vec3<f32>) -> vec3<f32> {
    let len = length(a);
    if (len > 0.0) {
        return a * (1.0 / len);
    }
    return vec3<f32>(0.0);
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world = draw.world * vec4<f32>(input.position, 1.0);
    let normal = draw.normal_matrix * vec4<f32>(input.normal, 0.0);
    output.clip_position = frame.view_projection * world;
    output.world_position = world.xyz;
    output.world_normal = safe_normalize(normal.xyz);
    output.texture_coordinate = input.texture_coordinate;
    return output;
}

// The sums of the lights at a point: ambient, Lambert and Blinn-Phong (`LightingTerms`).
struct LightingTerms {
    ambient: vec3<f32>,
    diffuse: vec3<f32>,
    specular: vec3<f32>,
};

// ProGPU's `ComputeWpfLighting` (`lighting::compute_wpf_lighting`): the loop over at most 16
// lights, the specular sum only when `with_specular`.
fn compute_wpf_lighting(n: vec3<f32>, v: vec3<f32>, world_position: vec3<f32>, shininess_in: f32, with_specular: bool) -> LightingTerms {
    var terms: LightingTerms;
    terms.ambient = vec3<f32>(0.0);
    terms.diffuse = vec3<f32>(0.0);
    terms.specular = vec3<f32>(0.0);
    let shininess = max(shininess_in, 0.001);
    let count = min(frame.light_count.x, MAX_LIGHTS);
    for (var index = 0u; index < MAX_LIGHTS; index = index + 1u) {
        if (index >= count) {
            break;
        }
        let light = frame.lights[index];
        let kind = u32(light.metadata.x + 0.5);
        let color = light.color.xyz;
        if (kind == 0u) {
            terms.ambient = terms.ambient + color;
            continue;
        }
        let direction = light.direction_inner_cos.xyz;
        var l = safe_normalize(direction * -1.0);
        var attenuation = 1.0;
        if (kind >= 2u) {
            let to_light = light.position_range.xyz - world_position;
            let distance = length(to_light);
            if (distance <= 0.000001 || distance > light.position_range.w) {
                continue;
            }
            l = to_light * (1.0 / distance);
            let factors = light.attenuation_outer_cos.xyz;
            attenuation = 1.0 / max(factors.x + factors.y * distance + factors.z * distance * distance, 1.0);
            if (kind >= 3u) {
                let rho = max(dot(safe_normalize(direction * -1.0), l), 0.0);
                let outer_cos = light.attenuation_outer_cos.w;
                attenuation = attenuation
                    * clamp((rho - outer_cos) / max(light.direction_inner_cos.w - outer_cos, 0.000001), 0.0, 1.0);
            }
        }
        let n_dot_l = max(dot(n, l), 0.0);
        if (n_dot_l <= 0.0) {
            continue;
        }
        terms.diffuse = terms.diffuse + color * (n_dot_l * attenuation);
        if (with_specular) {
            let h = safe_normalize(v + l);
            let highlight = pow(max(dot(n, h), 0.0), shininess) * attenuation;
            terms.specular = terms.specular + color * highlight;
        }
    }
    return terms;
}

// The direction from a point toward the viewer (`lighting::view_direction`).
fn view_direction(world_position: vec3<f32>) -> vec3<f32> {
    let camera = frame.camera_position;
    if (camera.w > 0.5) {
        return safe_normalize(camera.xyz - world_position);
    }
    return safe_normalize(camera.xyz);
}

// The colour a draw writes at a point, premultiplied (`lighting::shade`). `side` is 1 on a front
// face and -1 on a back face, whose normal is turned around.
fn shade(normal: vec3<f32>, world_position: vec3<f32>, side: f32) -> vec4<f32> {
    let color = draw.color.xyz;
    let alpha = clamp(draw.color.w, 0.0, 1.0);
    let kind = u32(draw.material.x + 0.5);
    if (kind >= 2u) {
        // Emissive: the colour without light, added.
        return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)) * alpha, 0.0);
    }
    let n = safe_normalize(normal * side);
    let specular = kind == 1u;
    var v = vec3<f32>(0.0);
    if (specular) {
        v = view_direction(world_position);
    }
    let terms = compute_wpf_lighting(n, v, world_position, draw.material.y, specular);
    if (specular) {
        return vec4<f32>(clamp(color * terms.specular, vec3<f32>(0.0), vec3<f32>(1.0)) * alpha, 0.0);
    }
    let light = terms.ambient * draw.ambient_color.xyz + terms.diffuse;
    return vec4<f32>(clamp(color * light, vec3<f32>(0.0), vec3<f32>(1.0)) * alpha, alpha);
}

@fragment
fn fs_main(input: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    // The pipeline of a draw culls the other side, so this is the sign of the draw's
    // `material.z`: ProGPU's `fs_main` turns the normal of a back face the same way.
    let side = select(-1.0, 1.0, front_facing);
    return shade(input.world_normal, input.world_position, side);
}

// The resolve of decision N: a triangle over the target, each pixel the resolved colour with its
// alpha raised to its largest colour channel.
@vertex
fn vs_resolve(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_resolve(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let c = textureLoad(resolved, vec2<i32>(floor(fragment.xy)), 0);
    return vec4<f32>(c.xyz, max(c.w, max(c.x, max(c.y, c.z))));
}
