use crate::animation::animators::{BlurEffectAnimator, CustomEffectAnimator, DropShadowEffectAnimator};
use crate::animation::{AnimatorTransitionObservable, Transition, TransitionBase, TransitionObservableBase};
use crate::ferro_transition_class;
use crate::media::effects::{EffectExtensions, IEffect, ImmutableBlurEffect, ImmutableDropShadowDirectionEffect};
use crate::media::Color;
use crate::reactive::IObservable;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles effect properties.
    ///
    /// Blur effects transition to and from blur effects, drop shadows to
    /// and from drop shadows, custom effects to and from custom effects of
    /// the same type; a missing effect stands for an effect of the other
    /// value's kind that has no visible result (for a custom effect, its
    /// neutral value, when the type has one).
    EffectTransition: Option<Rc<dyn IEffect>>
);

type EffectValue = Option<Rc<dyn IEffect>>;

/// Pairs the values up for an animator of one kind of effect: `None` when
/// the values are not both of that kind (or absent).
fn try_with_kind(
    old_value: &EffectValue,
    new_value: &EffectValue,
    is_kind: impl Fn(&dyn IEffect) -> bool,
    default_value: impl FnOnce() -> Rc<dyn IEffect>,
) -> Option<(EffectValue, EffectValue)> {
    let of_kind = |value: &EffectValue| value.as_ref().is_some_and(|effect| is_kind(&**effect));

    if of_kind(old_value) {
        if of_kind(new_value) {
            Some((old_value.clone(), new_value.clone()))
        } else if new_value.is_none() {
            Some((old_value.clone(), Some(default_value())))
        } else {
            None
        }
    } else if of_kind(new_value) {
        Some((Some(default_value()), new_value.clone()))
    } else {
        None
    }
}

/// Pairs the values up for the custom effect animator (#455): two custom
/// effects of the same type, or one and the neutral value of its type in
/// place of a missing effect. `None` otherwise.
fn try_with_custom(old_value: &EffectValue, new_value: &EffectValue) -> Option<(EffectValue, EffectValue)> {
    let old = old_value.as_ref().and_then(|effect| EffectExtensions::to_custom_effect(&**effect));
    let new = new_value.as_ref().and_then(|effect| EffectExtensions::to_custom_effect(&**effect));
    let value = |effect: crate::media::effects::ImmutableCustomEffect| -> EffectValue { Some(Rc::new(effect)) };
    match (old, new) {
        (Some(old), Some(new)) if old.value_type_id() == new.value_type_id() => {
            Some((old_value.clone(), new_value.clone()))
        }
        (Some(old), None) if new_value.is_none() => old.neutral().map(|neutral| (old_value.clone(), value(neutral))),
        (None, Some(new)) if old_value.is_none() => new.neutral().map(|neutral| (value(neutral), new_value.clone())),
        _ => None,
    }
}

impl Transition<EffectValue> for EffectTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: EffectValue,
        new_value: EffectValue,
    ) -> Rc<dyn IObservable<EffectValue>> {
        if old_value.is_some() || new_value.is_some() {
            if let Some((old_i, new_i)) = try_with_kind(
                &old_value,
                &new_value,
                |effect| effect.as_blur_effect().is_some(),
                || Rc::new(ImmutableBlurEffect::new(0.0)),
            ) {
                return AnimatorTransitionObservable::create(
                    BlurEffectAnimator::interpolate_core,
                    progress,
                    this.easing(),
                    old_i,
                    new_i,
                );
            }

            if let Some((old_i, new_i)) = try_with_kind(
                &old_value,
                &new_value,
                |effect| effect.as_drop_shadow_effect().is_some(),
                || Rc::new(ImmutableDropShadowDirectionEffect::new(0.0, 0.0, 0.0, Color::default(), 0.0)),
            ) {
                return AnimatorTransitionObservable::create(
                    DropShadowEffectAnimator::interpolate_core,
                    progress,
                    this.easing(),
                    old_i,
                    new_i,
                );
            }

            if let Some((old_i, new_i)) = try_with_custom(&old_value, &new_value) {
                return AnimatorTransitionObservable::create(
                    CustomEffectAnimator::interpolate_core,
                    progress,
                    this.easing(),
                    old_i,
                    new_i,
                );
            }
        }

        TransitionObservableBase::new(progress, this.easing(), move |progress| {
            if progress >= 0.5 {
                new_value.clone()
            } else {
                old_value.clone()
            }
        })
    }
}
