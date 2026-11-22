use crate::animation::animators::BoxShadowAnimator;
use crate::animation::Animatable;
use crate::media::BoxShadows;
use crate::animation::animators::{Animator, AnimatorBase};
use crate::StyledProperty;

/// Animator that interpolates [`BoxShadows`] properties.
#[derive(Default)]
pub struct BoxShadowsAnimator {
    base: AnimatorBase,
}

impl BoxShadowsAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the underlying list, including the default for an animation-only property.
    pub fn base_value(target: &Animatable, property: &'static StyledProperty<BoxShadows>) -> BoxShadows {
        target.get_base_value(property).unwrap_or_else(|| {
            if property.inherits() {
                target.values().inherited_or_default(target, property)
            } else {
                property.get_default_value_for(target)
            }
        })
    }

    /// Interpolates between two values using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &BoxShadows, new_value: &BoxShadows) -> BoxShadows {
        let cnt = if progress >= 1.0 { new_value.count() } else { old_value.count() };
        if cnt == 0 {
            return BoxShadows::default();
        }

        let first = if old_value.count() > 0 && new_value.count() > 0 {
            BoxShadowAnimator::interpolate_core(progress, &old_value.get(0), &new_value.get(0))
        } else if old_value.count() > 0 {
            old_value.get(0)
        } else {
            new_value.get(0)
        };

        if cnt == 1 {
            return BoxShadows::new(first);
        }

        let mut rest = Vec::with_capacity(cnt - 1);
        for c in 0..cnt - 1 {
            let idx = c + 1;
            rest.push(if old_value.count() > idx && new_value.count() > idx {
                BoxShadowAnimator::interpolate_core(progress, &old_value.get(idx), &new_value.get(idx))
            } else if old_value.count() > idx {
                old_value.get(idx)
            } else {
                new_value.get(idx)
            });
        }

        BoxShadows::with_rest(first, &rest)
    }
}

impl Animator for BoxShadowsAnimator {
    type Value = BoxShadows;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn neutral_value(&self, target: &Animatable) -> Self::Value {
        Self::base_value(target, self.base.styled_property())
    }

    #[inline]
    fn interpolate(&self, progress: f64, old_value: &BoxShadows, new_value: &BoxShadows) -> BoxShadows {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
