//! The Gaussian of a blurred shadow, shared by the routes that blur it in passes of their own on
//! the device: the compute mode (`gpu/shadow_blur.rs`) and WebGL2 (`web_gl/shadow_programs.rs`).
//! A large deviation is blurred at a reduced scale, so a pass never reads more than 25 texels for
//! a pixel.

/// The largest deviation a pass blurs by, in the texels of the reduced scale.
pub(crate) const LARGEST_PASS_DEVIATION: f64 = 4.0;
/// How far a pass reads, in deviations.
pub(crate) const REACH: f64 = 3.0;
/// The weights of a pass: the centre and each distance up to the reach of the largest deviation.
pub(crate) const WEIGHTS: usize = 16;

/// The weights of a Gaussian of `deviation` texels, which add up to one over both sides, and how
/// many texels it reads at each side.
pub(crate) fn gaussian_weights(deviation: f64) -> ([f32; WEIGHTS], usize) {
    let mut weights = [0.0f32; WEIGHTS];
    if !(deviation > 0.0) {
        weights[0] = 1.0;
        return (weights, 0);
    }
    let radius = ((deviation * REACH).ceil() as usize).min(WEIGHTS - 1);
    let raw: Vec<f64> = (0..=radius).map(|i| (-0.5 * (i as f64 / deviation).powi(2)).exp()).collect();
    let total = raw[0] + 2.0 * raw[1..].iter().sum::<f64>();
    for (weight, value) in weights.iter_mut().zip(&raw) {
        *weight = (value / total) as f32;
    }
    (weights, radius)
}

/// How many device pixels a texel of the blur covers for a deviation.
pub(crate) fn reduction_for(deviation: f64) -> f64 {
    (deviation / LARGEST_PASS_DEVIATION).ceil().max(1.0)
}

/// The longest side of a picture of a shadow that is kept.
const LARGEST_KEPT_SIDE: u32 = 1024;
/// The smallest deviation on the target a kept picture stands for: below it the shadow still has
/// an edge that a filter would soften.
const SMALLEST_KEPT_DEVIATION: f64 = 1.0;

/// How the picture of a shadow is drawn so that it serves every transform: in the space of the
/// shape, at the power of two that makes its deviation 2 to 4 texels. A Gaussian that wide has no
/// detail a bilinear filter loses, so the same picture is drawn at any scale or rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShadowRaster {
    /// From the space of the shape to the texels of the picture, its margin included.
    pub(crate) to_texels: kurbo::Affine,
    /// The deviation in texels.
    pub(crate) deviation: f64,
    /// The size of the picture in texels.
    pub(crate) size: (u32, u32),
}

/// The picture of a shape with these `bounds` blurred by `std_deviation` (in its space) that is
/// drawn under `transform`, or `None` when one picture cannot stand for it: a blur too narrow on
/// the target, or a shape too large for its deviation.
pub(crate) fn shadow_raster(
    bounds: kurbo::Rect,
    std_deviation: f64,
    transform: kurbo::Affine,
) -> Option<ShadowRaster> {
    let [a, b, c, d, _, _] = transform.as_coeffs();
    let on_target = std_deviation * a.hypot(b).min(c.hypot(d));
    if !(on_target >= SMALLEST_KEPT_DEVIATION) || !std_deviation.is_finite() {
        return None;
    }
    let scale = 2.0_f64.powf((2.0 / std_deviation).log2().ceil());
    let deviation = std_deviation * scale;
    let margin = (deviation * REACH).ceil();
    let side = |length: f64| (length * scale).ceil() + 2.0 * margin + 1.0;
    let (width, height) = (side(bounds.width()), side(bounds.height()));
    let largest = f64::from(LARGEST_KEPT_SIDE);
    if !(width <= largest && height <= largest) {
        return None;
    }
    let to_texels = kurbo::Affine::translate((margin, margin))
        * kurbo::Affine::scale(scale)
        * kurbo::Affine::translate((-bounds.x0, -bounds.y0));
    Some(ShadowRaster { to_texels, deviation, size: (width as u32, height as u32) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Affine, Rect};

    #[test]
    fn a_kept_picture_has_a_deviation_of_two_to_four_texels() {
        let bounds = Rect::new(10.0, 20.0, 70.0, 40.0);
        for deviation in [1.0, 1.9, 2.0, 3.96, 4.0, 6.27, 12.05, 23.6, 90.0] {
            let raster = shadow_raster(bounds, deviation, Affine::IDENTITY).unwrap();
            assert!((2.0..4.0).contains(&raster.deviation), "{deviation}: {}", raster.deviation);
            assert!(raster.deviation <= LARGEST_PASS_DEVIATION);
        }
    }

    #[test]
    fn the_picture_does_not_depend_on_the_scale_it_is_drawn_at() {
        let bounds = Rect::new(10.0, 20.0, 70.0, 40.0);
        let at = |scale: f64| shadow_raster(bounds, 6.27, Affine::scale(scale)).unwrap();
        assert_eq!(at(0.85), at(1.0));
        assert_eq!(at(1.0), at(2.5));
        let turned = shadow_raster(bounds, 6.27, Affine::rotate(0.3)).unwrap();
        assert_eq!(turned, at(1.0));
    }

    #[test]
    fn no_picture_for_a_narrow_blur_or_a_shape_too_large_for_it() {
        let bounds = Rect::new(0.0, 0.0, 60.0, 20.0);
        assert!(shadow_raster(bounds, 2.0, Affine::scale(0.25)).is_none());
        let large = Rect::new(0.0, 0.0, 4000.0, 300.0);
        assert!(shadow_raster(large, 1.5, Affine::IDENTITY).is_none());
        assert!(shadow_raster(large, 40.0, Affine::IDENTITY).is_some());
    }

    #[test]
    fn the_weights_add_up_to_one_and_stay_inside_the_uniform_block() {
        for deviation in [0.5, 1.0, 2.5, LARGEST_PASS_DEVIATION] {
            let (weights, radius) = gaussian_weights(deviation);
            assert!(radius < WEIGHTS, "{deviation}");
            let total = weights[0] + 2.0 * weights[1..=radius].iter().sum::<f32>();
            assert!((total - 1.0).abs() < 1e-5, "{deviation}: {total}");
        }
        assert_eq!(gaussian_weights(0.0).1, 0);
    }

    #[test]
    fn a_large_deviation_is_reduced_to_what_a_pass_blurs() {
        for deviation in [0.5, 4.0, 4.1, 6.27, 12.0, 23.6, 80.0] {
            assert!(deviation / reduction_for(deviation) <= LARGEST_PASS_DEVIATION, "{deviation}");
        }
        assert_eq!(reduction_for(3.0), 1.0);
        assert_eq!(reduction_for(6.0), 2.0);
    }
}
