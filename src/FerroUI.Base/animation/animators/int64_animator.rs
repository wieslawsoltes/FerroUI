use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles `i64` properties.
#[derive(Default)]
pub struct Int64Animator {
    base: AnimatorBase,
}

impl Int64Animator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &i64, new_value: &i64) -> i64 {
        const MAX_VAL: f64 = i64::MAX as f64;

        let norm_ov = *old_value as f64 / MAX_VAL;
        let norm_nv = *new_value as f64 / MAX_VAL;
        let delta_v = norm_nv - norm_ov;
        (MAX_VAL * ((delta_v * progress) + norm_ov)).round_ties_even() as i64
    }
}

impl Animator for Int64Animator {
    type Value = i64;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &i64, new_value: &i64) -> i64 {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
