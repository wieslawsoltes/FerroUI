use crate::animation::animators::{Animator, AnimatorBase};
use crate::media::FontVariationSettings;

/// Animator that handles `FontVariations` properties ([`FontVariationSettings`], upstream
/// dfee7f224b): per-axis linear interpolation when both key frames set the same axes,
/// discrete otherwise ([`FontVariationSettings::interpolate`]).
#[derive(Default)]
pub struct FontVariationSettingsAnimator {
    base: AnimatorBase,
}

impl FontVariationSettingsAnimator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Animator for FontVariationSettingsAnimator {
    type Value = Option<FontVariationSettings>;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn interpolate(
        &self,
        progress: f64,
        old_value: &Option<FontVariationSettings>,
        new_value: &Option<FontVariationSettings>,
    ) -> Option<FontVariationSettings> {
        FontVariationSettings::interpolate(old_value.as_ref(), new_value.as_ref(), progress)
    }
}
