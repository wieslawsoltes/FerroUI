use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles `u8` properties.
#[derive(Default)]
pub struct ByteAnimator {
    base: AnimatorBase,
}

impl ByteAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &u8, new_value: &u8) -> u8 {
        const MAX_VAL: f64 = u8::MAX as f64;

        let norm_ov = *old_value as f64 / MAX_VAL;
        let norm_nv = *new_value as f64 / MAX_VAL;
        let delta_v = norm_nv - norm_ov;
        (MAX_VAL * ((delta_v * progress) + norm_ov)).round_ties_even() as u8
    }
}

impl Animator for ByteAnimator {
    type Value = u8;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &u8, new_value: &u8) -> u8 {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
