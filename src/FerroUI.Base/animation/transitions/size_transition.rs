use crate::animation::animators::SizeAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use crate::Size;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with [`Size`] types.
    SizeTransition: Size
);

impl Transition<Size> for SizeTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: Size,
        new_value: Size,
    ) -> Rc<dyn IObservable<Size>> {
        AnimatorDrivenTransition::transition::<SizeAnimator>(this.easing(), progress, old_value, new_value)
    }
}
