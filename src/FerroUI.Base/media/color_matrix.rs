//! A colour matrix: the `Matrix5x4` of WinUI's colour matrix effect
//! (Win2D `ColorMatrixEffect.ColorMatrix`), which the drawing contexts apply
//! through [`IDrawingContextImplWithLayerEffects::push_color_matrix_layer`](crate::platform::IDrawingContextImplWithLayerEffects::push_color_matrix_layer).

/// A colour transform of the form `out = in * M`: `in` is the row
/// `[R, G, B, A, 1]` of a colour with straight (not premultiplied) alpha,
/// each channel from 0 to 1, and `M` has five rows of four columns.
///
/// Row `i` holds what input channel `i` (red, green, blue, alpha) adds to
/// each output channel (red, green, blue, alpha); the fifth row is the
/// offset of each output channel. This is the layout of the `Matrix5x4` of
/// WinUI (`M11` .. `M54`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorMatrix {
    /// The rows: red, green, blue and alpha in, then the offsets.
    pub rows: [[f32; 4]; 5],
}

impl Default for ColorMatrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// How a [`ColorMatrix`] treats the premultiplied colours it is applied to
/// (Win2D `CanvasAlphaMode` of `ColorMatrixEffect.AlphaMode`, Direct2D's
/// `D2D1_COLORMATRIX_ALPHA_MODE`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorMatrixAlphaMode {
    /// The colour is un-premultiplied, the matrix applied to `[R, G, B, a, 1]`
    /// and the result premultiplied again (the Win2D default).
    #[default]
    Premultiplied,
    /// The matrix is applied to the premultiplied colour as it is,
    /// `[R a, G a, B a, a, 1]`, and the result taken as premultiplied (its
    /// colour clamped to its alpha).
    Straight,
}

/// The weights of the luminance of WinUI's luminance-to-alpha matrix.
pub const LUMINANCE_WEIGHTS: [f32; 3] = [0.2125, 0.7154, 0.0721];

impl ColorMatrix {
    /// The matrix that changes nothing.
    pub const IDENTITY: ColorMatrix = ColorMatrix {
        rows: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0], [0.0; 4]],
    };

    /// A matrix from its rows.
    pub const fn new(rows: [[f32; 4]; 5]) -> ColorMatrix {
        ColorMatrix { rows }
    }

    /// A matrix from the twenty members of WinUI's `Matrix5x4`, in the order
    /// `M11, M12, M13, M14, M21, ..., M54`.
    pub fn from_matrix5x4(m: [f32; 20]) -> ColorMatrix {
        let mut rows = [[0.0; 4]; 5];
        for (index, value) in m.into_iter().enumerate() {
            rows[index / 4][index % 4] = value;
        }
        ColorMatrix { rows }
    }

    /// The saturation matrix of Win2D's `SaturationEffect` (the
    /// `saturate` colour matrix of SVG): 0 is grey, 1 changes nothing,
    /// values above 1 saturate more.
    pub fn saturation(saturation: f32) -> ColorMatrix {
        let s = saturation;
        ColorMatrix::new([
            [0.213 + 0.787 * s, 0.213 - 0.213 * s, 0.213 - 0.213 * s, 0.0],
            [0.715 - 0.715 * s, 0.715 + 0.285 * s, 0.715 - 0.715 * s, 0.0],
            [0.072 - 0.072 * s, 0.072 - 0.072 * s, 0.072 + 0.928 * s, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [0.0; 4],
        ])
    }

    /// Win2D's `LuminanceToAlphaEffect`: the luminance of the colour becomes
    /// its alpha, the colour black.
    pub fn luminance_to_alpha() -> ColorMatrix {
        let [r, g, b] = LUMINANCE_WEIGHTS;
        ColorMatrix::new([[0.0, 0.0, 0.0, r], [0.0, 0.0, 0.0, g], [0.0, 0.0, 0.0, b], [0.0; 4], [0.0; 4]])
    }

    /// Win2D's `TintEffect`: each channel multiplied by the channel of
    /// `tint` (red, green, blue, alpha from 0 to 1).
    pub fn tint(tint: [f32; 4]) -> ColorMatrix {
        ColorMatrix::new([
            [tint[0], 0.0, 0.0, 0.0],
            [0.0, tint[1], 0.0, 0.0],
            [0.0, 0.0, tint[2], 0.0],
            [0.0, 0.0, 0.0, tint[3]],
            [0.0; 4],
        ])
    }

    /// Win2D's `InvertEffect`: each colour channel `c` becomes `1 - c`; the
    /// alpha is kept.
    pub fn invert() -> ColorMatrix {
        ColorMatrix::new([
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 0.0],
        ])
    }

    /// Win2D's `GrayscaleEffect`: each colour channel becomes
    /// `0.299 R + 0.587 G + 0.114 B`.
    pub fn grayscale() -> ColorMatrix {
        let (r, g, b) = (0.299, 0.587, 0.114);
        ColorMatrix::new([[r, r, r, 0.0], [g, g, g, 0.0], [b, b, b, 0.0], [0.0, 0.0, 0.0, 1.0], [0.0; 4]])
    }

    /// The matrix that scales each channel and adds an offset:
    /// `out = in * scale + offset`, channel by channel.
    pub fn scale_and_offset(scale: [f32; 4], offset: [f32; 4]) -> ColorMatrix {
        let mut rows = [[0.0; 4]; 5];
        for channel in 0..4 {
            rows[channel][channel] = scale[channel];
        }
        rows[4] = offset;
        ColorMatrix { rows }
    }

    /// Whether the matrix changes nothing.
    pub fn is_identity(&self) -> bool {
        *self == ColorMatrix::IDENTITY
    }

    /// The matrix that applies `self`, then `next`.
    pub fn then(&self, next: &ColorMatrix) -> ColorMatrix {
        let mut rows = [[0.0; 4]; 5];
        for (row, out) in rows.iter_mut().enumerate() {
            for (column, value) in out.iter_mut().enumerate() {
                let mut sum = if row == 4 { next.rows[4][column] } else { 0.0 };
                for k in 0..4 {
                    sum += self.rows[row][k] * next.rows[k][column];
                }
                *value = sum;
            }
        }
        ColorMatrix { rows }
    }

    /// Applies the matrix to a colour with straight alpha, each channel from
    /// 0 to 1. The result is clamped to 0..1.
    pub fn apply(&self, color: [f32; 4]) -> [f32; 4] {
        let mut out = self.rows[4];
        for (channel, value) in color.into_iter().enumerate() {
            for (column, sum) in out.iter_mut().enumerate() {
                *sum += value * self.rows[channel][column];
            }
        }
        out.map(|value| if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) })
    }

    /// Applies the matrix to a premultiplied colour, each channel from 0 to
    /// 1, in `mode`; the result is premultiplied, clamped to 0..1, its colour
    /// channels no larger than its alpha.
    pub fn apply_premultiplied(&self, color: [f32; 4], mode: ColorMatrixAlphaMode) -> [f32; 4] {
        match mode {
            ColorMatrixAlphaMode::Straight => {
                let [r, g, b, a] = self.apply(color);
                [r.min(a), g.min(a), b.min(a), a]
            }
            ColorMatrixAlphaMode::Premultiplied => {
                let alpha = color[3];
                let straight = |value: f32| if alpha > 0.0 { (value / alpha).min(1.0) } else { 0.0 };
                let [r, g, b, a] = self.apply([straight(color[0]), straight(color[1]), straight(color[2]), alpha]);
                [r * a, g * a, b * a, a]
            }
        }
    }

    /// Applies the matrix in `mode` to premultiplied RGBA pixels of eight
    /// bits a channel, in place.
    pub fn apply_to_rgba8(&self, pixels: &mut [u8], mode: ColorMatrixAlphaMode) {
        if mode == ColorMatrixAlphaMode::Premultiplied {
            return self.apply_to_premultiplied_rgba8(pixels);
        }
        for pixel in pixels.chunks_exact_mut(4) {
            let color = [0, 1, 2, 3].map(|channel| pixel[channel] as f32 / 255.0);
            let out = self.apply_premultiplied(color, mode);
            for channel in 0..4 {
                pixel[channel] = (out[channel] * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    /// Applies the matrix to premultiplied RGBA pixels of eight bits a
    /// channel, in place: each pixel is unpremultiplied, transformed and
    /// premultiplied again (the straight alpha mode).
    pub fn apply_to_premultiplied_rgba8(&self, pixels: &mut [u8]) {
        for pixel in pixels.chunks_exact_mut(4) {
            let alpha = pixel[3] as f32 / 255.0;
            let straight = |value: u8| if alpha > 0.0 { (value as f32 / 255.0 / alpha).min(1.0) } else { 0.0 };
            let [r, g, b, a] = self.apply([straight(pixel[0]), straight(pixel[1]), straight(pixel[2]), alpha]);
            let premultiplied = |value: f32| (value * a * 255.0).round().clamp(0.0, 255.0) as u8;
            pixel[0] = premultiplied(r);
            pixel[1] = premultiplied(g);
            pixel[2] = premultiplied(b);
            pixel[3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }

    /// The matrix in the row-major 4 x 5 form of Skia and SVG: a row per
    /// output channel, its last column the offset.
    pub fn to_row_major_4x5(&self) -> [f32; 20] {
        let mut out = [0.0; 20];
        for output in 0..4 {
            for input in 0..5 {
                out[output * 5 + input] = self.rows[input][output];
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 4], b: [f32; 4]) -> bool {
        a.iter().zip(b.iter()).all(|(a, b)| (a - b).abs() < 1e-4)
    }

    #[test]
    fn identity_changes_nothing() {
        let color = [0.2, 0.4, 0.6, 0.8];
        assert!(close(ColorMatrix::IDENTITY.apply(color), color));
        assert!(ColorMatrix::default().is_identity());
    }

    #[test]
    fn rows_follow_the_matrix5x4_of_winui() {
        // M14 (red in to alpha out) and M54 (alpha offset).
        let mut members = [0.0; 20];
        members[3] = 1.0;
        members[19] = 0.25;
        let matrix = ColorMatrix::from_matrix5x4(members);
        assert_eq!(matrix.rows[0][3], 1.0);
        assert_eq!(matrix.rows[4][3], 0.25);
        assert!(close(matrix.apply([0.5, 0.0, 0.0, 0.0]), [0.0, 0.0, 0.0, 0.75]));
    }

    #[test]
    fn saturation_zero_is_grey_and_one_is_unchanged() {
        let color = [1.0, 0.0, 0.0, 1.0];
        let grey = ColorMatrix::saturation(0.0).apply(color);
        assert!((grey[0] - grey[1]).abs() < 1e-4 && (grey[1] - grey[2]).abs() < 1e-4);
        assert!(close(ColorMatrix::saturation(1.0).apply(color), color));
    }

    #[test]
    fn luminance_to_alpha_uses_the_weights_of_winui() {
        let out = ColorMatrix::luminance_to_alpha().apply([1.0, 1.0, 1.0, 1.0]);
        assert!(close(out, [0.0, 0.0, 0.0, 1.0]));
        let out = ColorMatrix::luminance_to_alpha().apply([0.0, 1.0, 0.0, 0.3]);
        assert!(close(out, [0.0, 0.0, 0.0, 0.7154]));
    }

    #[test]
    fn invert_keeps_alpha() {
        assert!(close(ColorMatrix::invert().apply([0.25, 0.5, 1.0, 0.5]), [0.75, 0.5, 0.0, 0.5]));
    }

    #[test]
    fn then_applies_in_order() {
        let a = ColorMatrix::scale_and_offset([0.5, 1.0, 1.0, 1.0], [0.0; 4]);
        let b = ColorMatrix::scale_and_offset([1.0; 4], [0.25, 0.0, 0.0, 0.0]);
        let color = [1.0, 0.0, 0.0, 1.0];
        assert!(close(a.then(&b).apply(color), b.apply(a.apply(color))));
        assert!(close(a.then(&b).apply(color), [0.75, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn premultiplied_pixels_are_transformed_straight() {
        // Half transparent red, premultiplied.
        let mut pixels = [128u8, 0, 0, 128];
        ColorMatrix::invert().apply_to_premultiplied_rgba8(&mut pixels);
        // Straight (0, 1, 1) at alpha 0.5.
        assert_eq!(pixels[0], 0);
        assert!((pixels[1] as i32 - 128).abs() <= 1);
        assert_eq!(pixels[3], 128);
    }

    #[test]
    fn premultiplied_mode_applies_to_premultiplied_values() {
        // Doubling every channel of half transparent grey.
        let matrix = ColorMatrix::scale_and_offset([2.0; 4], [0.0; 4]);
        let out = matrix.apply_premultiplied([0.25, 0.25, 0.25, 0.5], ColorMatrixAlphaMode::Straight);
        assert!(close(out, [0.5, 0.5, 0.5, 1.0]));
        // Colour channels never exceed the alpha.
        let out = ColorMatrix::scale_and_offset([1.0, 1.0, 1.0, 0.5], [0.0; 4])
            .apply_premultiplied([0.5, 0.5, 0.5, 0.5], ColorMatrixAlphaMode::Straight);
        assert!(close(out, [0.25, 0.25, 0.25, 0.25]));
        let mut pixels = [64u8, 64, 64, 128];
        matrix.apply_to_rgba8(&mut pixels, ColorMatrixAlphaMode::Straight);
        assert_eq!(pixels, [128, 128, 128, 255]);
    }

    #[test]
    fn row_major_form_is_a_row_per_output_channel() {
        let matrix = ColorMatrix::luminance_to_alpha();
        let row_major = matrix.to_row_major_4x5();
        // The alpha output row: 0.2125 R + 0.7154 G + 0.0721 B.
        assert_eq!(&row_major[15..18], &LUMINANCE_WEIGHTS);
        assert_eq!(row_major[0], 0.0);
    }
}
