use crate::animation::animators::CornerRadiusAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use crate::CornerRadius;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with [`CornerRadius`] types.
    CornerRadiusTransition: CornerRadius
);

impl Transition<CornerRadius> for CornerRadiusTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: CornerRadius,
        new_value: CornerRadius,
    ) -> Rc<dyn IObservable<CornerRadius>> {
        AnimatorDrivenTransition::transition::<CornerRadiusAnimator>(this.easing(), progress, old_value, new_value)
    }
}
