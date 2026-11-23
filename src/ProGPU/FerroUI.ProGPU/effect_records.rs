//! The records of the engine's layer effects (`progpu_native_layer_effects.h`), made from the
//! effects of the drawing context. Which pixels a pass makes is the engine's; what is here is the
//! translation of the types of the framework.

use ferroui_base::media::{ColorMatrix, ColorMatrixAlphaMode, ConvolveMatrix};
use ferroui_base::platform::{LightingParameters, PixelOperation, PositionedLight};
use ferroui_base::rendering::composition::effects::BlendEffectMode;
use ferroui_base::rendering::composition::server::{LightColor, LightSource, LIGHT_MAP_MAX_SCALE};
use ferroui_base::{Matrix, Rect};
use progpu_native_sys::layer_effects::*;
use progpu_native_sys::progpu::{progpu_native_affine_2d, progpu_native_image_rect};
use std::f64::consts::PI;
use std::mem::size_of;

const _: () = assert!(LightingParameters::MAX_LIGHTS == PROGPU_NATIVE_LAYER_EFFECT_MAX_LIGHTS);
const _: () = assert!(ConvolveMatrix::MAX_ELEMENTS == PROGPU_NATIVE_LAYER_EFFECT_MAX_KERNEL_ELEMENTS);

/// What a lit pass writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LightOutput {
    /// The light itself, opaque.
    Fill,
    /// The content of the layer times the light.
    Multiply,
}

fn affine(matrix: &Matrix) -> progpu_native_affine_2d {
    progpu_native_affine_2d {
        m11: matrix.m11 as f32,
        m12: matrix.m12 as f32,
        m21: matrix.m21 as f32,
        m22: matrix.m22 as f32,
        m31: matrix.m31 as f32,
        m32: matrix.m32 as f32,
    }
}

fn rgb(color: LightColor) -> [f32; 3] {
    [color.r as f32, color.g as f32, color.b as f32]
}

/// A distance of the attenuation in the range of the record.
fn single(value: f64) -> f32 {
    value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
}

pub(crate) fn color_matrix(matrix: &ColorMatrix, mode: ColorMatrixAlphaMode) -> progpu_native_layer_color_matrix {
    // The `Straight` mode of the framework is the matrix over the colours as they are stored.
    let flags = match mode {
        ColorMatrixAlphaMode::Straight => PROGPU_NATIVE_LAYER_COLOR_MATRIX_PREMULTIPLIED_INPUT,
        _ => 0,
    };
    progpu_native_layer_color_matrix {
        struct_size: size_of::<progpu_native_layer_color_matrix>() as u32,
        flags,
        rows: matrix.rows,
    }
}

/// The Rec. 709 luminance of the stored colours as the alpha of a black result.
pub(crate) fn luminance_mask() -> ColorMatrix {
    let mut rows = [[0.0; 4]; 5];
    rows[0][3] = 0.2126;
    rows[1][3] = 0.7152;
    rows[2][3] = 0.0722;
    ColorMatrix::new(rows)
}

pub(crate) fn convolution(matrix: &ConvolveMatrix) -> progpu_native_layer_convolution {
    let mut flags = 0;
    if matrix.preserve_alpha {
        flags |= PROGPU_NATIVE_LAYER_CONVOLUTION_KEEP_ALPHA;
    }
    if matrix.clamp_edges {
        flags |= PROGPU_NATIVE_LAYER_CONVOLUTION_REPEAT_EDGES;
    }
    progpu_native_layer_convolution {
        struct_size: size_of::<progpu_native_layer_convolution>() as u32,
        flags,
        columns: matrix.kernel_width.clamp(1, ConvolveMatrix::MAX_SIDE),
        rows: matrix.kernel_height.clamp(1, ConvolveMatrix::MAX_SIDE),
        gain: matrix.gain(),
        bias: matrix.bias,
        elements: matrix.kernel,
        reserved: 0,
    }
}

/// The record of two sources of `operation`; `to_drawing` maps a pixel of the area to the
/// coordinates of the grid of `Dissolve`. `None` for what the engine has no composite for.
pub(crate) fn composite(operation: &PixelOperation, to_drawing: &Matrix) -> Option<progpu_native_layer_composite> {
    let mut record = progpu_native_layer_composite {
        struct_size: size_of::<progpu_native_layer_composite>() as u32,
        operation: PROGPU_NATIVE_LAYER_COMPOSITE_BLEND,
        blend_mode: 0,
        grid_columns: 1,
        grid_rows: 1,
        reserved: 0,
        k: [0.0; 4],
        grid: progpu_native_image_rect { x: 0.0, y: 0.0, width: 1.0, height: 1.0 },
        pixel_to_drawing: affine(to_drawing),
    };
    match *operation {
        PixelOperation::Blend { mode, noise_grid, noise_size } => {
            record.blend_mode = mode as u32;
            if mode == BlendEffectMode::Dissolve {
                if noise_grid.width <= 0.0 || noise_grid.height <= 0.0 {
                    return None;
                }
                record.grid = progpu_native_image_rect {
                    x: noise_grid.x as f32,
                    y: noise_grid.y as f32,
                    width: noise_grid.width as f32,
                    height: noise_grid.height as f32,
                };
                record.grid_columns = noise_size.width.max(1) as u32;
                record.grid_rows = noise_size.height.max(1) as u32;
            }
        }
        PixelOperation::Arithmetic { k } => {
            record.operation = PROGPU_NATIVE_LAYER_COMPOSITE_ARITHMETIC;
            record.k = k;
        }
        PixelOperation::MaskInvert => record.operation = PROGPU_NATIVE_LAYER_COMPOSITE_MASK_INVERT,
        #[allow(unreachable_patterns)]
        _ => return None,
    }
    Some(record)
}

/// The record of a point or spot light; the parameters list no other kind.
fn light(positioned: &PositionedLight) -> Option<progpu_native_layer_light> {
    // SAFETY: all-zero is a valid record of numbers.
    let mut record: progpu_native_layer_light = unsafe { std::mem::zeroed() };
    let attenuation = match positioned.source {
        LightSource::Point { color, position, attenuation } => {
            record.kind = PROGPU_NATIVE_LAYER_LIGHT_POINT;
            record.color = rgb(color);
            record.outer_color = record.color;
            record.position = [position.x as f32, position.y as f32, position.z as f32];
            attenuation
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
            // An inner cone wider than the outer one is the outer one.
            let wider = outer_cone_angle.clamp(0.0, PI);
            record.kind = PROGPU_NATIVE_LAYER_LIGHT_SPOT;
            record.cos_inner_cone = inner_cone_angle.clamp(0.0, wider).cos() as f32;
            record.cos_outer_cone = wider.cos() as f32;
            record.color = rgb(inner_color);
            record.outer_color = rgb(outer_color);
            record.position = [position.x as f32, position.y as f32, position.z as f32];
            record.direction = [direction.x as f32, direction.y as f32, direction.z as f32];
            attenuation
        }
        _ => return None,
    };
    record.attenuation_constant = attenuation.constant as f32;
    record.attenuation_linear = attenuation.linear as f32;
    record.attenuation_quadratic = attenuation.quadratic as f32;
    record.min_cutoff = single(attenuation.min_cutoff);
    // The default cutoff, the largest number, and a cutoff that is not a number cut nothing.
    if attenuation.max_cutoff < f64::from(f32::MAX) {
        record.has_max_cutoff = 1;
        record.max_cutoff = single(attenuation.max_cutoff);
    }
    let m = positioned.to_light_space;
    record.to_light = [
        [m.m11 as f32, m.m21 as f32, m.m31 as f32],
        [m.m12 as f32, m.m22 as f32, m.m32 as f32],
        [m.m13 as f32, m.m23 as f32, m.m33 as f32],
    ];
    Some(record)
}

/// The record of the lights of `parameters`. `origin` is the device pixel of the top left of the
/// target and `to_drawing` maps a device pixel to the coordinates of the lit visual.
pub(crate) fn lighting(
    parameters: &LightingParameters,
    output: LightOutput,
    origin: (u32, u32),
    to_drawing: &Matrix,
) -> Option<Box<progpu_native_layer_lighting>> {
    if parameters.lights.len() > PROGPU_NATIVE_LAYER_EFFECT_MAX_LIGHTS {
        return None;
    }
    // SAFETY: all-zero is a valid record of numbers.
    let mut record: Box<progpu_native_layer_lighting> = Box::new(unsafe { std::mem::zeroed() });
    record.struct_size = size_of::<progpu_native_layer_lighting>() as u32;
    (record.output, record.max_factor) = match output {
        LightOutput::Fill => (PROGPU_NATIVE_LAYER_LIGHTING_FILL, 1.0),
        LightOutput::Multiply => (PROGPU_NATIVE_LAYER_LIGHTING_MULTIPLY, LIGHT_MAP_MAX_SCALE as f32),
    };
    (record.origin_x, record.origin_y) = origin;
    record.constant_light = rgb(parameters.constant);
    record.diffuse = parameters.material.diffuse as f32;
    record.specular = parameters.material.specular as f32;
    record.shine = parameters.material.shine as f32;
    record.pixel_to_drawing = affine(to_drawing);
    let mut count = 0;
    for made in parameters.lights.iter().filter_map(light) {
        record.lights[count] = made;
        count += 1;
    }
    record.light_count = count as u32;
    Some(record)
}

/// The record of a multiply by factors stretched over `bounds`; `to_drawing` maps a pixel of the
/// area to the coordinates of `bounds`.
pub(crate) fn factor_multiply(scale: f32, bounds: Rect, to_drawing: &Matrix) -> progpu_native_layer_factor_multiply {
    let to_unit = *to_drawing
        * Matrix::create_translation(-bounds.x, -bounds.y)
        * Matrix::create_scale(1.0 / bounds.width, 1.0 / bounds.height);
    progpu_native_layer_factor_multiply {
        struct_size: size_of::<progpu_native_layer_factor_multiply>() as u32,
        scale,
        pixel_to_unit: affine(&to_unit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::rendering::composition::server::{LightAttenuation, LightingMaterial};
    use ferroui_base::Vector3D;

    #[test]
    fn the_stored_colour_mode_of_a_matrix_is_the_flag_of_the_record() {
        let matrix = ColorMatrix::IDENTITY;
        assert_eq!(color_matrix(&matrix, ColorMatrixAlphaMode::Premultiplied).flags, 0);
        assert_eq!(
            color_matrix(&matrix, ColorMatrixAlphaMode::Straight).flags,
            PROGPU_NATIVE_LAYER_COLOR_MATRIX_PREMULTIPLIED_INPUT
        );
    }

    #[test]
    fn a_kernel_keeps_its_elements_and_its_flags() {
        let mut matrix = ConvolveMatrix::new(3, 1, &[1.0, 2.0, 3.0]).expect("the kernel");
        matrix.divisor = 2.0;
        matrix.clamp_edges = true;
        let record = convolution(&matrix);
        assert_eq!((record.columns, record.rows, record.gain), (3, 1, 0.5));
        assert_eq!(&record.elements[..3], &[1.0, 2.0, 3.0]);
        assert_eq!(record.flags, PROGPU_NATIVE_LAYER_CONVOLUTION_REPEAT_EDGES);
    }

    #[test]
    fn dissolve_needs_its_grid() {
        let empty = PixelOperation::Blend {
            mode: BlendEffectMode::Dissolve,
            noise_grid: Rect::new(0.0, 0.0, 0.0, 4.0),
            noise_size: ferroui_base::PixelSize::new(1, 1),
        };
        assert!(composite(&empty, &Matrix::IDENTITY).is_none());
        let record = composite(&PixelOperation::MaskInvert, &Matrix::create_translation(2.0, 3.0)).expect("mask");
        assert_eq!(record.operation, PROGPU_NATIVE_LAYER_COMPOSITE_MASK_INVERT);
        assert_eq!((record.pixel_to_drawing.m31, record.pixel_to_drawing.m32), (2.0, 3.0));
    }

    #[test]
    fn a_light_without_a_cutoff_has_none_in_its_record() {
        let positioned = PositionedLight {
            source: LightSource::Point {
                color: LightColor::BLACK,
                position: Vector3D::new(1.0, 2.0, 3.0),
                attenuation: LightAttenuation::NONE,
            },
            to_light_space: Matrix::create_translation(5.0, 6.0),
        };
        let parameters = LightingParameters {
            constant: LightColor::BLACK,
            lights: vec![positioned],
            material: LightingMaterial::VISUAL,
        };
        let record = lighting(&parameters, LightOutput::Multiply, (0, 0), &Matrix::IDENTITY).expect("one light");
        assert_eq!((record.light_count, record.lights[0].has_max_cutoff), (1, 0));
        assert_eq!(record.lights[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(record.lights[0].to_light[0], [1.0, 0.0, 5.0]);
        assert_eq!(record.max_factor, LIGHT_MAP_MAX_SCALE as f32);
    }

    #[test]
    fn factors_map_their_bounds_to_the_unit_square() {
        let record = factor_multiply(1.0, Rect::new(2.0, 4.0, 8.0, 16.0), &Matrix::IDENTITY);
        let unit = record.pixel_to_unit;
        assert_eq!((unit.m11, unit.m22, unit.m31, unit.m32), (0.125, 0.0625, -0.25, -0.25));
    }
}
