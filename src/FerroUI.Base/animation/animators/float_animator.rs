use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles `f32` properties.
#[derive(Default)]
pub struct FloatAnimator {
    base: AnimatorBase,
}

impl FloatAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    #[inline]
    pub fn interpolate_core(progress: f64, old_value: &f32, new_value: &f32) -> f32 {
        (((*new_value - *old_value) as f64 * progress) + *old_value as f64) as f32
    }
}

impl Animator for FloatAnimator {
    type Value = f32;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &f32, new_value: &f32) -> f32 {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
