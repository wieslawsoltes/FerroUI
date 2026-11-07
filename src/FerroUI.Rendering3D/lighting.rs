//! The lighting and the fragment colour of a material layer (`docs/porting/media3d.md`, section
//! 3.4, decision A): a port of ProGPU's `ComputeWpfLighting` (`Mesh3DSolid.wgsl`), per pixel, over
//! the [`LightUniforms`] of a render, and the colour each kind of layer writes. The WGSL of the
//! GPU executors (step D4) is the same functions, in the same order of operations.

use crate::math::{add, dot, length, mul, normalize, saturate, saturate3, scale, sub, xyz, Float3};
use crate::uniforms::{DrawUniforms, FrameUniforms, LayerKind, LightKind, LightUniforms};

/// The sums of the lights at a point: what `ComputeWpfLighting` adds up before it combines them
/// with the material.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct LightingTerms {
    /// The sum of the colours of the ambient lights.
    pub ambient: [f32; 3],
    /// The sum of the Lambert terms: the colour of each light times `max(N . L, 0)` and its
    /// attenuation.
    pub diffuse: [f32; 3],
    /// The sum of the Blinn-Phong terms: the colour of each light times `max(N . H, 0)` to the
    /// shininess, with its attenuation, where `N . L > 0`. Zero unless asked for.
    pub specular: [f32; 3],
}

/// The light at `world_position` with the unit normal `n`, seen from the unit direction `v`
/// toward the viewer: ProGPU's `ComputeWpfLighting` (the loop over at most 16 lights), with the
/// specular sum computed only when `with_specular` (a diffuse layer does not read it).
///
/// - An ambient light adds its colour to the ambient sum.
/// - A directional light lights along `-direction`.
/// - A point or a spot light lights from its position, not at all beyond its range (or at the
///   light itself), attenuated by `1 / max(c + l d + q d^2, 1)`.
/// - A spot light is further scaled by `clamp((rho - cos_outer) / max(cos_inner - cos_outer,
///   1e-6), 0, 1)`, where `rho` is the cosine of the angle between its axis and the direction to
///   the point (the cosines are of the half angles).
/// - A light behind the surface (`N . L <= 0`) adds nothing but its ambient part.
pub fn compute_wpf_lighting(
    lights: &[LightUniforms],
    n: Float3,
    v: Float3,
    world_position: Float3,
    shininess: f32,
    with_specular: bool,
) -> LightingTerms {
    let mut terms = LightingTerms::default();
    let shininess = shininess.max(0.001);
    for light in lights.iter().take(crate::uniforms::MAX_LIGHTS) {
        let kind = light.kind();
        let color = xyz(light.color);
        if kind == LightKind::Ambient {
            terms.ambient = add(terms.ambient, color);
            continue;
        }
        let direction = xyz(light.direction_inner_cos);
        let mut l = normalize(scale(direction, -1.0));
        let mut attenuation = 1.0f32;
        if kind == LightKind::Point || kind == LightKind::Spot {
            let to_light = sub(xyz(light.position_range), world_position);
            let distance = length(to_light);
            if distance <= 0.000001 || distance > light.position_range[3] {
                continue;
            }
            l = scale(to_light, 1.0 / distance);
            let terms_of_attenuation = xyz(light.attenuation_outer_cos);
            attenuation = 1.0
                / (terms_of_attenuation[0]
                    + terms_of_attenuation[1] * distance
                    + terms_of_attenuation[2] * distance * distance)
                    .max(1.0);
            if kind == LightKind::Spot {
                let rho = dot(normalize(scale(direction, -1.0)), l).max(0.0);
                let outer_cos = light.attenuation_outer_cos[3];
                attenuation *= ((rho - outer_cos) / (light.direction_inner_cos[3] - outer_cos).max(0.000001))
                    .clamp(0.0, 1.0);
            }
        }
        let n_dot_l = dot(n, l).max(0.0);
        if n_dot_l <= 0.0 {
            continue;
        }
        terms.diffuse = add(terms.diffuse, scale(color, n_dot_l * attenuation));
        if with_specular {
            let h = normalize(add(v, l));
            let highlight = dot(n, h).max(0.0).powf(shininess) * attenuation;
            terms.specular = add(terms.specular, scale(color, highlight));
        }
    }
    terms
}

/// The direction from `world_position` toward the viewer of the frame: toward the position of a
/// perspective or matrix camera, or the one direction of an orthographic camera.
pub fn view_direction(frame: &FrameUniforms, world_position: Float3) -> Float3 {
    let camera = frame.camera_position;
    if camera[3] > 0.5 {
        normalize(sub(xyz(camera), world_position))
    } else {
        normalize(xyz(camera))
    }
}

/// The colour a draw of a layer writes at a point, premultiplied: what the fragment stage
/// returns, before the blend.
///
/// `normal` is the interpolated normal of the face (not yet at unit length); a back face turns it
/// around (the sign in [`DrawUniforms::material`]), as ProGPU's `fs_main` does for
/// `!front_facing`.
///
/// - A diffuse layer: `saturate(color * (ambient * ambient_color + diffuse))` with the alpha of
///   the colour, blended source-over.
/// - A specular layer: `saturate(color * specular)` times the alpha of the colour, with alpha 0:
///   it is added (`One, One`).
/// - An emissive layer: the colour times its alpha, with alpha 0: added, without light.
pub fn shade(frame: &FrameUniforms, draw: &DrawUniforms, normal: Float3, world_position: Float3) -> [f32; 4] {
    let color = xyz(draw.color);
    let alpha = saturate(draw.color[3]);
    match draw.kind() {
        LayerKind::Emissive => {
            let rgb = scale(saturate3(color), alpha);
            [rgb[0], rgb[1], rgb[2], 0.0]
        }
        kind => {
            let n = normalize(scale(normal, draw.material[2]));
            let specular = kind == LayerKind::Specular;
            let v = if specular { view_direction(frame, world_position) } else { [0.0; 3] };
            let terms =
                compute_wpf_lighting(frame.active_lights(), n, v, world_position, draw.material[1], specular);
            if specular {
                let rgb = scale(saturate3(mul(color, terms.specular)), alpha);
                [rgb[0], rgb[1], rgb[2], 0.0]
            } else {
                let light = add(mul(terms.ambient, xyz(draw.ambient_color)), terms.diffuse);
                let rgb = scale(saturate3(mul(color, light)), alpha);
                [rgb[0], rgb[1], rgb[2], alpha]
            }
        }
    }
}

#[cfg(test)]
#[path = "lighting_tests.rs"]
mod tests;
