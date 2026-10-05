use crate::Size;
use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles [`Size`] properties.
#[derive(Default)]
pub struct SizeAnimator {
    base: AnimatorBase,
}

impl SizeAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &Size, new_value: &Size) -> Size {
        Size::new(
            ((new_value.width - old_value.width) * progress) + old_value.width,
            ((new_value.height - old_value.height) * progress) + old_value.height,
        )
    }
}

impl Animator for SizeAnimator {
    type Value = Size;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &Size, new_value: &Size) -> Size {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
