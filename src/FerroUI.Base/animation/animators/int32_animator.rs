use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles `i32` properties.
#[derive(Default)]
pub struct Int32Animator {
    base: AnimatorBase,
}

impl Int32Animator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &i32, new_value: &i32) -> i32 {
        const MAX_VAL: f64 = i32::MAX as f64;

        let norm_ov = *old_value as f64 / MAX_VAL;
        let norm_nv = *new_value as f64 / MAX_VAL;
        let delta_v = norm_nv - norm_ov;
        (MAX_VAL * ((delta_v * progress) + norm_ov)).round_ties_even() as i32
    }
}

impl Animator for Int32Animator {
    type Value = i32;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &i32, new_value: &i32) -> i32 {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
