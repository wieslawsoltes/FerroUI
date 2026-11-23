//! The blur of a box shadow on the WebGL2 context of a canvas (#1139). The renderer of the canvas
//! only renders scenes of the size of its drawing buffer, so a shadow does not go through it: a
//! program writes the coverage of the box in closed form into a texture of the size of the bounds
//! of the shadow, two passes blur it (at a reduced scale for a large deviation), and a last pass
//! scales a reduced result back with a bilinear filter, so that it is painted one texel per pixel.

use super::layer_programs::LayerProgram;
use super::{VelloWebGlGpu, VelloWebGlTexture};
use crate::drawing_context_impl::box_shadows::{ShadowKey, ShadowShape};
use crate::shadow_gaussian::{gaussian_weights, reduction_for, REACH, WEIGHTS};
use kurbo::Affine;
use std::rc::Rc;

pub(super) const COVERAGE_FLOATS: usize = 28;
pub(super) const GAUSSIAN_FLOATS: usize = 24;
pub(super) const SCALE_UP_FLOATS: usize = 4;

/// The coverage of a rounded box, or of what is around it for an inset shadow.
pub(super) const COVERAGE: &str = r#"
// 0: the box (left, top, right, bottom) in the space of the shape; 1: the radii (x, y) of its top
// left and top right corners; 2: of its bottom right and bottom left corners; 3, 4: the rows of
// the transform from the pixels of the area to the space of the shape; 5: the rectangle around
// the box of an inset shadow; 6: x 1 for an inset shadow, y the length of a pixel in that space.
uniform vec4 u_params[7];

float box_distance(vec2 p) {
    vec4 box = u_params[0];
    bool left = p.x < (box.x + box.z) * 0.5;
    bool top = p.y < (box.y + box.w) * 0.5;
    vec2 radii = top ? (left ? u_params[1].xy : u_params[1].zw) : (left ? u_params[2].zw : u_params[2].xy);
    float d = max(max(box.x - p.x, p.x - box.z), max(box.y - p.y, p.y - box.w));
    if (radii.x > 0.0 && radii.y > 0.0) {
        vec2 c = vec2(left ? box.x + radii.x : box.z - radii.x, top ? box.y + radii.y : box.w - radii.y);
        vec2 q = p - c;
        if ((left ? q.x < 0.0 : q.x > 0.0) && (top ? q.y < 0.0 : q.y > 0.0)) {
            vec2 n = q / radii;
            vec2 gradient = 2.0 * q / (radii * radii);
            float len = length(gradient);
            d = len > 0.0 ? (dot(n, n) - 1.0) / len : -min(radii.x, radii.y);
        }
    }
    return d;
}

void main() {
    ivec2 pixel = ivec2(floor(gl_FragCoord.xy));
    if (outside(pixel)) {
        o_color = vec4(0.0);
        return;
    }
    vec3 at = vec3(vec2(pixel) + 0.5, 1.0);
    vec2 p = vec2(dot(u_params[3].xyz, at), dot(u_params[4].xyz, at));
    float length_of_pixel = u_params[6].y;
    float c = clamp(0.5 - box_distance(p) / length_of_pixel, 0.0, 1.0);
    if (u_params[6].x > 0.5) {
        vec4 o = u_params[5];
        float around = max(max(o.x - p.x, p.x - o.z), max(o.y - p.y, p.y - o.w));
        c = clamp(0.5 - around / length_of_pixel, 0.0, 1.0) * (1.0 - c);
    }
    o_color = vec4(c);
}
"#;

/// One direction of the Gaussian, as `BLUR_WGSL` of `gpu/shadow_blur.rs`.
pub(super) const GAUSSIAN: &str = r#"
// 0 to 3: the weights; 4: what the blurred coverage is multiplied by, a premultiplied colour;
// 5: x, y the direction of the pass, z how many texels it reads at each side.
uniform vec4 u_params[6];

float weight(int index) {
    return u_params[index / 4][index % 4];
}

float coverage(ivec2 p) {
    if (p.x < 0 || p.y < 0 || float(p.x) >= u_area.z || float(p.y) >= u_area.w) {
        return 0.0;
    }
    return load_content(p).a;
}

void main() {
    ivec2 pixel = ivec2(floor(gl_FragCoord.xy));
    if (outside(pixel)) {
        o_color = vec4(0.0);
        return;
    }
    ivec2 direction = ivec2(u_params[5].xy);
    int radius = int(u_params[5].z);
    float sum = weight(0) * coverage(pixel);
    for (int i = 1; i <= radius; i++) {
        ivec2 offset = direction * i;
        sum += weight(i) * (coverage(pixel + offset) + coverage(pixel - offset));
    }
    o_color = u_params[4] * sum;
}
"#;

/// A reduced result scaled back to one texel per pixel.
pub(super) const SCALE_UP: &str = r#"
// 0: x the texels per pixel, y the margin of the source in texels, z, w its size in texels.
uniform vec4 u_params[1];

vec4 source(ivec2 p) {
    if (p.x < 0 || p.y < 0 || float(p.x) >= u_params[0].z || float(p.y) >= u_params[0].w) {
        return vec4(0.0);
    }
    return texelFetch(u_content, p, 0);
}

void main() {
    ivec2 pixel = ivec2(floor(gl_FragCoord.xy));
    if (outside(pixel)) {
        o_color = vec4(0.0);
        return;
    }
    vec2 at = (vec2(pixel) + 0.5) * u_params[0].x + vec2(u_params[0].y) - 0.5;
    ivec2 corner = ivec2(floor(at));
    vec2 f = at - vec2(corner);
    vec4 top = mix(source(corner), source(corner + ivec2(1, 0)), f.x);
    vec4 bottom = mix(source(corner + ivec2(0, 1)), source(corner + ivec2(1, 1)), f.x);
    o_color = mix(top, bottom, f.y);
}
"#;

/// A blurred shadow in a texture of the context, upright.
#[derive(Clone)]
pub(crate) struct WebGlBlurredShape {
    pub(crate) texture: Rc<VelloWebGlTexture>,
    /// The texels of the texture that hold the result.
    pub(crate) size: (u32, u32),
    /// The texel (on both axes) that lies on the first pixel of the area.
    pub(crate) offset: f64,
}

/// The most shadows a context keeps, and the most bytes of their textures.
const MOST_SHADOWS: usize = 64;
const MOST_BYTES: usize = 32 * 1024 * 1024;

struct KeptShadow {
    key: ShadowKey,
    shadow: WebGlBlurredShape,
    used: u64,
    bytes: usize,
}

/// The blurred shadows of a context, in its state so that they end with it: the same shadow in
/// the next frame, or at another place on whole pixels, is drawn from the same texture. A texture
/// that is kept is not the pool's to hand out. The ones used least recently go first.
#[derive(Default)]
pub(super) struct ShadowCache {
    kept: Vec<KeptShadow>,
    uses: u64,
    bytes: usize,
}

impl ShadowCache {
    pub(super) fn get(&mut self, key: &ShadowKey) -> Option<WebGlBlurredShape> {
        self.uses += 1;
        let uses = self.uses;
        let found = self.kept.iter_mut().find(|kept| kept.key == *key)?;
        found.used = uses;
        Some(found.shadow.clone())
    }

    pub(super) fn insert(&mut self, key: ShadowKey, shadow: WebGlBlurredShape) {
        let (width, height) = shadow.texture.size();
        let bytes = width as usize * height as usize * 4;
        if bytes > MOST_BYTES {
            return;
        }
        while self.kept.len() >= MOST_SHADOWS || self.bytes + bytes > MOST_BYTES {
            let Some(oldest) = (0..self.kept.len()).min_by_key(|&index| self.kept[index].used) else {
                break;
            };
            self.bytes -= self.kept.swap_remove(oldest).bytes;
        }
        self.uses += 1;
        self.bytes += bytes;
        self.kept.push(KeptShadow { key, shadow, used: self.uses, bytes });
    }

    pub(super) fn clear(&mut self) {
        self.kept.clear();
        self.bytes = 0;
    }
}

fn coverage_uniforms(shape: &ShadowShape, from_texels: Affine, length_of_pixel: f64) -> [f32; COVERAGE_FLOATS] {
    let (rounded, around) = match shape {
        ShadowShape::Box(rounded) => (Some(rounded), None),
        ShadowShape::Around { outer, hole } => (hole.as_ref(), Some(*outer)),
    };
    let mut floats = [0.0f32; COVERAGE_FLOATS];
    match rounded {
        Some(box_) => {
            let r = box_.rect;
            floats[..4].copy_from_slice(&[r.x as f32, r.y as f32, (r.x + r.width) as f32, (r.y + r.height) as f32]);
            let radii = [box_.radii_top_left, box_.radii_top_right, box_.radii_bottom_right, box_.radii_bottom_left];
            for (slot, radius) in floats[4..12].chunks_exact_mut(2).zip(radii) {
                slot.copy_from_slice(&[radius.x as f32, radius.y as f32]);
            }
        }
        // No hole: a box that holds no point, so that everything is around it.
        None => floats[..4].copy_from_slice(&[f32::MAX, f32::MAX, -f32::MAX, -f32::MAX]),
    }
    let [a, b, c, d, e, f] = from_texels.as_coeffs();
    floats[12..16].copy_from_slice(&[a as f32, c as f32, e as f32, 0.0]);
    floats[16..20].copy_from_slice(&[b as f32, d as f32, f as f32, 0.0]);
    if let Some(outer) = around {
        floats[20..24].copy_from_slice(&[
            outer.x as f32,
            outer.y as f32,
            (outer.x + outer.width) as f32,
            (outer.y + outer.height) as f32,
        ]);
        floats[24] = 1.0;
    }
    floats[25] = length_of_pixel as f32;
    floats
}

fn gaussian_uniforms(direction: (f32, f32), deviation: f64, color: [f32; 4]) -> [f32; GAUSSIAN_FLOATS] {
    let (weights, radius) = gaussian_weights(deviation);
    let mut floats = [0.0f32; GAUSSIAN_FLOATS];
    floats[..WEIGHTS].copy_from_slice(&weights);
    floats[16..20].copy_from_slice(&color);
    floats[20..24].copy_from_slice(&[direction.0, direction.1, radius as f32, 0.0]);
    floats
}

/// `shape` blurred by a Gaussian of `std_deviation` (in the units of the shape) in the
/// premultiplied `color`, for an area of `width` by `height` pixels that `to_area` maps the shape
/// to. Only the bounds of the shadow and the reach of the blur around them are drawn. `Ok(None)`
/// when a program did not compile or the transform collapses the shape.
pub(crate) fn blurred_shadow(
    gpu: &VelloWebGlGpu,
    shape: &ShadowShape,
    to_area: Affine,
    std_deviation: f64,
    color: [f32; 4],
    width: u32,
    height: u32,
) -> Result<Option<WebGlBlurredShape>, String> {
    let [a, b, c, d, _, _] = to_area.as_coeffs();
    let (deviation_x, deviation_y) = (std_deviation * a.hypot(b), std_deviation * c.hypot(d));
    let largest = deviation_x.max(deviation_y);
    let reduction = reduction_for(largest);
    let margin = (largest / reduction * REACH).ceil();
    let texels = |pixels: u32| (f64::from(pixels) / reduction).ceil() as u32 + 2 * margin as u32 + 1;
    let area = (texels(width), texels(height));

    let to_texels = Affine::translate((margin, margin)) * Affine::scale(1.0 / reduction) * to_area;
    let determinant = to_texels.determinant();
    if !determinant.is_finite() || determinant == 0.0 {
        return Ok(None);
    }
    let coverage = coverage_uniforms(shape, to_texels.inverse(), 1.0 / determinant.abs().sqrt());
    let Some(content) = gpu.layer_texture(LayerProgram::ShadowCoverage, &[], area, &coverage)? else {
        return Ok(None);
    };
    let across = gaussian_uniforms((1.0, 0.0), deviation_x / reduction, [1.0; 4]);
    let Some(across) = gpu.layer_texture(LayerProgram::ShadowGaussian, &[(&content, false)], area, &across)? else {
        return Ok(None);
    };
    let down = gaussian_uniforms((0.0, 1.0), deviation_y / reduction, color);
    let Some(blurred) = gpu.layer_texture(LayerProgram::ShadowGaussian, &[(&across, false)], area, &down)? else {
        return Ok(None);
    };
    if reduction <= 1.0 {
        return Ok(Some(WebGlBlurredShape { texture: blurred, size: area, offset: margin }));
    }
    let scale_up = [(1.0 / reduction) as f32, margin as f32, area.0 as f32, area.1 as f32];
    let size = (width, height);
    let Some(texture) = gpu.layer_texture(LayerProgram::ShadowScaleUp, &[(&blurred, false)], size, &scale_up)? else {
        return Ok(None);
    };
    Ok(Some(WebGlBlurredShape { texture, size, offset: 0.0 }))
}
