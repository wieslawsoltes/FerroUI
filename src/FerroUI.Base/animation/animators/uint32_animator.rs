use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles `u32` properties.
#[derive(Default)]
pub struct UInt32Animator {
    base: AnimatorBase,
}

impl UInt32Animator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &u32, new_value: &u32) -> u32 {
        const MAX_VAL: f64 = u32::MAX as f64;

        let norm_ov = *old_value as f64 / MAX_VAL;
        let norm_nv = *new_value as f64 / MAX_VAL;
        let delta_v = norm_nv - norm_ov;
        (MAX_VAL * ((delta_v * progress) + norm_ov)).round_ties_even() as u32
    }
}

impl Animator for UInt32Animator {
    type Value = u32;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &u32, new_value: &u32) -> u32 {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
