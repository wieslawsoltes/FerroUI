use crate::Rect;
use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles [`Rect`] properties.
#[derive(Default)]
pub struct RectAnimator {
    base: AnimatorBase,
}

impl RectAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &Rect, new_value: &Rect) -> Rect {
        Rect::new(
            ((new_value.x - old_value.x) * progress) + old_value.x,
            ((new_value.y - old_value.y) * progress) + old_value.y,
            ((new_value.width - old_value.width) * progress) + old_value.width,
            ((new_value.height - old_value.height) * progress) + old_value.height,
        )
    }
}

impl Animator for RectAnimator {
    type Value = Rect;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &Rect, new_value: &Rect) -> Rect {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
