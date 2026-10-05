use crate::animation::animators::RelativePointAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use crate::RelativePoint;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with [`RelativePoint`] types.
    RelativePointTransition: RelativePoint
);

impl Transition<RelativePoint> for RelativePointTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: RelativePoint,
        new_value: RelativePoint,
    ) -> Rc<dyn IObservable<RelativePoint>> {
        AnimatorDrivenTransition::transition::<RelativePointAnimator>(this.easing(), progress, old_value, new_value)
    }
}
