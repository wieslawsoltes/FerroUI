use crate::animation::animators::Int32Animator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with `i32` types.
    IntegerTransition: i32
);

impl Transition<i32> for IntegerTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: i32,
        new_value: i32,
    ) -> Rc<dyn IObservable<i32>> {
        AnimatorDrivenTransition::transition::<Int32Animator>(this.easing(), progress, old_value, new_value)
    }
}
