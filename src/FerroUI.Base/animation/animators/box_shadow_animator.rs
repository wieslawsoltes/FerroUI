use crate::animation::animators::{BoolAnimator, ColorAnimator, DoubleAnimator};
use crate::media::BoxShadow;
use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that interpolates [`BoxShadow`] properties.
#[derive(Default)]
pub struct BoxShadowAnimator {
    base: AnimatorBase,
}

impl BoxShadowAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &BoxShadow, new_value: &BoxShadow) -> BoxShadow {
        BoxShadow {
            offset_x: DoubleAnimator::interpolate_core(progress, &old_value.offset_x, &new_value.offset_x),
            offset_y: DoubleAnimator::interpolate_core(progress, &old_value.offset_y, &new_value.offset_y),
            blur: DoubleAnimator::interpolate_core(progress, &old_value.blur, &new_value.blur),
            spread: DoubleAnimator::interpolate_core(progress, &old_value.spread, &new_value.spread),
            color: ColorAnimator::interpolate_core(progress, &old_value.color, &new_value.color),
            is_inset: BoolAnimator::interpolate_core(progress, &old_value.is_inset, &new_value.is_inset),
        }
    }
}

impl Animator for BoxShadowAnimator {
    type Value = BoxShadow;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &BoxShadow, new_value: &BoxShadow) -> BoxShadow {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
