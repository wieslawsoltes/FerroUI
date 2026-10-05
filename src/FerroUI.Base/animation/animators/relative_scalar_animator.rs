use crate::animation::animators::DoubleAnimator;
use crate::RelativeScalar;
use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles [`RelativeScalar`] properties.
#[derive(Default)]
pub struct RelativeScalarAnimator {
    base: AnimatorBase,
}

impl RelativeScalarAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &RelativeScalar, new_value: &RelativeScalar) -> RelativeScalar {
        if old_value.unit != new_value.unit {
            return if progress >= 0.5 { *new_value } else { *old_value };
        }

        RelativeScalar::new(DoubleAnimator::interpolate_core(progress, &old_value.scalar, &new_value.scalar), old_value.unit)
    }
}

impl Animator for RelativeScalarAnimator {
    type Value = RelativeScalar;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &RelativeScalar, new_value: &RelativeScalar) -> RelativeScalar {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
