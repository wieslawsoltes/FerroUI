use crate::media::effects::{Effect, IEffect, IImmutableEffect, ImmutableCustomEffect};
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
    /// content it is applied to. For a custom effect, the padding of its
    /// snapshot ([`ImmutableCustomEffect::padding`]).
    ///
    /// Panics for an effect of an unknown kind.
    pub fn get_effect_output_padding(effect: Option<&dyn IEffect>) -> Thickness {
        let Some(effect) = effect else { return Thickness::default() };
        if effect.as_any().is::<super::ImmutableLuminanceToAlphaEffect>() {
            return Thickness::default();
        }
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

        if let Some(custom) = effect.as_custom_effect() {
            return custom.padding();
        }
        if let Some(custom) = Self::custom_class_snapshot(effect) {
            return custom.padding();
        }

        panic!("Unknown effect type");
    }

    /// The snapshot of a custom effect (#455, not from upstream): the effect
    /// itself when it is an [`ImmutableCustomEffect`], the snapshot of its
    /// values when it is a custom effect class
    /// ([`Effect::construct_custom`]); `None` for the other effects.
    pub fn to_custom_effect(effect: &dyn IEffect) -> Option<ImmutableCustomEffect> {
        match effect.as_custom_effect() {
            Some(custom) => Some(custom.clone()),
            None => Self::custom_class_snapshot(effect),
        }
    }

    fn custom_class_snapshot(effect: &dyn IEffect) -> Option<ImmutableCustomEffect> {
        effect.as_object()?.downcast_ref::<Effect>()?.custom_snapshot()
    }

    /// Converts an effect to an immutable effect: the result of
    /// [`IMutableEffect::to_immutable`](crate::media::effects::IMutableEffect::to_immutable)
    /// if the effect is mutable, otherwise the effect itself.
    pub fn to_immutable(effect: &Rc<dyn IEffect>) -> std::sync::Arc<dyn IImmutableEffect> {
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
