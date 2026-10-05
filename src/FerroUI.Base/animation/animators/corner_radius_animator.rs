use crate::CornerRadius;
use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles [`CornerRadius`] properties.
#[derive(Default)]
pub struct CornerRadiusAnimator {
    base: AnimatorBase,
}

impl CornerRadiusAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &CornerRadius, new_value: &CornerRadius) -> CornerRadius {
        let delta_tl = new_value.top_left - old_value.top_left;
        let delta_tr = new_value.top_right - old_value.top_right;
        let delta_br = new_value.bottom_right - old_value.bottom_right;
        let delta_bl = new_value.bottom_left - old_value.bottom_left;

        let n_tl = progress * delta_tl + old_value.top_left;
        let n_tr = progress * delta_tr + old_value.top_right;
        let n_br = progress * delta_br + old_value.bottom_right;
        let n_bl = progress * delta_bl + old_value.bottom_left;

        CornerRadius::new(n_tl, n_tr, n_br, n_bl)
    }
}

impl Animator for CornerRadiusAnimator {
    type Value = CornerRadius;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &CornerRadius, new_value: &CornerRadius) -> CornerRadius {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
