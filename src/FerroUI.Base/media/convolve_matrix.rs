//! A convolution kernel: Win2D's `ConvolveMatrixEffect` as the drawing
//! contexts apply it to the content of a layer
//! ([`IDrawingContextImplWithLayerEffects::push_convolve_matrix_layer`](crate::platform::IDrawingContextImplWithLayerEffects::push_convolve_matrix_layer)).

/// A matrix convolution of the pixels of a layer (Win2D
/// `ConvolveMatrixEffect`, the SVG `feConvolveMatrix` filter primitive):
/// each output pixel is the sum of the pixels around it weighted by the
/// kernel, divided by the divisor, plus the bias.
///
/// The kernel is applied as SVG and Direct2D apply it, turned by half a
/// turn: with `w` and `h` the width and height of the kernel and
/// `(cx, cy) = (w / 2, h / 2)` (rounded down) its target element, the output
/// at `(x, y)` is
///
/// ```text
/// sum over row i < h, column j < w of
///     source(x - cx + j, y - cy + i) * kernel[(h - 1 - i) * w + (w - 1 - j)]
/// ```
///
/// divided by [`divisor`](Self::divisor) and plus [`bias`](Self::bias). The
/// kernel steps over device pixels (Skia's matrix convolution does too).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConvolveMatrix {
    /// The number of columns of the kernel, 1 to [`MAX_SIDE`](Self::MAX_SIDE).
    pub kernel_width: u32,
    /// The number of rows of the kernel, 1 to [`MAX_SIDE`](Self::MAX_SIDE).
    pub kernel_height: u32,
    /// The elements of the kernel row by row; the first
    /// `kernel_width * kernel_height` are used, the rest are 0.
    pub kernel: [f32; ConvolveMatrix::MAX_ELEMENTS],
    /// What the weighted sum is divided by (default 1; 0 is taken as 1).
    pub divisor: f32,
    /// What is added to the result, on the scale of a channel from 0 to 1
    /// (default 0).
    pub bias: f32,
    /// Whether the pixels beyond the edges of the layer are its edge pixels
    /// repeated (Win2D's `Hard` border mode) rather than transparent (the
    /// `Soft` mode, the default).
    pub clamp_edges: bool,
    /// Whether the alpha is kept and the colours are convolved
    /// un-premultiplied (Win2D's `PreserveAlpha`); otherwise the four
    /// premultiplied channels are convolved (the default).
    pub preserve_alpha: bool,
}

impl Default for ConvolveMatrix {
    /// The 3 by 3 kernel that changes nothing.
    fn default() -> Self {
        let mut kernel = [0.0; ConvolveMatrix::MAX_ELEMENTS];
        kernel[4] = 1.0;
        Self { kernel_width: 3, kernel_height: 3, kernel, divisor: 1.0, bias: 0.0, clamp_edges: false, preserve_alpha: false }
    }
}

impl ConvolveMatrix {
    /// The largest number of rows or columns of a kernel.
    pub const MAX_SIDE: u32 = 5;
    /// The most elements of a kernel.
    pub const MAX_ELEMENTS: usize = (Self::MAX_SIDE * Self::MAX_SIDE) as usize;

    /// A kernel of `kernel_width` by `kernel_height` elements, `elements`
    /// row by row, with a divisor of 1, no bias, soft edges and the alpha
    /// convolved; `None` when a side is not 1 to [`MAX_SIDE`](Self::MAX_SIDE)
    /// or `elements` has another length.
    pub fn new(kernel_width: u32, kernel_height: u32, elements: &[f32]) -> Option<ConvolveMatrix> {
        if !(1..=Self::MAX_SIDE).contains(&kernel_width) || !(1..=Self::MAX_SIDE).contains(&kernel_height) {
            return None;
        }
        if elements.len() != (kernel_width * kernel_height) as usize {
            return None;
        }
        let mut kernel = [0.0; Self::MAX_ELEMENTS];
        kernel[..elements.len()].copy_from_slice(elements);
        Some(ConvolveMatrix { kernel_width, kernel_height, kernel, ..ConvolveMatrix::default() })
    }

    /// The number of elements of the kernel that are used.
    pub fn element_count(&self) -> usize {
        (self.kernel_width.clamp(1, Self::MAX_SIDE) * self.kernel_height.clamp(1, Self::MAX_SIDE)) as usize
    }

    /// The elements of the kernel that are used, row by row.
    pub fn elements(&self) -> &[f32] {
        &self.kernel[..self.element_count()]
    }

    /// The target element of the kernel, `(w / 2, h / 2)`: the column and
    /// row of the kernel over the output pixel.
    pub fn target(&self) -> (u32, u32) {
        (self.kernel_width.clamp(1, Self::MAX_SIDE) / 2, self.kernel_height.clamp(1, Self::MAX_SIDE) / 2)
    }

    /// The factor of the weighted sum: one over the divisor (one for a
    /// divisor of 0 or not a number).
    pub fn gain(&self) -> f32 {
        if self.divisor == 0.0 || !self.divisor.is_finite() {
            1.0
        } else {
            1.0 / self.divisor
        }
    }

    /// The weight of the source pixel `column - cx` pixels to the right and
    /// `row - cy` pixels below the output pixel: the element turned by half
    /// a turn (see the type).
    pub fn weight(&self, row: usize, column: usize) -> f32 {
        let count = self.element_count();
        let width = self.kernel_width.clamp(1, Self::MAX_SIDE) as usize;
        self.kernel[count - 1 - (row * width + column)]
    }

    /// The elements in the order of a correlation (Skia's matrix
    /// convolution): the weight of row `i`, column `j` at `i * w + j`.
    pub fn correlation_kernel(&self) -> Vec<f32> {
        self.elements().iter().rev().copied().collect()
    }

    /// Convolves premultiplied RGBA pixels of eight bits a channel (`width`
    /// by `height`, row by row), in place: the CPU reference of the drawing
    /// contexts. The edges of the pixels are the edges of the layer.
    pub fn apply_to_premultiplied_rgba8(&self, pixels: &mut [u8], width: usize, height: usize) {
        let length = width.saturating_mul(height).saturating_mul(4);
        if width == 0 || height == 0 || pixels.len() < length {
            return;
        }
        let source = pixels[..length].to_vec();
        let (kernel_width, kernel_height) =
            (self.kernel_width.clamp(1, Self::MAX_SIDE) as usize, self.kernel_height.clamp(1, Self::MAX_SIDE) as usize);
        let (target_x, target_y) = self.target();
        let (target_x, target_y) = (target_x as isize, target_y as isize);
        let gain = self.gain();
        let fetch = |x: isize, y: isize| -> [f32; 4] {
            let (x, y) = if self.clamp_edges {
                (x.clamp(0, width as isize - 1), y.clamp(0, height as isize - 1))
            } else if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
                return [0.0; 4];
            } else {
                (x, y)
            };
            let at = (y as usize * width + x as usize) * 4;
            [0, 1, 2, 3].map(|channel| f32::from(source[at + channel]) / 255.0)
        };
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0f32; 4];
                for row in 0..kernel_height {
                    for column in 0..kernel_width {
                        let weight = self.weight(row, column);
                        let mut color = fetch(x as isize + column as isize - target_x, y as isize + row as isize - target_y);
                        if self.preserve_alpha {
                            let alpha = color[3];
                            for channel in &mut color[..3] {
                                *channel = if alpha > 0.0 { *channel / alpha } else { 0.0 };
                            }
                        }
                        for (total, value) in sum.iter_mut().zip(color) {
                            *total += value * weight;
                        }
                    }
                }
                let mut out = sum.map(|value| value * gain + self.bias);
                let at = (y * width + x) * 4;
                if self.preserve_alpha {
                    let alpha = f32::from(source[at + 3]) / 255.0;
                    out = [out[0] * alpha, out[1] * alpha, out[2] * alpha, alpha];
                } else {
                    out[3] = out[3].clamp(0.0, 1.0);
                }
                let alpha = if out[3].is_nan() { 0.0 } else { out[3] };
                for channel in 0..3 {
                    out[channel] = out[channel].max(0.0).min(alpha);
                }
                out[3] = alpha;
                for channel in 0..4 {
                    pixels[at + channel] = (out[channel] * 255.0).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opaque(width: usize, height: usize, value: impl Fn(usize, usize) -> u8) -> Vec<u8> {
        let mut pixels = Vec::with_capacity(width * height * 4);
        for y in 0..height {
            for x in 0..width {
                let v = value(x, y);
                pixels.extend_from_slice(&[v, v, v, 255]);
            }
        }
        pixels
    }

    #[test]
    fn the_identity_kernel_changes_nothing() {
        let mut pixels = vec![10, 20, 30, 40, 200, 100, 0, 255, 0, 0, 0, 0, 60, 60, 60, 128];
        let before = pixels.clone();
        ConvolveMatrix::default().apply_to_premultiplied_rgba8(&mut pixels, 2, 2);
        assert_eq!(before, pixels);
        let mut preserved = before.clone();
        ConvolveMatrix { preserve_alpha: true, ..ConvolveMatrix::default() }.apply_to_premultiplied_rgba8(&mut preserved, 2, 2);
        assert!(preserved.iter().zip(&before).all(|(a, b)| a.abs_diff(*b) <= 1), "{preserved:?}");
    }

    #[test]
    fn the_kernel_is_turned_by_half_a_turn() {
        // A kernel that takes the pixel to the left of the output pixel: in
        // SVG's order, the element right of the target.
        let shift = ConvolveMatrix::new(3, 1, &[0.0, 0.0, 1.0]).unwrap();
        assert_eq!(1.0, shift.weight(0, 0));
        assert_eq!(vec![1.0, 0.0, 0.0], shift.correlation_kernel());
        let mut pixels = opaque(4, 1, |x, _| (x * 50) as u8);
        shift.apply_to_premultiplied_rgba8(&mut pixels, 4, 1);
        // The first pixel reads beyond the edge: transparent.
        assert_eq!(vec![0, 0, 0, 0, 0, 0, 0, 255, 50, 50, 50, 255, 100, 100, 100, 255], pixels);
        let mut clamped = opaque(4, 1, |x, _| (x * 50) as u8);
        ConvolveMatrix { clamp_edges: true, ..shift }.apply_to_premultiplied_rgba8(&mut clamped, 4, 1);
        assert_eq!([0, 0, 0, 255], clamped[..4]);
    }

    #[test]
    fn the_divisor_and_the_bias() {
        let mut pixels = opaque(3, 3, |_, _| 100);
        let mut box_blur = ConvolveMatrix::new(3, 3, &[1.0; 9]).unwrap();
        box_blur.divisor = 9.0;
        box_blur.clamp_edges = true;
        box_blur.bias = 0.2;
        box_blur.preserve_alpha = true;
        box_blur.apply_to_premultiplied_rgba8(&mut pixels, 3, 3);
        // 100 / 255 + 0.2, opaque: 151.
        let expected = 151;
        assert!(pixels.chunks_exact(4).all(|pixel| pixel == [expected, expected, expected, 255]), "{pixels:?}");
        // A divisor of 0 is 1.
        assert_eq!(1.0, ConvolveMatrix { divisor: 0.0, ..ConvolveMatrix::default() }.gain());
    }

    #[test]
    fn the_colour_is_clamped_to_the_alpha() {
        let sharpen = ConvolveMatrix::new(3, 3, &[0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0]).unwrap();
        let mut pixels = opaque(3, 3, |x, y| if (x, y) == (1, 1) { 200 } else { 50 });
        sharpen.apply_to_premultiplied_rgba8(&mut pixels, 3, 3);
        let centre = &pixels[16..20];
        assert_eq!([255, 255, 255, 255], centre);
        assert!(pixels.chunks_exact(4).all(|pixel| pixel[..3].iter().all(|&c| c <= pixel[3])));
    }

    #[test]
    fn a_kernel_of_another_size_is_refused() {
        assert!(ConvolveMatrix::new(6, 1, &[0.0; 6]).is_none());
        assert!(ConvolveMatrix::new(0, 1, &[]).is_none());
        assert!(ConvolveMatrix::new(3, 3, &[0.0; 8]).is_none());
        assert_eq!(25, ConvolveMatrix::new(5, 5, &[0.0; 25]).unwrap().element_count());
    }
}
