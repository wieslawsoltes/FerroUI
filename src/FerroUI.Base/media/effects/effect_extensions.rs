use crate::media::effects::{IEffect, IImmutableEffect};
use crate::{Rect, Thickness, Vector};
use std::rc::Rc;

/// Extension methods for effects.
pub struct EffectExtensions;

impl EffectExtensions {
    fn adjust_padding_radius(radius: f64) -> f64 {
        if radius <= 0.0 {
            return 0.0;
        }
        radius.ceil() + 1.0
    }

    /// How far the output of the effect extends beyond the bounds of the
    /// content it is applied to.
    ///
    /// Panics for an effect of an unknown kind.
    pub fn get_effect_output_padding(effect: Option<&dyn IEffect>) -> Thickness {
        let Some(effect) = effect else { return Thickness::default() };
        if let Some(blur) = effect.as_blur_effect() {
            return Thickness::uniform(Self::adjust_padding_radius(blur.radius()));
        }
        if let Some(drop_shadow_effect) = effect.as_drop_shadow_effect() {
            let radius = Self::adjust_padding_radius(drop_shadow_effect.blur_radius());
            let rc = Rect::new(-radius, -radius, radius * 2.0, radius * 2.0);
            let rc = rc.translate(Vector::new(drop_shadow_effect.offset_x(), drop_shadow_effect.offset_y()));
            return Thickness::new(
                f64::max(0.0, 0.0 - rc.x),
                f64::max(0.0, 0.0 - rc.y),
                f64::max(0.0, rc.right()),
                f64::max(0.0, rc.bottom()),
            );
        }

        panic!("Unknown effect type");
    }

    /// Converts an effect to an immutable effect: the result of
    /// [`IMutableEffect::to_immutable`](crate::media::effects::IMutableEffect::to_immutable)
    /// if the effect is mutable, otherwise the effect itself.
    pub fn to_immutable(effect: &Rc<dyn IEffect>) -> Rc<dyn IImmutableEffect> {
        if let Some(mutable) = effect.as_mutable_effect() {
            return mutable.to_immutable();
        }
        effect.clone().into_immutable_effect().expect("an effect is either mutable or immutable")
    }

    /// Whether an immutable effect and another effect are both absent or
    /// structurally equal.
    pub fn effect_equals(immutable: Option<&dyn IImmutableEffect>, right: Option<&dyn IEffect>) -> bool {
        match (immutable, right) {
            (None, None) => true,
            (Some(immutable), Some(right)) => immutable.equals(Some(right)),
            _ => false,
        }
    }
}
