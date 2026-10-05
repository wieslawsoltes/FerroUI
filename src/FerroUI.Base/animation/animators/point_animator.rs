use crate::Point;
use crate::animation::animators::{Animator, AnimatorBase};

/// Animator that handles [`Point`] properties.
#[derive(Default)]
pub struct PointAnimator {
    base: AnimatorBase,
}

impl PointAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two values using the specified progress.
    #[inline]
    pub fn interpolate_core(progress: f64, old_value: &Point, new_value: &Point) -> Point {
        ((*new_value - *old_value) * progress) + *old_value
    }
}

impl Animator for PointAnimator {
    type Value = Point;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &Point, new_value: &Point) -> Point {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
