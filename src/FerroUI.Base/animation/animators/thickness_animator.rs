use crate::Thickness;
use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles [`Thickness`] properties.
#[derive(Default)]
pub struct ThicknessAnimator {
    base: AnimatorBase,
}

impl ThicknessAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    #[inline]
    pub fn interpolate_core(progress: f64, old_value: &Thickness, new_value: &Thickness) -> Thickness {
        ((*new_value - *old_value) * progress) + *old_value
    }
}

impl Animator for ThicknessAnimator {
    type Value = Thickness;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &Thickness, new_value: &Thickness) -> Thickness {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
