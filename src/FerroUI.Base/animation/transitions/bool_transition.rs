use crate::animation::animators::BoolAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with `bool` types.
    BoolTransition: bool
);

impl Transition<bool> for BoolTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: bool,
        new_value: bool,
    ) -> Rc<dyn IObservable<bool>> {
        AnimatorDrivenTransition::transition::<BoolAnimator>(this.easing(), progress, old_value, new_value)
    }
}
