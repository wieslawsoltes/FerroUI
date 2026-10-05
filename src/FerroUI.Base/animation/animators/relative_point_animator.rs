use crate::animation::animators::PointAnimator;
use crate::RelativePoint;
use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles [`RelativePoint`] properties.
#[derive(Default)]
pub struct RelativePointAnimator {
    base: AnimatorBase,
}

impl RelativePointAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &RelativePoint, new_value: &RelativePoint) -> RelativePoint {
        if old_value.unit != new_value.unit {
            return if progress >= 0.5 { *new_value } else { *old_value };
        }

        RelativePoint::from_point(PointAnimator::interpolate_core(progress, &old_value.point, &new_value.point), old_value.unit)
    }
}

impl Animator for RelativePointAnimator {
    type Value = RelativePoint;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &RelativePoint, new_value: &RelativePoint) -> RelativePoint {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
