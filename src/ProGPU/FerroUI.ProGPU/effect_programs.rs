//! The WGSL programs of the layers with an effect (`effect_passes.rs`). They are the programs of
//! the Vello backend, so that both GPU backends draw an effect with the same math; moving them to
//! one shared place is a follow-up. The multiply layer of factors is this backend's own: the
//! Vello backend applies those factors on the processor.

/// The light math of the lighting passes: the uniforms, the vertex stage
/// and `light_at`, followed by the fragment stage of a pass.
pub(crate) const LIGHT_COMMON_WGSL: &str = r#"
struct Light {
    // x: 1 point, 2 spot; y, z: the cosines of the inner and outer cones.
    kind: vec4<f32>,
    // The colour of a point light, the inner colour of a spot light.
    color: vec4<f32>,
    outer_color: vec4<f32>,
    position: vec4<f32>,
    direction: vec4<f32>,
    // constant, linear, quadratic, minimum cutoff
    attenuation: vec4<f32>,
    // x: the maximum cutoff, y: 1 when there is one
    cutoff: vec4<f32>,
    // The rows of the transform into the space of the light.
    to_x: vec4<f32>,
    to_y: vec4<f32>,
    to_w: vec4<f32>,
}

struct Params {
    constant_light: vec4<f32>,
    // ambient, diffuse, specular amounts and the shine
    material: vec4<f32>,
    // x: 1 for factors (the light over y), 0 for the light clamped to 0..1;
    // z: the number of lights
    output_mode: vec4<f32>,
    // x, y: the device pixel of the top left of the texture; z, w: the size
    // of the area of the content (the multiply pass)
    origin: vec4<f32>,
    // The rows of the transform from device pixels to the drawing.
    to_local_x: vec4<f32>,
    to_local_y: vec4<f32>,
    lights: array<Light, 8>,
}

@group(0) @binding(0) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn attenuation_factor(a: vec4<f32>, c: vec4<f32>, d: f32) -> f32 {
    if (c.y > 0.5 && d > c.x) {
        return 0.0;
    }
    let dd = max(max(d, a.w), 0.0);
    let denominator = a.x + a.y * dd + a.z * dd * dd;
    if (denominator <= 0.0) {
        return 1.0;
    }
    return 1.0 / denominator;
}

fn specular_factor(to_light: vec3<f32>, shine: f32) -> f32 {
    let len = length(to_light);
    if (len <= 0.0 || to_light.z <= 0.0) {
        return 0.0;
    }
    let h = to_light / len + vec3<f32>(0.0, 0.0, 1.0);
    let half_length = length(h);
    if (half_length <= 0.0) {
        return 0.0;
    }
    let x = max(h.z / half_length, 0.0);
    if (x <= 0.0) {
        return 0.0;
    }
    return pow(x, max(shine, 1.0));
}

fn spot_cone(pos: vec3<f32>, dir: vec3<f32>, cos_inner: f32, cos_outer: f32, p: vec2<f32>) -> f32 {
    let to_point = vec3<f32>(p - pos.xy, -pos.z);
    let to_point_length = length(to_point);
    let direction_length = length(dir);
    if (to_point_length <= 0.0 || direction_length <= 0.0) {
        return 0.0;
    }
    let c = dot(to_point, dir) / (to_point_length * direction_length);
    if (c >= cos_inner) {
        return 1.0;
    }
    if (c <= cos_outer || cos_inner <= cos_outer) {
        return 0.0;
    }
    let t = clamp((c - cos_outer) / (cos_inner - cos_outer), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

// The light reflected at the device pixel of the fragment, not clamped.
fn light_at(fragment: vec4<f32>) -> vec3<f32> {
    let device = vec3<f32>(params.origin.xy + fragment.xy, 1.0);
    let coord = vec2<f32>(dot(params.to_local_x.xyz, device), dot(params.to_local_y.xyz, device));
    var light = params.constant_light.rgb;
    let count = i32(params.output_mode.z);
    let material = params.material;
    for (var i = 0; i < 8; i = i + 1) {
        if (i >= count) {
            break;
        }
        let l = params.lights[i];
        let point = vec3<f32>(coord, 1.0);
        let p = vec2<f32>(dot(l.to_x.xyz, point), dot(l.to_y.xyz, point)) / dot(l.to_w.xyz, point);
        let pos = l.position.xyz;
        let to_light = vec3<f32>(pos.xy - p, pos.z);
        let d = length(to_light);
        var incidence = 0.0;
        if (d > 0.0 && pos.z > 0.0) {
            incidence = pos.z / d;
        }
        let att = attenuation_factor(l.attenuation, l.cutoff, d);
        var c = l.color.rgb;
        var cone = 1.0;
        if (l.kind.x > 1.5) {
            cone = spot_cone(pos, l.direction.xyz, l.kind.y, l.kind.z, p);
            if (cone <= 0.0) {
                continue;
            }
            c = mix(l.outer_color.rgb, l.color.rgb, cone);
        }
        var lit = vec3<f32>(0.0);
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
"#;

/// The fragment stage of the lighting pass: the light, opaque.
pub(crate) const LIGHTING_FRAGMENT_WGSL: &str = r#"
@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let light = light_at(fragment);
    if (params.output_mode.x > 0.5) {
        let scale = params.output_mode.y;
        return vec4<f32>(clamp(light, vec3<f32>(0.0), vec3<f32>(scale)) / scale, 1.0);
    }
    return vec4<f32>(clamp(light, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
"#;

/// The fragment stage of the multiply layer of lit visuals: each colour
/// channel of the content times the light at its pixel, clamped to
/// 0..`scale` (`output_mode.y`), the result clamped to the alpha, the alpha
/// kept (`MultiplyLayerFactors::apply_to_premultiplied_rgba8` with the
/// light evaluated at each pixel).
pub(crate) const LIGHT_MULTIPLY_FRAGMENT_WGSL: &str = r#"
@group(0) @binding(1) var content: texture_2d<f32>;

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.origin.z || f32(pixel.y) >= params.origin.w) {
        return vec4<f32>(0.0);
    }
    let c = textureLoad(content, pixel, 0);
    let factors = clamp(light_at(fragment), vec3<f32>(0.0), vec3<f32>(params.output_mode.y));
    return vec4<f32>(min(c.rgb * factors, vec3<f32>(c.a)), c.a);
}
"#;

/// The vertex stage of the layer passes: a triangle over the target.
pub(crate) const LAYER_VERTEX_WGSL: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}
"#;

/// A colour matrix (`ColorMatrix::apply_premultiplied`): the matrix applied
/// to the colour un-premultiplied and the result premultiplied again, or to
/// the premultiplied colour as it is with the colour clamped to the alpha.
pub(crate) const COLOR_MATRIX_WGSL: &str = r#"
struct Params {
    // What red, green, blue and alpha add to each output channel, then the
    // offsets (the rows of `ColorMatrix`).
    rows: array<vec4<f32>, 5>,
    // x: 1 to apply the matrix to the premultiplied colour as it is, 0 to
    // un-premultiply it first; y, z: the size of the area in pixels.
    mode: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var content: texture_2d<f32>;

fn apply_matrix(c: vec4<f32>) -> vec4<f32> {
    let m = params.rows[4] + c.r * params.rows[0] + c.g * params.rows[1] + c.b * params.rows[2] + c.a * params.rows[3];
    return clamp(m, vec4<f32>(0.0), vec4<f32>(1.0));
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.mode.y || f32(pixel.y) >= params.mode.z) {
        return vec4<f32>(0.0);
    }
    let c = textureLoad(content, pixel, 0);
    if (params.mode.x > 0.5) {
        let result = apply_matrix(c);
        return vec4<f32>(min(result.rgb, vec3<f32>(result.a)), result.a);
    }
    var straight = vec3<f32>(0.0);
    if (c.a > 0.0) {
        straight = min(c.rgb / c.a, vec3<f32>(1.0));
    }
    let result = apply_matrix(vec4<f32>(straight, c.a));
    return vec4<f32>(result.rgb * result.a, result.a);
}
"#;

/// The pixel operations of two sources (`PixelOperation`): the formulas of
/// `blend_pixels`, `arithmetic_pixels` and `mask_invert_pixels` of the
/// compositor, the next source (`source`) combined into what the sources
/// before it made (`destination`).
pub(crate) const PIXEL_OPERATION_WGSL: &str = r#"
struct Params {
    // x: 0 a blend, 1 the arithmetic composite, 2 MaskInvert; y: the blend
    // mode (`BlendEffectMode`); z, w: the size of the area in pixels.
    kind: vec4<f32>,
    // The multiply, destination, source and offset amounts.
    k: vec4<f32>,
    // The grid of Dissolve in the coordinates of the drawing (x, y, width,
    // height), and its cells along each side (x, y).
    grid: vec4<f32>,
    cells: vec4<f32>,
    // The rows of the transform from the pixels of the area to the drawing.
    to_local_x: vec4<f32>,
    to_local_y: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
// What the sources before made, and the next source.
@group(0) @binding(1) var first_texture: texture_2d<f32>;
@group(0) @binding(2) var next_texture: texture_2d<f32>;

fn straight(c: vec4<f32>) -> vec3<f32> {
    if (c.a > 0.0) {
        return min(c.rgb / c.a, vec3<f32>(1.0));
    }
    return vec3<f32>(0.0);
}

fn luminosity(c: vec3<f32>) -> f32 {
    return 0.3 * c.r + 0.59 * c.g + 0.11 * c.b;
}

fn color_dodge(b: f32, s: f32) -> f32 {
    if (b <= 0.0) {
        return 0.0;
    }
    if (s >= 1.0) {
        return 1.0;
    }
    return min(b / (1.0 - s), 1.0);
}

fn color_burn(b: f32, s: f32) -> f32 {
    if (b >= 1.0) {
        return 1.0;
    }
    if (s <= 0.0) {
        return 0.0;
    }
    return 1.0 - min((1.0 - b) / s, 1.0);
}

fn blend_channel(mode: i32, b: f32, s: f32) -> f32 {
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
fn dissolve_noise(index: u32) -> f32 {
    var x = index ^ 0x9E3779B9u;
    x = x * 0x85EBCA6Bu;
    x = x ^ (x >> 13u);
    x = x * 0xC2B2AE35u;
    x = x ^ (x >> 16u);
    return f32(x & 0xFFFFu) / 65535.0;
}

fn blend(d: vec4<f32>, s: vec4<f32>, fragment: vec2<f32>) -> vec4<f32> {
    let mode = i32(params.kind.y + 0.5);
    let cb = straight(d);
    let cs = straight(s);
    if (mode == 4) {
        // Dissolve: the foreground, opaque, where the noise of its cell is
        // under its alpha; the background elsewhere.
        let point = vec3<f32>(fragment, 1.0);
        let at = vec2<f32>(dot(params.to_local_x.xyz, point), dot(params.to_local_y.xyz, point));
        let column = clamp(floor((at.x - params.grid.x) * params.cells.x / params.grid.z), 0.0, params.cells.x - 1.0);
        let row = clamp(floor((at.y - params.grid.y) * params.cells.y / params.grid.w), 0.0, params.cells.y - 1.0);
        let index = u32(row) * u32(params.cells.x) + u32(column);
        if (s.a > 0.0 && dissolve_noise(index) < s.a) {
            return vec4<f32>(cs, 1.0);
        }
        return d;
    }
    var blended = vec3<f32>(0.0);
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
        blended = vec3<f32>(blend_channel(mode, cb.r, cs.r), blend_channel(mode, cb.g, cs.g), blend_channel(mode, cb.b, cs.b));
    }
    let alpha = s.a + d.a - s.a * d.a;
    if (alpha <= 0.0) {
        return vec4<f32>(0.0);
    }
    let premultiplied = (1.0 - s.a) * d.a * cb + (1.0 - d.a) * s.a * cs + s.a * d.a * blended;
    let a = clamp(alpha, 0.0, 1.0);
    return vec4<f32>(clamp(premultiplied / alpha, vec3<f32>(0.0), vec3<f32>(1.0)) * a, a);
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.kind.z || f32(pixel.y) >= params.kind.w) {
        return vec4<f32>(0.0);
    }
    let d = textureLoad(first_texture, pixel, 0);
    let s = textureLoad(next_texture, pixel, 0);
    if (params.kind.x < 0.5) {
        return blend(d, s, fragment.xy);
    }
    if (params.kind.x < 1.5) {
        let k = params.k;
        let result = clamp(k.x * d * s + k.y * d + k.z * s + vec4<f32>(k.w), vec4<f32>(0.0), vec4<f32>(1.0));
        return vec4<f32>(min(result.rgb, vec3<f32>(result.a)), result.a);
    }
    let c = s.a * (vec3<f32>(d.a) - d.rgb) + (1.0 - s.a) * d.rgb;
    return vec4<f32>(clamp(c, vec3<f32>(0.0), vec3<f32>(d.a)), d.a);
}
"#;

/// The matrix convolution (`ConvolveMatrix::apply_to_premultiplied_rgba8`):
/// the kernel as a correlation (the weight of row `i`, column `j` at
/// `i * width + j`), the sum times the gain plus the bias, the colours
/// un-premultiplied and the alpha kept, or the four channels convolved.
pub(crate) const CONVOLUTION_WGSL: &str = r#"
struct Params {
    weights: array<vec4<f32>, 7>,
    // x, y: the columns and rows of the kernel; z, w: its target element.
    size: vec4<f32>,
    // x: the gain, y: the bias, z: 1 to keep the alpha, w: 1 to repeat the
    // edge pixels.
    factors: vec4<f32>,
    // x, y: the size of the area in pixels.
    extent: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var content: texture_2d<f32>;

fn fetch(p: vec2<i32>) -> vec4<f32> {
    let last = vec2<i32>(i32(params.extent.x) - 1, i32(params.extent.y) - 1);
    if (params.factors.w > 0.5) {
        return textureLoad(content, clamp(p, vec2<i32>(0), last), 0);
    }
    if (any(p < vec2<i32>(0)) || any(p > last)) {
        return vec4<f32>(0.0);
    }
    return textureLoad(content, p, 0);
}

fn weight(index: i32) -> f32 {
    let quad = params.weights[index / 4];
    return quad[index % 4];
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.extent.x || f32(pixel.y) >= params.extent.y) {
        return vec4<f32>(0.0);
    }
    let columns = i32(params.size.x);
    let rows = i32(params.size.y);
    let anchor = vec2<i32>(i32(params.size.z), i32(params.size.w));
    let keep_alpha = params.factors.z > 0.5;
    var sum = vec4<f32>(0.0);
    for (var row = 0; row < 5; row = row + 1) {
        if (row >= rows) {
            break;
        }
        for (var column = 0; column < 5; column = column + 1) {
            if (column >= columns) {
                break;
            }
            var c = fetch(pixel + vec2<i32>(column, row) - anchor);
            if (keep_alpha) {
                if (c.a > 0.0) {
                    c = vec4<f32>(c.rgb / c.a, c.a);
                } else {
                    c = vec4<f32>(0.0);
                }
            }
            sum = sum + c * weight(row * columns + column);
        }
    }
    var color = sum * params.factors.x + vec4<f32>(params.factors.y);
    if (keep_alpha) {
        let a = textureLoad(content, pixel, 0).a;
        color = vec4<f32>(color.rgb * a, a);
    } else {
        color.a = clamp(color.a, 0.0, 1.0);
    }
    return vec4<f32>(clamp(color.rgb, vec3<f32>(0.0), vec3<f32>(color.a)), color.a);
}
"#;

/// A multiply layer of factors (`MultiplyLayerFactors::apply_to_premultiplied_rgba8`): each colour
/// channel of the content times the factors at its pixel, sampled between the centres of the
/// samples and clamped to the alpha; the alpha is kept and nothing outside the factors stays.
pub(crate) const FACTOR_MULTIPLY_WGSL: &str = r#"
struct Params {
    // The rows of the transform from the pixels of the area to the unit square of the factors.
    to_unit_x: vec4<f32>,
    to_unit_y: vec4<f32>,
    // x: what a sample of 1 stands for; y, z: the size of the area in pixels.
    mode: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var content: texture_2d<f32>;
@group(0) @binding(2) var factors: texture_2d<f32>;

fn factor_at(unit: vec2<f32>) -> vec3<f32> {
    let size = vec2<i32>(textureDimensions(factors));
    let last = vec2<f32>(size - vec2<i32>(1));
    let at = clamp(unit * vec2<f32>(size) - vec2<f32>(0.5), vec2<f32>(0.0), last);
    let first = vec2<i32>(floor(at));
    let next = min(first + vec2<i32>(1), size - vec2<i32>(1));
    let part = at - floor(at);
    let top_right = textureLoad(factors, vec2<i32>(next.x, first.y), 0).rgb;
    let bottom_left = textureLoad(factors, vec2<i32>(first.x, next.y), 0).rgb;
    let top = mix(textureLoad(factors, first, 0).rgb, top_right, part.x);
    let bottom = mix(bottom_left, textureLoad(factors, next, 0).rgb, part.x);
    return mix(top, bottom, part.y) * params.mode.x;
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(fragment.xy));
    if (f32(pixel.x) >= params.mode.y || f32(pixel.y) >= params.mode.z) {
        return vec4<f32>(0.0);
    }
    let device = vec3<f32>(fragment.xy, 1.0);
    let unit = vec2<f32>(dot(params.to_unit_x.xyz, device), dot(params.to_unit_y.xyz, device));
    if (unit.x < 0.0 || unit.x > 1.0 || unit.y < 0.0 || unit.y > 1.0) {
        return vec4<f32>(0.0);
    }
    let c = textureLoad(content, pixel, 0);
    return vec4<f32>(min(c.rgb * factor_at(unit), vec3<f32>(c.a)), c.a);
}
"#;
