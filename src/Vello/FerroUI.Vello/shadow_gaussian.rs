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

#[cfg(test)]
mod tests {
    use super::*;

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
