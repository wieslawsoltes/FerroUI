use crate::animation::animators::{Animator, AnimatorBase, ColorAnimator, DoubleAnimator};
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::IBrush;
use std::rc::Rc;

/// Animator that handles brush properties whose key frames are all solid
/// color brushes: interpolates the color and the opacity.
///
/// The values are brush handles, as the animated property holds them; a
/// value that is not a solid color brush is not interpolated.
#[derive(Default)]
pub struct ISolidColorBrushAnimator {
    base: AnimatorBase,
}

impl ISolidColorBrushAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two brushes using the specified progress.
    pub fn interpolate_core(
        progress: f64,
        old_value: &Option<Rc<dyn IBrush>>,
        new_value: &Option<Rc<dyn IBrush>>,
    ) -> Option<Rc<dyn IBrush>> {
        let discrete = || if progress >= 0.5 { new_value.clone() } else { old_value.clone() };
        let (Some(old_brush), Some(new_brush)) = (old_value, new_value) else {
            return discrete();
        };
        let (Some(old_solid), Some(new_solid)) = (old_brush.as_solid_color_brush(), new_brush.as_solid_color_brush())
        else {
            return discrete();
        };

        Some(Rc::new(ImmutableSolidColorBrush::with_opacity(
            ColorAnimator::interpolate_core(progress, &old_solid.color(), &new_solid.color()),
            DoubleAnimator::interpolate_core(progress, &old_brush.opacity(), &new_brush.opacity()),
        )))
    }
}

impl Animator for ISolidColorBrushAnimator {
    type Value = Option<Rc<dyn IBrush>>;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn interpolate(&self, progress: f64, old_value: &Self::Value, new_value: &Self::Value) -> Self::Value {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
