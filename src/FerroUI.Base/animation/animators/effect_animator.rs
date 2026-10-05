use crate::animation::animators::{apply_with_visual, Animator, AnimatorBase, ColorAnimator, DoubleAnimator};
use crate::animation::{Animatable, Animation, AnimatorFactory, AnimatorKeyFrame, IAnimator, IClock};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::effects::{
    IBlurEffect, IDropShadowEffect, IEffect, ImmutableBlurEffect, ImmutableDropShadowDirectionEffect,
    ImmutableDropShadowEffect,
};
use crate::reactive::{IDisposable, IObservable};
use crate::Visual;
use std::any::TypeId;
use std::rc::Rc;

type EffectValue = Option<Rc<dyn IEffect>>;

fn effect_of(frame: &AnimatorKeyFrame) -> EffectValue {
    frame.with_value(|value| value.and_then(|value| value.downcast_ref::<EffectValue>()).and_then(|effect| effect.clone()))
}

/// Animator that handles all animations on properties with an effect as
/// their type: redirects them to the animator for the kind of effects in
/// the key frames.
#[derive(Default)]
pub struct EffectAnimator {
    base: AnimatorBase,
}

impl EffectAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    fn try_create_animator<TAnimator: Animator + Default>(
        &self,
        is_kind: impl Fn(&dyn IEffect) -> bool,
    ) -> Option<Rc<dyn IAnimator>> {
        let mut created_animator: Option<Rc<TAnimator>> = None;
        let factory: AnimatorFactory = Rc::new(|| Rc::new(TAnimator::default()));

        for key_frame in self.base.to_vec() {
            if !effect_of(&key_frame).is_some_and(|effect| is_kind(&*effect)) {
                return None;
            }

            let animator = created_animator.get_or_insert_with(|| {
                let animator = Rc::new(TAnimator::default());
                animator.base().set_property(self.base.property());
                animator
            });
            let copy = AnimatorKeyFrame::new(
                Some(TypeId::of::<TAnimator>()),
                Some(factory.clone()),
                key_frame.cue(),
                key_frame.key_spline().cloned(),
            );
            copy.set_value(key_frame.value());
            copy.set_fill_before(key_frame.fill_before());
            copy.set_fill_after(key_frame.fill_after());
            IAnimator::add(&**animator, copy);
        }

        created_animator.map(|animator| animator as Rc<dyn IAnimator>)
    }

    /// The effect animator is part of the default animator registrations;
    /// there is nothing to do to make it available.
    pub fn ensure_registered() {}
}

impl Animator for EffectAnimator {
    type Value = EffectValue;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn apply(
        this: &Rc<Self>,
        animation: &Animation,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        match_: Rc<dyn IObservable<bool>>,
        on_complete: Option<Rc<dyn Fn()>>,
        should_pause_on_invisible: bool,
    ) -> Option<Rc<dyn IDisposable>> {
        let animator = this
            .try_create_animator::<BlurEffectAnimator>(|effect| effect.as_blur_effect().is_some())
            .or_else(|| this.try_create_animator::<DropShadowEffectAnimator>(|effect| effect.as_drop_shadow_effect().is_some()));
        if let Some(animator) = animator {
            return animator.apply(animation, control, clock, match_, on_complete, should_pause_on_invisible);
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::ANIMATIONS) {
            logger.log(None, "The animation's keyframe value types set is not supported.");
        }

        let visual_target = control.downcast_ref::<Visual>().map(Visual::to_ref);
        Some(apply_with_visual(
            this,
            animation,
            control,
            clock,
            match_,
            on_complete,
            should_pause_on_invisible,
            visual_target,
        ))
    }

    /// Fallback implementation of effect animation.
    fn interpolate(&self, progress: f64, old_value: &EffectValue, new_value: &EffectValue) -> EffectValue {
        if progress >= 0.5 {
            new_value.clone()
        } else {
            old_value.clone()
        }
    }
}

/// Animator that interpolates blur effects. Values that are not blur
/// effects are not interpolated.
#[derive(Default)]
pub struct BlurEffectAnimator {
    base: AnimatorBase,
}

impl BlurEffectAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two blur effects using the specified progress.
    pub fn interpolate_blur(progress: f64, old_value: &dyn IBlurEffect, new_value: &dyn IBlurEffect) -> ImmutableBlurEffect {
        ImmutableBlurEffect::new(DoubleAnimator::interpolate_core(progress, &old_value.radius(), &new_value.radius()))
    }

    /// Interpolates between two effects using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &EffectValue, new_value: &EffectValue) -> EffectValue {
        let old = old_value.as_ref().and_then(|effect| effect.as_blur_effect());
        let n = new_value.as_ref().and_then(|effect| effect.as_blur_effect());
        match (old, n) {
            (Some(old), Some(n)) => Some(Rc::new(Self::interpolate_blur(progress, old, n))),
            _ => {
                if progress >= 0.5 {
                    new_value.clone()
                } else {
                    old_value.clone()
                }
            }
        }
    }
}

impl Animator for BlurEffectAnimator {
    type Value = EffectValue;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn interpolate(&self, progress: f64, old_value: &EffectValue, new_value: &EffectValue) -> EffectValue {
        Self::interpolate_core(progress, old_value, new_value)
    }
}

/// Animator that interpolates drop shadow effects. Values that are not drop
/// shadow effects are not interpolated.
#[derive(Default)]
pub struct DropShadowEffectAnimator {
    base: AnimatorBase,
}

impl DropShadowEffectAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two drop shadow effects using the specified
    /// progress.
    pub fn interpolate_drop_shadow(
        progress: f64,
        old_value: &dyn IDropShadowEffect,
        new_value: &dyn IDropShadowEffect,
    ) -> Rc<dyn IEffect> {
        let blur = DoubleAnimator::interpolate_core(progress, &old_value.blur_radius(), &new_value.blur_radius());
        let color = ColorAnimator::interpolate_core(progress, &old_value.color(), &new_value.color());
        let opacity = DoubleAnimator::interpolate_core(progress, &old_value.opacity(), &new_value.opacity());

        if let (Some(old_direction), Some(new_direction)) =
            (old_value.as_direction_drop_shadow_effect(), new_value.as_direction_drop_shadow_effect())
        {
            return Rc::new(ImmutableDropShadowDirectionEffect::new(
                DoubleAnimator::interpolate_core(progress, &old_direction.direction(), &new_direction.direction()),
                DoubleAnimator::interpolate_core(progress, &old_direction.shadow_depth(), &new_direction.shadow_depth()),
                blur,
                color,
                opacity,
            ));
        }

        Rc::new(ImmutableDropShadowEffect::new(
            DoubleAnimator::interpolate_core(progress, &old_value.offset_x(), &new_value.offset_x()),
            DoubleAnimator::interpolate_core(progress, &old_value.offset_y(), &new_value.offset_y()),
            blur,
            color,
            opacity,
        ))
    }

    /// Interpolates between two effects using the specified progress.
    pub fn interpolate_core(progress: f64, old_value: &EffectValue, new_value: &EffectValue) -> EffectValue {
        let old = old_value.as_ref().and_then(|effect| effect.as_drop_shadow_effect());
        let n = new_value.as_ref().and_then(|effect| effect.as_drop_shadow_effect());
        match (old, n) {
            (Some(old), Some(n)) => Some(Self::interpolate_drop_shadow(progress, old, n)),
            _ => {
                if progress >= 0.5 {
                    new_value.clone()
                } else {
                    old_value.clone()
                }
            }
        }
    }
}

impl Animator for DropShadowEffectAnimator {
    type Value = EffectValue;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn interpolate(&self, progress: f64, old_value: &EffectValue, new_value: &EffectValue) -> EffectValue {
        Self::interpolate_core(progress, old_value, new_value)
    }
}
