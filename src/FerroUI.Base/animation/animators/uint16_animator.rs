use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles `u16` properties.
#[derive(Default)]
pub struct UInt16Animator {
    base: AnimatorBase,
}

impl UInt16Animator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &u16, new_value: &u16) -> u16 {
        const MAX_VAL: f64 = u16::MAX as f64;

        let norm_ov = *old_value as f64 / MAX_VAL;
        let norm_nv = *new_value as f64 / MAX_VAL;
        let delta_v = norm_nv - norm_ov;
        (MAX_VAL * ((delta_v * progress) + norm_ov)).round_ties_even() as u16
    }
}

impl Animator for UInt16Animator {
    type Value = u16;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &u16, new_value: &u16) -> u16 {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
