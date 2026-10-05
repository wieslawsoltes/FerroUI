use crate::animation::animators::Animator;
use crate::animation::easings::Easing;
use crate::animation::AnimatorTransitionObservable;
use crate::reactive::IObservable;
use std::rc::Rc;

/// Creates the observable of a transition whose values are interpolated by
/// the animator `A`.
pub struct AnimatorDrivenTransition;

impl AnimatorDrivenTransition {
    pub fn transition<A: Animator + Default>(
        easing: Easing,
        progress: Rc<dyn IObservable<f64>>,
        old_value: A::Value,
        new_value: A::Value,
    ) -> Rc<dyn IObservable<A::Value>> {
        let animator = A::default();
        AnimatorTransitionObservable::create(
            move |progress, old_value, new_value| animator.interpolate(progress, old_value, new_value),
            progress,
            easing,
            old_value,
            new_value,
        )
    }
}
