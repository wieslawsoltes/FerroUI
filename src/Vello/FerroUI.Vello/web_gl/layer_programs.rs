//! The programs of the layers with an effect on the WebGL2 context of a canvas: the programs of
//! `gpu/effect_passes.rs` in GLSL ES 3.0 (step 3 of the GPU route, docs/porting/gpu-rendering.md,
//! #623): the colour matrix (also the luminance mask), the multiply layer of lit visuals, the
//! pixel operations of two sources and the matrix convolution.
//!
//! Each program is compiled once per context, when a layer first needs it, and takes its values
//! as uniforms for each pass: `u_params`, the uniform block of its WGSL program as an array of
//! `vec4` of the same layout (`effect_uniforms.rs` writes both), and `u_area`, the size of the
//! area and how each input lies. An input is a texture of the pool, either as the copy from the
//! drawing buffer left it (its first row the bottom one of the area) or as a pass wrote it
//! (upright, its first row the top one); a pass writes its result upright, the way the renderer
//! samples a texture it is given. Every pixel of the target is written (the pixels outside the
//! area are transparent), so nothing is cleared first.
//!
//! The renderer of `vello_gpu` sets what it draws with itself; a pass sets what it needs, and
//! leaves the state it changes as it found it where the renderer may not set it again (the
//! tests of the framebuffer, the viewport), and unbound otherwise.

use super::frame_textures::VelloWebGlTexture;
use super::shadow_programs as shadows;
use crate::effect_uniforms as blocks;
use std::rc::Rc;
use web_sys::{WebGl2RenderingContext as Gl, WebGlProgram, WebGlShader, WebGlUniformLocation, WebGlVertexArrayObject};

/// The vertex stage of the passes: a triangle over the target, without attributes.
const VERTEX: &str = r#"#version 300 es
void main() {
    vec2 uv = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
}
"#;

/// What every fragment stage starts with: the inputs, the area, and the reading of an input in
/// the pixels of the area (its first row the top one).
const FRAGMENT_PRELUDE: &str = r#"#version 300 es
precision highp float;
precision highp int;

uniform highp sampler2D u_content;
uniform highp sampler2D u_next;
// x: 1 when the first row of `u_content` is the bottom one of the area, y: the same for `u_next`;
// z, w: the size of the area in pixels.
uniform vec4 u_area;

out vec4 o_color;

ivec2 texel_of(ivec2 pixel, float bottom_up) {
    if (bottom_up > 0.5) {
        return ivec2(pixel.x, int(u_area.w) - 1 - pixel.y);
    }
    return pixel;
}

vec4 load_content(ivec2 pixel) {
    return texelFetch(u_content, texel_of(pixel, u_area.x), 0);
}

vec4 load_next(ivec2 pixel) {
    return texelFetch(u_next, texel_of(pixel, u_area.y), 0);
}

// Whether the pixel of the fragment is outside the area (it is then transparent).
bool outside(ivec2 pixel) {
    return float(pixel.x) >= u_area.z || float(pixel.y) >= u_area.w;
}
"#;

/// A colour matrix (`COLOR_MATRIX_WGSL`): the matrix applied to the colour un-premultiplied and
/// the result premultiplied again, or to the premultiplied colour as it is with the colour
/// clamped to the alpha.
const COLOR_MATRIX: &str = r#"
// 0 to 4: what red, green, blue and alpha add to each output channel, then the offsets (the rows
// of `ColorMatrix`); 5: x 1 to apply the matrix to the premultiplied colour as it is, 0 to
// un-premultiply it first.
uniform vec4 u_params[6];

vec4 apply_matrix(vec4 c) {
    vec4 m = u_params[4] + c.r * u_params[0] + c.g * u_params[1] + c.b * u_params[2] + c.a * u_params[3];
    return clamp(m, vec4(0.0), vec4(1.0));
}

void main() {
    ivec2 pixel = ivec2(floor(gl_FragCoord.xy));
    if (outside(pixel)) {
        o_color = vec4(0.0);
        return;
    }
    vec4 c = load_content(pixel);
    if (u_params[5].x > 0.5) {
        vec4 result = apply_matrix(c);
        o_color = vec4(min(result.rgb, vec3(result.a)), result.a);
        return;
    }
    vec3 straight = vec3(0.0);
    if (c.a > 0.0) {
        straight = min(c.rgb / c.a, vec3(1.0));
    }
    vec4 result = apply_matrix(vec4(straight, c.a));
    o_color = vec4(result.rgb * result.a, result.a);
}
"#;

/// The lights at a pixel (`LIGHT_COMMON_WGSL`), which the two lighting programs start with.
macro_rules! light_common {
    () => {
        r#"
// 0: the constant light; 1: the ambient, diffuse, specular amounts and the shine; 2: x 1 for
// factors, y the scale of the factors, z the number of lights; 3: x, y the pixel of the top left
// of the target; 4, 5: the rows of the transform from the pixels of the area to the drawing; then
// 8 lights of 10 vec4 each: the kind (x 1 point, 2 spot; y, z the cosines of the inner and outer
// cones), the colour (the inner colour of a spot light), the outer colour, the position, the
// direction, the attenuation (constant, linear, quadratic, minimum cutoff), the cutoff (x the
// maximum cutoff, y 1 when there is one), the rows of the transform into the space of the light.
uniform vec4 u_params[86];

float attenuation_factor(vec4 a, vec4 c, float d) {
    if (c.y > 0.5 && d > c.x) {
        return 0.0;
    }
    float dd = max(max(d, a.w), 0.0);
    float denominator = a.x + a.y * dd + a.z * dd * dd;
    if (denominator <= 0.0) {
        return 1.0;
    }
    return 1.0 / denominator;
}

float specular_factor(vec3 to_light, float shine) {
    float len = length(to_light);
    if (len <= 0.0 || to_light.z <= 0.0) {
        return 0.0;
    }
    vec3 h = to_light / len + vec3(0.0, 0.0, 1.0);
    float half_length = length(h);
    if (half_length <= 0.0) {
        return 0.0;
    }
    float x = max(h.z / half_length, 0.0);
    if (x <= 0.0) {
        return 0.0;
    }
    return pow(x, max(shine, 1.0));
}

float spot_cone(vec3 pos, vec3 dir, float cos_inner, float cos_outer, vec2 p) {
    vec3 to_point = vec3(p - pos.xy, -pos.z);
    float to_point_length = length(to_point);
    float direction_length = length(dir);
    if (to_point_length <= 0.0 || direction_length <= 0.0) {
        return 0.0;
    }
    float c = dot(to_point, dir) / (to_point_length * direction_length);
    if (c >= cos_inner) {
        return 1.0;
    }
    if (c <= cos_outer || cos_inner <= cos_outer) {
        return 0.0;
    }
    float t = clamp((c - cos_outer) / (cos_inner - cos_outer), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

// The light reflected at the pixel of the fragment, not clamped.
vec3 light_at(vec2 fragment) {
    vec3 device = vec3(u_params[3].xy + fragment, 1.0);
    vec2 coord = vec2(dot(u_params[4].xyz, device), dot(u_params[5].xyz, device));
    vec3 light = u_params[0].rgb;
    int count = int(u_params[2].z);
    vec4 material = u_params[1];
    for (int i = 0; i < 8; i++) {
        if (i >= count) {
            break;
        }
        int at = 6 + i * 10;
        vec4 kind = u_params[at];
        vec3 point = vec3(coord, 1.0);
        vec2 p = vec2(dot(u_params[at + 7].xyz, point), dot(u_params[at + 8].xyz, point))
            / dot(u_params[at + 9].xyz, point);
        vec3 pos = u_params[at + 3].xyz;
        vec3 to_light = vec3(pos.xy - p, pos.z);
        float d = length(to_light);
        float incidence = 0.0;
        if (d > 0.0 && pos.z > 0.0) {
            incidence = pos.z / d;
        }
        float att = attenuation_factor(u_params[at + 5], u_params[at + 6], d);
        vec3 c = u_params[at + 1].rgb;
        float cone = 1.0;
        if (kind.x > 1.5) {
            cone = spot_cone(pos, u_params[at + 4].xyz, kind.y, kind.z, p);
            if (cone <= 0.0) {
                continue;
            }
            c = mix(u_params[at + 2].rgb, u_params[at + 1].rgb, cone);
        }
        vec3 lit = vec3(0.0);
        if (material.y != 0.0) {
            lit = c * (cone * incidence * att) * material.y;
        }
        if (material.z != 0.0) {
            lit = lit + c * (cone * specular_factor(to_light, material.w) * att) * material.z;
        }
        light = light + lit;
    }
    return light;
}
"#
    };
}

/// The multiply layer of lit visuals (`LIGHT_MULTIPLY_FRAGMENT_WGSL`): each colour channel of the
/// content times the light at its pixel, clamped to 0..`scale`, the result clamped to the alpha,
/// the alpha kept.
const LIGHT_MULTIPLY: &str = concat!(
    light_common!(),
    r#"
void main() {
    ivec2 pixel = ivec2(floor(gl_FragCoord.xy));
    if (outside(pixel)) {
        o_color = vec4(0.0);
        return;
    }
    vec4 c = load_content(pixel);
    vec3 factors = clamp(light_at(gl_FragCoord.xy), vec3(0.0), vec3(u_params[2].y));
    o_color = vec4(min(c.rgb * factors, vec3(c.a)), c.a);
}
"#
);

/// The lighting fill of a scene lighting effect (`LIGHTING_FRAGMENT_WGSL`): the light at each
/// pixel, opaque. It reads no texture.
const LIGHT_FILL: &str = concat!(
    light_common!(),
    r#"
void main() {
    ivec2 pixel = ivec2(floor(gl_FragCoord.xy));
    if (outside(pixel)) {
        o_color = vec4(0.0);
        return;
    }
    vec3 light = light_at(gl_FragCoord.xy);
    if (u_params[2].x > 0.5) {
        float scale = u_params[2].y;
        o_color = vec4(clamp(light, vec3(0.0), vec3(scale)) / scale, 1.0);
        return;
    }
    o_color = vec4(clamp(light, vec3(0.0), vec3(1.0)), 1.0);
}
"#
);

/// The pixel operations of two sources (`PIXEL_OPERATION_WGSL`): the next source (`u_next`)
/// combined into what the sources before it made (`u_content`).
const PIXEL_OPERATION: &str = r#"
// 0: x 0 a blend, 1 the arithmetic composite, 2 MaskInvert, y the blend mode
// (`BlendEffectMode`); 1: the multiply, destination, source and offset amounts; 2: the grid of
// Dissolve in the coordinates of the drawing (x, y, width, height); 3: its cells along each side
// (x, y); 4, 5: the rows of the transform from the pixels of the area to the drawing.
uniform vec4 u_params[6];

vec3 straight(vec4 c) {
    if (c.a > 0.0) {
        return min(c.rgb / c.a, vec3(1.0));
    }
    return vec3(0.0);
}

float luminosity(vec3 c) {
    return 0.3 * c.r + 0.59 * c.g + 0.11 * c.b;
}

float color_dodge(float b, float s) {
    if (b <= 0.0) {
        return 0.0;
    }
    if (s >= 1.0) {
        return 1.0;
    }
    return min(b / (1.0 - s), 1.0);
}

float color_burn(float b, float s) {
    if (b >= 1.0) {
        return 1.0;
    }
    if (s <= 0.0) {
        return 0.0;
    }
    return 1.0 - min((1.0 - b) / s, 1.0);
}

float blend_channel(int mode, float b, float s) {
    if (mode == 6) {
        return max(b + s - 1.0, 0.0);
    }
    if (mode == 14) {
        if (s <= 0.5) {
            return color_burn(b, 2.0 * s);
        }
        return color_dodge(b, 2.0 * (s - 0.5));
    }
    if (mode == 15) {
        return clamp(b + 2.0 * s - 1.0, 0.0, 1.0);
    }
    if (mode == 16) {
        if (s <= 0.5) {
            return min(b, 2.0 * s);
        }
        return max(b, 2.0 * s - 1.0);
    }
    if (mode == 17) {
        if (b + s >= 1.0) {
            return 1.0;
        }
        return 0.0;
    }
    if (mode == 24) {
        return max(b - s, 0.0);
    }
    if (mode == 25) {
        if (s <= 0.0) {
            if (b <= 0.0) {
                return 0.0;
            }
            return 1.0;
        }
        return min(b / s, 1.0);
    }
    return s;
}

// `dissolve_noise` of the compositor: a hash of the number of the cell.
float dissolve_noise(uint index) {
    uint x = index ^ 0x9E3779B9u;
    x = x * 0x85EBCA6Bu;
    x = x ^ (x >> 13u);
    x = x * 0xC2B2AE35u;
    x = x ^ (x >> 16u);
    return float(x & 0xFFFFu) / 65535.0;
}

vec4 blend(vec4 d, vec4 s, vec2 fragment) {
    int mode = int(u_params[0].y + 0.5);
    vec3 cb = straight(d);
    vec3 cs = straight(s);
    if (mode == 4) {
        // Dissolve: the foreground, opaque, where the noise of its cell is under its alpha; the
        // background elsewhere.
        vec3 point = vec3(fragment, 1.0);
        vec2 at = vec2(dot(u_params[4].xyz, point), dot(u_params[5].xyz, point));
        vec4 grid = u_params[2];
        vec4 cells = u_params[3];
        float column = clamp(floor((at.x - grid.x) * cells.x / grid.z), 0.0, cells.x - 1.0);
        float row = clamp(floor((at.y - grid.y) * cells.y / grid.w), 0.0, cells.y - 1.0);
        uint index = uint(row) * uint(cells.x) + uint(column);
        if (s.a > 0.0 && dissolve_noise(index) < s.a) {
            return vec4(cs, 1.0);
        }
        return d;
    }
    vec3 blended = vec3(0.0);
    if (mode == 7) {
        blended = cb;
        if (luminosity(cs) < luminosity(cb)) {
            blended = cs;
        }
    } else if (mode == 8) {
        blended = cb;
        if (luminosity(cs) > luminosity(cb)) {
            blended = cs;
        }
    } else {
        blended = vec3(
            blend_channel(mode, cb.r, cs.r),
            blend_channel(mode, cb.g, cs.g),
            blend_channel(mode, cb.b, cs.b));
    }
    float alpha = s.a + d.a - s.a * d.a;
    if (alpha <= 0.0) {
        return vec4(0.0);
    }
    vec3 premultiplied = (1.0 - s.a) * d.a * cb + (1.0 - d.a) * s.a * cs + s.a * d.a * blended;
    float a = clamp(alpha, 0.0, 1.0);
    return vec4(clamp(premultiplied / alpha, vec3(0.0), vec3(1.0)) * a, a);
}

void main() {
    ivec2 pixel = ivec2(floor(gl_FragCoord.xy));
    if (outside(pixel)) {
        o_color = vec4(0.0);
        return;
    }
    vec4 d = load_content(pixel);
    vec4 s = load_next(pixel);
    if (u_params[0].x < 0.5) {
        o_color = blend(d, s, gl_FragCoord.xy);
        return;
    }
    if (u_params[0].x < 1.5) {
        vec4 k = u_params[1];
        vec4 result = clamp(k.x * d * s + k.y * d + k.z * s + vec4(k.w), vec4(0.0), vec4(1.0));
        o_color = vec4(min(result.rgb, vec3(result.a)), result.a);
        return;
    }
    vec3 c = s.a * (vec3(d.a) - d.rgb) + (1.0 - s.a) * d.rgb;
    o_color = vec4(clamp(c, vec3(0.0), vec3(d.a)), d.a);
}
"#;

/// The matrix convolution (`CONVOLUTION_WGSL`): the kernel as a correlation (the weight of row
/// `i`, column `j` at `i * width + j`), the sum times the gain plus the bias, the colours
/// un-premultiplied and the alpha kept, or the four channels convolved.
const CONVOLUTION: &str = r#"
// 0 to 6: the weights; 7: x, y the columns and rows of the kernel, z, w its target element; 8: x
// the gain, y the bias, z 1 to keep the alpha, w 1 to repeat the edge pixels; 9: x, y the size of
// the area in pixels.
uniform vec4 u_params[10];

vec4 fetch(ivec2 p) {
    ivec2 last = ivec2(int(u_params[9].x) - 1, int(u_params[9].y) - 1);
    if (u_params[8].w > 0.5) {
        return load_content(clamp(p, ivec2(0), last));
    }
    if (any(lessThan(p, ivec2(0))) || any(greaterThan(p, last))) {
        return vec4(0.0);
    }
    return load_content(p);
}

float weight(int index) {
    vec4 quad = u_params[index / 4];
    int lane = index - (index / 4) * 4;
    if (lane == 0) {
        return quad.x;
    }
    if (lane == 1) {
        return quad.y;
    }
    if (lane == 2) {
        return quad.z;
    }
    return quad.w;
}

void main() {
    ivec2 pixel = ivec2(floor(gl_FragCoord.xy));
    if (outside(pixel)) {
        o_color = vec4(0.0);
        return;
    }
    int columns = int(u_params[7].x);
    int rows = int(u_params[7].y);
    ivec2 anchor = ivec2(int(u_params[7].z), int(u_params[7].w));
    bool keep_alpha = u_params[8].z > 0.5;
    vec4 sum = vec4(0.0);
    for (int row = 0; row < 5; row++) {
        if (row >= rows) {
            break;
        }
        for (int column = 0; column < 5; column++) {
            if (column >= columns) {
                break;
            }
            vec4 c = fetch(pixel + ivec2(column, row) - anchor);
            if (keep_alpha) {
                if (c.a > 0.0) {
                    c = vec4(c.rgb / c.a, c.a);
                } else {
                    c = vec4(0.0);
                }
            }
            sum = sum + c * weight(row * columns + column);
        }
    }
    vec4 color = sum * u_params[8].x + vec4(u_params[8].y);
    if (keep_alpha) {
        float a = load_content(pixel).a;
        color = vec4(color.rgb * a, a);
    } else {
        color.a = clamp(color.a, 0.0, 1.0);
    }
    o_color = vec4(clamp(color.rgb, vec3(0.0), vec3(color.a)), color.a);
}
"#;

/// A program of the layers with an effect on the WebGL2 context.
/// A multiply layer of factors (`MULTIPLY_FACTORS_WGSL`): each colour channel of the content times
/// the factor of its point, sampled bilinearly between the centres of the samples of `u_next`
/// (upright, of its own size), clamped to the alpha; a pixel outside the factors is cleared.
const MULTIPLY_FACTORS: &str = r#"
// 0, 1: the rows of the transform from the pixels of the area to the unit square of the factors;
// 2: x, y the size of the samples, z what a sample of 1 stands for; 3: x, y the size of the area.
uniform vec4 u_params[4];

vec3 factor_sample(ivec2 p) {
    return texelFetch(u_next, p, 0).rgb;
}

void main() {
    ivec2 pixel = ivec2(floor(gl_FragCoord.xy));
    if (outside(pixel)) {
        o_color = vec4(0.0);
        return;
    }
    vec4 c = load_content(pixel);
    if (c.a <= 0.0) {
        o_color = vec4(0.0);
        return;
    }
    vec3 at = vec3(vec2(pixel) + 0.5, 1.0);
    vec2 unit = vec2(dot(u_params[0].xyz, at), dot(u_params[1].xyz, at));
    if (unit.x < 0.0 || unit.x > 1.0 || unit.y < 0.0 || unit.y > 1.0) {
        o_color = vec4(0.0);
        return;
    }
    vec2 size = u_params[2].xy;
    vec2 p = clamp(unit * size - 0.5, vec2(0.0), size - 1.0);
    ivec2 a = ivec2(floor(p));
    ivec2 b = min(a + ivec2(1), ivec2(size) - ivec2(1));
    vec2 f = p - vec2(a);
    vec3 top = mix(factor_sample(a), factor_sample(ivec2(b.x, a.y)), f.x);
    vec3 bottom = mix(factor_sample(ivec2(a.x, b.y)), factor_sample(b), f.x);
    vec3 factor = mix(top, bottom, f.y) * u_params[2].z;
    o_color = vec4(min(c.rgb * factor, vec3(c.a)), c.a);
}
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayerProgram {
    /// The colour matrix layers and the luminance mask.
    ColorMatrix,
    /// The multiply layers of lit visuals.
    LightMultiply,
    /// The pixel operations of two sources.
    PixelOperation,
    /// The matrix convolution layers.
    Convolution,
    /// The coverage of the box of a shadow (`shadow_programs.rs`).
    ShadowCoverage,
    /// One direction of the Gaussian of a shadow.
    ShadowGaussian,
    /// A shadow blurred at a reduced scale, scaled back.
    ShadowScaleUp,
    /// A multiply layer of factors: the content and the image of its factors.
    MultiplyFactors,
    /// The lighting fill of a scene lighting effect.
    LightFill,
}

impl LayerProgram {
    const ALL: [LayerProgram; 9] = [
        LayerProgram::ColorMatrix,
        LayerProgram::LightMultiply,
        LayerProgram::PixelOperation,
        LayerProgram::Convolution,
        LayerProgram::ShadowCoverage,
        LayerProgram::ShadowGaussian,
        LayerProgram::ShadowScaleUp,
        LayerProgram::MultiplyFactors,
        LayerProgram::LightFill,
    ];

    fn index(self) -> usize {
        self as usize
    }

    /// The name the log gives the program.
    fn label(self) -> &'static str {
        match self {
            LayerProgram::ColorMatrix => "colour matrix",
            LayerProgram::LightMultiply => "light multiply",
            LayerProgram::PixelOperation => "pixel operation",
            LayerProgram::Convolution => "convolution",
            LayerProgram::ShadowCoverage => "shadow coverage",
            LayerProgram::ShadowGaussian => "shadow Gaussian",
            LayerProgram::ShadowScaleUp => "shadow scale up",
            LayerProgram::MultiplyFactors => "multiply factors",
            LayerProgram::LightFill => "lighting fill",
        }
    }

    /// The fragment stage after the prelude.
    fn fragment(self) -> &'static str {
        match self {
            LayerProgram::ColorMatrix => COLOR_MATRIX,
            LayerProgram::LightMultiply => LIGHT_MULTIPLY,
            LayerProgram::PixelOperation => PIXEL_OPERATION,
            LayerProgram::Convolution => CONVOLUTION,
            LayerProgram::ShadowCoverage => shadows::COVERAGE,
            LayerProgram::ShadowGaussian => shadows::GAUSSIAN,
            LayerProgram::ShadowScaleUp => shadows::SCALE_UP,
            LayerProgram::MultiplyFactors => MULTIPLY_FACTORS,
            LayerProgram::LightFill => LIGHT_FILL,
        }
    }

    /// The textures the program reads.
    fn inputs(self) -> usize {
        match self {
            LayerProgram::PixelOperation | LayerProgram::MultiplyFactors => 2,
            LayerProgram::ShadowCoverage | LayerProgram::LightFill => 0,
            _ => 1,
        }
    }

    /// The floats of `u_params` (the uniform block of the WGSL program).
    fn uniform_floats(self) -> usize {
        match self {
            LayerProgram::ColorMatrix => blocks::COLOR_MATRIX_FLOATS,
            LayerProgram::LightMultiply | LayerProgram::LightFill => blocks::LIGHTING_FLOATS,
            LayerProgram::PixelOperation => blocks::PIXEL_OPERATION_FLOATS,
            LayerProgram::Convolution => blocks::CONVOLUTION_FLOATS,
            LayerProgram::ShadowCoverage => shadows::COVERAGE_FLOATS,
            LayerProgram::ShadowGaussian => shadows::GAUSSIAN_FLOATS,
            LayerProgram::ShadowScaleUp => shadows::SCALE_UP_FLOATS,
            LayerProgram::MultiplyFactors => blocks::MULTIPLY_FACTORS_FLOATS,
        }
    }
}

/// A program and the places of its uniforms.
struct Program {
    program: WebGlProgram,
    content: Option<WebGlUniformLocation>,
    next: Option<WebGlUniformLocation>,
    area: Option<WebGlUniformLocation>,
    params: Option<WebGlUniformLocation>,
}

/// An input of a pass: a texture of the pool, and whether its first row is the bottom one of the
/// area (the copy from the drawing buffer) rather than the top one (a pass's result).
pub(crate) type PassInput<'a> = (&'a Rc<VelloWebGlTexture>, bool);

/// The programs of a context, compiled when first needed. A program that does not compile is
/// not compiled again: its layers are drawn on the processor, and the failure is logged once.
#[derive(Default)]
pub(crate) struct LayerPrograms {
    vertex_array: Option<WebGlVertexArrayObject>,
    programs: [Option<Result<Program, ()>>; LayerProgram::ALL.len()],
}

pub(super) fn compile(gl: &Gl, kind: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = gl.create_shader(kind).ok_or("the context did not create a shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if gl.get_shader_parameter(&shader, Gl::COMPILE_STATUS).as_bool().unwrap_or(false) {
        return Ok(shader);
    }
    let log = gl.get_shader_info_log(&shader).unwrap_or_default();
    gl.delete_shader(Some(&shader));
    Err(log)
}

fn link(gl: &Gl, fragment: &str) -> Result<WebGlProgram, String> {
    let vertex = compile(gl, Gl::VERTEX_SHADER, VERTEX)?;
    let fragment = match compile(gl, Gl::FRAGMENT_SHADER, fragment) {
        Ok(fragment) => fragment,
        Err(error) => {
            gl.delete_shader(Some(&vertex));
            return Err(error);
        }
    };
    let program = gl.create_program().ok_or("the context did not create a program")?;
    gl.attach_shader(&program, &vertex);
    gl.attach_shader(&program, &fragment);
    gl.link_program(&program);
    // The program keeps what it was linked from.
    gl.delete_shader(Some(&vertex));
    gl.delete_shader(Some(&fragment));
    if gl.get_program_parameter(&program, Gl::LINK_STATUS).as_bool().unwrap_or(false) {
        return Ok(program);
    }
    let log = gl.get_program_info_log(&program).unwrap_or_default();
    gl.delete_program(Some(&program));
    Err(log)
}

impl LayerPrograms {
    /// Whether `program` is there, compiled now if it was not.
    pub(crate) fn prepare(&mut self, gl: &Gl, program: LayerProgram) -> bool {
        self.program(gl, program).is_some()
    }

    fn program(&mut self, gl: &Gl, program: LayerProgram) -> Option<&Program> {
        let slot = &mut self.programs[program.index()];
        let compiled = slot.get_or_insert_with(|| {
            let source = format!("{FRAGMENT_PRELUDE}{}", program.fragment());
            match link(gl, &source) {
                Ok(linked) => Ok(Program {
                    content: gl.get_uniform_location(&linked, "u_content"),
                    next: gl.get_uniform_location(&linked, "u_next"),
                    area: gl.get_uniform_location(&linked, "u_area"),
                    params: gl.get_uniform_location(&linked, "u_params"),
                    program: linked,
                }),
                Err(error) => {
                    super::log_render_failure(&format!("the {} program did not compile: {error}", program.label()));
                    Err(())
                }
            }
        });
        compiled.as_ref().ok()
    }

    /// The pass of `program`: its `uniforms` (the floats of `u_params`) over the `width` by
    /// `height` pixels of `inputs` (as many as the program reads, textures of the pool), written
    /// into `target`. `false`, drawing nothing, when the program is not there or the inputs or
    /// uniforms do not fit it.
    pub(crate) fn pass(
        &mut self,
        gl: &Gl,
        program: LayerProgram,
        inputs: &[PassInput<'_>],
        target: &Rc<VelloWebGlTexture>,
        (width, height): (u32, u32),
        uniforms: &[f32],
    ) -> bool {
        if inputs.len() != program.inputs() || uniforms.len() != program.uniform_floats() {
            return false;
        }
        if self.vertex_array.is_none() {
            self.vertex_array = gl.create_vertex_array();
        }
        let vertex_array = self.vertex_array.clone();
        let Some(compiled) = self.program(gl, program) else {
            return false;
        };
        let bottom_up = |index: usize| if inputs.get(index).is_some_and(|input| input.1) { 1.0 } else { 0.0 };

        gl.use_program(Some(&compiled.program));
        gl.uniform1i(compiled.content.as_ref(), 0);
        gl.uniform1i(compiled.next.as_ref(), 1);
        gl.uniform4f(compiled.area.as_ref(), bottom_up(0), bottom_up(1), width as f32, height as f32);
        gl.uniform4fv_with_f32_array(compiled.params.as_ref(), uniforms);

        let tests = [Gl::BLEND, Gl::SCISSOR_TEST, Gl::DEPTH_TEST, Gl::STENCIL_TEST];
        let enabled: Vec<bool> = tests.iter().map(|test| gl.is_enabled(*test)).collect();
        for test in tests {
            gl.disable(test);
        }

        gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&target.framebuffer));
        gl.viewport(0, 0, target.width as i32, target.height as i32);
        // A program that reads one input samples it twice: both units hold it.
        for unit in 0..2 {
            let input = inputs.get(unit).or(inputs.first());
            gl.active_texture(Gl::TEXTURE0 + unit as u32);
            gl.bind_texture(Gl::TEXTURE_2D, input.map(|input| &input.0.texture));
        }
        gl.bind_vertex_array(vertex_array.as_ref());
        gl.draw_arrays(Gl::TRIANGLES, 0, 3);

        gl.bind_vertex_array(None);
        for unit in (0..2).rev() {
            gl.active_texture(Gl::TEXTURE0 + unit);
            gl.bind_texture(Gl::TEXTURE_2D, None);
        }
        gl.use_program(None);
        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
        gl.viewport(0, 0, gl.drawing_buffer_width(), gl.drawing_buffer_height());
        for (test, enabled) in tests.iter().zip(enabled) {
            if enabled {
                gl.enable(*test);
            }
        }
        true
    }

    /// Deletes the programs: the context is released.
    pub(crate) fn dispose(&mut self, gl: &Gl) {
        for slot in &mut self.programs {
            if let Some(Ok(program)) = slot.take() {
                gl.delete_program(Some(&program.program));
            }
        }
        if let Some(vertex_array) = self.vertex_array.take() {
            gl.delete_vertex_array(Some(&vertex_array));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uniform_arrays_of_the_programs_hold_their_blocks() {
        // `u_params[N]` holds the block of the program, four floats to a `vec4`.
        for program in LayerProgram::ALL {
            let declaration = format!("uniform vec4 u_params[{}];", program.uniform_floats() / 4);
            assert!(program.fragment().contains(&declaration), "{program:?}: {declaration}");
            assert_eq!(program.uniform_floats() % 4, 0);
        }
        let indices: Vec<usize> = LayerProgram::ALL.iter().map(|program| program.index()).collect();
        assert_eq!(indices, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
