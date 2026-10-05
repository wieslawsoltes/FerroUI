use crate::animation::animators::BoxShadowsAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use crate::media::BoxShadows;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with [`BoxShadows`] types.
    BoxShadowsTransition: BoxShadows
);

impl Transition<BoxShadows> for BoxShadowsTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: BoxShadows,
        new_value: BoxShadows,
    ) -> Rc<dyn IObservable<BoxShadows>> {
        AnimatorDrivenTransition::transition::<BoxShadowsAnimator>(this.easing(), progress, old_value, new_value)
    }
}
