use crate::Vector;
use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles [`Vector`] properties.
#[derive(Default)]
pub struct VectorAnimator {
    base: AnimatorBase,
}

impl VectorAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    #[inline]
    pub fn interpolate_core(progress: f64, old_value: &Vector, new_value: &Vector) -> Vector {
        ((*new_value - *old_value) * progress) + *old_value
    }
}

impl Animator for VectorAnimator {
    type Value = Vector;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &Vector, new_value: &Vector) -> Vector {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
