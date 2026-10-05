use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles `f64` properties.
#[derive(Default)]
pub struct DoubleAnimator {
    base: AnimatorBase,
}

impl DoubleAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    #[inline]
    pub fn interpolate_core(progress: f64, old_value: &f64, new_value: &f64) -> f64 {
        ((*new_value - *old_value) * progress) + *old_value
    }
}

impl Animator for DoubleAnimator {
    type Value = f64;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &f64, new_value: &f64) -> f64 {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
