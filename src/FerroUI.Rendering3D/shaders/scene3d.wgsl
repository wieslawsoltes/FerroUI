// The shader of FerroUI's 3D scenes (#608, docs/porting/media3d.md, sections 3.4 and 3.15): one
// WGSL source for every GPU executor. The wgpu executor (Vello hybrid and GPU) hands it to wgpu as
// it is; the GL, Metal and Vulkan executors (steps D5, D8, D9) translate it with naga at build time.
//
// Ported from ProGPU's `Mesh3DSolid.wgsl` (MIT License): the vertex stage and `ComputeWpfLighting`,
// rewritten to the subset that WebGL2 and OpenGL ES 3.0 run:
//
// - uniform buffers only (no storage buffers, no vertex pulling): the camera and the lights of a
//   render in one block (group 0), the matrices and colours of a draw and the uniforms of its brush
//   in two more, each bound at a dynamic offset per draw (group 1);
// - one texture with its sampler for the brush of a draw (group 2: a gradient's lookup texture, the
//   pixels of an image brush), sampled with `textureSampleLevel` (no derivatives);
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
//   premultiplied colour (decision N);
// - the brushes of step D6 are sampled as `brush.rs` samples them (`brush_point`, `gradient_t`,
//   `inline_gradient`, `gradient_texture_coordinate`, `image_coordinate`), one fragment entry
//   point per variant of the brush shader (`fs_main` for solid colours, `fs_gradient`,
//   `fs_gradient_texture`, `fs_image`), a bit of the pipeline key.
//
// The structures below are `uniforms::WGSL_DECLARATIONS` and `brush::BRUSH_WGSL_DECLARATIONS` of
// the crate, member for member: the tests of `shader.rs` check that this file holds them unchanged
// and that naga lays them out at the offsets of the Rust structures.

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

struct BrushUniforms {
    metadata: vec4<f32>,
    to_brush_x: vec4<f32>,
    to_brush_y: vec4<f32>,
    geometry: vec4<f32>,
    geometry2: vec4<f32>,
    clip: vec4<f32>,
    stop_offsets: array<vec4<f32>, 2>,
    stop_colors: array<vec4<f32>, 8>,
};

// The inline stops of a gradient (`brush::MAX_INLINE_STOPS`).
const MAX_STOPS: u32 = 8u;
// The texels of a gradient's lookup texture (`brush::GRADIENT_TEXTURE_WIDTH`).
const GRADIENT_TEXTURE_WIDTH: f32 = 256.0;
// The tolerance of the tests against the edges of a tile or a viewbox (`brush::EDGE_EPSILON`).
const EDGE_EPSILON: f32 = 0.00001;

// The camera and the lights of the render.
@group(0) @binding(0) var<uniform> frame: FrameUniforms;
// The draw: one material layer of one model on one side of its faces, at a dynamic offset.
@group(1) @binding(0) var<uniform> draw: DrawUniforms;
// The brush of the draw, at a dynamic offset of its own (step D6).
@group(1) @binding(1) var<uniform> brush: BrushUniforms;
// The texture of the brush: a gradient's lookup texture or the pixels of an image (step D6).
@group(2) @binding(0) var brush_texture: texture_2d<f32>;
@group(2) @binding(1) var brush_sampler: sampler;
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
    // Read by the brushes of step D6, interpolated with the perspective.
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
            let light_distance = length(to_light);
            if (light_distance <= 0.000001 || light_distance > light.position_range.w) {
                continue;
            }
            l = to_light * (1.0 / light_distance);
            let factors = light.attenuation_outer_cos.xyz;
            attenuation = 1.0
                / max(factors.x + factors.y * light_distance + factors.z * light_distance * light_distance, 1.0);
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
    return shade_color(draw.color, normal, world_position, side);
}

// `shade` with the straight colour of the layer at the point: the colour of the draw times the
// sample of its brush (`lighting::shade_color`).
fn shade_color(layer_color: vec4<f32>, normal: vec3<f32>, world_position: vec3<f32>, side: f32) -> vec4<f32> {
    let color = layer_color.xyz;
    let alpha = clamp(layer_color.w, 0.0, 1.0);
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

// The brushes of step D6 (`brush.rs`).

// The point of the brush's space at the texture coordinate `uv` (`brush::brush_point`).
fn brush_point(uv: vec2<f32>) -> vec2<f32> {
    let x = brush.to_brush_x;
    let y = brush.to_brush_y;
    return vec2<f32>(uv.x * x.x + uv.y * x.y + x.z, uv.x * y.x + uv.y * y.y + y.z);
}

// `t` after the spread method of its value: 0 `Pad`, 1 `Reflect`, 2 `Repeat`.
fn spread(t: f32, method: f32) -> f32 {
    if (method < 0.5) {
        return clamp(t, 0.0, 1.0);
    }
    if (method < 1.5) {
        let m = t - 2.0 * floor(t * 0.5);
        if (m > 1.0) {
            return 2.0 - m;
        }
        return m;
    }
    return t - floor(t);
}

// The parameter of a gradient at the point `p` of the brush's space (`brush::gradient_t`).
fn gradient_t(p: vec2<f32>) -> f32 {
    let g = brush.geometry;
    var t = 0.0;
    if (brush.metadata.x < 1.5) {
        // Linear: the projection onto the line from the start to the end point.
        let d = vec2<f32>(g.z - g.x, g.w - g.y);
        let length_squared = d.x * d.x + d.y * d.y;
        if (length_squared > 0.0) {
            t = ((p.x - g.x) * d.x + (p.y - g.y) * d.y) / length_squared;
        }
    } else {
        // Radial: in the unit circle of the ellipse, from the focus to the circle.
        let q = vec2<f32>((p.x - g.x) * g.z, (p.y - g.y) * g.w);
        let f = brush.geometry2.xy;
        let d = vec2<f32>(q.x - f.x, q.y - f.y);
        let a = d.x * d.x + d.y * d.y;
        if (a > 1e-12) {
            let b = f.x * d.x + f.y * d.y;
            let c = f.x * f.x + f.y * f.y - 1.0;
            let s = (-b + sqrt(max(b * b - a * c, 0.0))) / a;
            if (s > 0.0) {
                t = 1.0 / s;
            }
        }
    }
    return spread(t, brush.metadata.y);
}

fn stop_offset(index: u32) -> f32 {
    return brush.stop_offsets[index / 4u][index % 4u];
}

// The colour of the inline stops at `t`, straight (`brush::inline_gradient`).
fn inline_gradient(t: f32) -> vec4<f32> {
    let count = min(u32(brush.metadata.z + 0.5), MAX_STOPS);
    if (count == 0u) {
        return vec4<f32>(0.0);
    }
    if (t <= stop_offset(0u)) {
        return brush.stop_colors[0];
    }
    for (var index = 1u; index < MAX_STOPS; index = index + 1u) {
        if (index >= count) {
            break;
        }
        let end = stop_offset(index);
        if (t <= end) {
            let start = stop_offset(index - 1u);
            let span = end - start;
            var f = 1.0;
            if (span > 0.0) {
                f = (t - start) / span;
            }
            let a = brush.stop_colors[index - 1u];
            let b = brush.stop_colors[index];
            return a + (b - a) * f;
        }
    }
    return brush.stop_colors[count - 1u];
}

// The horizontal coordinate of `t` in a gradient's lookup texture: the centre of texel
// `t * (width - 1)` (`brush::gradient_texture_coordinate`).
fn gradient_texture_coordinate(t: f32) -> f32 {
    return (t * (GRADIENT_TEXTURE_WIDTH - 1.0) + 0.5) / GRADIENT_TEXTURE_WIDTH;
}

// The texture coordinate of the bitmap of an image brush at the point `p` of the brush's space
// in `xy`, and in `z` 1 where the brush paints, 0 where it is transparent
// (`brush::image_coordinate`).
fn image_coordinate(p: vec2<f32>) -> vec3<f32> {
    if (brush.metadata.w > 0.5) {
        return vec3<f32>(0.0);
    }
    let g = brush.geometry;
    var local = vec2<f32>(p.x - g.x, p.y - g.y);
    let mode = u32(brush.metadata.y + 0.5);
    if (mode == 0u) {
        let outside = local.x < -EDGE_EPSILON || local.y < -EDGE_EPSILON
            || local.x > g.z + EDGE_EPSILON || local.y > g.w + EDGE_EPSILON;
        if (outside) {
            return vec3<f32>(0.0);
        }
    } else {
        let size = vec2<f32>(g.z, g.w);
        let tile = floor(local / size);
        var inside = local - tile * size;
        // FlipX (1) and FlipXY (3) mirror the odd columns, FlipY (2) and FlipXY the odd rows.
        if ((mode == 1u || mode == 3u) && (tile.x - 2.0 * floor(tile.x * 0.5)) > 0.5) {
            inside.x = size.x - inside.x;
        }
        if ((mode == 2u || mode == 3u) && (tile.y - 2.0 * floor(tile.y * 0.5)) > 0.5) {
            inside.y = size.y - inside.y;
        }
        local = inside;
    }
    let g2 = brush.geometry2;
    let uv = vec2<f32>(local.x * g2.x + g2.z, local.y * g2.y + g2.w);
    let clip = brush.clip;
    let outside = uv.x < clip.x - EDGE_EPSILON || uv.y < clip.y - EDGE_EPSILON
        || uv.x > clip.z + EDGE_EPSILON || uv.y > clip.w + EDGE_EPSILON;
    if (outside) {
        return vec3<f32>(0.0);
    }
    return vec3<f32>(uv, 1.0);
}

// The straight colour of a premultiplied sample.
fn unpremultiply(sample: vec4<f32>) -> vec4<f32> {
    if (sample.w > 0.0) {
        return vec4<f32>(sample.xyz * (1.0 / sample.w), sample.w);
    }
    return vec4<f32>(0.0);
}

// A gradient with its stops inline (`BrushShader::Gradient`).
@fragment
fn fs_gradient(input: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let side = select(-1.0, 1.0, front_facing);
    var sample = vec4<f32>(0.0);
    if (brush.metadata.w < 0.5) {
        sample = inline_gradient(gradient_t(brush_point(input.texture_coordinate)));
    }
    return shade_color(draw.color * sample, input.world_normal, input.world_position, side);
}

// A gradient read from its lookup texture (`BrushShader::GradientTexture`): straight texels.
@fragment
fn fs_gradient_texture(input: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let side = select(-1.0, 1.0, front_facing);
    let t = gradient_t(brush_point(input.texture_coordinate));
    let sample = textureSampleLevel(brush_texture, brush_sampler, vec2<f32>(gradient_texture_coordinate(t), 0.5), 0.0);
    return shade_color(draw.color * sample, input.world_normal, input.world_position, side);
}

// The frame of a video stored in full-range Rec. 709, on the curve of the canvas: the BT.1886
// reference EOTF, then the sRGB curve (what the 2D path of Skia applies to such a frame).
fn frame_transfer(c: vec3<f32>) -> vec3<f32> {
    let v = pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.4));
    let low = v * 12.92;
    let high = 1.055 * pow(v, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, low, v < vec3<f32>(0.0031308));
}

// An image brush (`BrushShader::Image`): premultiplied texels, sampled bilinearly with the edges
// clamped by the sampler, divided by their alpha.
@fragment
fn fs_image(input: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let side = select(-1.0, 1.0, front_facing);
    let coordinate = image_coordinate(brush_point(input.texture_coordinate));
    var sample = vec4<f32>(0.0);
    if (coordinate.z > 0.5) {
        sample = unpremultiply(textureSampleLevel(brush_texture, brush_sampler, coordinate.xy, 0.0));
        // A frame sampled as it is stored (step D7).
        if (brush.metadata.z > 0.5) {
            sample = vec4<f32>(frame_transfer(sample.xyz), sample.w);
        }
    }
    return shade_color(draw.color * sample, input.world_normal, input.world_position, side);
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
