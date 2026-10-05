use crate::animation::animators::VectorAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use crate::Vector;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with [`Vector`] types.
    VectorTransition: Vector
);

impl Transition<Vector> for VectorTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: Vector,
        new_value: Vector,
    ) -> Rc<dyn IObservable<Vector>> {
        AnimatorDrivenTransition::transition::<VectorAnimator>(this.easing(), progress, old_value, new_value)
    }
}
