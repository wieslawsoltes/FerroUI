//! The GPU forms of the lights and the pixel operations of WinUI's
//! composition: SkSL programs ([`crate::runtime_effects`]) compiled once,
//! given the lights and the parameters of an operation as uniforms on each
//! draw (docs/porting/winui-composition.md, "How the effects are drawn").
//!
//! - The lighting shader ports the light math of the compositor
//!   (`server_composition_light.rs`) line by line: the attenuation with its
//!   cutoffs, the incidence of point lights, the cones of spot lights with
//!   their smooth step and the lerp of their colours, the Blinn-Phong
//!   highlight and the amounts of the material. It evaluates the point and
//!   spot lights at the point of the lit visual each pixel shows (Skia hands
//!   a shader the local coordinates of the pixel: the inverse of the
//!   transform of the canvas is applied for it), mapped into the space of
//!   each light; the ambient and distant lights, the same everywhere, come
//!   summed as one constant light.
//! - The multiply blender multiplies what a layer holds by the factors the
//!   lighting shader gives (the light over [`MULTIPLY_SCALE`], a power of
//!   two, so the division is exact), clamped to the alpha.
//! - The blend blender has the modes of Direct2D's blend effect the blend
//!   layers lack, with the formulas of the CPU path; `MaskInvert` is a
//!   blender too, the arithmetic composite is Skia's arithmetic blender.
//! - `Dissolve` needs the position of a pixel, which a blender is not
//!   given: a shader computes the hash of the CPU path (in floats, as
//!   runtime effects have no integer bit operations), and a blender keeps
//!   the foreground where it falls under its alpha.

use crate::runtime_effects::{make_blender, make_shader, ProgramKind, RuntimeProgram, Uniforms};
use ferroui_base::platform::{LightingParameters, PixelOperation};
use ferroui_base::rendering::composition::effects::BlendEffectMode;
use ferroui_base::rendering::composition::server::{LightSource, LIGHT_MAP_MAX_SCALE};
use skia_safe as sk;

/// The most point and spot lights the lighting shader evaluates.
const MAX_LIGHTS: usize = LightingParameters::MAX_LIGHTS;

/// What the factors of the multiply layer are divided by in the shader and
/// multiplied by in the blender: the largest factor, a power of two.
pub(crate) const MULTIPLY_SCALE: f32 = LIGHT_MAP_MAX_SCALE as f32;

const _: () = assert!(MAX_LIGHTS == 8, "the arrays of the lighting shader have 8 elements");
const _: () = assert!(LIGHT_MAP_MAX_SCALE == 16.0, "MULTIPLY_SCALE must be a power of two");

/// The light reflected at each point: the light math of
/// `server_composition_light.rs`.
static LIGHTING: RuntimeProgram = RuntimeProgram {
    name: "lighting",
    kind: ProgramKind::Shader,
    source: r#"
// The light of the ambient and distant lights, the same everywhere.
uniform float4 constantLight;
// The amounts of the material: ambient, diffuse, specular, and the shine.
uniform float4 material;
// x: 1 for the factors of a multiply layer (the light over y), 0 for the
// light clamped to 0..1.
uniform float4 outputMode;
uniform int lightCount;
// x: 1 point, 2 spot; y, z: the cosines of the inner and outer cones.
uniform float4 kind[8];
// The colour of a point light, the inner colour of a spot light.
uniform float4 color[8];
uniform float4 outerColor[8];
uniform float4 position[8];
uniform float4 direction[8];
// constant, linear, quadratic, minimum cutoff
uniform float4 attenuation[8];
// x: the maximum cutoff, y: 1 when there is one
uniform float4 cutoff[8];
// The rows of the transform into the space of the light, applied to
// (x, y, 1): x, y and the perspective w.
uniform float3 toLightX[8];
uniform float3 toLightY[8];
uniform float3 toLightW[8];

// LightAttenuation::factor
float attenuationFactor(float4 a, float4 c, float d) {
    if (c.y > 0.5 && d > c.x) {
        return 0.0;
    }
    float dd = max(max(d, a.w), 0.0);
    float denominator = a.x + a.y * dd + a.z * dd * dd;
    return denominator <= 0.0 ? 1.0 : 1.0 / denominator;
}

// specular_factor: (N.H)^shine of a surface facing +Z seen from +Z.
float specularFactor(float3 toLight, float shine) {
    float len = length(toLight);
    if (len <= 0.0 || toLight.z <= 0.0) {
        return 0.0;
    }
    float3 h = toLight / len + float3(0.0, 0.0, 1.0);
    float halfLength = length(h);
    if (halfLength <= 0.0) {
        return 0.0;
    }
    float x = max(h.z / halfLength, 0.0);
    if (x <= 0.0) {
        return 0.0;
    }
    return pow(x, max(shine, 1.0));
}

// spot_light_cone, with the cosines of the clamped angles.
float spotCone(float3 pos, float3 dir, float cosInner, float cosOuter, float2 p) {
    float3 toPoint = float3(p - pos.xy, -pos.z);
    float toPointLength = length(toPoint);
    float directionLength = length(dir);
    if (toPointLength <= 0.0 || directionLength <= 0.0) {
        return 0.0;
    }
    float c = dot(toPoint, dir) / (toPointLength * directionLength);
    if (c >= cosInner) {
        return 1.0;
    }
    if (c <= cosOuter || cosInner <= cosOuter) {
        return 0.0;
    }
    float t = clamp((c - cosOuter) / (cosInner - cosOuter), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

half4 main(float2 coord) {
    float3 light = constantLight.rgb;
    for (int i = 0; i < 8; i++) {
        if (i >= lightCount) {
            break;
        }
        float3 point = float3(coord, 1.0);
        float2 p = float2(dot(toLightX[i], point), dot(toLightY[i], point)) / dot(toLightW[i], point);
        float3 pos = position[i].xyz;
        // point_light_incidence
        float3 toLight = float3(pos.xy - p, pos.z);
        float d = length(toLight);
        float incidence = (d > 0.0 && pos.z > 0.0) ? pos.z / d : 0.0;
        float att = attenuationFactor(attenuation[i], cutoff[i], d);
        float3 c = color[i].rgb;
        float cone = 1.0;
        if (kind[i].x > 1.5) {
            cone = spotCone(pos, direction[i].xyz, kind[i].y, kind[i].z, p);
            if (cone <= 0.0) {
                continue;
            }
            // From the outer colour (at the edge of the outer cone) to the
            // inner one.
            c = mix(outerColor[i].rgb, color[i].rgb, cone);
        }
        // LightSource::lit_at
        float3 lit = float3(0.0);
        if (material.y != 0.0) {
            lit = c * (cone * incidence * att) * material.y;
        }
        if (material.z != 0.0) {
            lit += c * (cone * specularFactor(toLight, material.w) * att) * material.z;
        }
        light += lit;
    }
    if (outputMode.x > 0.5) {
        return half4(half3(clamp(light, 0.0, outputMode.y) / outputMode.y), 1.0);
    }
    return half4(half3(clamp(light, 0.0, 1.0)), 1.0);
}
"#,
};

/// What a multiply layer holds times the factors of the lighting shader
/// (`src`, over `scale`), clamped to the alpha; the alpha is kept.
static MULTIPLY: RuntimeProgram = RuntimeProgram {
    name: "light multiply",
    kind: ProgramKind::Blender,
    source: r#"
uniform float scale;
half4 main(half4 src, half4 dst) {
    float3 f = float3(src.rgb) * scale;
    float a = float(dst.a);
    return half4(half3(min(float3(dst.rgb) * f, float3(a))), dst.a);
}
"#,
};

/// The modes of Direct2D's blend effect the blend layers lack, with the
/// formulas of `blend_pixels` of the compositor: the foreground (`src`)
/// over the background (`dst`), composed as a separable blend is.
static BLEND: RuntimeProgram = RuntimeProgram {
    name: "blend",
    kind: ProgramKind::Blender,
    source: r#"
uniform float mode;

float3 straight(float4 c) {
    return c.a > 0.0 ? min(c.rgb / c.a, float3(1.0)) : float3(0.0);
}

float luminosity(float3 c) {
    return 0.3 * c.r + 0.59 * c.g + 0.11 * c.b;
}

float colorDodge(float b, float s) {
    if (b <= 0.0) { return 0.0; }
    if (s >= 1.0) { return 1.0; }
    return min(b / (1.0 - s), 1.0);
}

float colorBurn(float b, float s) {
    if (b >= 1.0) { return 1.0; }
    if (s <= 0.0) { return 0.0; }
    return 1.0 - min((1.0 - b) / s, 1.0);
}

float channel(float b, float s) {
    if (mode == 6.0) { return max(b + s - 1.0, 0.0); }
    if (mode == 14.0) { return s <= 0.5 ? colorBurn(b, 2.0 * s) : colorDodge(b, 2.0 * (s - 0.5)); }
    if (mode == 15.0) { return clamp(b + 2.0 * s - 1.0, 0.0, 1.0); }
    if (mode == 16.0) { return s <= 0.5 ? min(b, 2.0 * s) : max(b, 2.0 * s - 1.0); }
    if (mode == 17.0) { return b + s >= 1.0 ? 1.0 : 0.0; }
    if (mode == 24.0) { return max(b - s, 0.0); }
    if (mode == 25.0) {
        if (s <= 0.0) { return b <= 0.0 ? 0.0 : 1.0; }
        return min(b / s, 1.0);
    }
    return s;
}

half4 main(half4 srcColor, half4 dstColor) {
    float4 src = float4(srcColor);
    float4 dst = float4(dstColor);
    float as = src.a;
    float ab = dst.a;
    float3 cs = straight(src);
    float3 cb = straight(dst);
    float3 blended;
    if (mode == 7.0) {
        blended = luminosity(cs) < luminosity(cb) ? cs : cb;
    } else if (mode == 8.0) {
        blended = luminosity(cs) > luminosity(cb) ? cs : cb;
    } else {
        blended = float3(channel(cb.r, cs.r), channel(cb.g, cs.g), channel(cb.b, cs.b));
    }
    float alpha = as + ab - as * ab;
    if (alpha <= 0.0) {
        return half4(0.0);
    }
    float3 premultiplied = (1.0 - as) * ab * cb + (1.0 - ab) * as * cs + as * ab * blended;
    alpha = clamp(alpha, 0.0, 1.0);
    return half4(half3(clamp(premultiplied / alpha, 0.0, 1.0) * alpha), alpha);
}
"#,
};

/// `MaskInvert`: the colour of the destination inverted where the source
/// covers it, the alpha of the destination kept.
static MASK_INVERT: RuntimeProgram = RuntimeProgram {
    name: "mask invert",
    kind: ProgramKind::Blender,
    source: r#"
half4 main(half4 srcColor, half4 dstColor) {
    float4 s = float4(srcColor);
    float4 d = float4(dstColor);
    float3 c = s.a * (d.a - d.rgb) + (1.0 - s.a) * d.rgb;
    return half4(half3(clamp(c, 0.0, d.a)), dstColor.a);
}
"#,
};

/// The random choice of `Dissolve` at each pixel: `dissolve_noise` of the
/// compositor of the number of the cell of the grid the pixel is in, as
/// `floor(noise / 257)` over 255 in red (the foreground is kept where that
/// is under its alpha in bytes, which is exactly `noise < alpha`).
///
/// The 32-bit integer operations of the hash are done on four bytes held in
/// floats: every intermediate value is an integer below 2^24, exact in a
/// float.
static DISSOLVE_NOISE: RuntimeProgram = RuntimeProgram {
    name: "dissolve noise",
    kind: ProgramKind::Shader,
    source: r#"
// The area of the grid (x, y, width, height) and its cells along each side.
uniform float4 grid;
uniform float2 cells;

float xor8(float a, float b) {
    float result = 0.0;
    float bit = 1.0;
    for (int i = 0; i < 8; i++) {
        float x = mod(a, 2.0);
        float y = mod(b, 2.0);
        result += bit * abs(x - y);
        a = floor(a * 0.5);
        b = floor(b * 0.5);
        bit *= 2.0;
    }
    return result;
}

// Little-endian bytes: x the lowest.
float4 xor32(float4 a, float4 b) {
    return float4(xor8(a.x, b.x), xor8(a.y, b.y), xor8(a.z, b.z), xor8(a.w, b.w));
}

// The product modulo 2^32.
float4 mul32(float4 a, float4 b) {
    float c0 = a.x * b.x;
    float c1 = a.x * b.y + a.y * b.x + floor(c0 / 256.0);
    float c2 = a.x * b.z + a.y * b.y + a.z * b.x + floor(c1 / 256.0);
    float c3 = a.x * b.w + a.y * b.z + a.z * b.y + a.w * b.x + floor(c2 / 256.0);
    return float4(mod(c0, 256.0), mod(c1, 256.0), mod(c2, 256.0), mod(c3, 256.0));
}

float4 shr13(float4 x) {
    return float4(floor(x.y / 32.0) + mod(x.z, 32.0) * 8.0, floor(x.z / 32.0) + mod(x.w, 32.0) * 8.0, floor(x.w / 32.0), 0.0);
}

float4 shr16(float4 x) {
    return float4(x.z, x.w, 0.0, 0.0);
}

half4 main(float2 p) {
    float column = clamp(floor((p.x - grid.x) * cells.x / grid.z), 0.0, cells.x - 1.0);
    float row = clamp(floor((p.y - grid.y) * cells.y / grid.w), 0.0, cells.y - 1.0);
    float index = row * cells.x + column;
    float4 x = float4(mod(index, 256.0), mod(floor(index / 256.0), 256.0), floor(index / 65536.0), 0.0);
    // index ^ 0x9E3779B9
    x = xor32(x, float4(185.0, 121.0, 55.0, 158.0));
    // * 0x85EBCA6B
    x = mul32(x, float4(107.0, 202.0, 235.0, 133.0));
    x = xor32(x, shr13(x));
    // * 0xC2B2AE35
    x = mul32(x, float4(53.0, 174.0, 178.0, 194.0));
    x = xor32(x, shr16(x));
    float noise = x.x + 256.0 * x.y;
    float level = floor((noise + 0.5) / 257.0);
    return half4(half(level / 255.0), 0.0, 0.0, 1.0);
}
"#,
};

/// Keeps the foreground (`dst`, the layer) opaque where the level of the
/// noise (`src`) is under its alpha, and nothing elsewhere.
static DISSOLVE_SELECT: RuntimeProgram = RuntimeProgram {
    name: "dissolve select",
    kind: ProgramKind::Blender,
    source: r#"
half4 main(half4 srcColor, half4 dstColor) {
    float4 d = float4(dstColor);
    float level = floor(float(srcColor.r) * 255.0 + 0.5);
    float alpha = floor(d.a * 255.0 + 0.5);
    if (level < alpha) {
        return half4(half3(min(d.rgb / d.a, float3(1.0))), 1.0);
    }
    return half4(0.0);
}
"#,
};

ferroui_base::ferro_thread_local! {
    static PIXEL_IMAGES: std::cell::Cell<u64> = std::cell::Cell::new(0);
}

/// Counts an image made of pixels in memory for drawing (a bitmap loaded
/// from pixels, the factors of a CPU multiply layer): an upload to the GPU
/// when it is drawn on a GPU surface.
pub(crate) fn note_pixel_image() {
    PIXEL_IMAGES.with(|count| count.set(count.get() + 1));
}

/// How many images were made of pixels on this thread
/// ([`note_pixel_image`]).
#[allow(dead_code)]
pub(crate) fn pixel_image_count() -> u64 {
    PIXEL_IMAGES.with(std::cell::Cell::get)
}

/// What the lighting shader gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LightingOutput {
    /// The light clamped to 0..1, opaque.
    Light,
    /// The factors of a multiply layer: the light clamped to
    /// 0..[`MULTIPLY_SCALE`], over it.
    Factors,
}

/// The values of a cutoff: none beyond what a float holds.
fn max_cutoff(value: f64) -> [f32; 2] {
    if value.is_nan() || value >= f64::from(f32::MAX) {
        [0.0, 0.0]
    } else if value <= -f64::from(f32::MAX) {
        [-f32::MAX, 1.0]
    } else {
        [value as f32, 1.0]
    }
}

/// The lighting shader of `lighting`; `None` with more than
/// [`MAX_LIGHTS`] point and spot lights or without runtime effects.
pub(crate) fn lighting_shader(lighting: &LightingParameters, output: LightingOutput) -> Option<sk::Shader> {
    if lighting.lights.len() > MAX_LIGHTS {
        return None;
    }
    let effect = LIGHTING.effect()?;
    let mut kind = [0f32; 4 * MAX_LIGHTS];
    let mut color = [0f32; 4 * MAX_LIGHTS];
    let mut outer_color = [0f32; 4 * MAX_LIGHTS];
    let mut position = [0f32; 4 * MAX_LIGHTS];
    let mut direction = [0f32; 4 * MAX_LIGHTS];
    let mut attenuation = [0f32; 4 * MAX_LIGHTS];
    let mut cutoff = [0f32; 4 * MAX_LIGHTS];
    let mut to_x = [0f32; 3 * MAX_LIGHTS];
    let mut to_y = [0f32; 3 * MAX_LIGHTS];
    let mut to_w = [0f32; 3 * MAX_LIGHTS];
    let mut count = 0;
    for (index, light) in lighting.lights.iter().enumerate() {
        let (four, three) = (index * 4, index * 3);
        let m = light.to_light_space;
        to_x[three..three + 3].copy_from_slice(&[m.m11 as f32, m.m21 as f32, m.m31 as f32]);
        to_y[three..three + 3].copy_from_slice(&[m.m12 as f32, m.m22 as f32, m.m32 as f32]);
        to_w[three..three + 3].copy_from_slice(&[m.m13 as f32, m.m23 as f32, m.m33 as f32]);
        let (light_attenuation, light_position) = match light.source {
            LightSource::Point { color: c, position: p, attenuation: a } => {
                kind[four] = 1.0;
                color[four..four + 3].copy_from_slice(&[c.r as f32, c.g as f32, c.b as f32]);
                (a, p)
            }
            LightSource::Spot { inner_color, outer_color: o, position: p, direction: d, inner_cone_angle, outer_cone_angle, attenuation: a } => {
                // spot_light_cone: the angles clamped, an inner angle wider
                // than the outer one is the outer one.
                let outer = outer_cone_angle.clamp(0.0, std::f64::consts::PI);
                let inner = inner_cone_angle.clamp(0.0, outer);
                kind[four..four + 3].copy_from_slice(&[2.0, inner.cos() as f32, outer.cos() as f32]);
                color[four..four + 3].copy_from_slice(&[inner_color.r as f32, inner_color.g as f32, inner_color.b as f32]);
                outer_color[four..four + 3].copy_from_slice(&[o.r as f32, o.g as f32, o.b as f32]);
                direction[four..four + 3].copy_from_slice(&[d.x as f32, d.y as f32, d.z as f32]);
                (a, p)
            }
            // The parameters list only point and spot lights.
            LightSource::Ambient { .. } | LightSource::Distant { .. } => continue,
        };
        position[four..four + 3].copy_from_slice(&[light_position.x as f32, light_position.y as f32, light_position.z as f32]);
        let min_cutoff = light_attenuation.min_cutoff.clamp(-f64::from(f32::MAX), f64::from(f32::MAX));
        attenuation[four..four + 4].copy_from_slice(&[
            light_attenuation.constant as f32,
            light_attenuation.linear as f32,
            light_attenuation.quadratic as f32,
            min_cutoff as f32,
        ]);
        cutoff[four..four + 2].copy_from_slice(&max_cutoff(light_attenuation.max_cutoff));
        count = index + 1;
    }
    let (mode, scale) = match output {
        LightingOutput::Light => (0.0, 1.0),
        LightingOutput::Factors => (1.0, MULTIPLY_SCALE),
    };
    let constant = lighting.constant;
    let material = lighting.material;
    let mut uniforms = Uniforms::new(&effect);
    uniforms
        .float4("constantLight", [constant.r as f32, constant.g as f32, constant.b as f32, 0.0])
        .float4("material", [material.ambient as f32, material.diffuse as f32, material.specular as f32, material.shine as f32])
        .float4("outputMode", [mode, scale, 0.0, 0.0])
        .int("lightCount", count as i32)
        .floats("kind", &kind)
        .floats("color", &color)
        .floats("outerColor", &outer_color)
        .floats("position", &position)
        .floats("direction", &direction)
        .floats("attenuation", &attenuation)
        .floats("cutoff", &cutoff)
        .floats("toLightX", &to_x)
        .floats("toLightY", &to_y)
        .floats("toLightW", &to_w);
    make_shader(&effect, &uniforms, &[], None)
}

/// The blender of a multiply layer of the lighting shader's factors.
pub(crate) fn light_multiply_blender() -> Option<sk::Blender> {
    let effect = MULTIPLY.effect()?;
    let mut uniforms = Uniforms::new(&effect);
    uniforms.float("scale", MULTIPLY_SCALE);
    make_blender(&effect, &uniforms, &[])
}

/// How a pixel operation composes each next source onto what the sources
/// before it made.
#[derive(Clone)]
pub(crate) enum PixelOperationPrograms {
    /// The layer of the source is composed with the blender.
    Blender(sk::Blender),
    /// `Dissolve`: the layer of the foreground is reduced to what the noise
    /// keeps (a rectangle of the noise shader drawn with the select
    /// blender), then composed source-over.
    Dissolve { noise: sk::Shader, select: sk::Blender },
}

/// The programs of `operation`; `None` without runtime effects (or for an
/// arithmetic composite Skia does not take).
pub(crate) fn pixel_operation_programs(operation: &PixelOperation) -> Option<PixelOperationPrograms> {
    match *operation {
        PixelOperation::Blend { mode: BlendEffectMode::Dissolve, noise_grid, noise_size } => {
            if noise_grid.width <= 0.0 || noise_grid.height <= 0.0 {
                return None;
            }
            let effect = DISSOLVE_NOISE.effect()?;
            let mut uniforms = Uniforms::new(&effect);
            uniforms
                .float4("grid", [noise_grid.x as f32, noise_grid.y as f32, noise_grid.width as f32, noise_grid.height as f32])
                .floats("cells", &[noise_size.width.max(1) as f32, noise_size.height.max(1) as f32]);
            let noise = make_shader(&effect, &uniforms, &[], None)?;
            let effect = DISSOLVE_SELECT.effect()?;
            let select = make_blender(&effect, &Uniforms::new(&effect), &[])?;
            Some(PixelOperationPrograms::Dissolve { noise, select })
        }
        PixelOperation::Blend { mode, .. } => {
            let effect = BLEND.effect()?;
            let mut uniforms = Uniforms::new(&effect);
            uniforms.float("mode", mode as i32 as f32);
            make_blender(&effect, &uniforms, &[]).map(PixelOperationPrograms::Blender)
        }
        PixelOperation::Arithmetic { k } => {
            if !crate::runtime_effects::is_enabled() {
                return None;
            }
            // Skia's `k1 src dst + k2 src + k3 dst + k4`: the layer is the
            // second source, what is under it the first.
            sk::Blender::arithmetic(k[0], k[2], k[1], k[3], true).map(PixelOperationPrograms::Blender)
        }
        PixelOperation::MaskInvert => {
            let effect = MASK_INVERT.effect()?;
            make_blender(&effect, &Uniforms::new(&effect), &[]).map(PixelOperationPrograms::Blender)
        }
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_program_compiles() {
        for program in [&LIGHTING, &MULTIPLY, &BLEND, &MASK_INVERT, &DISSOLVE_NOISE, &DISSOLVE_SELECT] {
            assert!(program.effect().is_some(), "{} compiles", program.name);
        }
    }
}
