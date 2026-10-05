use crate::animation::animators::PointAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use crate::Point;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with [`Point`] types.
    PointTransition: Point
);

impl Transition<Point> for PointTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: Point,
        new_value: Point,
    ) -> Rc<dyn IObservable<Point>> {
        AnimatorDrivenTransition::transition::<PointAnimator>(this.easing(), progress, old_value, new_value)
    }
}
