use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles `bool` properties: the value switches when the progress reaches the end.
#[derive(Default)]
pub struct BoolAnimator {
    base: AnimatorBase,
}

impl BoolAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    #[inline]
    pub fn interpolate_core(progress: f64, old_value: &bool, new_value: &bool) -> bool {
        if progress >= 1.0 {
            return *new_value;
        }
        *old_value
    }
}

impl Animator for BoolAnimator {
    type Value = bool;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &bool, new_value: &bool) -> bool {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
