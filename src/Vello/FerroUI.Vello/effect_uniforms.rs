//! The uniform blocks of the passes of the layers with an effect, written once for both GPU
//! routes: the WGSL programs of the device (`gpu/effect_passes.rs`) read them as their `Params`
//! struct, the GLSL ES 3.0 programs of the WebGL2 context (`web_gl/layer_programs.rs`) as an
//! array of `vec4` of the same layout (docs/porting/gpu-rendering.md, steps 2 and 3).
//!
//! Each block is a list of floats, four to a `vec4`, in the order of the fields of `Params`.

// Each route uses what it draws: a build with one of them, or neither, leaves the rest unused.
#![cfg_attr(not(all(feature = "hybrid", feature = "hybrid-webgl")), allow(dead_code))]

use ferroui_base::media::{ColorMatrix, ColorMatrixAlphaMode, ConvolveMatrix};
use ferroui_base::platform::{LightingParameters, PixelOperation};
use ferroui_base::rendering::composition::effects::BlendEffectMode;
use ferroui_base::rendering::composition::server::{LightSource, LIGHT_MAP_MAX_SCALE};

/// The most point and spot lights the lighting programs evaluate.
pub(crate) const MAX_LIGHTS: usize = LightingParameters::MAX_LIGHTS;

const _: () = assert!(MAX_LIGHTS == 8, "the arrays of lights of the programs have 8 elements");

/// Floats of a light in the lighting block: 10 `vec4`.
pub(crate) const LIGHT_FLOATS: usize = 40;
/// Floats of the lighting block before the lights: 6 `vec4`.
pub(crate) const HEAD_FLOATS: usize = 24;
/// Floats of the lighting block.
pub(crate) const LIGHTING_FLOATS: usize = HEAD_FLOATS + LIGHT_FLOATS * MAX_LIGHTS;
/// Floats of the colour matrix block: the five rows and the mode.
pub(crate) const COLOR_MATRIX_FLOATS: usize = 6 * 4;
/// Floats of the pixel operation block.
pub(crate) const PIXEL_OPERATION_FLOATS: usize = 6 * 4;
/// Floats of the convolution block: seven `vec4` of weights, the size, the factors, the extent.
pub(crate) const CONVOLUTION_FLOATS: usize = 10 * 4;

/// What a lighting program writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum LightingOutput {
    /// The light clamped to 0..1, opaque.
    Light,
    /// The light clamped to 0..`scale`, over it.
    Factors { scale_bits: u32 },
}

/// The lighting block: the lights of `lighting`, what the program writes, the device pixel of
/// the top left of the target (`origin`) and the transform from device pixels to the
/// coordinates of the lit visual (`to_local`).
pub(crate) fn lighting(
    lighting: &LightingParameters,
    output: LightingOutput,
    origin: (u32, u32),
    to_local: kurbo::Affine,
) -> Vec<f32> {
    let mut floats = vec![0f32; LIGHTING_FLOATS];
    let constant = lighting.constant;
    let material = lighting.material;
    let (mode, scale) = match output {
        LightingOutput::Light => (0.0, 1.0),
        LightingOutput::Factors { scale_bits } => (1.0, f32::from_bits(scale_bits)),
    };
    let [a, b, c, d, e, f] = to_local.as_coeffs();
    floats[0..4].copy_from_slice(&[constant.r as f32, constant.g as f32, constant.b as f32, 0.0]);
    floats[4..8].copy_from_slice(&[
        material.ambient as f32,
        material.diffuse as f32,
        material.specular as f32,
        material.shine as f32,
    ]);
    floats[8..12].copy_from_slice(&[mode, scale, lighting.lights.len().min(MAX_LIGHTS) as f32, 0.0]);
    floats[12..16].copy_from_slice(&[origin.0 as f32, origin.1 as f32, 0.0, 0.0]);
    // x' = a x + c y + e, y' = b x + d y + f
    floats[16..20].copy_from_slice(&[a as f32, c as f32, e as f32, 0.0]);
    floats[20..24].copy_from_slice(&[b as f32, d as f32, f as f32, 0.0]);
    let max_cutoff = |value: f64| -> [f32; 2] {
        if value.is_nan() || value >= f64::from(f32::MAX) {
            [0.0, 0.0]
        } else if value <= -f64::from(f32::MAX) {
            [-f32::MAX, 1.0]
        } else {
            [value as f32, 1.0]
        }
    };
    for (index, light) in lighting.lights.iter().take(MAX_LIGHTS).enumerate() {
        let at = HEAD_FLOATS + index * LIGHT_FLOATS;
        let slot = |field: usize| at + field * 4;
        let m = light.to_light_space;
        floats[slot(7)..slot(7) + 3].copy_from_slice(&[m.m11 as f32, m.m21 as f32, m.m31 as f32]);
        floats[slot(8)..slot(8) + 3].copy_from_slice(&[m.m12 as f32, m.m22 as f32, m.m32 as f32]);
        floats[slot(9)..slot(9) + 3].copy_from_slice(&[m.m13 as f32, m.m23 as f32, m.m33 as f32]);
        let (attenuation, position) = match light.source {
            LightSource::Point { color, position, attenuation } => {
                floats[slot(0)] = 1.0;
                floats[slot(1)..slot(1) + 3].copy_from_slice(&[color.r as f32, color.g as f32, color.b as f32]);
                (attenuation, position)
            }
            LightSource::Spot {
                inner_color,
                outer_color,
                position,
                direction,
                inner_cone_angle,
                outer_cone_angle,
                attenuation,
            } => {
                let outer = outer_cone_angle.clamp(0.0, std::f64::consts::PI);
                let inner = inner_cone_angle.clamp(0.0, outer);
                floats[slot(0)..slot(0) + 3].copy_from_slice(&[2.0, inner.cos() as f32, outer.cos() as f32]);
                floats[slot(1)..slot(1) + 3].copy_from_slice(&[
                    inner_color.r as f32,
                    inner_color.g as f32,
                    inner_color.b as f32,
                ]);
                floats[slot(2)..slot(2) + 3].copy_from_slice(&[
                    outer_color.r as f32,
                    outer_color.g as f32,
                    outer_color.b as f32,
                ]);
                floats[slot(4)..slot(4) + 3].copy_from_slice(&[
                    direction.x as f32,
                    direction.y as f32,
                    direction.z as f32,
                ]);
                (attenuation, position)
            }
            // The parameters list only point and spot lights.
            LightSource::Ambient { .. } | LightSource::Distant { .. } => continue,
        };
        floats[slot(3)..slot(3) + 3].copy_from_slice(&[position.x as f32, position.y as f32, position.z as f32]);
        let min_cutoff = attenuation.min_cutoff.clamp(-f64::from(f32::MAX), f64::from(f32::MAX));
        floats[slot(5)..slot(5) + 4].copy_from_slice(&[
            attenuation.constant as f32,
            attenuation.linear as f32,
            attenuation.quadratic as f32,
            min_cutoff as f32,
        ]);
        floats[slot(6)..slot(6) + 2].copy_from_slice(&max_cutoff(attenuation.max_cutoff));
    }
    floats
}

/// The block of the multiply layer of lit visuals over an area of `width` by `height` pixels:
/// the lights as factors clamped to 0..`LIGHT_MAP_MAX_SCALE`; `to_local` maps a pixel of the area
/// to the coordinates of the lit visual. `None` for more lights than the programs evaluate.
pub(crate) fn light_multiply(
    lighting: &LightingParameters,
    (width, height): (u32, u32),
    to_local: kurbo::Affine,
) -> Option<Vec<f32>> {
    if lighting.lights.len() > MAX_LIGHTS {
        return None;
    }
    let scale = LIGHT_MAP_MAX_SCALE as f32;
    let output = LightingOutput::Factors { scale_bits: scale.to_bits() };
    let mut floats = self::lighting(lighting, output, (0, 0), to_local);
    floats[14] = width as f32;
    floats[15] = height as f32;
    Some(floats)
}

/// The colour matrix block over an area of `width` by `height` pixels.
pub(crate) fn color_matrix(
    matrix: &ColorMatrix,
    alpha_mode: ColorMatrixAlphaMode,
    (width, height): (u32, u32),
) -> Vec<f32> {
    let mut floats = Vec::with_capacity(COLOR_MATRIX_FLOATS);
    for row in matrix.rows {
        floats.extend_from_slice(&row);
    }
    let as_it_is = if alpha_mode == ColorMatrixAlphaMode::Straight { 1.0 } else { 0.0 };
    floats.extend_from_slice(&[as_it_is, width as f32, height as f32, 0.0]);
    floats
}

/// The colour matrix of the luminance mask (`ImmutableLuminanceToAlphaEffect`), applied to the
/// premultiplied colours as they are ([`ColorMatrixAlphaMode::Straight`]): the luminance of the
/// colour as the alpha, black (the weights of the CPU path of the drawing context).
pub(crate) fn luminance_mask_matrix() -> ColorMatrix {
    let (r, g, b) = (0.2126, 0.7152, 0.0722);
    ColorMatrix::new([[0.0, 0.0, 0.0, r], [0.0, 0.0, 0.0, g], [0.0, 0.0, 0.0, b], [0.0; 4], [0.0; 4]])
}

/// The block of a pixel operation of two sources over an area of `width` by `height` pixels;
/// `to_local` maps a pixel of the area to the coordinates of the drawing (where the grid of
/// `Dissolve` is). `None` for a `Dissolve` without a grid.
pub(crate) fn pixel_operation(
    operation: &PixelOperation,
    (width, height): (u32, u32),
    to_local: kurbo::Affine,
) -> Option<Vec<f32>> {
    let (kind, mode, k, grid, cells) = match *operation {
        PixelOperation::Blend { mode, noise_grid, noise_size } => {
            if mode == BlendEffectMode::Dissolve && (noise_grid.width <= 0.0 || noise_grid.height <= 0.0) {
                return None;
            }
            let grid = [noise_grid.x as f32, noise_grid.y as f32, noise_grid.width as f32, noise_grid.height as f32];
            let cells = [noise_size.width.max(1) as f32, noise_size.height.max(1) as f32, 0.0, 0.0];
            (0.0, mode as i32 as f32, [0.0; 4], grid, cells)
        }
        PixelOperation::Arithmetic { k } => (1.0, 0.0, k, [0.0; 4], [0.0; 4]),
        PixelOperation::MaskInvert => (2.0, 0.0, [0.0; 4], [0.0; 4], [0.0; 4]),
        #[allow(unreachable_patterns)]
        _ => return None,
    };
    let [a, b, c, d, e, f] = to_local.as_coeffs();
    let mut floats = Vec::with_capacity(PIXEL_OPERATION_FLOATS);
    floats.extend_from_slice(&[kind, mode, width as f32, height as f32]);
    floats.extend_from_slice(&k);
    floats.extend_from_slice(&grid);
    floats.extend_from_slice(&cells);
    // x' = a x + c y + e, y' = b x + d y + f
    floats.extend_from_slice(&[a as f32, c as f32, e as f32, 0.0]);
    floats.extend_from_slice(&[b as f32, d as f32, f as f32, 0.0]);
    Some(floats)
}

/// The convolution block over an area of `width` by `height` pixels.
pub(crate) fn convolution(convolution: &ConvolveMatrix, (width, height): (u32, u32)) -> Vec<f32> {
    let mut floats = vec![0.0f32; CONVOLUTION_FLOATS];
    for (slot, weight) in floats.iter_mut().take(28).zip(convolution.correlation_kernel()) {
        *slot = weight;
    }
    let (target_x, target_y) = convolution.target();
    let columns = convolution.kernel_width.clamp(1, ConvolveMatrix::MAX_SIDE);
    let rows = convolution.kernel_height.clamp(1, ConvolveMatrix::MAX_SIDE);
    floats[28..32].copy_from_slice(&[columns as f32, rows as f32, target_x as f32, target_y as f32]);
    let keep_alpha = if convolution.preserve_alpha { 1.0 } else { 0.0 };
    let repeat_edges = if convolution.clamp_edges { 1.0 } else { 0.0 };
    floats[32..36].copy_from_slice(&[convolution.gain(), convolution.bias, keep_alpha, repeat_edges]);
    floats[36..40].copy_from_slice(&[width as f32, height as f32, 0.0, 0.0]);
    floats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blocks_have_the_sizes_of_the_programs() {
        let identity =
            [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0], [0.0; 4]];
        let matrix = ColorMatrix::new(identity);
        assert_eq!(color_matrix(&matrix, ColorMatrixAlphaMode::Straight, (3, 4)).len(), COLOR_MATRIX_FLOATS);
        assert_eq!(luminance_mask_matrix().rows[3], [0.0; 4]);
        use ferroui_base::rendering::composition::server::{LightColor, LightingMaterial};
        let lighting =
            LightingParameters { constant: LightColor::BLACK, lights: Vec::new(), material: LightingMaterial::VISUAL };
        let block = self::lighting(&lighting, LightingOutput::Light, (0, 0), kurbo::Affine::IDENTITY);
        assert_eq!(block.len(), LIGHTING_FLOATS);
        let block = light_multiply(&lighting, (7, 9), kurbo::Affine::IDENTITY).expect("no lights");
        assert_eq!(block.len(), LIGHTING_FLOATS);
        // The size of the area is the second half of `origin`, after the factors mode.
        assert_eq!((block[8], block[14], block[15]), (1.0, 7.0, 9.0));
        let block = pixel_operation(&PixelOperation::MaskInvert, (5, 6), kurbo::Affine::translate((2.0, 3.0)))
            .expect("MaskInvert has no grid");
        assert_eq!(block.len(), PIXEL_OPERATION_FLOATS);
        assert_eq!(&block[0..4], &[2.0, 0.0, 5.0, 6.0]);
        // The translation of the transform from the pixels of the area to the drawing.
        assert_eq!((block[18], block[22]), (2.0, 3.0));
        assert_eq!(convolution(&ConvolveMatrix::default(), (8, 2)).len(), CONVOLUTION_FLOATS);
    }

    #[test]
    fn the_colour_matrix_block_is_its_rows_then_the_mode() {
        let mut rows = [[0.0f32; 4]; 5];
        for (index, row) in rows.iter_mut().enumerate() {
            *row = [index as f32; 4];
        }
        let block = color_matrix(&ColorMatrix::new(rows), ColorMatrixAlphaMode::Premultiplied, (10, 20));
        assert_eq!(&block[16..20], &[4.0; 4]);
        assert_eq!(&block[20..24], &[0.0, 10.0, 20.0, 0.0]);
    }
}
