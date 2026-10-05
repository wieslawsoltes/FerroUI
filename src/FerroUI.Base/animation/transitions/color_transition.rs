use crate::animation::animators::ColorAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use crate::media::Color;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with [`Color`] types.
    ColorTransition: Color
);

impl Transition<Color> for ColorTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: Color,
        new_value: Color,
    ) -> Rc<dyn IObservable<Color>> {
        AnimatorDrivenTransition::transition::<ColorAnimator>(this.easing(), progress, old_value, new_value)
    }
}
