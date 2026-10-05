use crate::animation::animators::FloatAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with `f32` types.
    FloatTransition: f32
);

impl Transition<f32> for FloatTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: f32,
        new_value: f32,
    ) -> Rc<dyn IObservable<f32>> {
        AnimatorDrivenTransition::transition::<FloatAnimator>(this.easing(), progress, old_value, new_value)
    }
}
