use crate::animation::animators::DoubleAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with `f64` types.
    DoubleTransition: f64
);

impl Transition<f64> for DoubleTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: f64,
        new_value: f64,
    ) -> Rc<dyn IObservable<f64>> {
        AnimatorDrivenTransition::transition::<DoubleAnimator>(this.easing(), progress, old_value, new_value)
    }
}
