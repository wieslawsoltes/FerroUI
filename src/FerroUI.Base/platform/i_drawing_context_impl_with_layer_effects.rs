use super::IDrawingContextImpl;
use crate::media::imaging::BitmapBlendingMode;
use crate::media::{ColorMatrix, ColorMatrixAlphaMode};
use crate::{PixelSize, Rect};
use std::sync::Arc;

/// A drawing context implementation that can compose a layer onto what is
/// under it with a blend mode or through a colour matrix: what the effects
/// of WinUI's composition (`BlendEffect`, `ColorMatrixEffect`,
/// `SaturationEffect`, the composite modes) are drawn with.
///
/// Every push has its pop; the pushes nest with the other pushes of the
/// context (clips, layers, opacities, effects).
pub trait IDrawingContextImplWithLayerEffects: IDrawingContextImpl {
    /// Pushes a layer: what is drawn until the matching
    /// [`pop_blend_layer`](Self::pop_blend_layer) is composed onto what is
    /// under it with `mode` (the Porter-Duff modes compose, the others mix
    /// and compose source-over). `bounds` is the area drawn into, in the
    /// coordinates of the context.
    fn push_blend_layer(&mut self, bounds: Rect, mode: BitmapBlendingMode);

    /// Pops the latest blend layer.
    fn pop_blend_layer(&mut self);

    /// Pushes a layer: what is drawn until the matching
    /// [`pop_color_matrix_layer`](Self::pop_color_matrix_layer) is
    /// transformed by `matrix` (applied to premultiplied or straight colours,
    /// as `alpha_mode` says) and composed source-over onto what is under it.
    /// `bounds` is the area drawn into, in the coordinates of the context;
    /// nothing outside it is kept.
    fn push_color_matrix_layer(&mut self, bounds: Rect, matrix: &ColorMatrix, alpha_mode: ColorMatrixAlphaMode);

    /// Pops the latest colour matrix layer.
    fn pop_color_matrix_layer(&mut self);

    /// Pushes a layer: each colour channel of what is drawn until the
    /// matching [`pop_multiply_layer`](Self::pop_multiply_layer) is
    /// multiplied by the factor `factors` has at that point (stretched over
    /// `bounds`, sampled bilinearly), the alpha is kept, and the result is
    /// composed source-over onto what is under it. A factor can exceed 1
    /// (up to [`MultiplyLayerFactors::scale`]); the result is clamped to the
    /// alpha. `bounds` is the area drawn into, in the coordinates of the
    /// context; nothing outside it is kept. What the lights of WinUI's
    /// composition are drawn with (winui-materials.md).
    fn push_multiply_layer(&mut self, bounds: Rect, factors: &MultiplyLayerFactors);

    /// Pops the latest multiply layer.
    fn pop_multiply_layer(&mut self);
}

/// The per-channel factors of a multiply layer: an opaque RGBA image whose
/// channels, divided by 255 and times [`scale`](Self::scale), are the
/// factors of the red, green and blue channels.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiplyLayerFactors {
    /// The size of the image in samples.
    pub size: PixelSize,
    /// The samples, row by row from the top, 4 bytes each (the fourth 255).
    pub rgba: Arc<Vec<u8>>,
    /// What a sample of 255 stands for (1 or more).
    pub scale: f32,
}

impl MultiplyLayerFactors {
    /// The factors of the red, green and blue channels at `(u, v)` of the
    /// unit square of the image (0 to 1 across it), sampled bilinearly
    /// between the centres of the samples, as the backends sample a bitmap.
    pub fn factor_at(&self, u: f64, v: f64) -> [f32; 3] {
        let (width, height) = (self.size.width.max(1) as usize, self.size.height.max(1) as usize);
        if self.rgba.len() < width * height * 4 {
            return [1.0; 3];
        }
        let x = (u * width as f64 - 0.5).clamp(0.0, (width - 1) as f64);
        let y = (v * height as f64 - 0.5).clamp(0.0, (height - 1) as f64);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
        let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
        let sample = |x: usize, y: usize, channel: usize| f32::from(self.rgba[(y * width + x) * 4 + channel]) / 255.0;
        let mut factors = [0.0; 3];
        for (channel, factor) in factors.iter_mut().enumerate() {
            let top = sample(x0, y0, channel) * (1.0 - fx) + sample(x1, y0, channel) * fx;
            let bottom = sample(x0, y1, channel) * (1.0 - fx) + sample(x1, y1, channel) * fx;
            *factor = (top * (1.0 - fy) + bottom * fy) * self.scale;
        }
        factors
    }

    /// Multiplies the premultiplied RGBA pixels `rgba` (row by row, `width`
    /// pixels a row) by the factors; `to_unit` maps the centre of a pixel to
    /// the unit square of the factors (`None` outside them: the pixel is
    /// cleared, as nothing outside the bounds of the layer is kept).
    pub fn apply_to_premultiplied_rgba8(&self, rgba: &mut [u8], width: usize, to_unit: impl Fn(f64, f64) -> Option<(f64, f64)>) {
        if width == 0 {
            return;
        }
        for (index, pixel) in rgba.chunks_exact_mut(4).enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            let (x, y) = ((index % width) as f64 + 0.5, (index / width) as f64 + 0.5);
            let Some((u, v)) = to_unit(x, y) else {
                pixel.copy_from_slice(&[0, 0, 0, 0]);
                continue;
            };
            let factors = self.factor_at(u, v);
            let alpha = f32::from(pixel[3]);
            for channel in 0..3 {
                pixel[channel] = (f32::from(pixel[channel]) * factors[channel]).round().clamp(0.0, alpha) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn factors(rgba: Vec<u8>, width: i32, height: i32, scale: f32) -> MultiplyLayerFactors {
        MultiplyLayerFactors { size: PixelSize::new(width, height), rgba: Arc::new(rgba), scale }
    }

    #[test]
    fn the_factors_are_sampled_between_the_centres_of_the_samples() {
        let map = factors(vec![0, 0, 0, 255, 255, 255, 255, 255], 2, 1, 1.0);
        assert_eq!([0.0; 3], map.factor_at(0.0, 0.5));
        assert_eq!([1.0; 3], map.factor_at(1.0, 0.5));
        let middle = map.factor_at(0.5, 0.5);
        assert!((middle[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_scale_brightens_and_the_result_is_clamped_to_the_alpha() {
        let map = factors(vec![255, 128, 0, 255], 1, 1, 2.0);
        let mut pixels = vec![100, 100, 100, 200, 50, 50, 50, 50];
        map.apply_to_premultiplied_rgba8(&mut pixels, 2, |x, y| Some((x / 2.0, y)));
        assert_eq!(vec![200, 100, 0, 200, 50, 50, 0, 50], pixels);
    }

    #[test]
    fn pixels_outside_the_factors_are_cleared() {
        let map = factors(vec![255, 255, 255, 255], 1, 1, 1.0);
        let mut pixels = vec![10, 10, 10, 10];
        map.apply_to_premultiplied_rgba8(&mut pixels, 1, |_, _| None);
        assert_eq!(vec![0, 0, 0, 0], pixels);
    }
}
