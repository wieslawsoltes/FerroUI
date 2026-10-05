// Original color interpolation code was written by Romain Guy and Francois Blavoet
// and adopted from LottieSharp Project (https://github.com/ascora/LottieSharp).

use crate::media::Color;
use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that interpolates [`Color`] properties in linear RGB space.
#[derive(Default)]
pub struct ColorAnimator {
    base: AnimatorBase,
}

impl ColorAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &Color, new_value: &Color) -> Color {
        // normalize sRGB values.
        let old_a = old_value.a as f64 / 255.0;
        let mut old_r = old_value.r as f64 / 255.0;
        let mut old_g = old_value.g as f64 / 255.0;
        let mut old_b = old_value.b as f64 / 255.0;

        let new_a = new_value.a as f64 / 255.0;
        let mut new_r = new_value.r as f64 / 255.0;
        let mut new_g = new_value.g as f64 / 255.0;
        let mut new_b = new_value.b as f64 / 255.0;

        // convert from sRGB to linear
        old_r = eocf_srgb(old_r);
        old_g = eocf_srgb(old_g);
        old_b = eocf_srgb(old_b);

        new_r = eocf_srgb(new_r);
        new_g = eocf_srgb(new_g);
        new_b = eocf_srgb(new_b);

        // compute the interpolated color in linear space
        let mut a = old_a + progress * (new_a - old_a);
        let mut r = old_r + progress * (new_r - old_r);
        let mut g = old_g + progress * (new_g - old_g);
        let mut b = old_b + progress * (new_b - old_b);

        // convert back to sRGB in the [0..255] range
        a *= 255.0;
        r = oecf_srgb(r) * 255.0;
        g = oecf_srgb(g) * 255.0;
        b = oecf_srgb(b) * 255.0;

        Color::new(
            a.round_ties_even() as u8,
            r.round_ties_even() as u8,
            g.round_ties_even() as u8,
            b.round_ties_even() as u8,
        )
    }
}

impl Animator for ColorAnimator {
    type Value = Color;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &Color, new_value: &Color) -> Color {
        Self::interpolate_core(progress, old_value, new_value)
    }
}

fn oecf_srgb(linear: f64) -> f64 {
    // IEC 61966-2-1:1999
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        linear.powf(1.0 / 2.4) * 1.055 - 0.055
    }
}

fn eocf_srgb(srgb: f64) -> f64 {
    // IEC 61966-2-1:1999
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}
